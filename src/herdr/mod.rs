pub mod client;
pub mod context;
pub mod executor;
pub mod layout;
mod protocol;
pub mod snapshot;
mod socket;

use crate::config::resolve_pattern_specs;
use crate::herdr::client::SocketHerdrClient;
use crate::herdr::context::HerdrContext;
use crate::herdr::executor::{
    cleanup_session, launch_layout_tab_picker, run_snapshot_picker, zoom_picker,
};
use crate::herdr::snapshot::{read_snapshot_file, wait_for_ready, PickerLaunchFiles};
use crate::model::PaneId;
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

    pub fn open_layout_tab_picker(&self, target: &PaneId) -> Result<()> {
        let binary = std::env::current_exe().context("failed to locate herdr-pluck binary")?;
        let patterns = resolve_pattern_specs(self.context.focused_pane_cwd().as_deref());
        let mut client = SocketHerdrClient::from_context(&self.context)?;
        launch_layout_tab_picker(&mut client, target, &binary, patterns)?;
        Ok(())
    }

    /// Waits for layout completion, runs the picker, and always cleans up explicit resources.
    pub fn run_picker_from_snapshot(&self, snapshot_path: &Path, ready_path: &Path) -> Result<()> {
        let snapshot = read_snapshot_file(snapshot_path)?;
        let temp_tab = self
            .context
            .tab_id
            .clone()
            .context("picker process is missing HERDR_TAB_ID")?;
        let pane = self
            .context
            .pane_id
            .clone()
            .map(PaneId::new)
            .context("picker process is missing HERDR_PANE_ID")?;
        let files = PickerLaunchFiles {
            snapshot_path: snapshot_path.to_path_buf(),
            ready_path: ready_path.to_path_buf(),
            marker_temp_path: ready_path.with_extension("ready.tmp"),
        };
        let mut client = SocketHerdrClient::from_context(&self.context)?;
        let primary = wait_for_ready(ready_path, Duration::from_secs(10))
            .and_then(|_| zoom_picker(&mut client, &snapshot, &pane))
            .and_then(|_| run_snapshot_picker(&snapshot));
        let cleanup = cleanup_session(&mut client, &snapshot.session, &temp_tab);
        let files_cleanup = files.cleanup();
        match primary {
            Err(e) => {
                if let Err(c) = cleanup {
                    eprintln!("cleanup also failed: {c:#}");
                }
                if let Err(c) = files_cleanup {
                    eprintln!("file cleanup also failed: {c:#}");
                }
                Err(e)
            }
            Ok(()) => {
                cleanup?;
                files_cleanup?;
                Ok(())
            }
        }
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
