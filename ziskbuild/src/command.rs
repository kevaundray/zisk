use crate::{BuildArgs, HELPER_TARGET_SUBDIR, ZISK_TARGET};
use cargo_metadata::camino::Utf8PathBuf;
use std::process::Command;

/// Get the command to build the program locally.
pub(crate) fn create_command(
    args: &BuildArgs,
    program_dir: &Utf8PathBuf,
    program_metadata: &cargo_metadata::Metadata,
) -> Command {
    // Construct the cargo build command using the nightly toolchain
    let mut command = Command::new("cargo");
    command.args(["+nightly", "build"]);
    // Add the feature selection flags
    if let Some(features) = &args.features {
        command.arg("--features").arg(features);
    }
    if args.all_features {
        command.arg("--all-features");
    }

    if args.no_default_features {
        command.arg("--no-default-features");
    }
    if args.release {
        command.arg("--release");
    }

    command.args(["--target", ZISK_TARGET]);

    // Set RUSTFLAGS for the standard RISC-V target
    let mut rustflags = String::from("-Cpasses=lower-atomic");
    if let Some(ld_script) = ziskos_linker_script(program_metadata) {
        rustflags.push_str(" -Clink-arg=-T");
        rustflags.push_str(&ld_script);
    }
    command.env("CARGO_TARGET_RISCV64IMAC_UNKNOWN_NONE_ELF_RUSTFLAGS", rustflags);

    let canonicalized_program_dir =
        program_dir.canonicalize().expect("Failed to canonicalize program directory");
    command.current_dir(canonicalized_program_dir);

    // Use a separate subdirectory to avoid conflicts with the host build
    command.env("CARGO_TARGET_DIR", program_metadata.target_directory.join(HELPER_TARGET_SUBDIR));

    command
}

fn ziskos_linker_script(program_metadata: &cargo_metadata::Metadata) -> Option<String> {
    let package = program_metadata.packages.iter().find(|pkg| pkg.name == "ziskos")?;
    let manifest_parent = package.manifest_path.parent()?;
    let ld_script = manifest_parent.join("zisk.ld");
    if ld_script.exists() {
        Some(ld_script.to_string())
    } else {
        None
    }
}
