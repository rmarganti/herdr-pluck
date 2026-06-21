use crate::clipboard::{Clipboard, ClipboardError};
use anyhow::{Context, Result};

/// Copies a selected picker match and attaches user-facing context on failure.
pub(crate) fn copy_selected_text(clipboard: &impl Clipboard, text: &str) -> Result<()> {
    clipboard
        .copy(text)
        .map(|_| ())
        .map_err(|error| copy_error(text, error))
}

fn copy_error(text: &str, error: ClipboardError) -> anyhow::Error {
    Err::<(), ClipboardError>(error)
        .with_context(|| format!("failed to copy selected text {text:?} to clipboard"))
        .unwrap_err()
}
