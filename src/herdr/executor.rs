use crate::herdr::commands::{CommandRunner, HerdrCommands};
use crate::herdr::layout::{
    derive_layout_recreation_plan, derive_source_geometry, parse_layout_snapshot,
};
use crate::herdr::snapshot::{
    build_source_snapshot, choose_picker_snapshot_transport, inert_pane_command, picker_command,
    remove_snapshot_file, write_snapshot_file, SnapshotFile, SnapshotTransport,
};
use crate::model::{LayoutNode, PaneId, PickerSnapshot, TempTabSession};
use crate::viewport::map_visible_viewport;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Result of launching a temporary layout-tab picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutTabLaunch {
    pub session: TempTabSession,
    pub target_temp_pane_id: PaneId,
    pub snapshot_file: SnapshotFile,
    pub pane_mapping: HashMap<PaneId, PaneId>,
}

/// Captures the source pane, recreates its tab layout, and launches picker mode in the target pane.
pub fn launch_layout_tab_picker<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    target: &PaneId,
    binary_path: &Path,
) -> Result<LayoutTabLaunch> {
    let layout_bytes = {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        commands.pane_layout(target)?
    };
    let layout = parse_layout_snapshot(&layout_bytes)?;
    let plan = derive_layout_recreation_plan(&layout, target)?;

    let read_lines = derive_source_geometry(&layout, target)
        .source_content_rect
        .height;
    if read_lines == 0 {
        anyhow::bail!("target pane {target} has zero visible content height");
    }
    let visible_text = {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        commands.pane_read_visible(target, read_lines)?
    };
    let visible_rows = visible_text.lines().map(str::to_string).collect::<Vec<_>>();
    let visible_viewport = map_visible_viewport(
        visible_rows,
        derive_source_geometry(&layout, target)
            .source_content_rect
            .width,
        read_lines,
    );
    let logical_lines = visible_viewport.logical_lines.clone();

    let workspace_id = layout
        .workspace_id
        .as_deref()
        .context("pane layout did not include workspace id")?;
    let return_tab_id = layout
        .tab_id
        .clone()
        .context("pane layout did not include return tab id")?;

    let tab = {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        commands.tab_create(workspace_id, "Herdr Pluck", true)?
    };
    let session = TempTabSession {
        temp_tab_id: tab.tab.tab_id.clone(),
        return_tab_id,
        return_pane_id: target.clone(),
    };

    let replay_result = replay_layout_tree(
        herdr_bin,
        runner,
        &plan.root,
        PaneId::new(tab.root_pane.pane_id.clone()),
        target,
    )
    .inspect_err(|_| {
        let _ = cleanup_session(herdr_bin, runner, &session);
    })?;

    let target_temp_pane_id = replay_result
        .pane_mapping
        .get(target)
        .cloned()
        .ok_or_else(|| anyhow!("layout replay did not create a temp pane for target {target}"))?;

    let snapshot = build_source_snapshot(
        &layout,
        target,
        logical_lines,
        Some(visible_viewport),
        session.clone(),
    )?;
    let snapshot_file = match choose_picker_snapshot_transport(&snapshot)? {
        SnapshotTransport::TempFile => write_snapshot_file(&snapshot)?,
        SnapshotTransport::EnvJson => {
            anyhow::bail!("env-json picker snapshot transport is not supported for pane run")
        }
    };

    if layout.zoomed && layout_target_is_focused(&layout, target) {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        if let Err(error) = commands.pane_zoom_on(&target_temp_pane_id) {
            cleanup_failed_launch(herdr_bin, runner, &session, &snapshot_file);
            return Err(error);
        }
    }

    if let Err(error) = launch_pane_commands(
        herdr_bin,
        runner,
        binary_path,
        &target_temp_pane_id,
        &snapshot_file,
        &replay_result.pane_mapping,
    ) {
        cleanup_failed_launch(herdr_bin, runner, &session, &snapshot_file);
        return Err(error);
    }

    Ok(LayoutTabLaunch {
        session,
        target_temp_pane_id,
        snapshot_file,
        pane_mapping: replay_result.pane_mapping,
    })
}

