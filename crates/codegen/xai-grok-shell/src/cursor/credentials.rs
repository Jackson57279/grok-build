//! Resolve and persist Cursor API credentials.

use crate::auth::{
    clear_cursor_api_key, read_cursor_api_key_from_store, store_cursor_api_key as persist_cursor_key,
};

pub use crate::auth::store_cursor_api_key;
use crate::util::grok_home::grok_home;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CURSOR_API_KEY_ENV: &str = "CURSOR_API_KEY";
pub const CURSOR_AUTH_SCOPE: &str = crate::auth::CURSOR_API_KEY_SCOPE;
/// API routing slug sent to the Cursor bridge (`@cursor/sdk` model id).
pub const CURSOR_DEFAULT_MODEL: &str = "grok-4.5";
/// Catalog / picker id — distinct from the API slug so the UI can label it.
pub const CURSOR_CATALOG_MODEL_ID: &str = "grok-4.5-cursor";
pub const CURSOR_MODEL_DISPLAY_NAME: &str = "Grok 4.5 (Cursor)";
pub const AUTH_PROVIDER_ENV: &str = "GROK_AUTH_PROVIDER";

fn cursor_auth_json_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cursor")
        .join("auth.json")
}

/// True when config/env selects Cursor as the auth+inference provider.
pub fn is_cursor_provider() -> bool {
    if let Ok(v) = std::env::var(AUTH_PROVIDER_ENV) {
        let v = v.trim().to_ascii_lowercase();
        if v == "cursor" {
            return true;
        }
        if v == "xai" || v == "grok" {
            return false;
        }
    }
    if let Ok(raw) = crate::config::load_effective_config_disk_only() {
        if let Some(provider) = raw
            .get("auth")
            .and_then(|a| a.get("preferred_provider"))
            .and_then(|v| v.as_str())
        {
            return provider.eq_ignore_ascii_case("cursor");
        }
    }
    false
}

pub fn read_cursor_api_key(grok_home_path: &Path) -> Option<String> {
    if let Ok(key) = std::env::var(CURSOR_API_KEY_ENV) {
        let key = key.trim().to_owned();
        if !key.is_empty() {
            return Some(key);
        }
    }
    if let Some(key) = read_cursor_api_key_from_store(grok_home_path) {
        if !key.is_empty() {
            return Some(key);
        }
    }
    read_cursor_cli_auth_json()
}

fn read_cursor_cli_auth_json() -> Option<String> {
    let path = cursor_auth_json_path();
    let raw = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value
        .get("apiKey")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            value
                .get("accessToken")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        })
}

pub fn clear_cursor_auth(grok_home_path: &Path) -> std::io::Result<()> {
    clear_cursor_api_key(grok_home_path)
}

/// Ensure `~/.grok/config.toml` pins Cursor as preferred provider.
pub fn ensure_cursor_provider_config(config_path: &Path) -> std::io::Result<()> {
    let existing = std::fs::read_to_string(config_path).unwrap_or_default();
    if existing.contains("preferred_provider")
        && existing
            .lines()
            .any(|l| l.contains("preferred_provider") && l.contains("cursor"))
    {
        return Ok(());
    }
    let mut out = existing;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.contains("[auth]") {
        out.push_str("\n[auth]\npreferred_provider = \"cursor\"\n");
    } else if !out.contains("preferred_provider") {
        // Insert under existing [auth] table — simple append is fine; TOML allows duplicate tables merged by many loaders, but our loader may not. Prefer rewrite.
        out.push_str("preferred_provider = \"cursor\"\n");
    } else {
        // Replace existing preferred_provider value.
        let mut rewritten = String::new();
        for line in out.lines() {
            if line.trim_start().starts_with("preferred_provider") {
                rewritten.push_str("preferred_provider = \"cursor\"\n");
            } else {
                rewritten.push_str(line);
                rewritten.push('\n');
            }
        }
        out = rewritten;
    }
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(config_path, out)
}

/// Resolve a Cursor API key, optionally running `agent login` when missing.
pub fn resolve_cursor_api_key(interactive: bool) -> anyhow::Result<String> {
    let home = grok_home();
    if let Some(key) = read_cursor_api_key(&home) {
        let _ = persist_cursor_key(&home, &key);
        return Ok(key);
    }
    if !interactive {
        anyhow::bail!(
            "No Cursor credentials found. Set {CURSOR_API_KEY_ENV}, run `agent login`, or `grok login --cursor`."
        );
    }
    login_cursor()
}

/// Run Cursor CLI login, then read credentials.
pub fn login_cursor() -> anyhow::Result<String> {
    let agent = which_agent_bin().ok_or_else(|| {
        anyhow::anyhow!(
            "Cursor CLI (`agent`) not found on PATH. Install from https://cursor.com/docs/cli or set {CURSOR_API_KEY_ENV}."
        )
    })?;
    eprintln!("Opening Cursor sign-in…");
    let status = Command::new(&agent).arg("login").status()?;
    if !status.success() {
        anyhow::bail!("`agent login` failed with status {status}");
    }
    let key = read_cursor_cli_auth_json().ok_or_else(|| {
        anyhow::anyhow!(
            "Cursor login finished but no apiKey/accessToken found in ~/.config/cursor/auth.json"
        )
    })?;
    persist_cursor_key(&grok_home(), &key)?;
    Ok(key)
}

fn which_agent_bin() -> Option<PathBuf> {
    for candidate in ["agent", "cursor-agent"] {
        if let Ok(output) = Command::new("which").arg(candidate).output() {
            if output.status.success() {
                let p = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                if !p.is_empty() {
                    return Some(PathBuf::from(p));
                }
            }
        }
    }
    let home = dirs::home_dir()?;
    for rel in [".local/bin/cursor-agent", ".local/bin/agent"] {
        let path = home.join(rel);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn store_and_read_cursor_scope() {
        let dir = TempDir::new().unwrap();
        persist_cursor_key(dir.path(), "crsr_test_key").unwrap();
        assert_eq!(
            read_cursor_api_key(dir.path()).as_deref(),
            Some("crsr_test_key")
        );
        clear_cursor_auth(dir.path()).unwrap();
    }
}
