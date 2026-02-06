mod app;

use crossterm::event::{self, Event, KeyCode};
use ratatui::widgets::TableState;
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self};
use std::time::{Duration, Instant};

use app::{draw_ui, init_system, refresh_system_data};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    let mut stats = init_system();
    let mut table_state = TableState::default();
    table_state.select(Some(0));
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_secs(1);

    loop {
        terminal.draw(|f| {
            draw_ui(f, &mut stats.sys, stats.cpu_count, &mut table_state);
        })?;

        if last_tick.elapsed() >= tick_rate {
            refresh_system_data(&mut stats.sys);
            last_tick = Instant::now();
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        let i = match table_state.selected() {
                            Some(i) => (i + 1).min(stats.sys.processes().len() - 1),
                            None => 0,
                        };
                        table_state.select(Some(i));
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
    }

    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
