use crate::model::PaneId;
use anyhow::{anyhow, Context, Result};
use crossterm::event::{read, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use serde_json::Value;
use std::env;
use std::io::{self, Write};
use std::process::Command;

pub const HERDR_PLUCK_TARGET_PANE_ID: &str = "HERDR_PLUCK_TARGET_PANE_ID";

#[derive(Debug, Clone)]
pub struct HerdrAdapter {
    herdr_bin: String,
    plugin_id: Option<String>,
    context_json: Option<String>,
    pane_id: Option<String>,
}

impl HerdrAdapter {
    pub fn from_env() -> Self {
        Self {
            herdr_bin: env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string()),
            plugin_id: env::var("HERDR_PLUGIN_ID").ok(),
            context_json: env::var("HERDR_PLUGIN_CONTEXT_JSON").ok(),
            pane_id: env::var("HERDR_PANE_ID")
                .or_else(|_| env::var("HERDR_ACTIVE_PANE_ID"))
                .ok(),
        }
    }

    pub fn new(
        herdr_bin: impl Into<String>,
        plugin_id: Option<String>,
        context_json: Option<String>,
        pane_id: Option<String>,
    ) -> Self {
        Self {
            herdr_bin: herdr_bin.into(),
            plugin_id,
            context_json,
            pane_id,
        }
    }

    pub fn target_pane_from_context(&self) -> Option<PaneId> {
        if let Some(pane_id) = &self.pane_id {
            return Some(PaneId::new(pane_id.clone()));
        }

        let context = self.context_json.as_ref()?;
        let value: Value = serde_json::from_str(context).ok()?;
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

    pub fn open_picker_overlay(&self, target: &PaneId) -> Result<()> {
        let plugin_id = self
            .plugin_id
            .as_deref()
            .ok_or_else(|| anyhow!("HERDR_PLUGIN_ID is required to open the plugin pane"))?;

        let status = Command::new(&self.herdr_bin)
            .args([
                "plugin",
                "pane",
                "open",
                "--plugin",
                plugin_id,
                "--entrypoint",
                "picker",
                "--placement",
                "overlay",
                "--env",
                &format!("{HERDR_PLUCK_TARGET_PANE_ID}={}", target.0),
            ])
            .status()
            .with_context(|| format!("failed to launch {} plugin pane", self.herdr_bin))?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("Herdr overlay launch failed with status {status}"))
        }
    }

    pub fn run_picker_placeholder(&self, target: &PaneId) -> Result<()> {
        println!("Herdr Pluck picker scaffold");
        println!();
        println!("Target pane: {}", target.0);
        println!("Matching, rendering, input, and copy flow are implemented in follow-up ishes.");
        println!();
        print!("Press any key to close...");
        io::stdout().flush()?;

        enable_raw_mode().context("failed to enable raw mode for placeholder picker")?;
        let _raw_mode_guard = RawModeGuard;

        loop {
            if matches!(read()?, Event::Key(_)) {
                break;
            }
        }

        println!();
        Ok(())
    }
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
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
        let adapter = HerdrAdapter::new(
            "herdr",
            Some("rmarganti.herdr-pluck".to_string()),
            Some(r#"{"focused_pane":{"id":"pane-123"}}"#.to_string()),
            None,
        );

        assert_eq!(
            adapter.target_pane_from_context(),
            Some(PaneId::new("pane-123"))
        );
    }

    #[test]
    fn prefers_direct_herdr_pane_id_env_over_context_json() {
        let adapter = HerdrAdapter::new(
            "herdr",
            Some("rmarganti.herdr-pluck".to_string()),
            Some(r#"{"focused_pane":{"id":"pane-from-context"}}"#.to_string()),
            Some("pane-from-env".to_string()),
        );

        assert_eq!(
            adapter.target_pane_from_context(),
            Some(PaneId::new("pane-from-env"))
        );
    }

    #[test]
    fn extracts_flat_pane_id_from_context_json() {
        let adapter = HerdrAdapter::new(
            "herdr",
            Some("rmarganti.herdr-pluck".to_string()),
            Some(r#"{"focused_pane_id":"pane-flat"}"#.to_string()),
            None,
        );

        assert_eq!(
            adapter.target_pane_from_context(),
            Some(PaneId::new("pane-flat"))
        );
    }
}
