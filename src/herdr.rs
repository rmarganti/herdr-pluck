use crate::model::{PaneId, Rect, SourceGeometrySnapshot};
use anyhow::{anyhow, Context, Result};
use crossterm::event::{read, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::io::{self, Write};
use std::process::Command;

pub const HERDR_PLUCK_TARGET_PANE_ID: &str = "HERDR_PLUCK_TARGET_PANE_ID";
pub const HERDR_PLUCK_SOURCE_GEOMETRY_JSON: &str = "HERDR_PLUCK_SOURCE_GEOMETRY_JSON";

/// Picker snapshot transport selected by the Herdr launcher after considering payload constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotTransport {
    EnvJson,
    TempFile,
}

/// Constraints used to choose how a picker snapshot is passed to the temporary pane process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotTransportConstraints {
    pub payload_bytes: usize,
    pub command_involves_shell: bool,
    pub supports_direct_env: bool,
}

impl SnapshotTransportConstraints {
    pub const SAFE_ENV_JSON_BYTES: usize = 16 * 1024;

    /// Chooses the simplest safe transport, reserving temp files for large or shell-fragile payloads.
    pub fn choose_transport(self) -> SnapshotTransport {
        if self.supports_direct_env
            && !self.command_involves_shell
            && self.payload_bytes <= Self::SAFE_ENV_JSON_BYTES
        {
            SnapshotTransport::EnvJson
        } else {
            SnapshotTransport::TempFile
        }
    }
}

// Layout-tab snapshot domain.

/// How pane text was captured for a picker snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneTextCaptureMode {
    ExactVisibleUnwrapped,
    RecentUnwrappedBottomApproximation,
    VisibleWrapped,
}

/// One source pane's Herdr-global geometry captured before creating a temporary layout tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePaneGeometry {
    pub pane_id: PaneId,
    pub outer_rect: Rect,
    pub content_rect: Rect,
    pub content_width: u16,
    pub content_height: u16,
}

/// Immutable source tab state needed to launch and render a layout-tab picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePaneSnapshot {
    pub target_pane_id: PaneId,
    pub source_tab_id: String,
    pub workspace_id: String,
    pub source_panes: Vec<SourcePaneGeometry>,
    pub target_content_width: u16,
    pub target_content_height: u16,
    pub logical_lines: Vec<String>,
    pub capture_mode: PaneTextCaptureMode,
}

/// Temporary layout-tab session ids required for explicit cleanup and focus restoration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TempTabSession {
    pub temp_tab_id: String,
    pub return_tab_id: String,
    pub return_pane_id: PaneId,
}

// Layout planning domain.

/// Direction of a Herdr binary pane split as exposed by layout snapshots and replay commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDirection {
    Right,
    Down,
}

/// Binary Herdr layout tree with source pane ids preserved at leaves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutNode {
    Pane {
        source_pane_id: PaneId,
        rect: Rect,
    },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
        rect: Rect,
    },
}

/// Replayable layout plan plus the source pane that must receive the picker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutRecreationPlan {
    pub root: LayoutNode,
    pub target_source_pane_id: PaneId,
}

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
    pub area: Rect,
    pub focused_pane_id: Option<String>,
    pub panes: Vec<LayoutPane>,
    #[serde(default)]
    pub splits: Vec<LayoutSplit>,
    #[serde(default)]
    pub tab_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub zoomed: bool,
}

/// Pane entry within a Herdr layout snapshot.
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutPane {
    #[serde(default)]
    pub focused: bool,
    pub pane_id: String,
    pub rect: Rect,
}

/// Split entry within a Herdr layout snapshot.
#[derive(Debug, Clone, Deserialize)]
pub struct LayoutSplit {
    pub direction: SplitDirection,
    pub ratio: f32,
    pub rect: Rect,
}

/// Parses Herdr's `pane layout` CLI response into a frozen source geometry snapshot.
pub fn parse_layout_snapshot(bytes: &[u8], target: &PaneId) -> Result<SourceGeometrySnapshot> {
    let envelope: LayoutEnvelope =
        serde_json::from_slice(bytes).context("invalid pane layout JSON")?;
    Ok(derive_source_geometry(&envelope.result.layout, target))
}

/// Builds a pure replay plan from Herdr layout data without running Herdr commands.
pub fn derive_layout_recreation_plan(
    layout: &LayoutSnapshot,
    target: &PaneId,
) -> Result<LayoutRecreationPlan> {
    let root = build_layout_node(layout.area, &layout.panes, &layout.splits)?;
    Ok(LayoutRecreationPlan {
        root,
        target_source_pane_id: target.clone(),
    })
}

