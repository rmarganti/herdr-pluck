use crate::model::{
    LayoutNode, LayoutRecreationPlan, PaneId, Rect, SourceGeometrySnapshot, SourcePaneGeometry,
    SplitDirection,
};
use anyhow::{anyhow, Context, Result};
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

/// Builds a pure replay plan from Herdr layout data without running Herdr commands.
pub fn derive_layout_recreation_plan(
    layout: &LayoutSnapshot,
    target: &PaneId,
) -> Result<LayoutRecreationPlan> {
    if !layout.panes.iter().any(|pane| pane.pane_id == target.0) {
        return Err(anyhow!("target pane {target} was not present in layout"));
    }

    let root = build_layout_node(layout.area, &layout.panes, &layout.splits)?;
    Ok(LayoutRecreationPlan {
        root,
        target_source_pane_id: target.clone(),
    })
}

/// Returns source-pane geometry records with content rects derived from Herdr borders/gutter.
pub fn derive_source_pane_geometries(layout: &LayoutSnapshot) -> Vec<SourcePaneGeometry> {
    let border_inset = u16::from(layout.panes.len() > 1);
    layout
        .panes
        .iter()
        .map(|pane| {
            let content_rect = pane
                .rect
                .inset(border_inset)
                .reserve_right_gutter(u16::from(pane.rect.inset(border_inset).width > 1));
            SourcePaneGeometry {
                pane_id: PaneId::new(pane.pane_id.clone()),
                outer_rect: pane.rect,
                content_rect,
                content_width: content_rect.width,
                content_height: content_rect.height,
            }
        })
        .collect()
}

/// Derives legacy source-pane content geometry from Herdr-global pre-overlay layout coordinates.
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
    let mut first = Vec::new();
    let mut second = Vec::new();
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
                ))
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
    fn derives_single_pane_layout_plan_without_splits() {
        let layout = layout(vec![pane("p1", Rect::new(0, 0, 100, 40), true)], vec![]);
        let plan = derive_layout_recreation_plan(&layout, &PaneId::new("p1")).unwrap();
        assert_eq!(
            plan.root,
            LayoutNode::Pane {
                source_pane_id: PaneId::new("p1"),
                rect: Rect::new(0, 0, 100, 40)
            }
        );
    }

    #[test]
    fn derives_uneven_right_split_layout_plan() {
        let layout = layout(
            vec![
                pane("p1", Rect::new(0, 0, 20, 40), false),
                pane("p2", Rect::new(20, 0, 80, 40), true),
            ],
            vec![LayoutSplit {
                direction: SplitDirection::Right,
                ratio: 0.2,
                rect: Rect::new(0, 0, 100, 40),
            }],
        );
        let plan = derive_layout_recreation_plan(&layout, &PaneId::new("p2")).unwrap();
        assert!(matches!(
            plan.root,
            LayoutNode::Split {
                direction: SplitDirection::Right,
                ..
            }
        ));
    }

    #[test]
    fn derives_uneven_down_split_layout_plan() {
        let layout = layout(
            vec![
                pane("p1", Rect::new(0, 0, 100, 8), false),
                pane("p2", Rect::new(0, 8, 100, 32), true),
            ],
            vec![LayoutSplit {
                direction: SplitDirection::Down,
                ratio: 0.2,
                rect: Rect::new(0, 0, 100, 40),
            }],
        );
        let plan = derive_layout_recreation_plan(&layout, &PaneId::new("p1")).unwrap();
        assert!(matches!(
            plan.root,
            LayoutNode::Split {
                direction: SplitDirection::Down,
                ..
            }
        ));
    }

    #[test]
    fn derives_nested_two_by_two_layout() {
        let layout = layout(
            vec![
                pane("tl", Rect::new(0, 0, 50, 20), false),
                pane("bl", Rect::new(0, 20, 50, 20), false),
                pane("tr", Rect::new(50, 0, 50, 20), false),
                pane("br", Rect::new(50, 20, 50, 20), true),
            ],
            vec![
                LayoutSplit {
                    direction: SplitDirection::Right,
                    ratio: 0.5,
                    rect: Rect::new(0, 0, 100, 40),
                },
                LayoutSplit {
                    direction: SplitDirection::Down,
                    ratio: 0.5,
                    rect: Rect::new(0, 0, 50, 40),
                },
                LayoutSplit {
                    direction: SplitDirection::Down,
                    ratio: 0.5,
                    rect: Rect::new(50, 0, 50, 40),
                },
            ],
        );
        let plan = derive_layout_recreation_plan(&layout, &PaneId::new("br")).unwrap();
        assert_eq!(plan.target_source_pane_id, PaneId::new("br"));
    }

    #[test]
    fn target_must_be_present() {
        let layout = layout(vec![pane("p1", Rect::new(0, 0, 100, 40), true)], vec![]);
        let error = derive_layout_recreation_plan(&layout, &PaneId::new("missing")).unwrap_err();
        assert!(error.to_string().contains("target pane missing"));
    }

    #[test]
    fn pane_crossing_split_boundary_fails_clearly() {
        let layout = layout(
            vec![
                pane("p1", Rect::new(0, 0, 60, 40), true),
                pane("p2", Rect::new(60, 0, 40, 40), false),
            ],
            vec![LayoutSplit {
                direction: SplitDirection::Right,
                ratio: 0.5,
                rect: Rect::new(0, 0, 100, 40),
            }],
        );
        let error = derive_layout_recreation_plan(&layout, &PaneId::new("p1")).unwrap_err();
        assert!(error.to_string().contains("crosses split boundary"));
    }
}
