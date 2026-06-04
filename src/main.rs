use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod cli;
mod java;
mod provider;
mod server;
mod tui;

use cli::{Cli, Commands};
use provider::GameProvider;

const GITHUB_REPO: &str = "Mosuzzzz/mc-cli";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            dir,
            version,
            ram,
            provider,
            online,
        } => {
            validate_ram(&ram)?;
            java::check_java()?;

            let base_dir = PathBuf::from(&dir);
            let server_dir = base_dir.join("server");
            std::fs::create_dir_all(&server_dir)?;

            let p = GameProvider::from_str(&provider).ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown provider: {}. Supported: paper, vanilla, fabric",
                    provider
                )
            })?;

            let mut is_first_run = false;
            let (jar_path, actual_version) = if let Some(v) = version {
                let j_path = server_dir.join(format!("{}-{}.jar", provider, v));
                if !j_path.exists() {
                    println!("Downloading {} {}...", provider, v);
                    p.download(&v, &j_path).await?;
                    println!("Download complete!");
                    is_first_run = true;
                } else {
                    println!("Using cached server jar at {:?}", j_path);
                }
                (j_path, v)
            } else {
                let mut found = None;
                for entry in std::fs::read_dir(&server_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jar") {
                        found = Some(path);
                        break;
                    }
                }
                if let Some(path) = found {
                    let v = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.replace(&format!("{}-", provider), ""))
                        .unwrap_or_else(|| "unknown".to_string());
                    println!("Using existing server jar at {:?}", path);
                    (path, v)
                } else {
                    anyhow::bail!(
                        "No server jar found in {:?}. Please specify --version to download a new one.",
                        server_dir
                    );
                }
            };

            println!(
                "Starting {} server version {} with {} RAM...",
                provider, actual_version, ram
            );

            let min_java: u32 = if let Some(v) = actual_version
                .split('.')
                .nth(1)
                .and_then(|s: &str| s.parse::<u32>().ok())
            {
                if v >= 21 { 21 } else if v >= 17 { 17 } else { 8 }
            } else {
                8
            };

            java::require_java_version(min_java)?;

            let eula_path = server_dir.join("eula.txt");
            if !eula_path.exists() {
                println!("Generating eula.txt (accepting EULA)...");
                std::fs::write(&eula_path, "eula=true\n")?;
            }

            let server_props = server_dir.join("server.properties");
            if !server_props.exists() {
                let mode = if online { "true" } else { "false" };
                println!("Generating server.properties (online-mode={mode})...");
                std::fs::write(&server_props, format!("online-mode={mode}\n"))?;
            }

            server::run_server(server_dir, jar_path, ram, is_first_run).await?;
        }
        Commands::Update => {
            update_cli().await?;
        }
        Commands::ListVersions { provider } => {
            list_versions(&provider).await?;
        }
        Commands::Uninstall => {
            uninstall_cli()?;
        }
    }

    Ok(())
}

fn validate_ram(ram: &str) -> Result<()> {
    if ram.is_empty() {
        anyhow::bail!("RAM value cannot be empty");
    }
    let last = &ram[ram.len() - 1..];
    let num = &ram[..ram.len() - 1];
    if !matches!(last.to_uppercase().as_str(), "G" | "M" | "K")
        || num.is_empty()
        || num.parse::<u64>().is_err()
    {
        anyhow::bail!(
            "Invalid RAM format '{}'. Use a number followed by G or M (e.g., 2G, 512M)",
            ram
        );
    }
    Ok(())
}

// Maps the running platform to the binary name published in GitHub Releases.
fn platform_binary_name() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "x86_64")   => Some("mc-cli-x86_64-apple-darwin"),
        ("macos", "aarch64")  => Some("mc-cli-aarch64-apple-darwin"),
        ("linux", "x86_64")   => Some("mc-cli-x86_64-unknown-linux-gnu"),
        ("linux", "aarch64")  => Some("mc-cli-aarch64-unknown-linux-gnu"),
        ("windows", "x86_64") => Some("mc-cli-x86_64-pc-windows-msvc.exe"),
        _ => None,
    }
}

