use super::error::ClipboardError;
use super::tool::ClipboardTool;

/// Injectable command boundary for clipboard detection and copy execution.
pub(crate) trait ClipboardCommandRunner {
    fn command_exists(&self, command: &str) -> bool;
    fn run_with_stdin(&self, tool: ClipboardTool, stdin: &str) -> Result<(), ClipboardError>;
}
