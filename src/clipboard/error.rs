use thiserror::Error;

/// User-facing failure while copying text to the system clipboard.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClipboardError {
    #[error("no supported clipboard tool found: tried {tried}")]
    NoToolFound { tried: String },
    #[error("failed to start {tool}: {message}")]
    SpawnFailed { tool: String, message: String },
    #[error("failed to write clipboard text to {tool}: {message}")]
    WriteFailed { tool: String, message: String },
    #[error("{tool} exited with status {status}")]
    CommandFailed { tool: String, status: String },
    #[error("failed waiting for {tool}: {message}")]
    WaitFailed { tool: String, message: String },
}

impl ClipboardError {
    pub(crate) fn no_tool_found(tools: &[crate::clipboard::tool::ClipboardTool]) -> Self {
        Self::NoToolFound {
            tried: tools
                .iter()
                .map(|tool| tool.name)
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}
