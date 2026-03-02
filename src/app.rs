use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
};
use std::cmp::Reverse;
use std::ffi::OsStr;
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::UpdateKind;
use sysinfo::Users;
use sysinfo::{Pid, Process, Signal, System};

pub struct SystemStats {
    pub sys: System,
    pub cpu_count: f32,
    pub users: Users,
}

#[derive(Clone, Copy)]
pub enum SortBy {
    CpuDesc,
    MemoryDesc,
    DiskReadDesc,
    DiskWriteDesc,
    NameAsc,
    PidAsc,
}

pub enum SoftKillOutcome {
    Terminated,
    EscalatedToForceKill,
}

pub fn init_system() -> SystemStats {
    let mut sys = System::new_all();
    sys.refresh_all();
    let users = Users::new_with_refreshed_list();
    let cpu_count = sys.cpus().len() as f32;
    SystemStats {
        sys,
        cpu_count,
        users,
    }
}

pub fn draw_ui(
    f: &mut Frame,
    processes: &[&Process],
    users: &Users,
    cpu_count: f32,
    state: &mut TableState,
    status: &str,
    mode_label: &str,
    paused: bool,
) {
    let rows: Vec<Row> = processes
        .iter()
        .map(|p| {
            let cpu_usage = p.cpu_usage();
            let owner_name = if let Some(user_id) = p.user_id() {
                if let Some(user) = users.get_user_by_id(user_id) {
                    user.name().to_string()
                } else {
                    format!("UID: {:?}", user_id)
                }
            } else {
                "Unknown".to_string()
            };
            let status = format!("{:?}", p.status());
            let disk = p.disk_usage();
            let disk_str = format!(
                "R:{}/W:{}",
                disk.total_read_bytes / 1024,
                disk.total_written_bytes / 1024
            );
            Row::new(vec![
                p.pid().to_string(),
                status,
                p.name().to_string_lossy().to_string(),
                format_duration(p.run_time()),
                p.cmd().join(OsStr::new(" ")).to_string_lossy().to_string(),
                owner_name,
                format_memory(p.memory()),
                disk_str,
                format!("{:.1}%", cpu_usage / cpu_count),
                format!("{:.1}%", cpu_usage),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(30),
        Constraint::Length(10),
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec![
                "PID",
                "Status",
                "Name",
                "Uptime",
                "Command",
                "Owner",
                "Memory",
                "Disk Usage",
                "CPU %",
                "Core %",
            ])
            .style(Style::new().blue().bold()),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Process Manager ")
                .title_bottom(
                    " Use ↑/↓ Scroll, q Quit, p Pause, / Search, Enter Apply/Confirm, Esc Clear/Cancel, y Confirm, n Cancel, k Soft Kill, K Force Kill, c CPU, m Memory, r Disk Read, w Disk Write, n Name, i PID ",
                ),
        )
        .row_highlight_style(Style::new().bg(Color::Cyan).fg(Color::Black).bold())
        .highlight_symbol(">> ");

    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(f.area());
    let footer_chunks =
        Layout::horizontal([Constraint::Min(1), Constraint::Length(18)]).split(chunks[1]);
    let mode_color = match mode_label {
        "NORMAL" => Color::Green,
        "SEARCH" => Color::Cyan,
        "CONFIRM" => Color::Red,
        _ => Color::White,
    };
    let status_color = if status.contains("failed") {
        Color::Red
    } else if status.contains("cancelled") || status.contains("cleared") {
        Color::LightYellow
    } else {
        Color::Yellow
    };
    let footer = Line::from(vec![
        Span::styled("[", Style::new().fg(Color::DarkGray)),
        Span::styled(mode_label, Style::new().fg(mode_color).bold()),
        Span::styled("] ", Style::new().fg(Color::DarkGray)),
        Span::styled(status, Style::new().fg(status_color)),
    ]);

    let pause_label = if paused { "PAUSED" } else { "UNPAUSED" };
    let pause_color = if paused { Color::Yellow } else { Color::Green };
    let pause_badge = Line::from(vec![
        Span::styled("[", Style::new().fg(Color::DarkGray)),
        Span::styled(pause_label, Style::new().fg(pause_color).bold()),
        Span::styled("]", Style::new().fg(Color::DarkGray)),
    ]);

    f.render_stateful_widget(table, chunks[0], state);
    f.render_widget(Paragraph::new(footer), footer_chunks[0]);
    f.render_widget(
        Paragraph::new(pause_badge).alignment(ratatui::layout::Alignment::Right),
        footer_chunks[1],
    );
}

pub fn sort_label(sort_by: SortBy) -> &'static str {
    match sort_by {
        SortBy::CpuDesc => "CPU desc",
        SortBy::MemoryDesc => "Memory desc",
        SortBy::DiskReadDesc => "Disk Read desc",
        SortBy::DiskWriteDesc => "Disk Write desc",
        SortBy::NameAsc => "Name asc",
        SortBy::PidAsc => "PID asc",
    }
}

pub fn sorted_processes<'a>(
    sys: &'a System,
    sort_by: SortBy,
    search_query: &str,
) -> Vec<&'a Process> {
    let mut processes: Vec<_> = sys.processes().values().collect();
    let query = search_query.trim();

    if !query.is_empty() {
        processes.retain(|p| {
            let name = p.name().to_string_lossy();
            let cmd_joined = p.cmd().join(OsStr::new(" "));
            let cmd = cmd_joined.to_string_lossy();
            process_matches_query(&name, &cmd, p.pid(), query)
        });
    }

    match sort_by {
        SortBy::CpuDesc => processes.sort_by(|a, b| {
            b.cpu_usage()
                .partial_cmp(&a.cpu_usage())
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        SortBy::MemoryDesc => processes.sort_by_key(|p| Reverse(p.memory())),
        SortBy::DiskReadDesc => processes.sort_by_key(|p| Reverse(p.disk_usage().total_read_bytes)),
        SortBy::DiskWriteDesc => {
            processes.sort_by_key(|p| Reverse(p.disk_usage().total_written_bytes))
        }
        SortBy::NameAsc => processes.sort_by(|a, b| a.name().cmp(b.name())),
        SortBy::PidAsc => processes.sort_by_key(|p| p.pid().as_u32()),
    }

    processes
}

pub fn selected_pid_from_visible(visible_pids: &[Pid], selected: Option<usize>) -> Option<Pid> {
    let index = selected?;
    visible_pids.get(index).copied()
}

fn process_matches_query(name: &str, cmd: &str, pid: Pid, query: &str) -> bool {
    let query_lc = query.to_lowercase();
    let name_lc = name.to_lowercase();
    let cmd_lc = cmd.to_lowercase();
    let pid_str = pid.to_string();

    name_lc.contains(&query_lc) || cmd_lc.contains(&query_lc) || pid_str.contains(&query_lc)
}

pub fn refresh_system_data(sys: &mut System) {
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::nothing()
            .with_cpu()
            .with_user(UpdateKind::Always)
            .with_disk_usage()
            .with_memory(),
    );
}