fn build_layout_node(
    region: Rect,
    panes: &[LayoutPane],
    splits: &[LayoutSplit],
) -> Result<LayoutNode> {
    let panes_in_region: Vec<&LayoutPane> = panes
        .iter()
        .filter(|pane| rect_contains_rect(region, pane.rect))
        .collect();

    if panes_in_region.is_empty() {
        return Err(anyhow!(
            "layout region x={} y={} w={} h={} contains no panes",
            region.x,
            region.y,
            region.width,
            region.height
        ));
    }

    if panes_in_region.len() == 1 {
        let pane = panes_in_region[0];
        return Ok(LayoutNode::Pane {
            source_pane_id: PaneId::new(pane.pane_id.clone()),
            rect: pane.rect,
        });
    }

    let split = splits
        .iter()
        .find(|split| split.rect == region)
        .ok_or_else(|| {
            anyhow!(
                "no split describes layout region with {} panes",
                panes_in_region.len()
            )
        })?;

    let (first_region, second_region) = partition_region(region, split, &panes_in_region)?;
    Ok(LayoutNode::Split {
        direction: split.direction,
        ratio: split.ratio,
        first: Box::new(build_layout_node(first_region, panes, splits)?),
        second: Box::new(build_layout_node(second_region, panes, splits)?),
        rect: region,
    })
}

fn partition_region(
    region: Rect,
    split: &LayoutSplit,
    panes: &[&LayoutPane],
) -> Result<(Rect, Rect)> {
    let mut first: Vec<Rect> = Vec::new();
    let mut second: Vec<Rect> = Vec::new();
    let split_at = match split.direction {
        SplitDirection::Right => region.x + ((region.width as f32 * split.ratio).round() as u16),
        SplitDirection::Down => region.y + ((region.height as f32 * split.ratio).round() as u16),
    };

    for pane in panes {
        match split.direction {
            SplitDirection::Right if pane.rect.x.saturating_add(pane.rect.width) <= split_at => {
                first.push(pane.rect)
            }
            SplitDirection::Right if pane.rect.x >= split_at => second.push(pane.rect),
            SplitDirection::Down if pane.rect.y.saturating_add(pane.rect.height) <= split_at => {
                first.push(pane.rect)
            }
            SplitDirection::Down if pane.rect.y >= split_at => second.push(pane.rect),
            _ => {
                return Err(anyhow!(
                    "pane rect x={} y={} w={} h={} crosses split boundary {}",
                    pane.rect.x,
                    pane.rect.y,
                    pane.rect.width,
                    pane.rect.height,
                    split_at
                ));
            }
        }
    }

    let first = bounding_rect(&first).ok_or_else(|| anyhow!("split first child has no panes"))?;
    let second =
        bounding_rect(&second).ok_or_else(|| anyhow!("split second child has no panes"))?;
    Ok((first, second))
}

fn rect_contains_rect(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x.saturating_add(inner.width) <= outer.x.saturating_add(outer.width)
        && inner.y.saturating_add(inner.height) <= outer.y.saturating_add(outer.height)
}

