use crate::herdr::client::HerdrClient;
use crate::herdr::layout::derive_source_pane_geometries;
use crate::herdr::snapshot::{wait_for_acknowledgements, PickerLaunchFiles};
use crate::model::{
    PaneDimensions, PaneId, PaneTextCaptureMode, PatternSpec, PickerAction, PickerPaneSnapshot,
    PickerReturnContext, PickerSnapshot, TabPickerSnapshot,
};
use crate::viewport::map_visible_viewport;
use anyhow::{bail, Result};
use std::path::Path;
use std::time::Duration;

/// Captures source state and atomically applies the temporary picker layout.
pub fn launch_layout_tab_picker<C: HerdrClient>(
    client: &mut C,
    target: &PaneId,
    binary_path: &Path,
    action: PickerAction,
    custom_patterns: Vec<PatternSpec>,
) -> Result<()> {
    let layout = client.pane_layout(target)?;
    if layout.zoomed {
        bail!("tab-wide picking does not yet support zoomed source tabs");
    }
    let exported = client.export_layout(target)?;
    let geometries = derive_source_pane_geometries(&layout);
    let mut panes = Vec::with_capacity(geometries.len());
    for geometry in &geometries {
        if geometry.content_height == 0 {
            bail!(
                "source pane {} has zero visible content height",
                geometry.pane_id
            );
        }
        let visible_text = client.pane_read_visible(&geometry.pane_id, geometry.content_height)?;
        let viewport = map_visible_viewport(
            visible_text.lines().map(str::to_string).collect(),
            geometry.content_width,
            geometry.content_height,
        );
        panes.push(PickerPaneSnapshot {
            source_pane_id: geometry.pane_id.clone(),
            content_dimensions: PaneDimensions {
                width: geometry.content_width,
                height: geometry.content_height,
            },
            logical_lines: viewport.logical_lines.clone(),
            visible_viewport: Some(viewport),
            capture_mode: PaneTextCaptureMode::ExactVisibleUnwrapped,
        });
    }

    let coordinator = exported.focused_pane_id.clone();
    if !panes.iter().any(|pane| pane.source_pane_id == coordinator) {
        bail!("focused pane {coordinator} is missing from visible source geometry");
    }
    let return_context = PickerReturnContext {
        return_tab_id: exported.tab_id.clone(),
        return_pane_id: coordinator.clone(),
        zoom_picker: false,
    };
    let snapshot = TabPickerSnapshot {
        panes,
        session: return_context.clone(),
        action,
        custom_patterns,
    };
    let files = PickerLaunchFiles::create(&snapshot)?;
    let mut acknowledgement_paths = Vec::with_capacity(snapshot.panes.len());
    let root = match exported.root.replace_commands(&mut |pane| {
        let acknowledgement = files.acknowledgement_path(acknowledgement_paths.len());
        acknowledgement_paths.push(acknowledgement.clone());
        worker_command(
            pane,
            &coordinator,
            binary_path,
            &files.snapshot_path,
            &files.ready_path,
            &acknowledgement,
        )
    }) {
        Ok(root) => root,
        Err(error) => {
            let _ = files.cleanup();
            return Err(error);
        }
    };
    let workspace_id = &exported.workspace_id;
    // `layout.export` has no tab label, so the temporary tab deliberately keeps Pluck branding.
    let tab_label = match action {
        PickerAction::Copy => "Herdr Pluck",
        PickerAction::OpenUrl => "Herdr Pluck: Open URL",
    };
    let applied = match client.apply_layout(workspace_id, tab_label, &root) {
        Ok(value) => value,
        Err(error) => {
            let _ = files.cleanup();
            return Err(error);
        }
    };

    let coordinator_pane = match applied.pane_ids.get(&coordinator) {
        Some(pane) => pane,
        None => {
            let error = anyhow::anyhow!("layout.apply did not map the coordinator pane");
            if let Err(cleanup) = cleanup_session(client, &return_context, &applied.tab_id) {
                eprintln!("launch cleanup also failed: {cleanup:#}");
            }
            if let Err(cleanup) = files.cleanup() {
                eprintln!("launch file cleanup also failed: {cleanup:#}");
            }
            return Err(error);
        }
    };
    let startup_result = wait_for_acknowledgements(&acknowledgement_paths, Duration::from_secs(10))
        .and_then(|_| client.focus_pane(coordinator_pane))
        .and_then(|_| files.signal_ready());

    if let Err(error) = startup_result {
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

fn worker_command(
    pane: &PaneId,
    coordinator: &PaneId,
    binary: &Path,
    snapshot: &Path,
    ready: &Path,
    acknowledgement: &Path,
) -> Vec<String> {
    vec![
        binary.to_string_lossy().into_owned(),
        if pane == coordinator {
            "coordinate"
        } else {
            "render"
        }
        .into(),
        "--snapshot".into(),
        snapshot.to_string_lossy().into_owned(),
        "--ready".into(),
        ready.to_string_lossy().into_owned(),
        "--acknowledge".into(),
        acknowledgement.to_string_lossy().into_owned(),
        "--source-pane".into(),
        pane.0.clone(),
    ]
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

    #[test]
    fn worker_commands_are_shell_free_and_role_specific() {
        let coordinator = PaneId::new("p1");
        let coordinate = worker_command(
            &coordinator,
            &coordinator,
            Path::new("/a b/pluck"),
            Path::new("/tmp/snapshot"),
            Path::new("/tmp/ready"),
            Path::new("/tmp/ack-1"),
        );
        let render = worker_command(
            &PaneId::new("p2"),
            &coordinator,
            Path::new("/a b/pluck"),
            Path::new("/tmp/snapshot"),
            Path::new("/tmp/ready"),
            Path::new("/tmp/ack-2"),
        );
        assert_eq!(coordinate[1], "coordinate");
        assert_eq!(render[1], "render");
        assert_eq!(render.last().map(String::as_str), Some("p2"));
        assert_eq!(coordinate[0], "/a b/pluck");
    }

    #[test]
    fn cleanup_rejects_the_source_tab() {
        struct Client;
        impl HerdrClient for Client {
            fn pane_layout(&mut self, _: &PaneId) -> Result<crate::herdr::layout::LayoutSnapshot> {
                unreachable!()
            }
            fn pane_read_visible(&mut self, _: &PaneId, _: u16) -> Result<String> {
                unreachable!()
            }
            fn apply_layout(
                &mut self,
                _: &str,
                _: &str,
                _: &crate::herdr::client::LaunchLayoutNode,
            ) -> Result<crate::herdr::client::AppliedLayout> {
                unreachable!()
            }
            fn focus_pane(&mut self, _: &PaneId) -> Result<()> {
                unreachable!()
            }
            fn zoom_pane(&mut self, _: &PaneId) -> Result<()> {
                unreachable!()
            }
            fn focus_tab(&mut self, _: &str) -> Result<()> {
                Ok(())
            }
            fn close_tab(&mut self, _: &str) -> Result<()> {
                Ok(())
            }
        }
        let session = PickerReturnContext {
            return_tab_id: "t1".into(),
            return_pane_id: PaneId::new("p1"),
            zoom_picker: false,
        };
        assert!(cleanup_session(&mut Client, &session, "t1").is_err());
    }
}
