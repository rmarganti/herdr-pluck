mod detect;
mod error;
mod runner;
mod system;
mod tool;

pub use error::ClipboardError;

use detect::select_tool;
use runner::ClipboardCommandRunner;
use system::SystemCommandRunner;
use tool::ClipboardEnvironment;

/// Successful clipboard copy metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopySuccess {
    pub tool: String,
}

/// Clipboard abstraction used by picker code and tests.
pub trait Clipboard {
    fn copy(&self, text: &str) -> Result<CopySuccess, ClipboardError>;
}

/// System clipboard implementation using available platform command-line tools.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClipboard;

impl Clipboard for SystemClipboard {
    fn copy(&self, text: &str) -> Result<CopySuccess, ClipboardError> {
        copy_with_runner(text, &SystemCommandRunner, ClipboardEnvironment::current())
    }
}

/// Copies text to the system clipboard with the default fallback adapter.
pub fn copy_to_system_clipboard(text: &str) -> Result<CopySuccess, ClipboardError> {
    SystemClipboard.copy(text)
}

fn copy_with_runner(
    text: &str,
    runner: &impl ClipboardCommandRunner,
    env: ClipboardEnvironment,
) -> Result<CopySuccess, ClipboardError> {
    let (selected, candidates) = select_tool(runner, env);
    let Some(tool) = selected else {
        return Err(ClipboardError::no_tool_found(&candidates));
    };

    runner.run_with_stdin(tool, text)?;
    Ok(CopySuccess {
        tool: tool.name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::tool::{ClipboardOs, ClipboardTool};
    use std::cell::RefCell;
    use std::collections::HashSet;

    #[derive(Default)]
    struct FakeRunner {
        available: HashSet<&'static str>,
        runs: RefCell<Vec<(&'static str, Vec<&'static str>, String)>>,
        failure: Option<ClipboardError>,
    }

    impl ClipboardCommandRunner for FakeRunner {
        fn command_exists(&self, command: &str) -> bool {
            self.available.contains(command)
        }

        fn run_with_stdin(&self, tool: ClipboardTool, stdin: &str) -> Result<(), ClipboardError> {
            self.runs
                .borrow_mut()
                .push((tool.name, tool.args.to_vec(), stdin.to_string()));
            if let Some(error) = &self.failure {
                Err(error.clone())
            } else {
                Ok(())
            }
        }
    }

    fn env(os: ClipboardOs, wayland: bool, x11: bool) -> ClipboardEnvironment {
        ClipboardEnvironment { os, wayland, x11 }
    }

    #[test]
    fn copies_with_pbcopy_on_macos() {
        let runner = FakeRunner {
            available: HashSet::from(["pbcopy"]),
            ..FakeRunner::default()
        };

        let success = copy_with_runner(
            "https://example.com",
            &runner,
            env(ClipboardOs::Macos, false, false),
        )
        .unwrap();

        assert_eq!(success.tool, "pbcopy");
        assert_eq!(
            runner.runs.borrow().as_slice(),
            &[("pbcopy", Vec::new(), "https://example.com".to_string())]
        );
    }

    #[test]
    fn copies_with_wayland_tool_when_available() {
        let runner = FakeRunner {
            available: HashSet::from(["wl-copy", "xclip"]),
            ..FakeRunner::default()
        };

        let success =
            copy_with_runner("token", &runner, env(ClipboardOs::Other, true, true)).unwrap();

        assert_eq!(success.tool, "wl-copy");
        assert_eq!(runner.runs.borrow()[0].0, "wl-copy");
    }

    #[test]
    fn copies_with_xclip_arguments_on_x11() {
        let runner = FakeRunner {
            available: HashSet::from(["xclip"]),
            ..FakeRunner::default()
        };

        let success =
            copy_with_runner("/tmp/file", &runner, env(ClipboardOs::Other, false, true)).unwrap();

        assert_eq!(success.tool, "xclip");
        assert_eq!(
            runner.runs.borrow().as_slice(),
            &[(
                "xclip",
                vec!["-selection", "clipboard"],
                "/tmp/file".to_string()
            )]
        );
    }

    #[test]
    fn copies_with_xsel_arguments_when_xclip_missing() {
        let runner = FakeRunner {
            available: HashSet::from(["xsel"]),
            ..FakeRunner::default()
        };

        let success =
            copy_with_runner("abcdef1", &runner, env(ClipboardOs::Other, false, true)).unwrap();

        assert_eq!(success.tool, "xsel");
        assert_eq!(
            runner.runs.borrow().as_slice(),
            &[(
                "xsel",
                vec!["--clipboard", "--input"],
                "abcdef1".to_string()
            )]
        );
    }

    #[test]
    fn reports_no_supported_tool_with_tried_list() {
        let runner = FakeRunner::default();

        let error =
            copy_with_runner("unused", &runner, env(ClipboardOs::Other, false, false)).unwrap_err();

        assert_eq!(
            error,
            ClipboardError::NoToolFound {
                tried: "pbcopy, wl-copy, xclip, xsel".to_string()
            }
        );
        assert!(runner.runs.borrow().is_empty());
    }

    #[test]
    fn surfaces_command_execution_failure() {
        let runner = FakeRunner {
            available: HashSet::from(["wl-copy"]),
            failure: Some(ClipboardError::CommandFailed {
                tool: "wl-copy".to_string(),
                status: "exit status: 1".to_string(),
            }),
            ..FakeRunner::default()
        };

        let error =
            copy_with_runner("token", &runner, env(ClipboardOs::Other, true, false)).unwrap_err();

        assert_eq!(
            error,
            ClipboardError::CommandFailed {
                tool: "wl-copy".to_string(),
                status: "exit status: 1".to_string(),
            }
        );
        assert_eq!(runner.runs.borrow()[0].2, "token");
    }
}
