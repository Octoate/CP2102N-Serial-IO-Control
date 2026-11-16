use std::path::{Path, PathBuf};

pub fn profile_output_dir(out_dir: &Path) -> Result<PathBuf, String> {
    // Cargo before 1.100: <profile>/build/<package>-<hash>/out
    // Cargo 1.100 onward: <profile>/build/<package>/<hash>/out
    // Limit the search to these layouts, so a higher ancestor named `build`
    // cannot accidentally be treated as Cargo's build directory.
    let profile_dir = out_dir
        .ancestors()
        .skip(2)
        .take(2)
        .find(|path| path.file_name().is_some_and(|name| name == "build"))
        .and_then(Path::parent)
        .filter(|path| !path.as_os_str().is_empty());

    if out_dir.file_name().is_some_and(|name| name == "out")
        && let Some(profile_dir) = profile_dir
    {
        return Ok(profile_dir.to_path_buf());
    }

    Err(format!(
        "cannot derive the profile output directory from OUT_DIR {}; expected <profile>/build/<package>-<hash>/out or <profile>/build/<package>/<hash>/out",
        out_dir.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_legacy_and_current_cargo_layouts() {
        for root in ["target", "custom output/build/target"] {
            for profile in ["debug", "release", "custom-profile"] {
                for target in ["", "x86_64-pc-windows-msvc"] {
                    let expected = Path::new(root).join(target).join(profile);
                    for package_path in ["example-0123456789abcdef", "example/0123456789abcdef"] {
                        let out_dir = expected.join("build").join(package_path).join("out");
                        assert_eq!(profile_output_dir(&out_dir), Ok(expected.clone()));
                    }
                }
            }
        }
    }

    #[test]
    fn rejects_unrecognized_layouts() {
        for path in [
            "",
            "out",
            "target/debug/out",
            "target/debug/not-build/example-hash/out",
            "target/debug/not-build/example/hash/out",
            "target/debug/build/example/hash/not-out",
            "target/debug/build/extra/example/hash/out",
            "build/example-hash/out",
        ] {
            let out_dir = Path::new(path);
            let error = profile_output_dir(out_dir).unwrap_err();
            assert!(error.contains(&out_dir.display().to_string()));
            assert!(error.contains("expected <profile>/build/"));
        }
    }
}
