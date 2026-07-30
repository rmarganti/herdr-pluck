use super::error::UrlOpenError;
use super::tool::UrlOpenTool;

/**
 * Injectable command seam for URL opener detection and execution.
 */
pub(crate) trait UrlOpenCommandRunner {
    fn command_exists(&self, command: &str) -> bool;
    fn run(&self, tool: UrlOpenTool, url: &str) -> Result<(), UrlOpenError>;
}
