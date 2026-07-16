//! Spawn and manage the local Cursor OpenAI-compatible bridge process.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use super::credentials::CURSOR_DEFAULT_MODEL;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeInfo {
    pub host: String,
    pub port: u16,
    pub base_url: String,
}

pub struct BridgeHandle {
    child: Child,
    pub info: BridgeInfo,
}

static BRIDGE: OnceLock<Mutex<Option<BridgeHandle>>> = OnceLock::new();

fn bridge_slot() -> &'static Mutex<Option<BridgeHandle>> {
    BRIDGE.get_or_init(|| Mutex::new(None))
}

fn runtime_file(grok_home: &Path) -> PathBuf {
    grok_home.join("cursor-bridge.json")
}

fn bridge_script_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("GROK_CURSOR_BRIDGE_DIR") {
        let path = PathBuf::from(p);
        if path.join("package.json").is_file() {
            return Some(path);
        }
    }
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/codegen/xai-grok-shell -> repo root
    for _ in 0..3 {
        dir = dir.parent()?.to_path_buf();
    }
    let candidate = dir.join("scripts/cursor-bridge");
    if candidate.join("package.json").is_file() {
        Some(candidate)
    } else {
        None
    }
}

fn which_js_runtime() -> Option<(PathBuf, Vec<&'static str>)> {
    for (bin, args) in [
        ("bun", vec!["run", "src/index.ts"]),
        ("npx", vec!["tsx", "src/index.ts"]),
        ("node", vec!["--import", "tsx", "src/index.ts"]),
    ] {
        if let Ok(output) = Command::new("which").arg(bin).output() {
            if output.status.success() {
                let p = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if !p.is_empty() {
                    return Some((PathBuf::from(p), args));
                }
            }
        }
    }
    None
}

/// Ensure the Cursor bridge is running; returns connection info.
pub fn ensure_bridge(api_key: &str, cwd: &Path, grok_home: &Path) -> anyhow::Result<BridgeInfo> {
    {
        let guard = bridge_slot()
            .lock()
            .map_err(|_| anyhow::anyhow!("cursor bridge lock poisoned"))?;
        if let Some(handle) = guard.as_ref() {
            if health_ok(&handle.info) {
                return Ok(handle.info.clone());
            }
        }
    }
    stop_bridge();

    let script_dir = bridge_script_dir().ok_or_else(|| {
        anyhow::anyhow!(
            "Cursor bridge not found. Expected scripts/cursor-bridge (or set GROK_CURSOR_BRIDGE_DIR)."
        )
    })?;
    let (runtime, args) = which_js_runtime().ok_or_else(|| {
        anyhow::anyhow!(
            "Need `bun` or `node`+`tsx` to run the Cursor bridge. Install Bun or run `bun install` in scripts/cursor-bridge."
        )
    })?;

    let mut cmd = Command::new(&runtime);
    cmd.args(&args)
        .current_dir(&script_dir)
        .env("CURSOR_API_KEY", api_key)
        .env("CURSOR_CWD", cwd)
        .env("DEFAULT_MODEL", CURSOR_DEFAULT_MODEL)
        .env("HOST", "127.0.0.1")
        .env("PORT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn Cursor bridge: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("cursor bridge missing stdout"))?;
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut info: Option<BridgeInfo> = None;
    while Instant::now() < deadline {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("CURSOR_BRIDGE_READY ") {
                    let (host, port_s) = rest
                        .rsplit_once(':')
                        .ok_or_else(|| anyhow::anyhow!("bad bridge ready line: {trimmed}"))?;
                    let port: u16 = port_s.parse()?;
                    let base_url = format!("http://{host}:{port}/v1");
                    info = Some(BridgeInfo {
                        host: host.to_owned(),
                        port,
                        base_url,
                    });
                    break;
                }
            }
            Err(e) => return Err(anyhow::anyhow!("reading bridge stdout: {e}")),
        }
    }
    let info = info.ok_or_else(|| {
        let _ = child.kill();
        anyhow::anyhow!("Cursor bridge did not become ready in time")
    })?;

    if !health_ok(&info) {
        let _ = child.kill();
        anyhow::bail!("Cursor bridge health check failed at {}", info.base_url);
    }

    std::fs::write(
        runtime_file(grok_home),
        serde_json::to_vec_pretty(&info)?,
    )?;

    let mut guard = bridge_slot()
        .lock()
        .map_err(|_| anyhow::anyhow!("cursor bridge lock poisoned"))?;
    *guard = Some(BridgeHandle { child, info: info.clone() });
    Ok(info)
}

pub fn stop_bridge() {
    if let Ok(mut guard) = bridge_slot().lock() {
        if let Some(mut handle) = guard.take() {
            let _ = handle.child.kill();
            let _ = handle.child.wait();
        }
    }
}

fn health_ok(info: &BridgeInfo) -> bool {
    let url = format!("http://{}:{}/health", info.host, info.port);
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    client
        .get(&url)
        .send()
        .ok()
        .and_then(|r| r.error_for_status().ok())
        .is_some()
}

/// Load previously written bridge info if the process is still healthy.
pub fn load_bridge_info(grok_home: &Path) -> Option<BridgeInfo> {
    let raw = std::fs::read_to_string(runtime_file(grok_home)).ok()?;
    let info: BridgeInfo = serde_json::from_str(&raw).ok()?;
    if health_ok(&info) {
        Some(info)
    } else {
        None
    }
}
