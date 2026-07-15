//! Terminal lifecycle and the async event loop for the dashboard.
//!
//! [`run_dashboard`] is the public entry point wired up under `--tui`. It owns
//! a [`TerminalGuard`] (which enables raw mode and the alternate screen, and
//! restores both on drop, even on panic) and drives a fixed-tick loop: sample
//! the shared [`LiveStats`] and [`LogBuffer`], render, then poll crossterm for
//! key input. The polling `read()` is blocking, but the timeout is small so the
//! render cadence stays smooth; this loop only runs behind an interactive
//! terminal, never in tests.

use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;

use crate::engine::LiveStats;
use crate::tui::app::Dashboard;
use crate::tui::logbuf::LogBuffer;
use crate::tui::ui;

/// How often the dashboard resamples telemetry and redraws.
const TICK: Duration = Duration::from_millis(150);
/// How long each input poll blocks before yielding to the next tick.
const POLL: Duration = Duration::from_millis(100);

/// RAII terminal setup/teardown.
///
/// Construction enables raw mode and switches to the alternate screen;
/// [`Drop`] unwinds both and restores the cursor, so the terminal is always
/// left usable even if the event loop returns early or panics.
struct TerminalGuard {
    terminal: ratatui::Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalGuard {
    /// Enter raw mode and the alternate screen, returning a drawable terminal.
    fn new() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = ratatui::Terminal::new(backend).context("failed to build terminal")?;
        Ok(Self { terminal })
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Run the live dashboard until the user quits.
///
/// `stats` and `logs` are the shared handles the engine writes to; this loop
/// only reads snapshots from them. Returns once the user presses `q`/`Esc`; the
/// terminal is restored via the [`TerminalGuard`] drop.
pub async fn run_dashboard(stats: Arc<LiveStats>, logs: LogBuffer, is_server: bool) -> Result<()> {
    let mut app = Dashboard::new(is_server);
    let mut guard = TerminalGuard::new()?;

    while !app.should_quit() {
        app.on_tick(stats.snapshot(), logs.snapshot());

        guard
            .terminal
            .draw(|frame| ui::render(frame, &app))
            .context("failed to draw dashboard")?;

        // Drain input for up to one tick, so keys feel responsive but the UI
        // still refreshes on schedule when idle.
        let deadline = Instant::now() + TICK;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let timeout = remaining.min(POLL);
            if !event::poll(timeout).context("failed to poll for input")? {
                if Instant::now() >= deadline {
                    break;
                }
                continue;
            }
            if let Event::Key(key) = event::read().context("failed to read input")? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key.code);
                }
            }
            if app.should_quit() || Instant::now() >= deadline {
                break;
            }
        }
    }

    Ok(())
}
