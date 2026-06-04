use crate::tui::{self, AppState};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, mpsc};

pub async fn run_server(
    server_dir: PathBuf,
    jar_path: PathBuf,
    ram: String,
    is_first_run: bool,
) -> Result<()> {
    let state = Arc::new(Mutex::new(AppState {
        logs: Vec::new(),
        cpu_usage: 0.0,
        ram_usage_mb: 0,
        online_players: 0u32,
        input: String::new(),
        is_running: true,
    }));

    let mut should_run = true;
    let mut first_boot_finished = false;

    while should_run {
        should_run = false;
        state.lock().await.is_running = true;

        let jar_filename = jar_path.file_name().unwrap();

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

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        // Track user restart request
        let restart_requested = Arc::new(AtomicBool::new(false));
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
                    st.online_players = st.online_players.saturating_sub(1);
                }

                // Handle user request to restart on first boot
                if is_first && !auto_trigger && line.contains("Done (") {
                    auto_trigger = true;
                    rr_auto.store(true, Ordering::SeqCst);
                    let _ = auto_tx.send("stop".to_string());
                    st.push_log("--- First time auto-restart triggered for client ---".to_string());
                }
                st.push_log(line);
            }
        });

        let state_c2 = state.clone();
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let mut st = state_c2.lock().await;
                st.push_log(line);
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
                    rr_clone.store(true, Ordering::SeqCst);
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

        if restart_requested.load(Ordering::SeqCst) {
            should_run = true;
            first_boot_finished = true;
            state
                .lock()
                .await
                .push_log("--- Server Restarting ---".to_string());
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

    Ok(())
}
