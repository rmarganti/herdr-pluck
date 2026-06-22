pub mod commands;
pub mod context;
pub mod executor;
pub mod layout;
pub mod snapshot;

use crate::config::resolve_pattern_specs;
use crate::herdr::commands::ProcessCommandRunner;
use crate::herdr::context::HerdrContext;
use crate::herdr::executor::{cleanup_session, launch_layout_tab_picker, run_snapshot_picker};
use crate::herdr::snapshot::{read_snapshot_file, remove_snapshot_file};
use crate::model::PaneId;
use anyhow::{Context, Result};
use std::path::Path;

pub use context::{HERDR_PLUCK_SNAPSHOT_JSON, HERDR_PLUCK_TARGET_PANE_ID};
pub use layout::{derive_layout_recreation_plan, parse_layout_snapshot};
pub use snapshot::{SnapshotTransport, SnapshotTransportConstraints};

/// Narrow production adapter for Herdr layout-tab launch and picker cleanup.
#[derive(Debug, Clone)]
pub struct HerdrAdapter {
    context: HerdrContext,
}

impl HerdrAdapter {
    pub fn from_env() -> Self {
        Self {
            context: HerdrContext::from_env(),
        }
    }

    pub fn new(context: HerdrContext) -> Self {
        Self { context }
    }

    pub fn target_pane_from_context(&self) -> Option<PaneId> {
        self.context.target_pane()
    }

    /// Launches the production layout-tab picker for the requested source pane.
    pub fn open_layout_tab_picker(&self, target: &PaneId) -> Result<()> {
        let binary_path = std::env::current_exe().context("failed to locate herdr-pluck binary")?;
        let mut runner = ProcessCommandRunner;
        let focused_pane_cwd = self.context.focused_pane_cwd();
        let custom_patterns = resolve_pattern_specs(focused_pane_cwd.as_deref());
        launch_layout_tab_picker(
            &self.context.herdr_bin,
            &mut runner,
            target,
            &binary_path,
            custom_patterns,
        )?;
        Ok(())
    }

    /// Runs picker placeholder from a snapshot file and then performs session cleanup.
    pub fn run_picker_from_snapshot(&self, snapshot_path: &Path) -> Result<()> {
        let snapshot = read_snapshot_file(snapshot_path)?;
        let picker_result = run_snapshot_picker(&snapshot);
        let mut runner = ProcessCommandRunner;
        let cleanup_result =
            cleanup_session(&self.context.herdr_bin, &mut runner, &snapshot.session);
        let remove_result = remove_snapshot_file(snapshot_path);

        picker_result?;
        cleanup_result?;
        remove_result?;
        Ok(())
    }
}
