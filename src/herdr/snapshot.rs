use crate::herdr::layout::{derive_source_geometry, LayoutSnapshot};
use crate::model::{
    PaneId, PaneTextCaptureMode, PatternSpec, PickerSnapshot, SourcePaneSnapshot, VisibleViewport,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static SNAPSHOT_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Filesystem-backed picker snapshot handed to the overlay pane process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotFile {
    pub path: PathBuf,
}

/// Builds the immutable picker snapshot from source layout and captured text.
pub fn build_source_snapshot(
    layout: &LayoutSnapshot,
    target: &PaneId,
    logical_lines: Vec<String>,
    visible_viewport: Option<VisibleViewport>,
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
    if !layout.panes.iter().any(|pane| pane.pane_id == target.0) {
        anyhow::bail!("target pane {target} missing from source layout");
    }
    let target_geometry = derive_source_geometry(layout, target);

    Ok(PickerSnapshot {
        source: SourcePaneSnapshot {
            target_pane_id: target.clone(),
            source_tab_id,
            workspace_id,
            tab_area: layout.area,
            target_content_rect: target_geometry.source_content_rect,
            target_content_width: target_geometry.source_content_rect.width,
            target_content_height: target_geometry.source_content_rect.height,
            logical_lines,
            visible_viewport,
            capture_mode: PaneTextCaptureMode::ExactVisibleUnwrapped,
        },
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
    use crate::model::Rect;

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
                tab_area: Rect::new(0, 0, 80, 24),
                target_content_rect: Rect::new(0, 0, 79, 24),
                target_content_width: 79,
                target_content_height: 24,
                logical_lines: vec!["https://example.com".to_string()],
                visible_viewport: None,
                capture_mode: PaneTextCaptureMode::RecentUnwrappedBottomApproximation,
            },
            custom_patterns: Vec::new(),
        }
    }
}
