use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{io, sync::Arc};
use tokio::sync::Mutex;

pub struct AppState {
    pub logs: Vec<String>,
    pub cpu_usage: f32,
    pub ram_usage_mb: u64,
    pub online_players: i32,
    pub input: String,
    pub is_running: bool,
}

pub async fn run_dashboard(
    state: Arc<Mutex<AppState>>,
    command_tx: tokio::sync::mpsc::UnboundedSender<String>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_ui_loop(&mut terminal, state, command_tx).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

async fn run_ui_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: Arc<Mutex<AppState>>,
    command_tx: tokio::sync::mpsc::UnboundedSender<String>,
) -> Result<()> {
    loop {
        let app_state_lock = state.lock().await;
        if !app_state_lock.is_running {
            break;
        }
        terminal.draw(|f| {
            let size = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(size);

            let header_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                ])
                .split(chunks[0]);

            let cpu_text = format!(" CPU: {:.1}% ", app_state_lock.cpu_usage);
            let cpu_block = Paragraph::new(cpu_text)
                .block(Block::default().borders(Borders::ALL).title(" sysinfo "))
                .alignment(Alignment::Center);
            f.render_widget(cpu_block, header_chunks[0]);

            let ram_text = format!(" RAM: {} MB ", app_state_lock.ram_usage_mb);
            let ram_block = Paragraph::new(ram_text)
                .block(Block::default().borders(Borders::ALL).title(" memory "))
                .alignment(Alignment::Center);
            f.render_widget(ram_block, header_chunks[1]);

            let player_text = format!(" Players: {} ", app_state_lock.online_players);
            let player_block = Paragraph::new(player_text)
                .block(Block::default().borders(Borders::ALL).title(" status "))
                .alignment(Alignment::Center);
            f.render_widget(player_block, header_chunks[2]);

            // Logs
            let log_lines: Vec<Line> = app_state_lock
                .logs
                .iter()
                .map(|l| {
                    if l.contains("ERROR") {
                        Line::from(Span::styled(l, Style::default().fg(Color::Red)))
                    } else if l.contains("WARN") {
                        Line::from(Span::styled(l, Style::default().fg(Color::Yellow)))
                    } else {
                        Line::from(Span::raw(l))
                    }
                })
                .collect();

            let log_len = log_lines.len() as u16;
            let console_height = chunks[1].height.saturating_sub(2);
            let scroll = if log_len > console_height {
                log_len - console_height
            } else {
                0
            };

            let logs_block = Paragraph::new(log_lines)
                .block(Block::default().borders(Borders::ALL).title(" Console "))
                .scroll((scroll, 0));

            f.render_widget(logs_block, chunks[1]);

            // Input Bar
            let input_text = format!("> {}", app_state_lock.input);
            let input_block = Paragraph::new(input_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Input Command (Enter to execute) "),
            );
            f.render_widget(input_block, chunks[2]);
        })?;
        drop(app_state_lock);

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != event::KeyEventKind::Press {
                    continue;
                }

                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }

                let mut app_state = state.lock().await;
                match key.code {
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app_state.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app_state.input.pop();
                    }
                    KeyCode::Enter => {
                        let cmd = std::mem::take(&mut app_state.input);
                        if !cmd.trim().is_empty() {
                            let _ = command_tx.send(cmd);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
