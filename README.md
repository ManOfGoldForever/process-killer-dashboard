# process-killer-dashboard 🖥️

A fast terminal process dashboard built in Rust. Monitor system processes, sort by resource usage, filter quickly, and terminate processes with confirmation.

## Table of Contents

- [Why use this?](#why-use-this)
- [Installation](#installation)
- [Build from Source](#build-from-source)
- [Usage](#usage)
- [Screenshot](#screenshot)
- [Keybinds](#keybinds)
- [Safety and Process Signals](#safety-and-process-signals)
- [Future Plans](#future-plans)
- [Contributing](#contributing)
- [License](#license)

### Why use this?

- **Fast TUI:** Lightweight terminal UI with instant startup.
- **Actionable Process View:** See CPU, memory, disk I/O, owner, uptime, and command line.
- **Interactive Sorting + Filtering:** Sort by multiple resources and search by name, PID, or command text.
- **Safer Kill Flow:** Soft/force kill paths with explicit confirmation.

### Installation

Right now this project is source-first. Build it locally with Cargo.

### Build from Source

1. **Prerequisites**

- Install Rust via [rustup.rs](https://rustup.rs/).

2. **Run**

```bash
cd process-killer-dashboard
cargo run
```

3. **Build Release Binary**

```bash
cargo build --release
```

Binary path:

- `target/release/process-killer-dashboard`

### Usage

Start the app:

```bash
cargo run
```

The interface opens in an alternate terminal screen with a live process table.

### Keybinds

- `q`: Quit
- `p`: Pause/resume refresh
- `↑` / `↓`: Move selection
- `/`: Enter search mode
- `Enter`: Apply search (Search mode) or confirm kill (Confirm mode)
- `Esc`: Clear search (Search mode) or cancel kill (Confirm mode)
- `y`: Confirm kill (Confirm mode)
- `n`: Cancel kill (Confirm mode), or sort by name (Normal mode)
- `k`: Soft kill (TERM with timeout + fallback)
- `K`: Force kill (KILL)
- `c`: Sort by CPU descending
- `m`: Sort by memory descending
- `r`: Sort by disk read descending
- `w`: Sort by disk write descending
- `i`: Sort by PID ascending

### Safety and Process Signals

- Soft kill sends `Signal::Term`, waits up to 2 seconds, then falls back to `Signal::Kill` if needed.
- Force kill sends `Signal::Kill` and waits for termination.
- Signal support and permissions vary by OS and process privileges.
- Terminal state restoration is guarded for normal exits and panic paths.

### Future Plans

Potential next steps:

- Per-process detail panel
- Optional process-group kill controls
- Optional export/snapshot of filtered process table

### Contributing

Contributions are welcome. Open an issue or pull request with a clear problem statement and test coverage where possible.

### License

This project is licensed under the MIT License. See [LICENSE](./LICENSE).
