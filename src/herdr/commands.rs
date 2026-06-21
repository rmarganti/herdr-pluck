use crate::model::{PaneId, SplitDirection};
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
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

    pub fn pane_read_recent_unwrapped(&mut self, pane: &PaneId, lines: u16) -> Result<String> {
        let stdout = self.run_checked(vec![
            "pane",
            "read",
            &pane.0,
            "--source",
            "recent-unwrapped",
            "--lines",
            &lines.to_string(),
        ])?;
        Ok(String::from_utf8_lossy(&stdout).into_owned())
    }

    pub fn tab_create(
        &mut self,
        workspace_id: &str,
        label: &str,
        focus: bool,
    ) -> Result<TabCreateResponse> {
        let mut args = vec![
            "tab".to_string(),
            "create".to_string(),
            "--workspace".to_string(),
            workspace_id.to_string(),
            "--label".to_string(),
            label.to_string(),
        ];
        args.push(if focus { "--focus" } else { "--no-focus" }.to_string());
        let stdout = self.run_checked_owned(args)?;
        parse_json(&stdout, "tab create")
    }

    pub fn pane_split(
        &mut self,
        pane: &PaneId,
        direction: SplitDirection,
        ratio: f32,
        focus: bool,
    ) -> Result<PaneSplitResponse> {
        let mut args = vec![
            "pane".to_string(),
            "split".to_string(),
            pane.0.clone(),
            "--direction".to_string(),
            direction.as_cli_arg().to_string(),
            "--ratio".to_string(),
            ratio.to_string(),
        ];
        args.push(if focus { "--focus" } else { "--no-focus" }.to_string());
        let stdout = self.run_checked_owned(args)?;
        parse_json(&stdout, "pane split")
    }

    pub fn pane_run(&mut self, pane: &PaneId, command: &str) -> Result<()> {
        self.run_checked(vec!["pane", "run", &pane.0, command])?;
        Ok(())
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

#[derive(Debug, Clone, Deserialize)]
struct Envelope<T> {
    result: T,
}

/// Parsed `tab create` response fields used by layout-tab launching.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TabCreateResponse {
    pub tab: TabInfo,
    pub root_pane: PaneInfo,
}

/// Parsed `pane split` response fields used by split replay.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PaneSplitResponse {
    pub pane: PaneInfo,
}

/// Minimal tab metadata returned by Herdr CLI commands.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TabInfo {
    pub tab_id: String,
    pub workspace_id: String,
}

/// Minimal pane metadata returned by Herdr CLI commands.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PaneInfo {
    pub pane_id: String,
    pub tab_id: String,
    pub workspace_id: String,
}

fn parse_json<T: for<'de> Deserialize<'de>>(bytes: &[u8], context: &str) -> Result<T> {
    let envelope: Envelope<T> = serde_json::from_slice(bytes)
        .with_context(|| format!("failed to parse Herdr {context} JSON"))?;
    Ok(envelope.result)
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

        pub fn push_failure(&mut self, stderr: impl Into<Vec<u8>>) {
            self.outputs.push_back(CommandOutput {
                stdout: Vec::new(),
                stderr: stderr.into(),
                success: false,
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
    fn parses_tab_create_response() {
        let mut runner = FakeRunner::default();
        runner.push_stdout(r#"{"result":{"tab":{"tab_id":"w:t2","workspace_id":"w"},"root_pane":{"pane_id":"w:p2","tab_id":"w:t2","workspace_id":"w"}}}"#);
        let mut commands = HerdrCommands::new("herdr", &mut runner);

        let response = commands.tab_create("w", "label", false).unwrap();

        assert_eq!(response.tab.tab_id, "w:t2");
        assert_eq!(response.root_pane.pane_id, "w:p2");
    }
}
