mod build_support;

use std::env;
use std::fs;
use std::path::PathBuf;

use build_support::profile_output_dir;

const VENDOR_DLL: &str = "third_party/CP210xRuntime.dll";
const DLL_NAME: &str = "CP210xRuntime.dll";

fn main() {
    if env::var_os("CARGO_CFG_WINDOWS").is_none() || env::var_os("CARGO_FEATURE_CLI").is_none() {
        return;
    }

    println!("cargo:rerun-if-changed={VENDOR_DLL}");

    if let Err(error) = stage_runtime_dll() {
        panic!("failed to stage {DLL_NAME}: {error}");
    }
}

fn stage_runtime_dll() -> Result<(), String> {
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .ok_or_else(|| "CARGO_MANIFEST_DIR is not set".to_owned())?;
    let source = PathBuf::from(manifest_dir).join(VENDOR_DLL);

    if !source.is_file() {
        return Err(format!(
            "vendor DLL is missing or is not a regular file: {}",
            source.display()
        ));
    }

    let out_dir = env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "OUT_DIR is not set".to_owned())?;
    let profile_dir = profile_output_dir(&out_dir)?;
    fs::create_dir_all(&profile_dir).map_err(|error| {
        format!(
            "could not create executable output directory {}: {error}",
            profile_dir.display()
        )
    })?;

    let destination = profile_dir.join(DLL_NAME);
    fs::copy(&source, &destination).map_err(|error| {
        format!(
            "could not copy {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })?;

    Ok(())
}