fn format_memory(bytes: u64) -> String {
    let kb = bytes / 1024;
    let mb = kb / 1024;
    let gb = mb / 1024;

    if gb > 0 {
        format!("{:.2} GB", bytes as f32 / 1024.0 / 1024.0 / 1024.0)
    } else if mb > 0 {
        format!("{:.1} MB", bytes as f32 / 1024.0 / 1024.0)
    } else {
        format!("{} KB", kb)
    }
}

fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}

fn send_signal(pid: Pid, signal: Signal) -> Result<(), String> {
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let process = sys
        .process(pid)
        .ok_or_else(|| format!("PID {} not found", pid.as_u32()))?;

    match process.kill_with(signal) {
        Some(true) => Ok(()),
        Some(false) => Err(format!(
            "Failed to send {:?} to PID {}",
            signal,
            pid.as_u32()
        )),
        None => Err(format!(
            "Signal {:?} is not supported on this platform",
            signal
        )),
    }
}

fn wait_for_process_exit(pid: Pid, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        let mut sys = System::new_all();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        if sys.process(pid).is_none() {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    false
}

pub fn soft_kill_process(pid: Pid) -> Result<SoftKillOutcome, String> {
    send_signal(pid, Signal::Term)?;

    if wait_for_process_exit(pid, Duration::from_secs(2)) {
        return Ok(SoftKillOutcome::Terminated);
    }

    send_signal(pid, Signal::Kill)?;

    if wait_for_process_exit(pid, Duration::from_secs(1)) {
        Ok(SoftKillOutcome::EscalatedToForceKill)
    } else {
        Err(format!(
            "PID {} did not exit after TERM timeout and KILL fallback",
            pid.as_u32()
        ))
    }
}

pub fn force_kill_process(pid: Pid) -> Result<(), String> {
    send_signal(pid, Signal::Kill)?;
    if wait_for_process_exit(pid, Duration::from_secs(1)) {
        Ok(())
    } else {
        Err(format!(
            "PID {} did not exit after force kill",
            pid.as_u32()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_query_matches_name_cmd_and_pid_case_insensitive() {
        let pid = Pid::from(4242usize);
        assert!(process_matches_query(
            "Firefox",
            "firefox --private",
            pid,
            "fire"
        ));
        assert!(process_matches_query(
            "daemon",
            "python server.py",
            pid,
            "PYTHON"
        ));
        assert!(process_matches_query(
            "daemon",
            "python server.py",
            pid,
            "4242"
        ));
        assert!(!process_matches_query(
            "daemon",
            "python server.py",
            pid,
            "nomatch"
        ));
    }

    #[test]
    fn selected_pid_from_visible_returns_expected_index() {
        let pids = vec![Pid::from(10usize), Pid::from(22usize), Pid::from(35usize)];
        assert_eq!(
            selected_pid_from_visible(&pids, Some(1)),
            Some(Pid::from(22usize))
        );
        assert_eq!(selected_pid_from_visible(&pids, Some(3)), None);
        assert_eq!(selected_pid_from_visible(&pids, None), None);
    }

    #[test]
    fn sort_labels_are_stable() {
        assert_eq!(sort_label(SortBy::CpuDesc), "CPU desc");
        assert_eq!(sort_label(SortBy::MemoryDesc), "Memory desc");
        assert_eq!(sort_label(SortBy::DiskReadDesc), "Disk Read desc");
        assert_eq!(sort_label(SortBy::DiskWriteDesc), "Disk Write desc");
        assert_eq!(sort_label(SortBy::NameAsc), "Name asc");
        assert_eq!(sort_label(SortBy::PidAsc), "PID asc");
    }
}
