use crate::url_opener::{UrlOpenError, UrlOpener};
use anyhow::{Context, Result};

/**
 * Opens a selected picker URL and attaches user-facing context on failure.
 */
pub(crate) fn open_selected_url(opener: &impl UrlOpener, url: &str) -> Result<()> {
    opener
        .open(url)
        .map(|_| ())
        .map_err(|error| open_error(url, error))
}

fn open_error(url: &str, error: UrlOpenError) -> anyhow::Error {
    Err::<(), UrlOpenError>(error)
        .with_context(|| format!("failed to open selected URL {url:?}"))
        .unwrap_err()
}
