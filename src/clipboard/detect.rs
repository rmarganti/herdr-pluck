use super::runner::ClipboardCommandRunner;
use super::tool::{candidate_tools, ClipboardEnvironment, ClipboardTool};

/// Chooses the first supported clipboard command available in the current session.
pub(crate) fn select_tool(
    runner: &impl ClipboardCommandRunner,
    env: ClipboardEnvironment,
) -> (Option<ClipboardTool>, Vec<ClipboardTool>) {
    let candidates = candidate_tools(env);
    let selected = candidates
        .iter()
        .copied()
        .find(|tool| runner.command_exists(tool.name));
    (selected, candidates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::error::ClipboardError;
    use crate::clipboard::tool::ClipboardOs;
    use std::collections::HashSet;

    #[derive(Default)]
    struct FakeRunner {
        available: HashSet<&'static str>,
    }

    impl ClipboardCommandRunner for FakeRunner {
        fn command_exists(&self, command: &str) -> bool {
            self.available.contains(command)
        }

        fn run_with_stdin(&self, _tool: ClipboardTool, _stdin: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
    }

    #[test]
    fn selects_first_available_candidate() {
        let runner = FakeRunner {
            available: HashSet::from(["xsel"]),
        };

        let (selected, _) = select_tool(
            &runner,
            ClipboardEnvironment {
                os: ClipboardOs::Other,
                wayland: false,
                x11: true,
            },
        );

        assert_eq!(selected.map(|tool| tool.name), Some("xsel"));
    }

    #[test]
    fn reports_no_selection_when_no_tool_exists() {
        let runner = FakeRunner::default();

        let (selected, candidates) = select_tool(
            &runner,
            ClipboardEnvironment {
                os: ClipboardOs::Other,
                wayland: false,
                x11: false,
            },
        );

        assert!(selected.is_none());
        assert_eq!(candidates.len(), 4);
    }
}
