use clap::Subcommand;

use crate::toolchain::new::NewCmd;
use anyhow::Result;
use zisk_build::ZISK_VERSION_MESSAGE;

// Structure representing the 'sdk' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, args_conflicts_with_subcommands = true, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskSdk {
    #[clap(subcommand)]
    pub command: ZiskSdkCommands,
}

// Enum defining the available subcommands for `ZiskSdk`.
#[derive(Subcommand)]
pub enum ZiskSdkCommands {
    New(NewCmd),
}

impl ZiskSdkCommands {
    pub fn run(&self) -> Result<()> {
        match self {
            ZiskSdkCommands::New(cmd) => cmd.run(),
        }
    }
}
