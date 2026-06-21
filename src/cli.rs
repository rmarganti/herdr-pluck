use crate::herdr::HerdrAdapter;
use crate::model::PaneId;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    /// Action entrypoint: recreate the current layout in a temporary picker tab.
    Open {
        /// Override the pane to pluck from. Defaults to Herdr invocation context.
        #[arg(long)]
        target_pane: Option<String>,
    },

    /// Picker entrypoint: run inside the temporary layout-tab target pane.
    Pick {
        /// Temp JSON snapshot path produced by `open`.
        #[arg(long)]
        snapshot: PathBuf,
    },
}

pub fn run() -> Result<()> {
    run_with(Cli::parse())
}

pub fn run_with(cli: Cli) -> Result<()> {
    let adapter = HerdrAdapter::from_env();

    match cli.command {
        Command::Open { target_pane } => {
            let target = target_pane
                .map(PaneId::new)
                .or_else(|| adapter.target_pane_from_context())
                .context("could not determine target pane from --target-pane, HERDR_PANE_ID, HERDR_ACTIVE_PANE_ID, or Herdr context")?;

            adapter.open_layout_tab_picker(&target)?;
        }
        Command::Pick { snapshot } => {
            adapter.run_picker_from_snapshot(&snapshot)?;
        }
    }

    Ok(())
}
