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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            dir,
            version,
            ram,
            provider,
        } => {
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
                // version not specified.
                // Scan the `server_dir` for a jar.
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

            // Paper 1.17+ needs Java 17, Paper 1.21+ needs Java 21
            let min_java: u32 = if let Some(v) = actual_version
                .split('.')
                .nth(1)
                .and_then(|s: &str| s.parse::<u32>().ok())
            {
                if v >= 21 {
                    21
                } else if v >= 17 {
                    17
                } else {
                    8
                }
            } else {
                8
            };

            if provider == "paper" || provider == "fabric" {
                java::require_java_version(min_java)?;
            }

            // Accept EULA
            let eula_path = server_dir.join("eula.txt");
            if !eula_path.exists() {
                println!("Generating eula.txt (accepting EULA)...");
                std::fs::write(&eula_path, "eula=true\n")?;
            }

            // Client support (Offline mode by default)
            let server_props = server_dir.join("server.properties");
            if !server_props.exists() {
                println!("Generating server.properties (online-mode=false) for client support...");
                std::fs::write(&server_props, "online-mode=false\n")?;
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

async fn update_cli() -> Result<()> {
    println!("Updating mc-cli to the latest version...");

    // Install to a temp directory to avoid overwriting the running exe
    let temp_dir = std::env::temp_dir().join("mc-cli-update");
    std::fs::create_dir_all(&temp_dir)?;

    let status = std::process::Command::new("cargo")
        .args([
            "install",
            "--git",
            "https://github.com/Mosuzzzz/mc-cli.git",
            "--root",
            temp_dir.to_str().unwrap(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            let bin_name = if cfg!(windows) {
                "mc-cli.exe"
            } else {
                "mc-cli"
            };
            let new_bin = temp_dir.join("bin").join(bin_name);
            let current_exe = std::env::current_exe()?;

            if !new_bin.exists() {
                anyhow::bail!("Built binary not found at {:?}", new_bin);
            }

            #[cfg(windows)]
            {
                // On Windows we cannot overwrite a running .exe directly.
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
                    copy /y \"{src}\" \"{dst}\"\r\n\
                    echo mc-cli updated successfully!\r\n\
                    del \"%~f0\"\r\n",
                    pid = pid,
                    src = new_bin.display(),
                    dst = current_exe.display(),
                );
                std::fs::write(&bat_path, bat)?;

                std::process::Command::new("cmd")
                    .args(["/c", "start", "/min", "", bat_path.to_str().unwrap()])
                    .spawn()?;

                println!(
                    "Download complete! mc-cli will finish updating after this process exits."
                );
                println!("Please re-run mc-cli to use the new version.");
            }

            #[cfg(not(windows))]
            {
                // On Unix we can swap the binary while it's running
                std::fs::rename(&new_bin, &current_exe)?;
                println!("mc-cli updated successfully to the latest version!");
            }
        }
        Ok(s) => {
            anyhow::bail!("Failed to build mc-cli. Cargo exited with status: {}", s);
        }
        Err(e) => {
            anyhow::bail!("Failed to execute cargo. Is Rust installed? Error: {}", e);
        }
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
            pid = pid,
            exe = current_exe.display(),
        );
        std::fs::write(&bat_path, bat)?;

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
