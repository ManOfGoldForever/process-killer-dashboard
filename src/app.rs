use ratatui::{
    Frame,
    layout::Constraint,
    style::{Color, Style},
    widgets::{Block, Borders, Row, Table, TableState},
};
use sysinfo::System;

pub struct SystemStats {
    pub sys: System,
    pub cpu_count: f32,
}

pub fn init_system() -> SystemStats {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let cpu_count = sys.cpus().len() as f32;
    SystemStats { sys, cpu_count }
}

pub fn draw_ui(f: &mut Frame, sys: &mut System, cpu_count: f32, state: &mut TableState) {
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
            Row::new(vec![
                p.pid().to_string(),
                p.name().to_string_lossy().to_string(),
                format!("{:.1}%", cpu_usage / cpu_count),
                format!("{:.1}%", cpu_usage),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Fill(1),
        Constraint::Length(10),
        Constraint::Length(15),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["PID", "Name", "CPU %", "CPU Core %"]).style(Style::new().blue().bold()),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Process Manager ")
                .title_bottom(" Use ↑/↓ to Scroll, 'q' to Quit "),
        )
        .row_highlight_style(Style::new().bg(Color::Cyan).fg(Color::Black).bold())
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, f.area(), state);
}

pub fn refresh_system_data(sys: &mut System) {
    sys.refresh_processes_specifics(
        sysinfo::ProcessesToUpdate::All,
        true,
        sysinfo::ProcessRefreshKind::nothing().with_cpu(),
    );
}