fn cleanup_failed_launch<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    session: &TempTabSession,
    snapshot_file: &SnapshotFile,
) {
    let _ = cleanup_session(herdr_bin, runner, session);
    let _ = remove_snapshot_file(&snapshot_file.path);
}

fn layout_target_is_focused(
    layout: &crate::herdr::layout::LayoutSnapshot,
    target: &PaneId,
) -> bool {
    layout
        .focused_pane_id
        .as_ref()
        .is_some_and(|focused| focused == &target.0)
        || layout
            .panes
            .iter()
            .any(|pane| pane.pane_id == target.0 && pane.focused)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayResult {
    pane_mapping: HashMap<PaneId, PaneId>,
}

fn replay_layout_tree<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    root: &LayoutNode,
    root_temp_pane: PaneId,
    target: &PaneId,
) -> Result<ReplayResult> {
    let mut pane_mapping = HashMap::new();
    replay_node(
        herdr_bin,
        runner,
        root,
        root_temp_pane,
        target,
        &mut pane_mapping,
    )?;
    Ok(ReplayResult { pane_mapping })
}

fn replay_node<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    node: &LayoutNode,
    current_temp_pane: PaneId,
    target: &PaneId,
    pane_mapping: &mut HashMap<PaneId, PaneId>,
) -> Result<()> {
    match node {
        LayoutNode::Pane { source_pane_id, .. } => {
            pane_mapping.insert(source_pane_id.clone(), current_temp_pane);
            Ok(())
        }
        LayoutNode::Split {
            direction,
            ratio,
            first,
            second,
            ..
        } => {
            let target_in_second = node_contains_source(second, target);
            let split = {
                let mut commands = HerdrCommands::new(herdr_bin, runner);
                commands.pane_split(&current_temp_pane, *direction, *ratio, target_in_second)?
            };
            let second_temp_pane = PaneId::new(split.pane.pane_id);
            replay_node(
                herdr_bin,
                runner,
                first,
                current_temp_pane,
                target,
                pane_mapping,
            )?;
            replay_node(
                herdr_bin,
                runner,
                second,
                second_temp_pane,
                target,
                pane_mapping,
            )
        }
    }
}

fn node_contains_source(node: &LayoutNode, target: &PaneId) -> bool {
    match node {
        LayoutNode::Pane { source_pane_id, .. } => source_pane_id == target,
        LayoutNode::Split { first, second, .. } => {
            node_contains_source(first, target) || node_contains_source(second, target)
        }
    }
}

fn launch_pane_commands<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    binary_path: &Path,
    target_temp_pane_id: &PaneId,
    snapshot_file: &SnapshotFile,
    pane_mapping: &HashMap<PaneId, PaneId>,
) -> Result<()> {
    let picker_command = picker_command(binary_path, &snapshot_file.path);
    for temp_pane in pane_mapping.values() {
        let command = if temp_pane == target_temp_pane_id {
            picker_command.as_str()
        } else {
            inert_pane_command()
        };
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        commands.pane_run(temp_pane, command)?;
    }
    Ok(())
}

