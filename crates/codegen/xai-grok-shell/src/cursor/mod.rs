//! Cursor auth + local OpenAI-compatible bridge for Cursor-billed inference.

mod bridge;
mod credentials;

pub use bridge::{BridgeHandle, BridgeInfo, ensure_bridge, load_bridge_info, stop_bridge};
pub use credentials::{
    CURSOR_API_KEY_ENV, CURSOR_AUTH_SCOPE, CURSOR_CATALOG_MODEL_ID, CURSOR_DEFAULT_MODEL,
    CURSOR_MODEL_DISPLAY_NAME, clear_cursor_auth, ensure_cursor_provider_config,
    is_cursor_provider, login_cursor, read_cursor_api_key, resolve_cursor_api_key,
    store_cursor_api_key,
};

use crate::agent::config::{ModelEntry, ModelInfo, ResolvedCredentials};
use crate::sampling::ApiBackend;
use xai_chat_state::AuthType;

/// Apply Cursor bridge routing onto resolved credentials.
pub fn apply_cursor_routing(
    credentials: &mut ResolvedCredentials,
    bridge: &BridgeInfo,
    api_key: &str,
) {
    credentials.api_key = Some(api_key.to_owned());
    credentials.base_url = bridge.base_url.clone();
    credentials.auth_type = AuthType::ApiKey;
}

/// Synthetic catalog entry used when Cursor provider is active.
///
/// Catalog key / picker id is [`CURSOR_CATALOG_MODEL_ID`] (`grok-4.5-cursor`);
/// the API routing slug remains [`CURSOR_DEFAULT_MODEL`] (`grok-4.5`).
pub fn cursor_default_model_entry(bridge: &BridgeInfo) -> ModelEntry {
    let mut info = ModelInfo::fallback(CURSOR_DEFAULT_MODEL);
    info.id = Some(CURSOR_CATALOG_MODEL_ID.to_owned());
    info.base_url = bridge.base_url.clone();
    info.name = Some(CURSOR_MODEL_DISPLAY_NAME.to_owned());
    info.description = Some("Grok 4.5 billed through your Cursor plan".to_owned());
    info.api_backend = ApiBackend::ChatCompletions;
    info.supported_in_api = true;
    ModelEntry {
        info,
        api_key: None,
        env_key: None,
        api_base_url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_model_id_is_cursor_grok() {
        assert_eq!(CURSOR_DEFAULT_MODEL, "grok-4.5");
        assert_eq!(CURSOR_CATALOG_MODEL_ID, "grok-4.5-cursor");
        assert_eq!(CURSOR_MODEL_DISPLAY_NAME, "Grok 4.5 (Cursor)");
    }

    #[test]
    fn cursor_catalog_entry_keeps_api_slug_and_display_label() {
        let bridge = BridgeInfo {
            host: "127.0.0.1".into(),
            port: 9999,
            base_url: "http://127.0.0.1:9999/v1".into(),
        };
        let entry = cursor_default_model_entry(&bridge);
        assert_eq!(entry.info.model, CURSOR_DEFAULT_MODEL);
        assert_eq!(entry.info.id.as_deref(), Some(CURSOR_CATALOG_MODEL_ID));
        assert_eq!(
            entry.info.name.as_deref(),
            Some(CURSOR_MODEL_DISPLAY_NAME)
        );
        assert_eq!(entry.info.base_url, bridge.base_url);
    }

    #[test]
    fn apply_cursor_routing_sets_localhost_byok() {
        let bridge = BridgeInfo {
            host: "127.0.0.1".into(),
            port: 9999,
            base_url: "http://127.0.0.1:9999/v1".into(),
        };
        let mut creds = ResolvedCredentials {
            api_key: Some("old".into()),
            base_url: "https://cli-chat-proxy.grok.com/v1".into(),
            auth_type: AuthType::SessionToken,
            auth_scheme: Default::default(),
        };
        apply_cursor_routing(&mut creds, &bridge, "crsr_test");
        assert_eq!(creds.api_key.as_deref(), Some("crsr_test"));
        assert_eq!(creds.base_url, "http://127.0.0.1:9999/v1");
        assert_eq!(creds.auth_type, AuthType::ApiKey);
    }
}
