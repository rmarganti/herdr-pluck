use super::error::UrlOpenError;
use super::runner::UrlOpenCommandRunner;
use super::tool::UrlOpenTool;
use std::process::{Command, Stdio};

/**
 * Process-backed URL opener command runner.
 */
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SystemCommandRunner;

impl UrlOpenCommandRunner for SystemCommandRunner {
    fn command_exists(&self, command: &str) -> bool {
        which::which(command).is_ok()
    }

    fn run(&self, tool: UrlOpenTool, url: &str) -> Result<(), UrlOpenError> {
        let mut child = Command::new(tool.name)
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| UrlOpenError::SpawnFailed {
                tool: tool.name.to_string(),
                message: error.to_string(),
            })?;
        let status = child.wait().map_err(|error| UrlOpenError::WaitFailed {
            tool: tool.name.to_string(),
            message: error.to_string(),
        })?;
        if status.success() {
            Ok(())
        } else {
            Err(UrlOpenError::CommandFailed {
                tool: tool.name.to_string(),
                status: status.to_string(),
            })
        }
    }
}
