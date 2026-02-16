fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target == "riscv64imac-unknown-none-elf" {
        let ld_script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("zisk.ld");
        // Pass the linker script to the linker for any binary that depends on ziskos
        println!("cargo:rustc-link-arg=-T{}", ld_script.display());
        // Also export the path for downstream build scripts via DEP_ZISKOS_LD_SCRIPT
        println!("cargo:ld_script={}", ld_script.display());
        println!("cargo:rerun-if-changed=zisk.ld");
    }
}
