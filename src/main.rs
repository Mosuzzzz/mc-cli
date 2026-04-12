use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod provider;
mod tui;
use provider::GameProvider;

#[derive(Parser)]
#[command(name = "mc-cli")]
#[command(about = "Open Source Minecraft Server Manager", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a minecraft server
    Start {
        /// The target directory to start the server in
        #[arg(default_value = ".")]
        dir: String,

        /// The minecraft version to run
        #[arg(short, long)]
        version: Option<String>,

        /// The amount of RAM to allocate (e.g., 4G)
        #[arg(short, long, default_value = "2G")]
        ram: String,

        /// Server provider (e.g., paper, vanilla, fabric)
        #[arg(short, long, default_value = "paper")]
        provider: String,
    },
    /// List available versions for a provider
    ListVersions {
        /// Server provider (e.g., paper, vanilla, fabric)
        #[arg(short, long, default_value = "paper")]
        provider: String,
    },
    /// Update mc-cli to the latest version
    Update,
}

fn get_java_major_version() -> Option<u32> {
    // `java -version` prints to stderr: e.g. `openjdk version "21.0.3" ...`
    let out = std::process::Command::new("java")
        .arg("-version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    // Match patterns like `version "21.0.3"` or `version "1.8.0_xyz"`
    for token in text.split_whitespace() {
        let token = token.trim_matches('"');
        let major: &str = if token.starts_with("1.") {
            // Old-style: 1.8 → major 8
            token.splitn(3, '.').nth(1).unwrap_or("0")
        } else {
            token.splitn(2, '.').next().unwrap_or("0")
        };
        if let Ok(n) = major.parse::<u32>() {
            if n > 0 {
                return Some(n);
            }
        }
    }
    None
}

fn check_java() -> Result<()> {
    let output = std::process::Command::new("java").arg("-version").output();
    match output {
        Ok(out) if out.status.success() => Ok(()),
        _ => anyhow::bail!("Java is not installed or not in PATH. Please install Java."),
    }
}

fn require_java_version(min_major: u32) -> Result<()> {
    check_java()?;
    match get_java_major_version() {
        Some(v) if v >= min_major => Ok(()),
        Some(v) => anyhow::bail!(
            "This server requires Java {min_major}+, but Java {v} was found.\n\
             Please install a newer JDK: https://adoptium.net"
        ),
        None => {
            println!("[warn] Could not detect Java version — proceeding anyway.");
            Ok(())
        }
    }
}

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
            check_java()?;

            let base_dir = std::path::PathBuf::from(&dir);
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
                    let v = path.file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.replace(&format!("{}-", provider), ""))
                        .unwrap_or_else(|| "unknown".to_string());
                    println!("Using existing server jar at {:?}", path);
                    (path, v)
                } else {
                    anyhow::bail!("No server jar found in {:?}. Please specify --version to download a new one.", server_dir);
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
                if v >= 21 { 21 } else if v >= 17 { 17 } else { 8 }
            } else {
                8
            };
            if provider == "paper" || provider == "fabric" {
                require_java_version(min_java)?;
            }

            // Accept EULA
            let eula_path = server_dir.join("eula.txt");
            if !eula_path.exists() {
                println!("Generating eula.txt (accepting EULA)...");
                std::fs::write(&eula_path, "eula=true\n")?;
            }

            // Client support (Offline mode by default so standard and non-premium clients can join on first boot)
            let server_props = server_dir.join("server.properties");
            if !server_props.exists() {
                println!("Generating server.properties (online-mode=false) for client support...");
                std::fs::write(&server_props, "online-mode=false\n")?;
            }

            let state = std::sync::Arc::new(tokio::sync::Mutex::new(tui::AppState {
                logs: Vec::new(),
                cpu_usage: 0.0,
                ram_usage_mb: 0,
                online_players: 0,
                input: String::new(),
                is_running: true,
            }));

            let mut should_run = true;
            let mut first_boot_finished = false;

            while should_run {
                should_run = false;
                state.lock().await.is_running = true;
                
                let jar_filename = jar_path.file_name().unwrap();
                let java_cmd = format!(
                    "java -Xmx{ram} -jar {jar} nogui",
                    ram = ram,
                    jar = jar_filename.to_string_lossy()
                );
                println!("Running: {}", java_cmd);

                let mut child = tokio::process::Command::new("java")
                    .current_dir(&server_dir)
                    .arg(format!("-Xmx{}", ram))
                    .arg("-jar")
                    .arg(jar_filename)
                    .arg("nogui")
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .context("Failed to start Java process")?;

                let stdout = child.stdout.take().unwrap();
                let stderr = child.stderr.take().unwrap();
                
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
                
                // Track user restart request
                let restart_requested = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let rr_clone = restart_requested.clone();
                let rr_auto = restart_requested.clone();
                
                let state_c1 = state.clone();
                let auto_tx = tx.clone();
                let is_first = is_first_run && !first_boot_finished;
                
                tokio::spawn(async move {
                    use tokio::io::AsyncBufReadExt;
                    let mut reader = tokio::io::BufReader::new(stdout).lines();
                    let mut auto_trigger = false;
                    while let Ok(Some(line)) = reader.next_line().await {
                        let mut st = state_c1.lock().await;
                        if line.contains("joined the game") {
                            st.online_players += 1;
                        } else if line.contains("left the game") {
                            st.online_players -= 1;
                        }
                        
                        // Handle user request to restart on first boot
                        if is_first && !auto_trigger && line.contains("Done (") {
                            auto_trigger = true;
                            rr_auto.store(true, std::sync::atomic::Ordering::SeqCst);
                            let _ = auto_tx.send("stop".to_string());
                            st.logs.push("--- First time auto-restart triggered for client ---".to_string());
                        }
                        st.logs.push(line);
                    }
                });

                let state_c2 = state.clone();
                tokio::spawn(async move {
                    use tokio::io::AsyncBufReadExt;
                    let mut reader = tokio::io::BufReader::new(stderr).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        let mut st = state_c2.lock().await;
                        st.logs.push(line);
                    }
                });

                let state_c3 = state.clone();
                let pid_opt = child.id().map(|id| sysinfo::Pid::from_u32(id));
                tokio::spawn(async move {
                    if let Some(pid) = pid_opt {
                        let mut sys = sysinfo::System::new();
                        loop {
                            sys.refresh_cpu_usage();
                            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                            if let Some(process) = sys.process(pid) {
                                let mut st = state_c3.lock().await;
                                st.cpu_usage = process.cpu_usage();
                                st.ram_usage_mb = process.memory() / 1024 / 1024;
                            } else {
                                break;
                            }
                            // Important check to bail early if TUI is not running
                            if !state_c3.lock().await.is_running {
                                break;
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                });

                let mut stdin = child.stdin.take().unwrap();
                tokio::spawn(async move {
                    use tokio::io::AsyncWriteExt;
                    while let Some(cmd) = rx.recv().await {
                        if cmd.trim().eq_ignore_ascii_case("restart") {
                            rr_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                            let _ = stdin.write_all(b"stop\n").await;
                        } else {
                            let full_cmd = format!("{}\n", cmd);
                            let _ = stdin.write_all(full_cmd.as_bytes()).await;
                        }
                    }
                    // When rx is dropped, we send stop natively before exiting.
                    let _ = stdin.write_all(b"stop\n").await;
                });

                let ui_state = state.clone();
                let ui_tx = tx.clone();
                let tui_handle = tokio::spawn(async move {
                    let _ = tui::run_dashboard(ui_state, ui_tx).await;
                });

                let status = child.wait().await?;
                // Small delay to let log reader tasks flush their last lines
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                state.lock().await.is_running = false;
                let _ = tui_handle.await;

                let exit_ok = status.success();

                if restart_requested.load(std::sync::atomic::Ordering::SeqCst) {
                    should_run = true;
                    first_boot_finished = true;
                    state.lock().await.logs.push("--- Server Restarting ---".to_string());
                } else if exit_ok {
                    println!("Server stopped cleanly.");
                } else {
                    // Dump the captured logs so the user can see why it crashed
                    eprintln!("\n[mc-cli] Server exited with error status: {:?}", status);
                    eprintln!("[mc-cli] --- Last server output ---");
                    let logs = state.lock().await.logs.clone();
                    let start = logs.len().saturating_sub(40);
                    for line in &logs[start..] {
                        eprintln!("{}", line);
                    }
                    eprintln!("[mc-cli] --- End of output ---");
                    eprintln!("[mc-cli] Tip: check that your Java version matches the server version.");
                }
            }
        }
        Commands::Update => {
            println!("Updating mc-cli to the latest version...");

            // Install to a temp directory to avoid overwriting the running exe
            let temp_dir = std::env::temp_dir().join("mc-cli-update");
            std::fs::create_dir_all(&temp_dir)?;

            let status = std::process::Command::new("cargo")
                .args([
                    "install",
                    "--git", "https://github.com/Mosuzzzz/mc-cli.git",
                    "--root", temp_dir.to_str().unwrap(),
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    let new_bin = temp_dir.join("bin").join("mc-cli.exe");
                    let current_exe = std::env::current_exe()?;

                    if !new_bin.exists() {
                        anyhow::bail!("Built binary not found at {:?}", new_bin);
                    }

                    // On Windows we cannot overwrite a running .exe directly.
                    // Write a small batch script that waits for this process to
                    // exit and then performs the copy, then launch it detached.
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

                    // Launch the batch script minimised and detached
                    std::process::Command::new("cmd")
                        .args(["/c", "start", "/min", "", bat_path.to_str().unwrap()])
                        .spawn()?;

                    println!("Download complete! mc-cli will finish updating after this process exits.");
                    println!("Please re-run mc-cli to use the new version.");
                }
                Ok(s) => {
                    anyhow::bail!("Failed to build mc-cli. Cargo exited with status: {}", s);
                }
                Err(e) => {
                    anyhow::bail!("Failed to execute cargo. Is Rust installed? Error: {}", e);
                }
            }
        }
        Commands::ListVersions { provider } => {
            let p = GameProvider::from_str(&provider).ok_or_else(|| {
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
                println!("... and {} more (use --provider to filter)", versions.len() - 20);
            }
        }
    }

    Ok(())
}
