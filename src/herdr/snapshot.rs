use crate::herdr::layout::{derive_source_geometry, derive_source_pane_geometries, LayoutSnapshot};
use crate::model::{
    PaneId, PaneTextCaptureMode, PatternSpec, PickerSnapshot, SourcePaneSnapshot, TempTabSession,
    VisibleViewport,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// Filesystem-backed picker snapshot owned by the temporary Herdr session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotFile {
    pub path: PathBuf,
}

/// Chooses the currently supported picker snapshot transport for `pane run` launches.
pub fn choose_picker_snapshot_transport(snapshot: &PickerSnapshot) -> Result<SnapshotTransport> {
    let payload_bytes = serde_json::to_vec(snapshot)
        .context("failed to serialize picker snapshot for transport choice")?
        .len();
    Ok(SnapshotTransportConstraints {
        payload_bytes,
        command_involves_shell: true,
        supports_direct_env: false,
    }
    .choose_transport())
}

/// Builds the immutable picker snapshot from source layout, text, and cleanup session ids.
pub fn build_source_snapshot(
    layout: &LayoutSnapshot,
    target: &PaneId,
    logical_lines: Vec<String>,
    visible_viewport: Option<VisibleViewport>,
    session: TempTabSession,
    custom_patterns: Vec<PatternSpec>,
) -> Result<PickerSnapshot> {
    let source_tab_id = layout
        .tab_id
        .clone()
        .context("pane layout did not include source tab id")?;
    let workspace_id = layout
        .workspace_id
        .clone()
        .context("pane layout did not include workspace id")?;
    let source_panes = derive_source_pane_geometries(layout);
    let target_geometry = derive_source_geometry(layout, target);
    let (target_content_width, target_content_height) = (
        target_geometry.source_content_rect.width,
        target_geometry.source_content_rect.height,
    );
    if !source_panes.iter().any(|pane| pane.pane_id == *target) {
        anyhow::bail!("target pane geometry missing from source layout");
    }

    Ok(PickerSnapshot {
        source: SourcePaneSnapshot {
            target_pane_id: target.clone(),
            source_tab_id,
            workspace_id,
            source_panes,
            target_content_width,
            target_content_height,
            logical_lines,
            visible_viewport,
            capture_mode: PaneTextCaptureMode::ExactVisibleUnwrapped,
        },
        session,
        custom_patterns,
    })
}

/// Writes a picker snapshot to a unique temp JSON file.
pub fn write_snapshot_file(snapshot: &PickerSnapshot) -> Result<SnapshotFile> {
    let json = serde_json::to_vec(snapshot).context("failed to serialize picker snapshot")?;
    let path = unique_snapshot_path();
    fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(SnapshotFile { path })
}

/// Loads a picker snapshot from a temp JSON file.
pub fn read_snapshot_file(path: &Path) -> Result<PickerSnapshot> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

/// Removes a temp snapshot file, ignoring not-found races.
pub fn remove_snapshot_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).with_context(|| format!("failed to remove {}", path.display())),
    }
}

/// Shell-quotes one argument for `pane run`, which submits a command string to the pane shell.
pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// Builds the command string submitted to the temporary picker pane.
pub fn picker_command(binary_path: &Path, snapshot_path: &Path) -> String {
    format!(
        "{} pick --snapshot {}",
        shell_quote(&binary_path.display().to_string()),
        shell_quote(&snapshot_path.display().to_string())
    )
}

/// Quiet long-running command for non-target panes in the temporary layout tab.
pub fn inert_pane_command() -> &'static str {
    "printf '\\033[2J\\033[H'; sleep 86400"
}

fn unique_snapshot_path() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let counter = SNAPSHOT_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "herdr-pluck-{millis}-{}-{counter}.json",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Rect, SourcePaneGeometry};

    #[test]
    fn snapshot_transport_uses_temp_file_for_shell_commands() {
        let constraints = SnapshotTransportConstraints {
            payload_bytes: 512,
            command_involves_shell: true,
            supports_direct_env: true,
        };

        assert_eq!(constraints.choose_transport(), SnapshotTransport::TempFile);
    }

    #[test]
    fn picker_snapshot_transport_is_connected_to_pane_run_constraints() {
        let snapshot = test_snapshot();

        assert_eq!(
            choose_picker_snapshot_transport(&snapshot).unwrap(),
            SnapshotTransport::TempFile
        );
    }

    #[test]
    fn snapshot_file_round_trips() {
        let snapshot = test_snapshot();

        let file = write_snapshot_file(&snapshot).unwrap();
        assert_eq!(read_snapshot_file(&file.path).unwrap(), snapshot);
        remove_snapshot_file(&file.path).unwrap();
    }

    fn test_snapshot() -> PickerSnapshot {
        PickerSnapshot {
            source: SourcePaneSnapshot {
                target_pane_id: PaneId::new("p1"),
                source_tab_id: "t1".to_string(),
                workspace_id: "w1".to_string(),
                source_panes: vec![SourcePaneGeometry {
                    pane_id: PaneId::new("p1"),
                    outer_rect: Rect::new(0, 0, 80, 24),
                    content_rect: Rect::new(0, 0, 79, 24),
                    content_width: 79,
                    content_height: 24,
                }],
                target_content_width: 79,
                target_content_height: 24,
                logical_lines: vec!["https://example.com".to_string()],
                visible_viewport: None,
                capture_mode: PaneTextCaptureMode::RecentUnwrappedBottomApproximation,
            },
            session: TempTabSession {
                temp_tab_id: "t2".to_string(),
                return_tab_id: "t1".to_string(),
                return_pane_id: PaneId::new("p1"),
            },
            custom_patterns: Vec::new(),
        }
    }

    #[test]
    fn picker_command_quotes_paths() {
        let command = picker_command(
            Path::new("/tmp/herdr pluck/bin"),
            Path::new("/tmp/a'b.json"),
        );
        assert_eq!(
            command,
            "'/tmp/herdr pluck/bin' pick --snapshot '/tmp/a'\\''b.json'"
        );
    }
}
