use crate::herdr::client::{HerdrClient, LaunchLayoutNode};
use crate::herdr::layout::{derive_layout_recreation_plan, derive_source_geometry};
use crate::herdr::snapshot::{build_source_snapshot, PickerLaunchFiles};
use crate::model::{LayoutNode, PaneId, PatternSpec, PickerReturnContext, PickerSnapshot};
use crate::viewport::map_visible_viewport;
use anyhow::{bail, Context, Result};
use std::path::Path;

/// Captures source state and atomically applies the temporary picker layout.
pub fn launch_layout_tab_picker<C: HerdrClient>(
    client: &mut C,
    target: &PaneId,
    binary_path: &Path,
    custom_patterns: Vec<PatternSpec>,
) -> Result<()> {
    let layout = client.pane_layout(target)?;
    let plan = derive_layout_recreation_plan(&layout, target)?;
    let geometry = derive_source_geometry(&layout, target);
    let read_lines = geometry.source_content_rect.height;

    if read_lines == 0 {
        bail!("target pane {target} has zero visible content height");
    }

    let visible_text = client.pane_read_visible(target, read_lines)?;

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

    let workspace_id = layout
        .workspace_id
        .as_deref()
        .context("pane layout did not include workspace id")?;
    let applied = match client.apply_layout(workspace_id, "Herdr Pluck", &root) {
        Ok(value) => value,
        Err(error) => {
            let _ = files.cleanup();
            return Err(error);
        }
    };

    let focus_result = client.focus_pane(&applied.picker_pane_id);

    if let Err(error) = focus_result.and_then(|_| files.signal_ready()) {
        if let Err(cleanup) = cleanup_session(client, &return_context, &applied.tab_id) {
            eprintln!("launch cleanup also failed: {cleanup:#}");
        }

        if let Err(cleanup) = files.cleanup() {
            eprintln!("launch file cleanup also failed: {cleanup:#}");
        }

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
) -> LaunchLayoutNode {
    match node {
        LayoutNode::Pane { source_pane_id, .. } if source_pane_id == target => {
            LaunchLayoutNode::Pane {
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
        LayoutNode::Pane { .. } => LaunchLayoutNode::Pane {
            command: vec![binary.to_string_lossy().into_owned(), "idle".into()],
        },
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
            ..
        } => LaunchLayoutNode::Split {
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
pub fn cleanup_session<C: HerdrClient>(
    client: &mut C,
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

    if let Err(e) = client.focus_tab(&session.return_tab_id) {
        first = Some(e);
    }

    if let Err(e) = client.close_tab(temporary_tab_id) {
        if first.is_none() {
            first = Some(e);
        }
    }

    first.map_or(Ok(()), Err)
}

pub fn zoom_picker<C: HerdrClient>(
    client: &mut C,
    snapshot: &PickerSnapshot,
    pane_id: &PaneId,
) -> Result<()> {
    if snapshot.session.zoom_picker {
        client.zoom_pane(pane_id)?;
    }

    Ok(())
}

pub fn run_snapshot_picker(snapshot: &PickerSnapshot) -> Result<()> {
    crate::picker::run_picker(snapshot).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::client::AppliedLayout;
    use crate::herdr::layout::{LayoutPane, LayoutSnapshot};
    use crate::model::{
        PaneTextCaptureMode, Rect, SourcePaneSnapshot, SplitDirection, VisibleViewport,
    };
    use anyhow::anyhow;

    #[derive(Default)]
    struct FakeClient {
        layout: Option<LayoutSnapshot>,
        calls: Vec<String>,
        launch_paths: Option<(std::path::PathBuf, std::path::PathBuf)>,
        fail_focus_pane: bool,
        fail_focus_tab: bool,
    }

    impl HerdrClient for FakeClient {
        fn pane_layout(&mut self, _pane: &PaneId) -> Result<LayoutSnapshot> {
            self.calls.push("pane_layout".into());
            self.layout.take().context("missing fake layout")
        }

        fn pane_read_visible(&mut self, _pane: &PaneId, lines: u16) -> Result<String> {
            self.calls.push(format!("pane_read:{lines}"));
            Ok("https://example.com".into())
        }

        fn apply_layout(
            &mut self,
            workspace_id: &str,
            _tab_label: &str,
            root: &LaunchLayoutNode,
        ) -> Result<AppliedLayout> {
            self.calls.push(format!("apply:{workspace_id}"));
            self.launch_paths = picker_paths(root);
            Ok(AppliedLayout {
                tab_id: "w1:t2".into(),
                picker_pane_id: PaneId::new("w1:p2"),
            })
        }

        fn focus_pane(&mut self, pane: &PaneId) -> Result<()> {
            self.calls.push(format!("focus_pane:{pane}"));
            let (_, ready) = self.launch_paths.as_ref().context("missing launch paths")?;
            assert!(!ready.exists(), "barrier released before picker focus");
            if self.fail_focus_pane {
                Err(anyhow!("focus failed"))
            } else {
                Ok(())
            }
        }

        fn zoom_pane(&mut self, pane: &PaneId) -> Result<()> {
            self.calls.push(format!("zoom:{pane}"));
            Ok(())
        }

        fn focus_tab(&mut self, tab_id: &str) -> Result<()> {
            self.calls.push(format!("focus_tab:{tab_id}"));
            if self.fail_focus_tab {
                Err(anyhow!("tab focus failed"))
            } else {
                Ok(())
            }
        }

        fn close_tab(&mut self, tab_id: &str) -> Result<()> {
            self.calls.push(format!("close_tab:{tab_id}"));
            Ok(())
        }
    }

    fn picker_paths(node: &LaunchLayoutNode) -> Option<(std::path::PathBuf, std::path::PathBuf)> {
        match node {
            LaunchLayoutNode::Pane { command }
                if command.get(1).is_some_and(|argument| argument == "pick") =>
            {
                Some((command.get(3)?.into(), command.get(5)?.into()))
            }
            LaunchLayoutNode::Split { first, second, .. } => {
                picker_paths(first).or_else(|| picker_paths(second))
            }
            _ => None,
        }
    }

    fn source_layout(zoomed: bool) -> LayoutSnapshot {
        LayoutSnapshot {
            area: Rect::new(0, 0, 80, 24),
            focused_pane_id: Some("w1:p1".into()),
            panes: vec![LayoutPane {
                focused: true,
                pane_id: "w1:p1".into(),
                rect: Rect::new(0, 0, 80, 24),
            }],
            splits: Vec::new(),
            tab_id: Some("w1:t1".into()),
            workspace_id: Some("w1".into()),
            zoomed,
        }
    }

    fn picker_snapshot(zoom_picker: bool) -> PickerSnapshot {
        PickerSnapshot {
            source: SourcePaneSnapshot {
                target_pane_id: PaneId::new("w1:p1"),
                source_tab_id: "w1:t1".into(),
                workspace_id: "w1".into(),
                source_panes: Vec::new(),
                target_content_width: 80,
                target_content_height: 24,
                logical_lines: Vec::new(),
                visible_viewport: Some(VisibleViewport {
                    rows: Vec::new(),
                    logical_lines: Vec::new(),
                    segments: Vec::new(),
                }),
                capture_mode: PaneTextCaptureMode::ExactVisibleUnwrapped,
            },
            session: PickerReturnContext {
                return_tab_id: "w1:t1".into(),
                return_pane_id: PaneId::new("w1:p1"),
                zoom_picker,
            },
            custom_patterns: Vec::new(),
        }
    }

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
        let LaunchLayoutNode::Split {
            direction,
            ratio,
            first,
            second,
        } = converted
        else {
            panic!("expected split layout");
        };
        assert_eq!(direction, SplitDirection::Right);
        assert_eq!(ratio, 0.37);
        assert!(matches!(
            first.as_ref(),
            LaunchLayoutNode::Pane { command }
                if command == &vec!["/a b/π'".to_string(), "idle".to_string()]
        ));
        assert!(matches!(
            second.as_ref(),
            LaunchLayoutNode::Pane { command }
                if command.first().is_some_and(|value| value == "/a b/π'")
                    && command.get(1).is_some_and(|value| value == "pick")
        ));
    }

    #[test]
    fn launch_focuses_picker_before_releasing_barrier() {
        let mut client = FakeClient {
            layout: Some(source_layout(false)),
            ..FakeClient::default()
        };

        launch_layout_tab_picker(
            &mut client,
            &PaneId::new("w1:p1"),
            Path::new("/tmp/herdr pluck"),
            Vec::new(),
        )
        .unwrap();

        assert_eq!(
            client.calls,
            [
                "pane_layout",
                "pane_read:24",
                "apply:w1",
                "focus_pane:w1:p2"
            ]
        );
        let (snapshot, ready) = client.launch_paths.unwrap();
        assert!(ready.exists());
        let files = PickerLaunchFiles {
            snapshot_path: snapshot,
            marker_temp_path: ready.with_extension("ready.tmp"),
            ready_path: ready,
        };
        files.cleanup().unwrap();
    }

    #[test]
    fn failed_focus_compensates_with_returned_tab_id_and_preserves_primary_error() {
        let mut client = FakeClient {
            layout: Some(source_layout(false)),
            fail_focus_pane: true,
            fail_focus_tab: true,
            ..FakeClient::default()
        };

        let error = launch_layout_tab_picker(
            &mut client,
            &PaneId::new("w1:p1"),
            Path::new("/tmp/herdr-pluck"),
            Vec::new(),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "focus failed");
        assert!(client.calls.contains(&"focus_tab:w1:t1".into()));
        assert!(client.calls.contains(&"close_tab:w1:t2".into()));
        let (snapshot, ready) = client.launch_paths.unwrap();
        assert!(!snapshot.exists() && !ready.exists());
    }

    #[test]
    fn cleanup_attempts_close_after_focus_failure_and_rejects_source_tab() {
        let session = picker_snapshot(false).session;
        let mut client = FakeClient {
            fail_focus_tab: true,
            ..FakeClient::default()
        };

        let error = cleanup_session(&mut client, &session, "w1:t2").unwrap_err();

        assert_eq!(error.to_string(), "tab focus failed");
        assert_eq!(client.calls, ["focus_tab:w1:t1", "close_tab:w1:t2"]);
        assert!(cleanup_session(&mut client, &session, "w1:t1")
            .unwrap_err()
            .to_string()
            .contains("refusing"));
    }

    #[test]
    fn zooms_only_when_snapshot_requests_it() {
        let mut client = FakeClient::default();
        zoom_picker(&mut client, &picker_snapshot(false), &PaneId::new("w1:p2")).unwrap();
        zoom_picker(&mut client, &picker_snapshot(true), &PaneId::new("w1:p2")).unwrap();
        assert_eq!(client.calls, ["zoom:w1:p2"]);
    }
}