async fn fetch_latest_release_tag() -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("mc-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    let res: serde_json::Value = client
        .get(format!(
            "https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
        ))
        .send()
        .await?
        .error_for_status()
        .map_err(|_| {
            anyhow::anyhow!(
                "No releases found on GitHub — cannot update safely.\n\
                 Create a release at: https://github.com/{GITHUB_REPO}/releases/new"
            )
        })?
        .json()
        .await?;
    res["tag_name"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Unexpected response from GitHub releases API"))
}

async fn update_cli() -> Result<()> {
    let current = concat!("v", env!("CARGO_PKG_VERSION"));
    println!("Checking for updates...");
    let tag = fetch_latest_release_tag().await?;
    println!("Latest release: {tag}  (installed: {current})");

    if tag == current {
        println!("Already up to date.");
        return Ok(());
    }

    println!("Updating {current} → {tag}");

    // Primary path: download pre-built binary, verify SHA-256 before installing.
    match download_and_verify_binary(&tag).await {
        Ok(()) => return Ok(()),
        Err(e) => {
            println!("[warn] Pre-built binary unavailable: {e}");
            println!("       Falling back to cargo install (requires Rust toolchain)...");
        }
    }

    cargo_install_update(&tag).await
}

async fn download_and_verify_binary(tag: &str) -> Result<()> {
    let bin_name = platform_binary_name()
        .ok_or_else(|| anyhow::anyhow!("No pre-built binary for this platform"))?;

    let base = format!(
        "https://github.com/{GITHUB_REPO}/releases/download/{tag}"
    );
    let client = reqwest::Client::builder()
        .user_agent(concat!("mc-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    // 1. Fetch the checksums file first (separate request from the binary).
    println!("  Fetching release checksums...");
    let checksums_text = client
        .get(format!("{base}/sha256sums.txt"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let expected_hash = checksums_text
        .lines()
        .find_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?;
            if name == bin_name { Some(hash.to_string()) } else { None }
        })
        .ok_or_else(|| {
            anyhow::anyhow!("'{}' not listed in sha256sums.txt", bin_name)
        })?;

    // 2. Download binary.
    println!("  Downloading {bin_name}...");
    let bytes = client
        .get(format!("{base}/{bin_name}"))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    println!(
        "  {:.1} MB — verifying SHA-256...",
        bytes.len() as f64 / 1_048_576.0
    );

    // 3. Verify integrity before writing anything to disk.
    use sha2::{Digest, Sha256};
    let actual_hash = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    if actual_hash != expected_hash {
        anyhow::bail!(
            "Integrity check FAILED — refusing to install.\n\
             Expected: {expected_hash}\n\
             Got:      {actual_hash}"
        );
    }

    // 4. Write to a unique temp path, then atomically replace the running binary.
    let temp_path = std::env::temp_dir()
        .join(format!("mc-cli-update-{}", std::process::id()));
    std::fs::write(&temp_path, &bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755))?;
    }

    let current_exe = std::env::current_exe()?;
    swap_binary(temp_path, current_exe, tag)
}

async fn cargo_install_update(tag: &str) -> Result<()> {
    let temp_dir = std::env::temp_dir().join("mc-cli-update");
    std::fs::create_dir_all(&temp_dir)?;

    let status = std::process::Command::new("cargo")
        .args([
            "install",
            "--git",
            &format!("https://github.com/{GITHUB_REPO}.git"),
            "--tag",
            tag,
            "--locked", // use the exact Cargo.lock from that tag; prevents dep substitution
            "--root",
            temp_dir.to_str().unwrap(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            let bin_name = if cfg!(windows) { "mc-cli.exe" } else { "mc-cli" };
            let new_bin = temp_dir.join("bin").join(bin_name);
            if !new_bin.exists() {
                anyhow::bail!("Built binary not found at {:?}", new_bin);
            }
            let current_exe = std::env::current_exe()?;
            swap_binary(new_bin, current_exe, tag)
        }
        Ok(s) => anyhow::bail!("cargo install failed with status: {s}"),
        Err(e) => anyhow::bail!("Failed to run cargo. Is Rust installed? Error: {e}"),
    }
}

fn swap_binary(new_bin: PathBuf, current_exe: PathBuf, tag: &str) -> Result<()> {
    #[cfg(windows)]
    {
        let src = new_bin
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Non-UTF8 path for new binary"))?;
        let dst = current_exe
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Non-UTF8 path for current executable"))?;

        // Guard against BAT command injection via shell-special characters in paths.
        for ch in ['"', '&', '|', '<', '>'] {
            if src.contains(ch) || dst.contains(ch) {
                anyhow::bail!(
                    "Binary path contains shell-special character '{ch}'. \
                     Move mc-cli to a path without special characters and retry."
                );
            }
        }

        let pid = std::process::id();
        let bat_path = std::env::temp_dir().join("mc-cli-replace.bat");
        let bat = format!(
            "@echo off\r\n\
            :wait\r\n\
            tasklist /FI \"PID eq {pid}\" 2>NUL | find \"{pid}\" >NUL\r\n\
            if not errorlevel 1 (\r\n\
                timeout /t 1 /nobreak >NUL\r\n\
                goto wait\r\n\
            )\r\n\
            move /y \"{src}\" \"{dst}\"\r\n\
            echo mc-cli updated to {tag} successfully!\r\n\
            del \"%~f0\"\r\n",
        );
        std::fs::write(&bat_path, &bat)?;
        std::process::Command::new("cmd")
            .args(["/c", "start", "/min", "", bat_path.to_str().unwrap()])
            .spawn()?;
        println!("mc-cli will finish updating after this process exits.");
        println!("Please re-run mc-cli to use {tag}.");
    }

    #[cfg(not(windows))]
    {
        std::fs::rename(&new_bin, &current_exe)?;
        println!("mc-cli updated to {tag} successfully!");
    }

    Ok(())
}

async fn list_versions(provider: &str) -> Result<()> {
    let p = GameProvider::from_str(provider).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown provider: {}. Supported: paper, vanilla, fabric",
            provider
        )
    })?;
    println!("Fetching versions for {}...", provider);

    let versions = p.list_versions().await?;
    println!("Available versions (newest first):");
    for (i, v) in versions.iter().take(20).enumerate() {
        if i == 0 {
            println!("- {} ★ latest", v);
        } else {
            println!("- {}", v);
        }
    }
    if versions.len() > 20 {
        println!(
            "... and {} more (use --provider to filter)",
            versions.len() - 20
        );
    }
    Ok(())
}