/// Attempts to restore focus and close the temporary tab by explicit session ids.
pub fn cleanup_session<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    session: &TempTabSession,
) -> Result<()> {
    let mut first_error = None;
    {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        if let Err(error) = commands.tab_focus(&session.return_tab_id) {
            first_error = Some(error);
        }
    }
    {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        if let Err(error) = commands.tab_close(&session.temp_tab_id) {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Runs picker input/copy flow for a loaded layout-tab snapshot.
pub fn run_snapshot_picker(snapshot: &PickerSnapshot) -> Result<()> {
    crate::picker::run_picker(snapshot)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::commands::tests::FakeRunner;
    use crate::model::{Rect, SplitDirection};

    fn layout_json() -> &'static str {
        r#"{"result":{"layout":{"area":{"x":0,"y":0,"width":100,"height":40},"focused_pane_id":"p2","panes":[{"focused":false,"pane_id":"p1","rect":{"x":0,"y":0,"width":40,"height":40}},{"focused":true,"pane_id":"p2","rect":{"x":40,"y":0,"width":60,"height":40}}],"splits":[{"direction":"right","ratio":0.4,"rect":{"x":0,"y":0,"width":100,"height":40}}],"tab_id":"t1","workspace_id":"w1","zoomed":false}}}"#
    }

    fn zoomed_layout_json() -> &'static str {
        r#"{"result":{"layout":{"area":{"x":0,"y":0,"width":100,"height":40},"focused_pane_id":"p2","panes":[{"focused":false,"pane_id":"p1","rect":{"x":0,"y":0,"width":40,"height":40}},{"focused":true,"pane_id":"p2","rect":{"x":40,"y":0,"width":60,"height":40}}],"splits":[{"direction":"right","ratio":0.4,"rect":{"x":0,"y":0,"width":100,"height":40}}],"tab_id":"t1","workspace_id":"w1","zoomed":true}}}"#
    }

    fn zero_height_layout_json() -> &'static str {
        r#"{"result":{"layout":{"area":{"x":0,"y":0,"width":80,"height":0},"focused_pane_id":"p1","panes":[{"focused":true,"pane_id":"p1","rect":{"x":0,"y":0,"width":80,"height":0}}],"splits":[],"tab_id":"t1","workspace_id":"w1","zoomed":false}}}"#
    }

    #[test]
    fn replay_split_tree_maps_source_to_temp_panes() {
        let root = LayoutNode::Split {
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
        };
        let mut runner = FakeRunner::default();
        runner.push_stdout(
            r#"{"result":{"pane":{"pane_id":"temp2","tab_id":"tt","workspace_id":"w1"}}}"#,
        );

        let result = replay_layout_tree(
            "herdr",
            &mut runner,
            &root,
            PaneId::new("temp1"),
            &PaneId::new("p2"),
        )
        .unwrap();

        assert_eq!(
            result.pane_mapping.get(&PaneId::new("p1")),
            Some(&PaneId::new("temp1"))
        );
        assert_eq!(
            result.pane_mapping.get(&PaneId::new("p2")),
            Some(&PaneId::new("temp2"))
        );
        assert_eq!(
            runner.calls[0],
            vec![
                "pane",
                "split",
                "temp1",
                "--direction",
                "right",
                "--ratio",
                "0.4",
                "--focus"
            ]
        );
    }

    #[test]
    fn launch_creates_tab_replays_layout_and_runs_picker() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(layout_json());
        runner.push_stdout("https://example.com\n");
        runner.push_stdout(r#"{"result":{"tab":{"tab_id":"tt","workspace_id":"w1"},"root_pane":{"pane_id":"tp1","tab_id":"tt","workspace_id":"w1"}}}"#);
        runner.push_stdout(
            r#"{"result":{"pane":{"pane_id":"tp2","tab_id":"tt","workspace_id":"w1"}}}"#,
        );
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);

        let launch = launch_layout_tab_picker(
            "herdr",
            &mut runner,
            &PaneId::new("p2"),
            Path::new("/bin/herdr-pluck"),
        )
        .unwrap();

        assert_eq!(launch.target_temp_pane_id, PaneId::new("tp2"));
        let expected_picker_command = "'/bin/herdr-pluck' pick --snapshot ".to_string()
            + &crate::herdr::snapshot::shell_quote(
                &launch.snapshot_file.path.display().to_string(),
            );
        assert!(runner.calls.iter().any(|call| call
            == &vec![
                "pane".to_string(),
                "run".to_string(),
                "tp2".to_string(),
                expected_picker_command.clone(),
            ]));
        let _ = std::fs::remove_file(launch.snapshot_file.path);
    }

    #[test]
    fn zero_height_target_geometry_fails_before_reading_pane_text() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(zero_height_layout_json());

        let error = launch_layout_tab_picker(
            "herdr",
            &mut runner,
            &PaneId::new("p1"),
            Path::new("/bin/herdr-pluck"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("zero visible content height"));
        assert_eq!(runner.calls.len(), 1);
        assert_eq!(runner.calls[0][0..2], ["pane", "layout"]);
    }

    #[test]
    fn launch_zooms_temp_target_when_source_target_is_zoomed() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(zoomed_layout_json());
        runner.push_stdout("https://example.com\n");
        runner.push_stdout(r#"{"result":{"tab":{"tab_id":"tt","workspace_id":"w1"},"root_pane":{"pane_id":"tp1","tab_id":"tt","workspace_id":"w1"}}}"#);
        runner.push_stdout(
            r#"{"result":{"pane":{"pane_id":"tp2","tab_id":"tt","workspace_id":"w1"}}}"#,
        );
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);

        let launch = launch_layout_tab_picker(
            "herdr",
            &mut runner,
            &PaneId::new("p2"),
            Path::new("/bin/herdr-pluck"),
        )
        .unwrap();

        let zoom_call = vec![
            "pane".to_string(),
            "zoom".to_string(),
            "tp2".to_string(),
            "--on".to_string(),
        ];
        let zoom_index = runner
            .calls
            .iter()
            .position(|call| call == &zoom_call)
            .expect("expected temp target pane to be zoomed");
        let first_run_index = runner
            .calls
            .iter()
            .position(|call| call.get(1).is_some_and(|arg| arg == "run"))
            .expect("expected pane run commands");
        assert!(zoom_index < first_run_index);

        let snapshot = crate::herdr::snapshot::read_snapshot_file(&launch.snapshot_file.path)
            .expect("snapshot should be readable");
        assert_eq!(snapshot.source.target_content_width, 97);
        assert_eq!(snapshot.source.target_content_height, 38);
        let _ = std::fs::remove_file(launch.snapshot_file.path);
    }

    #[test]
    fn zoom_failure_cleans_up_session() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(zoomed_layout_json());
        runner.push_stdout("https://example.com\n");
        runner.push_stdout(r#"{"result":{"tab":{"tab_id":"tt","workspace_id":"w1"},"root_pane":{"pane_id":"tp1","tab_id":"tt","workspace_id":"w1"}}}"#);
        runner.push_stdout(
            r#"{"result":{"pane":{"pane_id":"tp2","tab_id":"tt","workspace_id":"w1"}}}"#,
        );
        runner.push_failure("zoom failed");
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);

        let error = launch_layout_tab_picker(
            "herdr",
            &mut runner,
            &PaneId::new("p2"),
            Path::new("/bin/herdr-pluck"),
        )
        .unwrap_err();

        assert!(error.to_string().contains("zoom failed"));
        assert!(runner
            .calls
            .iter()
            .any(|call| call == &vec!["tab".to_string(), "focus".to_string(), "t1".to_string(),]));
        assert!(runner
            .calls
            .iter()
            .any(|call| call == &vec!["tab".to_string(), "close".to_string(), "tt".to_string(),]));
    }

    #[test]
    fn failed_launch_cleanup_removes_snapshot_file() {
        let path =
            std::env::temp_dir().join(format!("herdr-pluck-cleanup-test-{}", std::process::id()));
        std::fs::write(&path, b"snapshot").unwrap();
        let snapshot_file = SnapshotFile { path: path.clone() };
        let session = TempTabSession {
            temp_tab_id: "tt".to_string(),
            return_tab_id: "t1".to_string(),
            return_pane_id: PaneId::new("p1"),
        };
        let mut runner = FakeRunner::default();
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);
        runner.push_stdout(r#"{"result":{"type":"ok"}}"#);

        cleanup_failed_launch("herdr", &mut runner, &session, &snapshot_file);

        assert!(!path.exists());
    }
}
