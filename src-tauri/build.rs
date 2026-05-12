use std::{env, fs, path::Path, process::Command};

const UV_VERSION: &str = "0.7.3";

fn main() {
    let target = env::var("TARGET").unwrap();
    // Make the build target available to lib.rs at compile time
    println!("cargo:rustc-env=BUILD_TARGET={target}");
    // Download uv BEFORE tauri_build::build() so it can validate externalBin.
    download_uv_if_needed(&target);
    tauri_build::build();
}

fn download_uv_if_needed(target: &str) {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let binaries_dir = Path::new(&manifest_dir).join("binaries");
    fs::create_dir_all(&binaries_dir).unwrap();

    let output_name = if target.contains("windows") {
        format!("uv-{target}.exe")
    } else {
        format!("uv-{target}")
    };
    let output_path = binaries_dir.join(&output_name);

    if output_path.exists() && output_path.metadata().map(|m| m.len()).unwrap_or(0) > 0 {
        return;
    }

    let (archive_name, binary_name_in_archive) = uv_archive_for_target(target);
    let url = format!("https://github.com/astral-sh/uv/releases/download/{UV_VERSION}/{archive_name}");
    let archive_path = binaries_dir.join(&archive_name);

    eprintln!("[build] Downloading uv {UV_VERSION} for {target}");
    let ok = Command::new("curl")
        .args(["-fsSL", "--retry", "3", "-o", archive_path.to_str().unwrap(), &url])
        .status()
        .expect("curl is required to download the uv binary. Install curl and re-run the build.")
        .success();
    assert!(ok, "curl failed to download {url}");

    extract_uv(&archive_path, &binary_name_in_archive, &output_path, target.contains("windows"));
    fs::remove_file(&archive_path).ok();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&output_path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    eprintln!("[build] uv binary ready: {}", output_path.display());
}

/// Returns (archive filename, uv binary name inside the archive).
fn uv_archive_for_target(target: &str) -> (String, String) {
    let (uv_triple, bin) = if target.contains("x86_64") && target.contains("linux") {
        ("x86_64-unknown-linux-musl", "uv")
    } else if target.contains("aarch64") && target.contains("linux") {
        ("aarch64-unknown-linux-musl", "uv")
    } else if target.contains("x86_64") && target.contains("apple") {
        ("x86_64-apple-darwin", "uv")
    } else if target.contains("aarch64") && target.contains("apple") {
        ("aarch64-apple-darwin", "uv")
    } else if target.contains("x86_64") && target.contains("windows") {
        ("x86_64-pc-windows-msvc", "uv.exe")
    } else if target.contains("aarch64") && target.contains("windows") {
        ("aarch64-pc-windows-msvc", "uv.exe")
    } else {
        panic!("Unsupported build target: {target}");
    };

    let ext = if target.contains("windows") { "zip" } else { "tar.gz" };
    (format!("uv-{uv_triple}.{ext}"), bin.to_string())
}

fn extract_uv(archive: &Path, binary_name: &str, output: &Path, is_windows: bool) {
    let temp = archive.parent().unwrap().join("_uv_tmp");
    fs::create_dir_all(&temp).unwrap();

    if is_windows {
        let ok = Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                &format!(
                    "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
                    archive.display(),
                    temp.display()
                ),
            ])
            .status()
            .expect("powershell not found")
            .success();
        assert!(ok, "Failed to unzip {}", archive.display());
    } else {
        let ok = Command::new("tar")
            .args(["-xzf", archive.to_str().unwrap(), "-C", temp.to_str().unwrap()])
            .status()
            .expect("tar not found")
            .success();
        assert!(ok, "Failed to extract {}", archive.display());
    }

    let found = find_file(&temp, binary_name)
        .unwrap_or_else(|| panic!("'{binary_name}' not found inside {}", archive.display()));
    fs::rename(&found, output).expect("Failed to place uv binary");
    fs::remove_dir_all(&temp).ok();
}

/// Recursively find the first file with the given name.
fn find_file(dir: &Path, name: &str) -> Option<std::path::PathBuf> {
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() && path.file_name()?.to_str()? == name {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(f) = find_file(&path, name) {
                return Some(f);
            }
        }
    }
    None
}
