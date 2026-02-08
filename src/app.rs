use ratatui::{
    Frame,
    layout::Constraint,
    style::{Color, Style},
    widgets::{Block, Borders, Row, Table, TableState},
};
use std::ffi::OsStr;
use sysinfo::System;
use sysinfo::UpdateKind;
use sysinfo::Users;

pub struct SystemStats {
    pub sys: System,
    pub cpu_count: f32,
    pub users: Users,
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
    sys: &mut System,
    users: &mut Users,
    cpu_count: f32,
    state: &mut TableState,
) {
    let mut processes: Vec<_> = sys.processes().values().collect();
    processes.sort_by(|a, b| {
        b.cpu_usage()
            .partial_cmp(&a.cpu_usage())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let rows: Vec<Row> = processes
        .iter()
        .map(|p| {
            let cpu_usage = p.cpu_usage();
            let owner_name = if let Some(user_id) = p.user_id() {
                if let Some(user) = users.get_user_by_id(user_id) {
                    user.name().to_string()
                } else {
                    format!("UID: {}", user_id.to_string())
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
                .title_bottom(" Use ↑/↓ to Scroll, 'q' to Quit, 'p' to Pause Refresh "),
        )
        .row_highlight_style(Style::new().bg(Color::Cyan).fg(Color::Black).bold())
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, f.area(), state);
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
