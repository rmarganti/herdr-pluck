use crate::model::PaneId;
use serde_json::Value;
use std::env;
use std::path::PathBuf;

const HERDR_BIN_PATH: &str = "HERDR_BIN_PATH";

/// Returns the Herdr binary path injected by Herdr, falling back to PATH lookup.
pub fn herdr_bin_from_env() -> String {
    env::var(HERDR_BIN_PATH).unwrap_or_else(|_| "herdr".to_string())
}

/// Runtime Herdr/plugin context used to discover binaries and the source pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HerdrContext {
    pub herdr_bin: String,
    pub context_json: Option<String>,
    pub pane_id: Option<String>,
    pub tab_id: Option<String>,
    pub socket_path: Option<PathBuf>,
}

impl HerdrContext {
    pub fn from_env() -> Self {
        Self {
            herdr_bin: herdr_bin_from_env(),
            context_json: env::var("HERDR_PLUGIN_CONTEXT_JSON").ok(),
            pane_id: env::var("HERDR_PANE_ID")
                .or_else(|_| env::var("HERDR_ACTIVE_PANE_ID"))
                .ok(),
            tab_id: env::var("HERDR_TAB_ID").ok(),
            socket_path: env::var_os("HERDR_SOCKET_PATH").map(PathBuf::from),
        }
    }

    pub fn target_pane(&self) -> Option<PaneId> {
        if let Some(pane_id) = &self.pane_id {
            return Some(PaneId::new(pane_id.clone()));
        }

        let value = self.context_value()?;
        find_string_at_paths(
            &value,
            &[
                &["focused_pane", "id"],
                &["pane", "id"],
                &["target_pane", "id"],
                &["focused_pane_id"],
                &["pane_id"],
                &["target_pane_id"],
            ],
        )
        .map(PaneId::new)
    }

    /// Working directory associated with the focused source pane, if Herdr provided one.
    pub fn focused_pane_cwd(&self) -> Option<PathBuf> {
        let value = self.context_value()?;
        find_string_at_paths(
            &value,
            &[
                &["focused_pane", "cwd"],
                &["pane", "cwd"],
                &["focused_pane_cwd"],
                &["workspace_cwd"],
            ],
        )
        .map(PathBuf::from)
    }

    fn context_value(&self) -> Option<Value> {
        let context = self.context_json.as_ref()?;
        serde_json::from_str(context).ok()
    }
}

fn find_string_at_paths(value: &Value, paths: &[&[&str]]) -> Option<String> {
    for path in paths {
        let mut cursor = value;
        let mut found_path = true;
        for segment in *path {
            if let Some(next) = cursor.get(*segment) {
                cursor = next;
            } else {
                found_path = false;
                break;
            }
        }
        if found_path {
            if let Some(text) = cursor.as_str() {
                return Some(text.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_focused_pane_from_context_json() {
        let context = HerdrContext {
            herdr_bin: "herdr".to_string(),
            context_json: Some(r#"{"focused_pane":{"id":"pane-123"}}"#.to_string()),
            pane_id: None,
            tab_id: None,
            socket_path: None,
        };

        assert_eq!(context.target_pane(), Some(PaneId::new("pane-123")));
    }

    #[test]
    fn prefers_direct_pane_id_over_context_json() {
        let context = HerdrContext {
            herdr_bin: "herdr".to_string(),
            context_json: Some(r#"{"focused_pane_id":"from-context"}"#.to_string()),
            pane_id: Some("from-env".to_string()),
            tab_id: None,
            socket_path: None,
        };

        assert_eq!(context.target_pane(), Some(PaneId::new("from-env")));
    }

    #[test]
    fn extracts_focused_pane_cwd_from_context_json() {
        let context = HerdrContext {
            herdr_bin: "herdr".to_string(),
            context_json: Some(r#"{"focused_pane_cwd":"/repo/subdir"}"#.to_string()),
            pane_id: None,
            tab_id: None,
            socket_path: None,
        };

        assert_eq!(
            context.focused_pane_cwd(),
            Some(PathBuf::from("/repo/subdir"))
        );
    }
}
