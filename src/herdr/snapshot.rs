use crate::herdr::layout::{derive_source_geometry, derive_source_pane_geometries, LayoutSnapshot};
use crate::model::{
    PaneId, PaneTextCaptureMode, PatternSpec, PickerReturnContext, PickerSnapshot,
    SourcePaneSnapshot, VisibleViewport,
};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Files used to transfer picker state and release its launch barrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerLaunchFiles {
    pub snapshot_path: PathBuf,
    pub ready_path: PathBuf,
    pub(crate) marker_temp_path: PathBuf,
}

impl PickerLaunchFiles {
    /// Allocates unique absent paths and writes the snapshot.
    pub fn create(snapshot: &PickerSnapshot) -> Result<Self> {
        let stem = unique_stem();
        let files = Self {
            snapshot_path: std::env::temp_dir().join(format!("{stem}.json")),
            ready_path: std::env::temp_dir().join(format!("{stem}.ready")),
            marker_temp_path: std::env::temp_dir().join(format!("{stem}.ready.tmp")),
        };
        files.cleanup()?;
        let json = serde_json::to_vec(snapshot).context("failed to serialize picker snapshot")?;
        fs::write(&files.snapshot_path, json)
            .with_context(|| format!("failed to write {}", files.snapshot_path.display()))?;
        Ok(files)
    }

    /// Atomically releases the picker after layout construction completes.
    pub fn signal_ready(&self) -> Result<()> {
        fs::write(&self.marker_temp_path, b"ready")?;
        fs::rename(&self.marker_temp_path, &self.ready_path).with_context(|| {
            format!(
                "failed to signal picker readiness at {}",
                self.ready_path.display()
            )
        })
    }

    /// Removes all launch files, ignoring already-removed files.
    pub fn cleanup(&self) -> Result<()> {
        let mut first = None;
        for path in [
            &self.snapshot_path,
            &self.ready_path,
            &self.marker_temp_path,
        ] {
            if let Err(error) = remove_file(path) {
                if first.is_none() {
                    first = Some(error);
                }
            }
        }
        first.map_or(Ok(()), Err)
    }
}

/// Waits a bounded duration for the launch barrier.
pub fn wait_for_ready(path: &Path, timeout: Duration) -> Result<()> {
    let started = Instant::now();
    while !path.exists() {
        if started.elapsed() >= timeout {
            bail!(
                "timed out waiting for Herdr layout launch barrier at {}",
                path.display()
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

pub fn build_source_snapshot(
    layout: &LayoutSnapshot,
    target: &PaneId,
    logical_lines: Vec<String>,
    visible_viewport: Option<VisibleViewport>,
    session: PickerReturnContext,
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
    let geometry = derive_source_geometry(layout, target);
    if !source_panes.iter().any(|pane| pane.pane_id == *target) {
        bail!("target pane geometry missing from source layout");
    }
    Ok(PickerSnapshot {
        source: SourcePaneSnapshot {
            target_pane_id: target.clone(),
            source_tab_id,
            workspace_id,
            source_panes,
            target_content_width: geometry.source_content_rect.width,
            target_content_height: geometry.source_content_rect.height,
            logical_lines,
            visible_viewport,
            capture_mode: PaneTextCaptureMode::ExactVisibleUnwrapped,
        },
        session,
        custom_patterns,
    })
}

pub fn read_snapshot_file(path: &Path) -> Result<PickerSnapshot> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("failed to parse {}", path.display()))
}

fn remove_file(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("failed to remove {}", path.display())),
    }
}
fn unique_stem() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or_default();
    let counter = FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("herdr-pluck-{millis}-{}-{counter}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn barrier_releases_and_cleanup_is_idempotent() {
        let path = std::env::temp_dir().join(format!("pluck-barrier-test-{}", std::process::id()));
        let _ = fs::remove_file(&path);
        let writer = path.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            fs::write(writer, b"x").unwrap();
        });
        wait_for_ready(&path, Duration::from_secs(1)).unwrap();
        remove_file(&path).unwrap();
        remove_file(&path).unwrap();
    }
    #[test]
    fn barrier_times_out() {
        let path = std::env::temp_dir().join(format!("pluck-missing-{}", std::process::id()));
        let _ = fs::remove_file(&path);
        assert!(wait_for_ready(&path, Duration::from_millis(20))
            .unwrap_err()
            .to_string()
            .contains("timed out"));
    }
}
