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
    /// Action entrypoint: apply an argv-backed temporary picker layout.
    Open {
        /// Override the pane to pluck from. Defaults to Herdr invocation context.
        #[arg(long)]
        target_pane: Option<String>,
    },

    /// Action entrypoint: open a selected visible URL in the default browser.
    OpenUrl {
        /// Override the pane to pluck from. Defaults to Herdr invocation context.
        #[arg(long)]
        target_pane: Option<String>,
    },

    /// Picker entrypoint: run inside the temporary layout-tab target pane.
    Pick {
        /// Temp JSON snapshot path produced by `open`.
        #[arg(long)]
        snapshot: PathBuf,
        /// One-shot launch barrier released after layout application.
        #[arg(long)]
        ready: PathBuf,
    },

    /// Internal shell-free placeholder for non-picker panes.
    #[command(hide = true)]
    Idle,
}

pub fn run() -> Result<()> {
    run_with(Cli::parse())
}

pub fn run_with(cli: Cli) -> Result<()> {
    let adapter = HerdrAdapter::from_env();

    match cli.command {
        Command::Open { target_pane } => {
            let target = resolve_target(&adapter, target_pane)?;
            adapter.open_copy_picker(&target)?;
        }
        Command::OpenUrl { target_pane } => {
            let target = resolve_target(&adapter, target_pane)?;
            adapter.open_url_picker(&target)?;
        }
        Command::Pick { snapshot, ready } => {
            adapter.run_picker_from_snapshot(&snapshot, &ready)?;
        }
        Command::Idle => crate::herdr::run_idle()?,
    }

    Ok(())
}

fn resolve_target(adapter: &HerdrAdapter, target_pane: Option<String>) -> Result<PaneId> {
    target_pane
        .map(PaneId::new)
        .or_else(|| adapter.target_pane_from_context())
        .context("could not determine target pane from --target-pane, HERDR_PANE_ID, HERDR_ACTIVE_PANE_ID, or Herdr context")
}
