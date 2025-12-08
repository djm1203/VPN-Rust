//! Terminal runner for the TUI.

use std::io::{self, stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tokio::sync::mpsc;

use super::app::{App, AppEvent, LogEntry, LogLevel};
use super::ui;

/// Terminal wrapper that handles setup and cleanup.
pub struct Terminal {
    terminal: ratatui::Terminal<CrosstermBackend<io::Stdout>>,
}

impl Terminal {
    /// Create and initialize the terminal.
    pub fn new() -> Result<Self> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

        let backend = CrosstermBackend::new(stdout);
        let terminal =
            ratatui::Terminal::new(backend).context("Failed to create terminal backend")?;

        Ok(Self { terminal })
    }

    /// Draw the UI.
    pub fn draw(&mut self, app: &App) -> Result<()> {
        self.terminal
            .draw(|frame| ui::render(frame, app))
            .context("Failed to draw UI")?;
        Ok(())
    }

    /// Restore the terminal to its original state.
    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .context("Failed to leave alternate screen")?;
        self.terminal.show_cursor().context("Failed to show cursor")?;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// TUI event loop runner.
pub struct TuiRunner {
    terminal: Terminal,
    app: App,
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl TuiRunner {
    /// Create a new TUI runner.
    pub fn new(is_server: bool) -> Result<Self> {
        let terminal = Terminal::new()?;
        let (app, event_tx) = App::new(is_server);

        Ok(Self {
            terminal,
            app,
            event_tx,
        })
    }

    /// Get a clone of the event sender for use in async tasks.
    pub fn event_sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.event_tx.clone()
    }

    /// Get a reference to the stats handle for use in async tasks.
    pub fn stats_handle(&self) -> std::sync::Arc<super::app::Stats> {
        self.app.stats_handle()
    }

    /// Run the TUI event loop.
    ///
    /// This runs until the user quits or an error occurs.
    /// The `tick_rate` parameter controls how often the UI refreshes.
    pub async fn run(&mut self, tick_rate: Duration) -> Result<()> {
        // Log startup
        self.app.log(LogLevel::Info, "TUI started");

        loop {
            // Process any pending app events
            self.app.process_events();

            // Check if we should quit
            if self.app.should_quit {
                break;
            }

            // Draw the UI
            self.terminal.draw(&self.app)?;

            // Poll for keyboard events
            if event::poll(tick_rate).context("Failed to poll for events")? {
                if let Event::Key(key) = event::read().context("Failed to read event")? {
                    // Only handle key press events (not release)
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle a key press.
    fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.app.should_quit = true;
            }
            KeyCode::Char('c') => {
                // Clear logs
                self.app.logs.clear();
                self.app.log(LogLevel::Info, "Logs cleared");
            }
            KeyCode::Char('r') => {
                // Request reconnection (send event)
                self.app.log(LogLevel::Info, "Reconnection requested");
                // The actual reconnection logic would be handled by the VPN code
                // that receives this event
            }
            _ => {}
        }
    }

    /// Restore the terminal (called automatically on drop, but can be called manually).
    pub fn restore(&mut self) -> Result<()> {
        self.terminal.restore()
    }
}

/// Helper function to create a log entry.
pub fn log_entry(level: LogLevel, message: impl Into<String>) -> LogEntry {
    LogEntry {
        timestamp: std::time::Instant::now(),
        level,
        message: message.into(),
    }
}

/// Helper to send a log event.
pub fn send_log(tx: &mpsc::UnboundedSender<AppEvent>, level: LogLevel, message: impl Into<String>) {
    let _ = tx.send(AppEvent::Log(log_entry(level, message)));
}