fn bounding_rect(rects: &[Rect]) -> Option<Rect> {
    rects.first()?;
    let min_x = rects.iter().map(|rect| rect.x).min()?;
    let min_y = rects.iter().map(|rect| rect.y).min()?;
    let max_x = rects
        .iter()
        .map(|rect| rect.x.saturating_add(rect.width))
        .max()?;
    let max_y = rects
        .iter()
        .map(|rect| rect.y.saturating_add(rect.height))
        .max()?;
    Some(Rect::new(
        min_x,
        min_y,
        max_x.saturating_sub(min_x),
        max_y.saturating_sub(min_y),
    ))
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
            splits: Vec::new(),
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
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
            splits: Vec::new(),
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
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
            splits: Vec::new(),
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
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

    #[test]
    fn snapshot_transport_prefers_env_for_small_direct_payloads() {
        let constraints = SnapshotTransportConstraints {
            payload_bytes: 512,
            command_involves_shell: false,
            supports_direct_env: true,
        };

        assert_eq!(constraints.choose_transport(), SnapshotTransport::EnvJson);
    }

    #[test]
    fn snapshot_transport_uses_temp_file_for_shell_or_large_payloads() {
        assert_eq!(
            SnapshotTransportConstraints {
                payload_bytes: 512,
                command_involves_shell: true,
                supports_direct_env: true,
            }
            .choose_transport(),
            SnapshotTransport::TempFile
        );
        assert_eq!(
            SnapshotTransportConstraints {
                payload_bytes: SnapshotTransportConstraints::SAFE_ENV_JSON_BYTES + 1,
                command_involves_shell: false,
                supports_direct_env: true,
            }
            .choose_transport(),
            SnapshotTransport::TempFile
        );
    }

    #[test]
    fn derives_single_pane_layout_plan_without_splits() {
        let layout = LayoutSnapshot {
            area: Rect::new(0, 0, 80, 24),
            focused_pane_id: Some("p1".to_string()),
            panes: vec![LayoutPane {
                focused: true,
                pane_id: "p1".to_string(),
                rect: Rect::new(0, 0, 80, 24),
            }],
            splits: Vec::new(),
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
            zoomed: false,
        };

        let plan = derive_layout_recreation_plan(&layout, &PaneId::new("p1")).unwrap();

        assert_eq!(
            plan.root,
            LayoutNode::Pane {
                source_pane_id: PaneId::new("p1"),
                rect: Rect::new(0, 0, 80, 24),
            }
        );
    }

    #[test]
    fn derives_right_split_layout_plan_from_herdr_splits() {
        let layout = LayoutSnapshot {
            area: Rect::new(0, 0, 100, 40),
            focused_pane_id: Some("p2".to_string()),
            panes: vec![
                LayoutPane {
                    focused: false,
                    pane_id: "p1".to_string(),
                    rect: Rect::new(0, 0, 40, 40),
                },
                LayoutPane {
                    focused: true,
                    pane_id: "p2".to_string(),
                    rect: Rect::new(40, 0, 60, 40),
                },
            ],
            splits: vec![LayoutSplit {
                direction: SplitDirection::Right,
                ratio: 0.4,
                rect: Rect::new(0, 0, 100, 40),
            }],
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
            zoomed: false,
        };

        let plan = derive_layout_recreation_plan(&layout, &PaneId::new("p2")).unwrap();

        assert_eq!(plan.target_source_pane_id, PaneId::new("p2"));
        assert_eq!(
            plan.root,
            LayoutNode::Split {
                direction: SplitDirection::Right,
                ratio: 0.4,
                first: Box::new(LayoutNode::Pane {
                    source_pane_id: PaneId::new("p1"),
                    rect: Rect::new(0, 0, 40, 40),
                }),
                second: Box::new(LayoutNode::Pane {
                    source_pane_id: PaneId::new("p2"),
                    rect: Rect::new(40, 0, 60, 40),
                }),
                rect: Rect::new(0, 0, 100, 40),
            }
        );
    }

    #[test]
    fn split_lookup_requires_exact_region_match_for_nested_layouts() {
        let layout = LayoutSnapshot {
            area: Rect::new(0, 0, 100, 40),
            focused_pane_id: Some("p3".to_string()),
            panes: vec![
                LayoutPane {
                    focused: false,
                    pane_id: "p1".to_string(),
                    rect: Rect::new(0, 0, 40, 40),
                },
                LayoutPane {
                    focused: false,
                    pane_id: "p2".to_string(),
                    rect: Rect::new(40, 0, 60, 20),
                },
                LayoutPane {
                    focused: true,
                    pane_id: "p3".to_string(),
                    rect: Rect::new(40, 20, 60, 20),
                },
            ],
            splits: vec![
                LayoutSplit {
                    direction: SplitDirection::Down,
                    ratio: 0.5,
                    rect: Rect::new(40, 0, 60, 40),
                },
                LayoutSplit {
                    direction: SplitDirection::Right,
                    ratio: 0.4,
                    rect: Rect::new(0, 0, 100, 40),
                },
            ],
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
            zoomed: false,
        };

        let plan = derive_layout_recreation_plan(&layout, &PaneId::new("p3")).unwrap();

        assert!(matches!(
            plan.root,
            LayoutNode::Split {
                direction: SplitDirection::Right,
                ..
            }
        ));
    }

    #[test]
    fn empty_layout_region_fails_clearly() {
        let layout = LayoutSnapshot {
            area: Rect::new(0, 0, 80, 24),
            focused_pane_id: None,
            panes: Vec::new(),
            splits: Vec::new(),
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
            zoomed: false,
        };

        let error = derive_layout_recreation_plan(&layout, &PaneId::new("missing")).unwrap_err();

        assert!(error.to_string().contains("contains no panes"));
    }

    #[test]
    fn pane_crossing_split_boundary_fails_clearly() {
        let layout = LayoutSnapshot {
            area: Rect::new(0, 0, 100, 40),
            focused_pane_id: Some("p1".to_string()),
            panes: vec![
                LayoutPane {
                    focused: true,
                    pane_id: "p1".to_string(),
                    rect: Rect::new(0, 0, 60, 40),
                },
                LayoutPane {
                    focused: false,
                    pane_id: "p2".to_string(),
                    rect: Rect::new(60, 0, 40, 40),
                },
            ],
            splits: vec![LayoutSplit {
                direction: SplitDirection::Right,
                ratio: 0.5,
                rect: Rect::new(0, 0, 100, 40),
            }],
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
            zoomed: false,
        };

        let error = derive_layout_recreation_plan(&layout, &PaneId::new("p1")).unwrap_err();

        assert!(error.to_string().contains("crosses split boundary"));
    }
}
