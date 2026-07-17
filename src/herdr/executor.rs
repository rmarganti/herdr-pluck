use crate::herdr::commands::{CommandRunner, HerdrCommands};
use crate::herdr::context::HERDR_PLUCK_SNAPSHOT_PATH;
use crate::herdr::layout::{derive_source_geometry, parse_layout_snapshot};
use crate::herdr::snapshot::{
    build_source_snapshot, remove_snapshot_file, write_snapshot_file, SnapshotFile,
};
use crate::model::{PaneId, PatternSpec, PickerSnapshot};
use crate::viewport::map_visible_viewport;
use anyhow::Result;

/// Manifest pane entrypoint id that runs picker mode inside the overlay.
pub const PICKER_PANE_ENTRYPOINT: &str = "picker";

/// Result of launching the overlay picker pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayLaunch {
    pub snapshot_file: SnapshotFile,
}

/// Captures the focused source pane and opens the picker as a Herdr overlay pane.
///
/// Herdr attaches overlay panes to the currently active pane, closes them when
/// the picker process exits, and restores focus itself — no explicit cleanup.
pub fn launch_overlay_picker<R: CommandRunner>(
    herdr_bin: &str,
    runner: &mut R,
    target: &PaneId,
    plugin_id: &str,
    custom_patterns: Vec<PatternSpec>,
) -> Result<OverlayLaunch> {
    let layout_bytes = {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        commands.pane_layout(target)?
    };
    let layout = parse_layout_snapshot(&layout_bytes)?;
    let geometry = derive_source_geometry(&layout, target);
    let content_rect = geometry.source_content_rect;
    if content_rect.height == 0 {
        anyhow::bail!("target pane {target} has zero visible content height");
    }

    let visible_text = {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        commands.pane_read_visible(target, content_rect.height)?
    };
    let visible_rows = visible_text.lines().map(str::to_string).collect::<Vec<_>>();
    let visible_viewport =
        map_visible_viewport(visible_rows, content_rect.width, content_rect.height);
    let logical_lines = visible_viewport.logical_lines.clone();

    let snapshot = build_source_snapshot(
        &layout,
        target,
        logical_lines,
        Some(visible_viewport),
        custom_patterns,
    )?;
    let snapshot_file = write_snapshot_file(&snapshot)?;

    let open_result = {
        let mut commands = HerdrCommands::new(herdr_bin, runner);
        commands.plugin_pane_open_overlay(
            plugin_id,
            PICKER_PANE_ENTRYPOINT,
            &[(
                HERDR_PLUCK_SNAPSHOT_PATH,
                snapshot_file.path.display().to_string(),
            )],
            true,
        )
    };
    if let Err(error) = open_result {
        let _ = remove_snapshot_file(&snapshot_file.path);
        return Err(error);
    }

    Ok(OverlayLaunch { snapshot_file })
}

/// Runs picker input/copy flow for a loaded overlay snapshot.
pub fn run_snapshot_picker(snapshot: &PickerSnapshot) -> Result<()> {
    crate::picker::run_picker(snapshot)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::herdr::commands::tests::FakeRunner;
    use crate::herdr::snapshot::read_snapshot_file;
    use crate::model::PatternSpec;

    fn layout_json() -> &'static str {
        r#"{"result":{"layout":{"area":{"x":0,"y":0,"width":100,"height":40},"focused_pane_id":"p2","panes":[{"focused":false,"pane_id":"p1","rect":{"x":0,"y":0,"width":40,"height":40}},{"focused":true,"pane_id":"p2","rect":{"x":40,"y":0,"width":60,"height":40}}],"splits":[{"direction":"right","ratio":0.4,"rect":{"x":0,"y":0,"width":100,"height":40}}],"tab_id":"t1","workspace_id":"w1","zoomed":false}}}"#
    }

    fn zero_height_layout_json() -> &'static str {
        r#"{"result":{"layout":{"area":{"x":0,"y":0,"width":80,"height":0},"focused_pane_id":"p1","panes":[{"focused":true,"pane_id":"p1","rect":{"x":0,"y":0,"width":80,"height":0}}],"splits":[],"tab_id":"t1","workspace_id":"w1","zoomed":false}}}"#
    }

    #[test]
    fn launch_reads_pane_and_opens_overlay_with_snapshot_env() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(layout_json());
        runner.push_stdout("https://example.com\n");
        runner.push_stdout(r#"{"result":{"type":"plugin_pane_opened"}}"#);

        let launch = launch_overlay_picker(
            "herdr",
            &mut runner,
            &PaneId::new("p2"),
            "rmarganti.herdr-pluck",
            vec![PatternSpec {
                name: "custom".to_string(),
                regex: "CUSTOM-[0-9]+".to_string(),
                priority: 25,
            }],
        )
        .unwrap();

        let expected_env = format!(
            "HERDR_PLUCK_SNAPSHOT_PATH={}",
            launch.snapshot_file.path.display()
        );
        assert_eq!(
            runner.calls[2],
            vec![
                "plugin".to_string(),
                "pane".to_string(),
                "open".to_string(),
                "--plugin".to_string(),
                "rmarganti.herdr-pluck".to_string(),
                "--entrypoint".to_string(),
                "picker".to_string(),
                "--placement".to_string(),
                "overlay".to_string(),
                "--env".to_string(),
                expected_env,
                "--focus".to_string(),
            ]
        );

        let snapshot = read_snapshot_file(&launch.snapshot_file.path).unwrap();
        assert_eq!(snapshot.source.target_pane_id, PaneId::new("p2"));
        assert_eq!(snapshot.custom_patterns[0].name, "custom");
        let _ = std::fs::remove_file(launch.snapshot_file.path);
    }

    #[test]
    fn zero_height_target_geometry_fails_before_reading_pane_text() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(zero_height_layout_json());

        let error = launch_overlay_picker(
            "herdr",
            &mut runner,
            &PaneId::new("p1"),
            "rmarganti.herdr-pluck",
            Vec::new(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("zero visible content height"));
        assert_eq!(runner.calls.len(), 1);
        assert_eq!(runner.calls[0][0..2], ["pane", "layout"]);
    }

    #[test]
    fn failed_overlay_open_removes_snapshot_file() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(layout_json());
        runner.push_stdout("https://example.com\n");
        runner.push_failure("no such entrypoint");

        let error = launch_overlay_picker(
            "herdr",
            &mut runner,
            &PaneId::new("p2"),
            "rmarganti.herdr-pluck",
            Vec::new(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("no such entrypoint"));
        let env_arg = runner.calls[2]
            .iter()
            .find(|arg| arg.starts_with("HERDR_PLUCK_SNAPSHOT_PATH="))
            .expect("open call should pass the snapshot path env");
        let path = env_arg.split_once('=').unwrap().1;
        assert!(!std::path::Path::new(path).exists());
    }
}
