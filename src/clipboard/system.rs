use super::error::ClipboardError;
use super::runner::ClipboardCommandRunner;
use super::tool::ClipboardTool;
use std::process::{Command, Stdio};

/// `std::process`-backed clipboard command runner for normal operation.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SystemCommandRunner;

impl ClipboardCommandRunner for SystemCommandRunner {
    fn command_exists(&self, command: &str) -> bool {
        which::which(command).is_ok()
    }

    fn run_with_stdin(&self, tool: ClipboardTool, stdin: &str) -> Result<(), ClipboardError> {
        let mut child = Command::new(tool.name)
            .args(tool.args)
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|err| ClipboardError::SpawnFailed {
                tool: tool.name.to_string(),
                message: err.to_string(),
            })?;

        if let Some(mut child_stdin) = child.stdin.take() {
            use std::io::Write;
            child_stdin
                .write_all(stdin.as_bytes())
                .map_err(|err| ClipboardError::WriteFailed {
                    tool: tool.name.to_string(),
                    message: err.to_string(),
                })?;
        }

        let status = child.wait().map_err(|err| ClipboardError::WaitFailed {
            tool: tool.name.to_string(),
            message: err.to_string(),
        })?;

        if status.success() {
            Ok(())
        } else {
            Err(ClipboardError::CommandFailed {
                tool: tool.name.to_string(),
                status: status.to_string(),
            })
        }
    }
}
