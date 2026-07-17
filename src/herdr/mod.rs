pub mod commands;
pub mod context;
pub mod executor;
pub mod layout;
pub mod snapshot;

use crate::config::{resolve_pattern_specs, PLUGIN_ID};
use crate::herdr::commands::ProcessCommandRunner;
use crate::herdr::context::HerdrContext;
use crate::herdr::executor::{launch_overlay_picker, run_snapshot_picker};
use crate::herdr::snapshot::{read_snapshot_file, remove_snapshot_file};
use crate::model::PaneId;
use anyhow::Result;
use std::path::Path;

pub use context::HERDR_PLUCK_SNAPSHOT_PATH;
pub use layout::parse_layout_snapshot;

/// Narrow production adapter for Herdr overlay-pane launch and picker mode.
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

    /// Launches the production overlay picker for the requested source pane.
    pub fn open_overlay_picker(&self, target: &PaneId) -> Result<()> {
        let mut runner = ProcessCommandRunner;
        let focused_pane_cwd = self.context.focused_pane_cwd();
        let custom_patterns = resolve_pattern_specs(focused_pane_cwd.as_deref());
        let plugin_id = self.context.plugin_id.as_deref().unwrap_or(PLUGIN_ID);
        launch_overlay_picker(
            &self.context.herdr_bin,
            &mut runner,
            target,
            plugin_id,
            custom_patterns,
        )?;
        Ok(())
    }

    /// Runs picker mode from a snapshot file; the overlay closes when this process exits.
    pub fn run_picker_from_snapshot(&self, snapshot_path: &Path) -> Result<()> {
        let snapshot = read_snapshot_file(snapshot_path)?;
        let picker_result = run_snapshot_picker(&snapshot);
        let remove_result = remove_snapshot_file(snapshot_path);

        picker_result?;
        remove_result?;
        Ok(())
    }
}
