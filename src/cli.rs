use crate::herdr::{HerdrAdapter, HERDR_PLUCK_SNAPSHOT_PATH};
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
    /// Action entrypoint: capture the focused pane and open the picker overlay.
    Open {
        /// Override the pane to pluck from. Defaults to Herdr invocation context.
        #[arg(long)]
        target_pane: Option<String>,
    },

    /// Picker entrypoint: run inside the Herdr overlay pane.
    Pick {
        /// Temp JSON snapshot path produced by `open`.
        /// Defaults to the HERDR_PLUCK_SNAPSHOT_PATH environment variable.
        #[arg(long)]
        snapshot: Option<PathBuf>,
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

            adapter.open_overlay_picker(&target)?;
        }
        Command::Pick { snapshot } => {
            let snapshot = snapshot
                .or_else(|| std::env::var(HERDR_PLUCK_SNAPSHOT_PATH).ok().map(PathBuf::from))
                .context("picker snapshot path missing: pass --snapshot or set HERDR_PLUCK_SNAPSHOT_PATH")?;
            adapter.run_picker_from_snapshot(&snapshot)?;
        }
    }

    Ok(())
}
