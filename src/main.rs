mod app;

use crossterm::event::{self, Event, KeyCode};
use crossterm::{cursor, execute};
use ratatui::widgets::TableState;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use sysinfo::Pid;

use app::{
    SoftKillOutcome, SortBy, draw_ui, force_kill_process, init_system, refresh_system_data,
    selected_pid_from_visible, soft_kill_process, sort_label, sorted_processes,
};

static TERMINAL_RESTORED: AtomicBool = AtomicBool::new(true);

enum InputMode {
    Normal,
    Search,
    ConfirmKill { pid: Pid, force: bool },
}

struct TerminalSession;

impl TerminalSession {
    fn enter() -> Result<Self, Box<dyn std::error::Error>> {
        TERMINAL_RESTORED.store(false, Ordering::SeqCst);
        crossterm::terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(err) = execute!(
            stdout,
            crossterm::terminal::EnterAlternateScreen,
            cursor::Hide
        ) {
            restore_terminal_state();
            return Err(Box::new(err));
        }
        Ok(Self)
    }

    fn restore(&mut self) {
        restore_terminal_state();
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        restore_terminal_state();
    }
}

fn restore_terminal_state() {
    if TERMINAL_RESTORED.swap(true, Ordering::SeqCst) {
        return;
    }

    let _ = crossterm::terminal::disable_raw_mode();
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        crossterm::terminal::LeaveAlternateScreen,
        cursor::Show
    );
}

fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        restore_terminal_state();
        default_hook(panic_info);
    }));
}

