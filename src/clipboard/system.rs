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
        let mut command = clipboard_command(tool);
        let mut child =
            command
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

#[cfg(unix)]
fn clipboard_command(tool: ClipboardTool) -> Command {
    let mut command = Command::new("setsid");
    command.arg(tool.name).args(tool.args);
    command
}

#[cfg(not(unix))]
fn clipboard_command(tool: ClipboardTool) -> Command {
    let mut command = Command::new(tool.name);
    command.args(tool.args);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn clipboard_command_is_wrapped_with_setsid_on_unix() {
        let command = clipboard_command(ClipboardTool {
            name: "wl-copy",
            args: &["--trim-newline"],
        });

        assert_eq!(command.get_program(), "setsid");
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            vec!["wl-copy", "--trim-newline"]
        );
    }
}
