use crate::herdr::{HerdrAdapter, HERDR_PLUCK_TARGET_PANE_ID};
use crate::model::PaneId;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "herdr-pluck",
    version,
    about = "Inline hint picker for Herdr panes"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Action entrypoint: capture the target pane and open the picker overlay.
    OpenOverlay {
        /// Override the pane to pluck from. Defaults to Herdr invocation context.
        #[arg(long)]
        target_pane: Option<String>,
    },

    /// Pane entrypoint: run the picker inside the Herdr overlay pane.
    Pick {
        /// Override the pane to pluck from. Defaults to HERDR_PLUCK_TARGET_PANE_ID.
        #[arg(long, env = HERDR_PLUCK_TARGET_PANE_ID)]
        target_pane: Option<String>,
    },
}

pub fn run() -> Result<()> {
    run_with(Cli::parse())
}

pub fn run_with(cli: Cli) -> Result<()> {
    let adapter = HerdrAdapter::from_env();

    match cli.command {
        Command::OpenOverlay { target_pane } => {
            let target = target_pane
                .map(PaneId::new)
                .or_else(|| adapter.target_pane_from_context())
                .context("could not determine target pane from --target-pane, HERDR_PANE_ID, HERDR_ACTIVE_PANE_ID, or Herdr context")?;

            adapter.open_picker_overlay(&target)?;
        }
        Command::Pick { target_pane } => {
            let target = target_pane
                .map(PaneId::new)
                .context("picker requires --target-pane or HERDR_PLUCK_TARGET_PANE_ID")?;

            adapter.run_picker_placeholder(&target)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_open_overlay_mode() {
        let cli = Cli::parse_from(["herdr-pluck", "open-overlay", "--target-pane", "pane-1"]);
        assert_eq!(
            cli.command,
            Command::OpenOverlay {
                target_pane: Some("pane-1".to_string())
            }
        );
    }

    #[test]
    fn parses_picker_mode() {
        let cli = Cli::parse_from(["herdr-pluck", "pick", "--target-pane", "pane-2"]);
        assert_eq!(
            cli.command,
            Command::Pick {
                target_pane: Some("pane-2".to_string())
            }
        );
    }
}
