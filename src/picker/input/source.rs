use super::{input_event_from_crossterm, PickerInputEvent};
use anyhow::{Context, Result};
use crossterm::{cursor, execute};
use std::io::{self, Write};

/// Source of picker-domain input events.
pub(crate) trait InputSource {
    fn read_event(&mut self) -> Result<PickerInputEvent>;
}

/// Crossterm-backed input source used by production picker mode.
#[derive(Debug, Default)]
pub(crate) struct CrosstermInputSource;

impl InputSource for CrosstermInputSource {
    fn read_event(&mut self) -> Result<PickerInputEvent> {
        let event = crossterm::event::read().context("failed to read picker input event")?;
        Ok(input_event_from_crossterm(event))
    }
}

/// Enables raw mode for picker input and restores terminal mode on drop.
pub(crate) struct RawModeGuard;

impl RawModeGuard {
    pub(crate) fn enable() -> Result<Self> {
        crossterm::terminal::enable_raw_mode().context("failed to enable raw mode for picker")?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Hides the picker cursor and restores it when the picker exits.
pub(crate) struct CursorGuard;

impl CursorGuard {
    pub(crate) fn hide() -> Result<Self> {
        let mut stdout = io::stdout();
        execute!(stdout, cursor::Hide).context("failed to hide picker cursor")?;
        stdout.flush().context("failed to flush hidden cursor")?;
        Ok(Self)
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show);
        let _ = stdout.flush();
    }
}
