pub mod client;
pub mod context;
pub mod executor;
pub mod exported_layout;
pub mod layout;
mod protocol;
pub mod snapshot;
mod socket;

use crate::config::resolve_pattern_specs;
use crate::herdr::client::SocketHerdrClient;
use crate::herdr::context::HerdrContext;
use crate::herdr::executor::{cleanup_session, launch_layout_tab_picker};
use crate::herdr::snapshot::{read_tab_snapshot_file, wait_for_ready, PickerLaunchFiles};
use crate::model::{PaneId, PickerAction};
use anyhow::{Context, Result};
use crossterm::{cursor, execute, terminal};
use std::io::{stdout, Write};
use std::path::Path;
use std::time::Duration;

pub use layout::derive_layout_recreation_plan;

/// Narrow production adapter for Herdr layout launch and picker cleanup.
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

    pub fn target_pane_from_context(&self) -> Option<PaneId> {
        self.context.target_pane()
    }

    /**
     * Opens the picker that copies any supported visible token.
     */
    pub fn open_copy_picker(&self, target: &PaneId) -> Result<()> {
        self.open_picker(target, PickerAction::Copy)
    }

    /**
     * Opens the picker that launches browser-openable visible URLs.
     */
    pub fn open_url_picker(&self, target: &PaneId) -> Result<()> {
        self.open_picker(target, PickerAction::OpenUrl)
    }

    fn open_picker(&self, target: &PaneId, action: PickerAction) -> Result<()> {
        let binary = std::env::current_exe().context("failed to locate herdr-pluck binary")?;
        let patterns = match action {
            PickerAction::Copy => resolve_pattern_specs(self.context.focused_pane_cwd().as_deref()),
            PickerAction::OpenUrl => Vec::new(),
        };
        let mut client = SocketHerdrClient::from_context(&self.context)?;
        launch_layout_tab_picker(&mut client, target, &binary, action, patterns)?;
        Ok(())
    }

    /// Runs one tab worker; only the coordinator owns input and cleanup.
    pub fn run_tab_worker(
        &self,
        snapshot_path: &Path,
        ready_path: &Path,
        source_pane: &PaneId,
        coordinator: bool,
    ) -> Result<()> {
        let snapshot = read_tab_snapshot_file(snapshot_path)?;
        wait_for_ready(ready_path, Duration::from_secs(10))?;
        if !coordinator {
            return crate::picker::run_tab_renderer(&snapshot, source_pane);
        }
        let temp_tab = self
            .context
            .tab_id
            .clone()
            .context("picker process is missing HERDR_TAB_ID")?;
        let files = PickerLaunchFiles {
            snapshot_path: snapshot_path.to_path_buf(),
            ready_path: ready_path.to_path_buf(),
            marker_temp_path: ready_path.with_extension("ready.tmp"),
        };
        let primary = crate::picker::run_tab_picker(&snapshot, source_pane).map(|_| ());
        let mut client = SocketHerdrClient::from_context(&self.context)?;
        let cleanup = cleanup_session(&mut client, &snapshot.session, &temp_tab);
        let files_cleanup = files.cleanup();
        primary.and(cleanup).and(files_cleanup)
    }
}

/// Clears an inert pane and remains alive until Herdr closes its tab.
pub fn run_idle() -> Result<()> {
    let mut out = stdout();
    execute!(
        out,
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    out.flush()?;
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
