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
        /// The minecraft version to run
        #[arg(short, long)]
        version: String,

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
}

fn check_java() -> Result<()> {
    let output = std::process::Command::new("java").arg("-version").output();
    match output {
        Ok(out) if out.status.success() => Ok(()),
        _ => anyhow::bail!("Java is not installed or not in PATH. Please install Java."),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start {
            version,
            ram,
            provider,
        } => {
            let p = GameProvider::from_str(&provider).ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown provider: {}. Supported: paper, vanilla, fabric",
                    provider
                )
            })?;
            println!(
                "Starting {} server version {} with {} RAM...",
                provider, version, ram
            );

            check_java()?;

            let server_dir = std::env::current_dir()?.join("server");
            std::fs::create_dir_all(&server_dir)?;

            let jar_path = server_dir.join(format!("{}-{}.jar", provider, version));
            if !jar_path.exists() {
                println!("Downloading {} {}...", provider, version);
                p.download(&version, &jar_path).await?;
                println!("Download complete!");
            } else {
                println!("Using cached server jar at {:?}", jar_path);
            }

            // Accept EULA
            let eula_path = server_dir.join("eula.txt");
            if !eula_path.exists() {
                println!("Generating eula.txt (accepting EULA)...");
                std::fs::write(&eula_path, "eula=true\n")?;
            }

            println!("Starting Java process...");
            let mut child = tokio::process::Command::new("java")
                .current_dir(&server_dir)
                .arg(format!("-Xmx{}", ram))
                .arg("-jar")
                .arg(&jar_path)
                .arg("nogui")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("Failed to start Java process")?;

            let state = std::sync::Arc::new(tokio::sync::Mutex::new(tui::AppState {
                logs: Vec::new(),
                cpu_usage: 0.0,
                ram_usage_mb: 0,
                online_players: 0,
                input: String::new(),
            }));

            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();

            let state_c1 = state.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncBufReadExt;
                let mut reader = tokio::io::BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut st = state_c1.lock().await;
                    if line.contains("joined the game") {
                        st.online_players += 1;
                    } else if line.contains("left the game") {
                        st.online_players -= 1;
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
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            });

            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
            
            let mut stdin = child.stdin.take().unwrap();
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                while let Some(cmd) = rx.recv().await {
                    let full_cmd = format!("{}\n", cmd);
                    let _ = stdin.write_all(full_cmd.as_bytes()).await;
                }
                // When rx is dropped, we send stop natively before exiting.
                let _ = stdin.write_all(b"stop\n").await;
            });

            tui::run_dashboard(state, tx).await?;

            println!("Stopping server gracefully...");
            let status = child.wait().await?;
            println!("Server exited cleanly with status: {:?}", status);
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
            println!("Available versions:");
            // Print top 20 or all
            for v in versions.iter().rev().take(20) {
                println!("- {}", v);
            }
            if versions.len() > 20 {
                println!("... and {} more", versions.len() - 20);
            }
        }
    }

    Ok(())
}