fn uninstall_cli() -> Result<()> {
    println!("Proceeding to uninstall mc-cli...");
    let current_exe = std::env::current_exe()?;

    #[cfg(windows)]
    {
        let exe = current_exe
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Non-UTF8 path for current executable"))?;

        for ch in ['"', '&', '|', '<', '>'] {
            if exe.contains(ch) {
                anyhow::bail!(
                    "Executable path contains shell-special character '{ch}'. \
                     Move mc-cli to a path without special characters and retry."
                );
            }
        }

        let pid = std::process::id();
        let bat_path = std::env::temp_dir().join("mc-cli-uninstall.bat");
        let bat = format!(
            "@echo off\r\n\
            :wait\r\n\
            tasklist /FI \"PID eq {pid}\" 2>NUL | find \"{pid}\" >NUL\r\n\
            if not errorlevel 1 (\r\n\
                timeout /t 1 /nobreak >NUL\r\n\
                goto wait\r\n\
            )\r\n\
            del /f /q \"{exe}\"\r\n\
            echo mc-cli has been uninstalled successfully.\r\n\
            del \"%~f0\"\r\n",
        );
        std::fs::write(&bat_path, &bat)?;
        std::process::Command::new("cmd")
            .args(["/c", "start", "/min", "", bat_path.to_str().unwrap()])
            .spawn()?;
        println!("Uninstalling... mc-cli will be removed after this process exits.");
    }

    #[cfg(not(windows))]
    {
        std::fs::remove_file(&current_exe)?;
        println!("mc-cli has been uninstalled successfully.");
    }
    Ok(())
}
