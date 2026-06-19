use crate::model::{PaneId, Rect, SourceGeometrySnapshot};
use anyhow::{anyhow, Context, Result};
use crossterm::event::{read, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use serde::Deserialize;
use serde_json::Value;
use std::env;
use std::io::{self, Write};
use std::process::Command;

pub const HERDR_PLUCK_TARGET_PANE_ID: &str = "HERDR_PLUCK_TARGET_PANE_ID";
pub const HERDR_PLUCK_SOURCE_GEOMETRY_JSON: &str = "HERDR_PLUCK_SOURCE_GEOMETRY_JSON";

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

    /// Capture source-pane geometry before an overlay is opened. Opening a Herdr overlay changes
    /// focus, zoom, pane count, and frame geometry, so picker mode must use this frozen snapshot.
    pub fn capture_source_geometry(&self, target: &PaneId) -> Result<SourceGeometrySnapshot> {
        let output = Command::new(&self.herdr_bin)
            .args(["pane", "layout", "--pane", &target.0])
            .output()
            .with_context(|| format!("failed to run {} pane layout", self.herdr_bin))?;

        if !output.status.success() {
            return Err(anyhow!(
                "Herdr pane layout failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        parse_layout_snapshot(&output.stdout, target)
    }

    pub fn open_picker_overlay(
        &self,
        target: &PaneId,
        geometry: &SourceGeometrySnapshot,
    ) -> Result<()> {
        let plugin_id = self
            .plugin_id
            .as_deref()
            .ok_or_else(|| anyhow!("HERDR_PLUGIN_ID is required to open the plugin pane"))?;
        let geometry_json = serialize_source_geometry(geometry)?;

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
                "--env",
                &format!("{HERDR_PLUCK_SOURCE_GEOMETRY_JSON}={geometry_json}"),
            ])
            .status()
            .with_context(|| format!("failed to launch {} plugin pane", self.herdr_bin))?;

        if status.success() {
            Ok(())
        } else {
            Err(anyhow!("Herdr overlay launch failed with status {status}"))
        }
    }

    pub fn run_picker_placeholder(
        &self,
        target: &PaneId,
        geometry: Option<&SourceGeometrySnapshot>,
    ) -> Result<()> {
        println!("Herdr Pluck picker scaffold");
        println!();
        println!("Target pane: {}", target.0);
        match geometry {
            Some(geometry) => {
                println!("Frozen pre-overlay geometry captured:");
                println!(
                    "  terminal area: x={} y={} w={} h={}",
                    geometry.terminal_area.x,
                    geometry.terminal_area.y,
                    geometry.terminal_area.width,
                    geometry.terminal_area.height
                );
                println!(
                    "  source outer:  x={} y={} w={} h={}",
                    geometry.source_outer_rect.x,
                    geometry.source_outer_rect.y,
                    geometry.source_outer_rect.width,
                    geometry.source_outer_rect.height
                );
                println!(
                    "  source content: x={} y={} w={} h={}",
                    geometry.source_content_rect.x,
                    geometry.source_content_rect.y,
                    geometry.source_content_rect.width,
                    geometry.source_content_rect.height
                );
                let local_content = geometry.source_content_rect_in_terminal();
                println!(
                    "  source content in terminal: x={} y={} w={} h={}",
                    local_content.x, local_content.y, local_content.width, local_content.height
                );
                println!(
                    "  panes={} zoomed={} target_focused={}",
                    geometry.pane_count, geometry.zoomed, geometry.target_focused
                );
            }
            None => println!("Frozen pre-overlay geometry: unavailable"),
        }
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

/// Top-level Herdr CLI response wrapper for `pane layout`.
#[derive(Debug, Clone, Deserialize)]
struct LayoutEnvelope {
    result: LayoutResult,
}

/// `pane layout` response payload containing the layout snapshot.
#[derive(Debug, Clone, Deserialize)]
struct LayoutResult {
    layout: LayoutSnapshot,
}

/// Herdr tab layout snapshot returned by `pane layout` in Herdr-global coordinates.
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutSnapshot {
    area: Rect,
    focused_pane_id: Option<String>,
    panes: Vec<LayoutPane>,
    #[serde(default)]
    zoomed: bool,
}

/// Pane entry within a Herdr layout snapshot.
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPane {
    #[serde(default)]
    focused: bool,
    pane_id: String,
    rect: Rect,
}

/// Parses Herdr's `pane layout` CLI response into a frozen source geometry snapshot.
pub fn parse_layout_snapshot(bytes: &[u8], target: &PaneId) -> Result<SourceGeometrySnapshot> {
    let envelope: LayoutEnvelope =
        serde_json::from_slice(bytes).context("invalid pane layout JSON")?;
    Ok(derive_source_geometry(&envelope.result.layout, target))
}

/// Derives source-pane content geometry from Herdr-global pre-overlay layout coordinates.
pub fn derive_source_geometry(layout: &LayoutSnapshot, target: &PaneId) -> SourceGeometrySnapshot {
    let pane_count = layout.panes.len();
    let target_pane = layout
        .panes
        .iter()
        .find(|pane| pane.pane_id == target.0)
        .or_else(|| layout.panes.iter().find(|pane| pane.focused));
    let target_focused = target_pane
        .map(|pane| pane.focused)
        .or_else(|| layout.focused_pane_id.as_ref().map(|id| id == &target.0))
        .unwrap_or(false);

    let use_full_area = layout.zoomed && target_focused;
    let source_outer_rect = if use_full_area {
        layout.area
    } else {
        target_pane.map(|pane| pane.rect).unwrap_or(layout.area)
    };

    let border_inset = u16::from(pane_count > 1);
    let source_content_rect = source_outer_rect
        .inset(border_inset)
        .reserve_right_gutter(u16::from(source_outer_rect.inset(border_inset).width > 1));

    SourceGeometrySnapshot {
        target_pane_id: target.clone(),
        terminal_area: layout.area,
        source_outer_rect,
        source_content_rect,
        pane_count,
        zoomed: layout.zoomed,
        target_focused,
    }
}

/// Serializes source geometry for transport through Herdr plugin pane environment variables.
pub fn serialize_source_geometry(geometry: &SourceGeometrySnapshot) -> Result<String> {
    serde_json::to_string(geometry).context("failed to serialize source geometry")
}

/// Deserializes source geometry passed to picker mode.
pub fn deserialize_source_geometry(json: &str) -> Result<SourceGeometrySnapshot> {
    serde_json::from_str(json).context("failed to parse source geometry")
}

/// Reads picker-mode source geometry from the Herdr Pluck environment variable.
pub fn source_geometry_from_env() -> Result<Option<SourceGeometrySnapshot>> {
    env::var(HERDR_PLUCK_SOURCE_GEOMETRY_JSON)
        .ok()
        .map(|json| deserialize_source_geometry(&json))
        .transpose()
}

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// Traverses the given JSON value for each of the provided paths,
/// returning the first string found at any of those paths.
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

    #[test]
    fn derives_single_pane_unframed_content_from_full_area_with_gutter() {
        let layout = LayoutSnapshot {
            area: Rect::new(26, 1, 100, 40),
            focused_pane_id: Some("p1".to_string()),
            panes: vec![LayoutPane {
                focused: true,
                pane_id: "p1".to_string(),
                rect: Rect::new(26, 1, 100, 40),
            }],
            zoomed: false,
        };

        let geometry = derive_source_geometry(&layout, &PaneId::new("p1"));

        assert_eq!(geometry.source_outer_rect, Rect::new(26, 1, 100, 40));
        assert_eq!(geometry.source_content_rect, Rect::new(26, 1, 99, 40));
        assert_eq!(
            geometry.source_content_rect_in_terminal(),
            Rect::new(0, 0, 99, 40)
        );
        assert_eq!(geometry.pane_count, 1);
    }

    #[test]
    fn derives_unzoomed_split_content_from_pane_rect_with_border_and_gutter() {
        let layout = LayoutSnapshot {
            area: Rect::new(0, 0, 200, 60),
            focused_pane_id: Some("p2".to_string()),
            panes: vec![
                LayoutPane {
                    focused: false,
                    pane_id: "p1".to_string(),
                    rect: Rect::new(0, 0, 100, 60),
                },
                LayoutPane {
                    focused: true,
                    pane_id: "p2".to_string(),
                    rect: Rect::new(100, 0, 100, 60),
                },
            ],
            zoomed: false,
        };

        let geometry = derive_source_geometry(&layout, &PaneId::new("p2"));

        assert_eq!(geometry.source_outer_rect, Rect::new(100, 0, 100, 60));
        assert_eq!(geometry.source_content_rect, Rect::new(101, 1, 97, 58));
        assert_eq!(
            geometry.source_content_rect_in_terminal(),
            Rect::new(101, 1, 97, 58)
        );
    }

    #[test]
    fn derives_zoomed_multi_pane_content_from_full_area_with_border_and_gutter() {
        let layout = LayoutSnapshot {
            area: Rect::new(26, 1, 226, 63),
            focused_pane_id: Some("p2".to_string()),
            panes: vec![
                LayoutPane {
                    focused: false,
                    pane_id: "p1".to_string(),
                    rect: Rect::new(26, 1, 113, 63),
                },
                LayoutPane {
                    focused: true,
                    pane_id: "p2".to_string(),
                    rect: Rect::new(139, 1, 113, 63),
                },
            ],
            zoomed: true,
        };

        let geometry = derive_source_geometry(&layout, &PaneId::new("p2"));

        assert_eq!(geometry.source_outer_rect, Rect::new(26, 1, 226, 63));
        assert_eq!(geometry.source_content_rect, Rect::new(27, 2, 223, 61));
        assert_eq!(
            geometry.source_content_rect_in_terminal(),
            Rect::new(1, 1, 223, 61)
        );
    }

    #[test]
    fn rect_relative_to_removes_global_sidebar_and_tab_offsets() {
        assert_eq!(
            Rect::new(26, 1, 225, 63).relative_to(Rect::new(26, 1, 226, 63)),
            Rect::new(0, 0, 225, 63)
        );
    }

    #[test]
    fn source_geometry_json_round_trips() {
        let geometry = SourceGeometrySnapshot {
            target_pane_id: PaneId::new("p1"),
            terminal_area: Rect::new(0, 0, 80, 24),
            source_outer_rect: Rect::new(0, 0, 80, 24),
            source_content_rect: Rect::new(0, 0, 79, 24),
            pane_count: 1,
            zoomed: false,
            target_focused: true,
        };

        let json = serialize_source_geometry(&geometry).unwrap();
        assert_eq!(deserialize_source_geometry(&json).unwrap(), geometry);
    }

    #[test]
    fn parses_actual_herdr_layout_envelope_shape() {
        let json = br#"{"id":"cli:pane:layout","result":{"layout":{"area":{"height":63,"width":226,"x":26,"y":1},"focused_pane_id":"wE:p2","panes":[{"focused":false,"pane_id":"wE:p1","rect":{"height":63,"width":113,"x":26,"y":1}},{"focused":true,"pane_id":"wE:p2","rect":{"height":63,"width":113,"x":139,"y":1}}],"splits":[],"tab_id":"wE:t1","workspace_id":"wE","zoomed":true},"type":"pane_layout"}}"#;

        let geometry = parse_layout_snapshot(json, &PaneId::new("wE:p2")).unwrap();
        assert_eq!(geometry.source_content_rect, Rect::new(27, 2, 223, 61));
    }
}
