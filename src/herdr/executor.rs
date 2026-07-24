use crate::herdr::commands::{CommandRunner, HerdrCommands};
use crate::herdr::layout::{
    derive_layout_recreation_plan, derive_source_geometry, parse_layout_snapshot,
};
use crate::herdr::snapshot::{build_source_snapshot, PickerLaunchFiles};
use crate::herdr::socket::{ApplyLayoutNode, LayoutApplier, LayoutApplyParams, LayoutApplyRequest};
use crate::model::{LayoutNode, PaneId, PatternSpec, PickerReturnContext, PickerSnapshot};
use crate::viewport::map_visible_viewport;
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Captures source state and atomically applies the temporary picker layout.
pub fn launch_layout_tab_picker<R: CommandRunner, A: LayoutApplier>(
    herdr_bin: &str,
    runner: &mut R,
    applier: &mut A,
    target: &PaneId,
    binary_path: &Path,
    custom_patterns: Vec<PatternSpec>,
) -> Result<()> {
    let layout_bytes = HerdrCommands::new(herdr_bin, runner).pane_layout(target)?;
    let layout = parse_layout_snapshot(&layout_bytes)?;
    let plan = derive_layout_recreation_plan(&layout, target)?;
    let geometry = derive_source_geometry(&layout, target);
    let read_lines = geometry.source_content_rect.height;

    if read_lines == 0 {
        bail!("target pane {target} has zero visible content height");
    }

    let visible_text =
        HerdrCommands::new(herdr_bin, runner).pane_read_visible(target, read_lines)?;

    let viewport = map_visible_viewport(
        visible_text.lines().map(str::to_string).collect(),
        geometry.source_content_rect.width,
        read_lines,
    );

    let return_context = PickerReturnContext {
        return_tab_id: layout
            .tab_id
            .clone()
            .context("pane layout did not include return tab id")?,
        return_pane_id: target.clone(),
        zoom_picker: layout.zoomed && layout_target_is_focused(&layout, target),
    };

    let snapshot = build_source_snapshot(
        &layout,
        target,
        viewport.logical_lines.clone(),
        Some(viewport),
        return_context.clone(),
        custom_patterns,
    )?;

    let files = PickerLaunchFiles::create(&snapshot)?;

    let root = convert_layout(
        &plan.root,
        target,
        binary_path,
        &files.snapshot_path,
        &files.ready_path,
    );

    let request = LayoutApplyRequest {
        id: format!(
            "pluck-{}-{}",
            std::process::id(),
            REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        ),
        method: "layout.apply",
        params: LayoutApplyParams {
            workspace_id: layout
                .workspace_id
                .clone()
                .context("pane layout did not include workspace id")?,
            tab_label: "Herdr Pluck".to_string(),
            focus: true,
            root,
        },
    };

    let applied = match applier.apply_layout(&request) {
        Ok(value) => value,
        Err(error) => {
            let _ = files.cleanup();
            return Err(error);
        }
    };

    let focus_result = applier.focus_pane(&applied.picker_pane_id);
    if let Err(error) = focus_result.and_then(|_| files.signal_ready()) {
        let _ = cleanup_session(herdr_bin, runner, &return_context, &applied.tab_id);
        let _ = files.cleanup();
        return Err(error);
    }

    Ok(())
}

fn convert_layout(
    node: &LayoutNode,
    target: &PaneId,
    binary: &Path,
    snapshot: &Path,
    ready: &Path,
) -> ApplyLayoutNode {
    match node {
        LayoutNode::Pane { source_pane_id, .. } if source_pane_id == target => {
            ApplyLayoutNode::Pane {
                command: vec![
                    binary.to_string_lossy().into_owned(),
                    "pick".into(),
                    "--snapshot".into(),
                    snapshot.to_string_lossy().into_owned(),
                    "--ready".into(),
                    ready.to_string_lossy().into_owned(),
                ],
            }
        }
        LayoutNode::Pane { .. } => ApplyLayoutNode::Pane {
            command: vec![binary.to_string_lossy().into_owned(), "idle".into()],
        },
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
            ..
        } => ApplyLayoutNode::Split {
            direction: *direction,
            ratio: *ratio,
            first: Box::new(convert_layout(first, target, binary, snapshot, ready)),
            second: Box::new(convert_layout(second, target, binary, snapshot, ready)),
        },
    }
}

fn layout_target_is_focused(
    layout: &crate::herdr::layout::LayoutSnapshot,
    target: &PaneId,
) -> bool {
    layout
        .focused_pane_id
        .as_ref()
        .is_some_and(|id| id == &target.0)
        || layout
            .panes
            .iter()
            .any(|p| p.pane_id == target.0 && p.focused)
}

/// Restores the source tab and closes only the explicit temporary tab.
pub fn cleanup_session<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    session: &PickerReturnContext,
    temporary_tab_id: &str,
) -> Result<()> {
    if temporary_tab_id.is_empty() {
        bail!("temporary picker tab id is missing");
    }
    if temporary_tab_id == session.return_tab_id {
        bail!(
            "refusing to close source tab {} as temporary picker tab",
            temporary_tab_id
        );
    }
    let mut first = None;
    if let Err(e) = HerdrCommands::new(herdr_bin, runner).tab_focus(&session.return_tab_id) {
        first = Some(e);
    }
    if let Err(e) = HerdrCommands::new(herdr_bin, runner).tab_close(temporary_tab_id) {
        if first.is_none() {
            first = Some(e);
        }
    }
    first.map_or(Ok(()), Err)
}

pub fn zoom_picker<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    snapshot: &PickerSnapshot,
    pane_id: &PaneId,
) -> Result<()> {
    if snapshot.session.zoom_picker {
        HerdrCommands::new(herdr_bin, runner).pane_zoom_on(pane_id)?;
    }
    Ok(())
}
pub fn run_snapshot_picker(snapshot: &PickerSnapshot) -> Result<()> {
    crate::picker::run_picker(snapshot).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Rect, SplitDirection};
    #[test]
    fn conversion_preserves_argv_and_split() {
        let tree = LayoutNode::Split {
            direction: SplitDirection::Right,
            ratio: 0.37,
            first: Box::new(LayoutNode::Pane {
                source_pane_id: PaneId::new("a"),
                rect: Rect::new(0, 0, 1, 1),
            }),
            second: Box::new(LayoutNode::Pane {
                source_pane_id: PaneId::new("b"),
                rect: Rect::new(0, 0, 1, 1),
            }),
            rect: Rect::new(0, 0, 2, 1),
        };
        let converted = convert_layout(
            &tree,
            &PaneId::new("b"),
            Path::new("/a b/π'"),
            Path::new("/s p"),
            Path::new("/r p"),
        );
        let json = serde_json::to_string(&converted).unwrap();
        assert!(json.contains("0.37"));
        assert!(json.contains("/a b/π'"));
        assert!(json.contains("\"idle\""));
        assert!(json.contains("\"pick\""));
        assert!(!json.contains("bash"));
    }
}
