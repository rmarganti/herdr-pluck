use crate::model::{PaneId, Rect, SourceGeometrySnapshot, SplitDirection};
use anyhow::{Context, Result};
use serde::Deserialize;

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

/// Parses Herdr's `pane layout` CLI response.
pub fn parse_layout_snapshot(bytes: &[u8]) -> Result<LayoutSnapshot> {
    let envelope: LayoutEnvelope =
        serde_json::from_slice(bytes).context("invalid pane layout JSON")?;
    Ok(envelope.result.layout)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(id: &str, rect: Rect, focused: bool) -> LayoutPane {
        LayoutPane {
            focused,
            pane_id: id.to_string(),
            rect,
        }
    }

    fn layout(panes: Vec<LayoutPane>, splits: Vec<LayoutSplit>) -> LayoutSnapshot {
        LayoutSnapshot {
            area: Rect::new(0, 0, 100, 40),
            focused_pane_id: Some("p1".to_string()),
            panes,
            splits,
            tab_id: Some("t1".to_string()),
            workspace_id: Some("w1".to_string()),
            zoomed: false,
        }
    }

    #[test]
    fn single_pane_geometry_has_no_border_inset() {
        let layout = layout(vec![pane("p1", Rect::new(0, 0, 100, 40), true)], vec![]);

        let geometry = derive_source_geometry(&layout, &PaneId::new("p1"));

        assert_eq!(geometry.source_outer_rect, Rect::new(0, 0, 100, 40));
        assert_eq!(geometry.source_content_rect, Rect::new(0, 0, 99, 40));
    }

    #[test]
    fn multi_pane_geometry_insets_borders_and_gutter() {
        let layout = layout(
            vec![
                pane("p1", Rect::new(0, 0, 40, 40), false),
                pane("p2", Rect::new(40, 0, 60, 40), true),
            ],
            vec![LayoutSplit {
                direction: SplitDirection::Right,
                ratio: 0.4,
                rect: Rect::new(0, 0, 100, 40),
            }],
        );

        let geometry = derive_source_geometry(&layout, &PaneId::new("p2"));

        assert_eq!(geometry.source_outer_rect, Rect::new(40, 0, 60, 40));
        assert_eq!(geometry.source_content_rect, Rect::new(41, 1, 57, 38));
    }

    #[test]
    fn zoomed_focused_target_uses_full_area() {
        let mut layout = layout(
            vec![
                pane("p1", Rect::new(0, 0, 40, 40), false),
                pane("p2", Rect::new(40, 0, 60, 40), true),
            ],
            vec![],
        );
        layout.zoomed = true;

        let geometry = derive_source_geometry(&layout, &PaneId::new("p2"));

        assert_eq!(geometry.source_outer_rect, Rect::new(0, 0, 100, 40));
    }
}
