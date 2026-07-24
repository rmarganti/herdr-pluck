use crate::model::PaneId;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Output captured from a Herdr CLI command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub success: bool,
}

/// Fakeable command runner for Herdr CLI interactions.
pub trait CommandRunner {
    fn run(&mut self, program: &str, args: &[String]) -> Result<CommandOutput>;
}

/// Production command runner backed by `std::process::Command`.
#[derive(Debug, Default)]
pub struct ProcessCommandRunner;

impl CommandRunner for ProcessCommandRunner {
    fn run(&mut self, program: &str, args: &[String]) -> Result<CommandOutput> {
        let output = Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("failed to run {program} {}", args.join(" ")))?;
        Ok(CommandOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            success: output.status.success(),
        })
    }
}

/// Small Herdr CLI wrapper that validates status and parses command responses.
pub struct HerdrCommands<'a, R> {
    herdr_bin: &'a str,
    runner: &'a mut R,
}

impl<'a, R: CommandRunner> HerdrCommands<'a, R> {
    pub fn new(herdr_bin: &'a str, runner: &'a mut R) -> Self {
        Self { herdr_bin, runner }
    }

    pub fn pane_layout(&mut self, pane: &PaneId) -> Result<Vec<u8>> {
        self.run_checked(vec!["pane", "layout", "--pane", &pane.0])
    }

    pub fn pane_read_visible(&mut self, pane: &PaneId, lines: u16) -> Result<String> {
        let stdout = self.run_checked(vec![
            "pane",
            "read",
            &pane.0,
            "--source",
            "visible",
            "--lines",
            &lines.to_string(),
        ])?;
        Ok(String::from_utf8_lossy(&stdout).into_owned())
    }

    pub fn pane_zoom_on(&mut self, pane: &PaneId) -> Result<()> {
        self.run_checked(vec!["pane", "zoom", &pane.0, "--on"])?;
        Ok(())
    }

    pub fn tab_focus(&mut self, tab_id: &str) -> Result<()> {
        self.run_checked(vec!["tab", "focus", tab_id])?;
        Ok(())
    }

    pub fn tab_close(&mut self, tab_id: &str) -> Result<()> {
        self.run_checked(vec!["tab", "close", tab_id])?;
        Ok(())
    }

    /// Returns Herdr's stable user-editable config directory for an installed plugin.
    pub fn plugin_config_dir(&mut self, plugin_id: &str) -> Result<PathBuf> {
        let stdout = self.run_checked(vec!["plugin", "config-dir", plugin_id])?;
        let path = String::from_utf8(stdout).context("plugin config-dir output was not UTF-8")?;
        Ok(PathBuf::from(path.trim()))
    }

    fn run_checked(&mut self, args: Vec<&str>) -> Result<Vec<u8>> {
        self.run_checked_owned(args.into_iter().map(str::to_string).collect())
    }

    fn run_checked_owned(&mut self, args: Vec<String>) -> Result<Vec<u8>> {
        let output = self.runner.run(self.herdr_bin, &args)?;
        if output.success {
            Ok(output.stdout)
        } else {
            Err(anyhow!(
                "Herdr command `{}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Debug, Default)]
    pub struct FakeRunner {
        pub calls: Vec<Vec<String>>,
        outputs: VecDeque<CommandOutput>,
    }

    impl FakeRunner {
        pub fn push_stdout(&mut self, stdout: impl Into<Vec<u8>>) {
            self.outputs.push_back(CommandOutput {
                stdout: stdout.into(),
                stderr: Vec::new(),
                success: true,
            });
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(&mut self, _program: &str, args: &[String]) -> Result<CommandOutput> {
            self.calls.push(args.to_vec());
            self.outputs
                .pop_front()
                .ok_or_else(|| anyhow!("fake runner had no output for {}", args.join(" ")))
        }
    }

    #[test]
    fn plugin_config_dir_returns_trimmed_path() {
        let mut runner = FakeRunner::default();
        runner.push_stdout("/tmp/plugin-config\n");
        let mut commands = HerdrCommands::new("herdr", &mut runner);

        let path = commands.plugin_config_dir("example.plugin").unwrap();

        assert_eq!(path, PathBuf::from("/tmp/plugin-config"));
        assert_eq!(
            runner.calls[0],
            vec!["plugin", "config-dir", "example.plugin"]
        );
    }
}
