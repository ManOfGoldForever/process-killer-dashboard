use std::thread;
use std::time::Duration;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

fn main() {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let cpu_count = sys.cpus().len() as f32;

    loop {
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing().with_cpu(),
        );
        print!("{esc}c", esc = 27 as char);
        println!(
            "{:<10} {:<30} {:<10} {:<10}",
            "PID", "Name", "CPU %", "CPU Core %"
        );
        println!("{}", "-".repeat(50));
        let mut procs: Vec<_> = sys.processes().values().collect();
        procs.sort_by(|a, b| b.cpu_usage().partial_cmp(&a.cpu_usage()).unwrap());
        for p in procs.iter() {
            let normalized_cpu = p.cpu_usage() / cpu_count;
            let mut name = p.name().to_string_lossy().into_owned();
            if name.len() > 30 {
                name.truncate(27);
                name.push_str("...");
            }
            println!(
                "{:<10} {:<30} {:<.2}% {:<.2}%",
                p.pid(),
                p.name().to_string_lossy(),
                normalized_cpu,
                p.cpu_usage(),
            );
        }
        thread::sleep(Duration::from_millis(1000));
    }
}
