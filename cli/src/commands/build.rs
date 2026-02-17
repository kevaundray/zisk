use anyhow::{anyhow, Context, Result};
use cargo_metadata::MetadataCommand;
use std::process::{Command, Stdio};
use zisk_build::{ZISK_TARGET, ZISK_VERSION_MESSAGE};

// Structure representing the 'build' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
pub struct ZiskBuild {
    #[clap(short = 'F', long)]
    features: Option<String>,

    #[clap(long)]
    all_features: bool,

    #[clap(long)]
    release: bool,

    #[clap(long)]
    no_default_features: bool,

    #[clap(short = 'z', long)]
    zisk_path: Option<String>,

    #[clap(long)]
    hints: bool,
}

impl ZiskBuild {
    pub fn run(&self) -> Result<()> {
        // Construct the cargo build command using the nightly toolchain
        let mut command = Command::new("cargo");
        command.args(["+nightly", "build"]);
        // Add the feature selection flags
        if let Some(features) = &self.features {
            command.arg("--features").arg(features);
        }
        if self.all_features {
            command.arg("--all-features");
        }
        if self.no_default_features {
            command.arg("--no-default-features");
        }
        if self.release {
            command.arg("--release");
        }

        command.args(["--target", ZISK_TARGET]);

        // Set RUSTFLAGS for the standard RISC-V target
        let mut rustflags = String::from("-Cpasses=lower-atomic");
        if let Some(ld_script) = ziskos_linker_script() {
            rustflags.push_str(" -Clink-arg=-T");
            rustflags.push_str(&ld_script);
        }
        command.env("CARGO_TARGET_RISCV64IMAC_UNKNOWN_NONE_ELF_RUSTFLAGS", rustflags);

        // Pass zisk_path to build scripts via environment variable
        if let Some(zisk_path) = &self.zisk_path {
            command.env("ZISK_PATH", zisk_path);
        }

        // Set up the command to inherit the parent's stdout and stderr
        command.stdout(Stdio::inherit());
        command.stderr(Stdio::inherit());

        // Execute the command
        let status = command.status().context("Failed to execute cargo build command")?;
        if !status.success() {
            return Err(anyhow!("Cargo run command failed with status {}", status));
        }

        Ok(())
    }
}

fn ziskos_linker_script() -> Option<String> {
    let metadata = MetadataCommand::new().exec().ok()?;
    let package = metadata.packages.iter().find(|pkg| pkg.name == "ziskos")?;
    let manifest_parent = package.manifest_path.parent()?;
    let ld_script = manifest_parent.join("zisk.ld");
    if ld_script.exists() {
        Some(ld_script.to_string())
    } else {
        None
    }
}
