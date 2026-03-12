use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::ux::print_banner;
use crate::ux::print_banner_field;
use zisk_sdk::{setup_logger, ZiskStdin};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ZiskConvertInput {
    #[clap(short = 'i', long)]
    pub input_path: PathBuf,

    /// Output path
    #[clap(short = 'o', long)]
    pub output_path: PathBuf,

    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8,
}

impl ZiskConvertInput {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        print_banner();

        print_banner_field("Command", "Convert Input");
        print_banner_field("Input", self.input_path.display());
        print_banner_field("Output", self.output_path.display());

        let input = std::fs::read(&self.input_path)?;
        let zisk_stdin = ZiskStdin::new();
        zisk_stdin.write_slice(&input);
        zisk_stdin.read_bytes();
        zisk_stdin.save(&self.output_path)?;

        println!("Input conversion completed successfully!");

        Ok(())
    }
}