fn mode_label(mode: &InputMode) -> &'static str {
    match mode {
        InputMode::Normal => "NORMAL",
        InputMode::Search => "SEARCH",
        InputMode::ConfirmKill { .. } => "CONFIRM",
    }
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let mut paused = false;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut stats = init_system();
    let mut table_state = TableState::default();
    table_state.select(Some(0));
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_secs(1);

    let mut status_message = String::from("Ready");
    let mut sort_by = SortBy::CpuDesc;
    let mut search_query = String::new();
    let mut mode = InputMode::Normal;

    loop {
        if !paused && last_tick.elapsed() >= tick_rate {
            refresh_system_data(&mut stats.sys);
            last_tick = Instant::now();
        }

        let visible_processes = sorted_processes(&stats.sys, sort_by, &search_query);
        let visible_pids: Vec<Pid> = visible_processes.iter().map(|p| p.pid()).collect();

        if visible_pids.is_empty() {
            table_state.select(None);
        } else {
            let selected = table_state
                .selected()
                .unwrap_or(0)
                .min(visible_pids.len() - 1);
            table_state.select(Some(selected));
        }

        terminal.draw(|f| {
            draw_ui(
                f,
                &visible_processes,
                &stats.users,
                stats.cpu_count,
                &mut table_state,
                &status_message,
                mode_label(&mode),
                paused,
            );
        })?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            match &mut mode {
                InputMode::Search => {
                    match key.code {
                        KeyCode::Esc => {
                            search_query.clear();
                            mode = InputMode::Normal;
                            status_message = "Search cleared".to_string();
                            table_state.select(Some(0));
                        }
                        KeyCode::Enter => {
                            mode = InputMode::Normal;
                            if search_query.trim().is_empty() {
                                status_message = "Search cleared".to_string();
                            } else {
                                status_message = format!("Filter: {}", search_query);
                            }
                            table_state.select(Some(0));
                        }
                        KeyCode::Backspace => {
                            search_query.pop();
                            status_message = format!("Search: {}", search_query);
                            table_state.select(Some(0));
                        }
                        KeyCode::Char(c) => {
                            search_query.push(c);
                            status_message = format!("Search: {}", search_query);
                            table_state.select(Some(0));
                        }
                        _ => {}
                    }
                    continue;
                }
                InputMode::ConfirmKill { pid, force } => {
                    match key.code {
                        KeyCode::Enter | KeyCode::Char('y') => {
                            if *force {
                                match force_kill_process(*pid) {
                                    Ok(()) => {
                                        status_message =
                                            format!("Force-killed PID {}", pid.as_u32());
                                    }
                                    Err(err) => {
                                        status_message = format!(
                                            "Force kill failed for PID {}: {}",
                                            pid.as_u32(),
                                            err
                                        );
                                    }
                                }
                            } else {
                                match soft_kill_process(*pid) {
                                    Ok(SoftKillOutcome::Terminated) => {
                                        status_message =
                                            format!("Soft-killed PID {}", pid.as_u32());
                                    }
                                    Ok(SoftKillOutcome::EscalatedToForceKill) => {
                                        status_message = format!(
                                            "Soft kill timed out; force-killed PID {}",
                                            pid.as_u32()
                                        );
                                    }
                                    Err(err) => {
                                        status_message = format!(
                                            "Soft kill failed for PID {}: {}",
                                            pid.as_u32(),
                                            err
                                        );
                                    }
                                }
                            }

                            refresh_system_data(&mut stats.sys);
                            mode = InputMode::Normal;
                        }
                        KeyCode::Esc | KeyCode::Char('n') => {
                            mode = InputMode::Normal;
                            status_message = "Kill cancelled".to_string();
                        }
                        _ => {}
                    }
                    continue;
                }
                InputMode::Normal => {}
            }

            if !matches!(mode, InputMode::Normal) {
                continue;
            }

            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('p') => paused = !paused,
                KeyCode::Char('/') => {
                    mode = InputMode::Search;
                    status_message = format!("Search: {}", search_query);
                }
                KeyCode::Char('k') => {
                    if let Some(pid) =
                        selected_pid_from_visible(&visible_pids, table_state.selected())
                    {
                        mode = InputMode::ConfirmKill { pid, force: false };
                        status_message = format!(
                            "Confirm soft kill PID {}? Press Enter/y to confirm, Esc/n to cancel",
                            pid.as_u32()
                        );
                    } else {
                        status_message = "No process selected".to_string();
                    }
                }
                KeyCode::Char('K') => {
                    if let Some(pid) =
                        selected_pid_from_visible(&visible_pids, table_state.selected())
                    {
                        mode = InputMode::ConfirmKill { pid, force: true };
                        status_message = format!(
                            "Confirm force kill PID {}? Press Enter/y to confirm, Esc/n to cancel",
                            pid.as_u32()
                        );
                    } else {
                        status_message = "No process selected".to_string();
                    }
                }
                KeyCode::Char('c') => {
                    sort_by = SortBy::CpuDesc;
                    status_message = format!("Sort: {}", sort_label(sort_by));
                    table_state.select(Some(0));
                }
                KeyCode::Char('m') => {
                    sort_by = SortBy::MemoryDesc;
                    status_message = format!("Sort: {}", sort_label(sort_by));
                    table_state.select(Some(0));
                }
                KeyCode::Char('r') => {
                    sort_by = SortBy::DiskReadDesc;
                    status_message = format!("Sort: {}", sort_label(sort_by));
                    table_state.select(Some(0));
                }
                KeyCode::Char('w') => {
                    sort_by = SortBy::DiskWriteDesc;
                    status_message = format!("Sort: {}", sort_label(sort_by));
                    table_state.select(Some(0));
                }
                KeyCode::Char('n') => {
                    sort_by = SortBy::NameAsc;
                    status_message = format!("Sort: {}", sort_label(sort_by));
                    table_state.select(Some(0));
                }
                KeyCode::Char('i') => {
                    sort_by = SortBy::PidAsc;
                    status_message = format!("Sort: {}", sort_label(sort_by));
                    table_state.select(Some(0));
                }
                KeyCode::Down => {
                    if !visible_pids.is_empty() {
                        let i = match table_state.selected() {
                            Some(i) => (i + 1).min(visible_pids.len() - 1),
                            None => 0,
                        };
                        table_state.select(Some(i));
                    }
                }
                KeyCode::Up => {
                    let i = match table_state.selected() {
                        Some(i) => i.saturating_sub(1),
                        None => 0,
                    };
                    table_state.select(Some(i));
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    install_panic_hook();
    let mut terminal_session = TerminalSession::enter()?;
    let run_result = run_app();
    terminal_session.restore();
    run_result
}
