//! Dashboard state for the live control cockpit.
//!
//! [`Dashboard`] holds everything the renderer needs and nothing that touches
//! the terminal: it owns the latest [`StatsSnapshot`], derives per-tick
//! throughput history by differencing the engine's cumulative byte counters,
//! keeps a filtered view over the captured logs, and tracks the small amount of
//! interactive UI state (log filter, scroll offset, help overlay). The runner
//! feeds it snapshots via [`Dashboard::on_tick`] and key presses via
//! [`Dashboard::on_key`]; `ui::render` reads it. Keeping all rendering-free
//! logic here makes the whole dashboard unit-testable without a TTY.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use crate::engine::{ConnectionState, StatsSnapshot};
use crate::tui::logbuf::{LogLevel, LogRecord};

/// Maximum number of throughput samples retained for the sparklines.
const MAX_SAMPLES: usize = 120;

/// Rendering-free state for the live dashboard.
pub struct Dashboard {
    /// Most recent telemetry snapshot from the engine.
    snapshot: StatsSnapshot,
    /// Per-tick bytes *sent*, oldest first, capped at [`MAX_SAMPLES`].
    up_history: VecDeque<u64>,
    /// Per-tick bytes *received*, oldest first, capped at [`MAX_SAMPLES`].
    down_history: VecDeque<u64>,
    /// Previous cumulative sent total, for delta computation.
    prev_bytes_up: u64,
    /// Previous cumulative received total, for delta computation.
    prev_bytes_down: u64,
    /// Latest log snapshot, oldest first.
    logs: Vec<LogRecord>,
    /// Minimum level a record must meet to be shown.
    filter: LogLevel,
    /// How many of the newest matching log lines are scrolled out of view.
    scroll: usize,
    /// Whether the help overlay is visible.
    show_help: bool,
    /// True when running as the server.
    is_server: bool,
    /// Set once the user asks to quit.
    should_quit: bool,
    /// When the dashboard was created (drives the uptime readout).
    started_at: Instant,
    /// Number of ticks processed so far.
    ticks: u64,
}

impl Dashboard {
    /// Create a fresh dashboard in the `Disconnected` state.
    pub fn new(is_server: bool) -> Self {
        Self {
            snapshot: empty_snapshot(is_server),
            up_history: VecDeque::with_capacity(MAX_SAMPLES),
            down_history: VecDeque::with_capacity(MAX_SAMPLES),
            prev_bytes_up: 0,
            prev_bytes_down: 0,
            logs: Vec::new(),
            filter: LogLevel::Info,
            scroll: 0,
            show_help: false,
            is_server,
            should_quit: false,
            started_at: Instant::now(),
            ticks: 0,
        }
    }

    /// Fold one tick of telemetry and logs into the dashboard state.
    ///
    /// Computes the sent/received byte deltas against the previously observed
    /// cumulative totals and pushes them onto the bounded throughput ring
    /// buffers, then stores the fresh snapshot and log records.
    pub fn on_tick(&mut self, snapshot: StatsSnapshot, logs: Vec<LogRecord>) {
        let up_delta = snapshot.bytes_up.saturating_sub(self.prev_bytes_up);
        let down_delta = snapshot.bytes_down.saturating_sub(self.prev_bytes_down);
        self.prev_bytes_up = snapshot.bytes_up;
        self.prev_bytes_down = snapshot.bytes_down;

        push_capped(&mut self.up_history, up_delta);
        push_capped(&mut self.down_history, down_delta);

        self.snapshot = snapshot;
        self.logs = logs;
        self.ticks += 1;
    }

    /// The most recent telemetry snapshot.
    pub fn snapshot(&self) -> &StatsSnapshot {
        &self.snapshot
    }

    /// Per-tick sent-byte history for the TX sparkline.
    pub fn up_history(&self) -> &VecDeque<u64> {
        &self.up_history
    }

    /// Per-tick received-byte history for the RX sparkline.
    pub fn down_history(&self) -> &VecDeque<u64> {
        &self.down_history
    }

    /// The most recent per-tick sent delta, i.e. the current TX rate sample.
    pub fn up_rate(&self) -> u64 {
        self.up_history.back().copied().unwrap_or(0)
    }

    /// The most recent per-tick received delta, i.e. the current RX rate sample.
    pub fn down_rate(&self) -> u64 {
        self.down_history.back().copied().unwrap_or(0)
    }

    /// Logs at or above the active filter level, oldest first.
    pub fn filtered_logs(&self) -> impl Iterator<Item = &LogRecord> {
        let filter = self.filter;
        self.logs.iter().filter(move |r| r.level >= filter)
    }

    /// The active minimum log level.
    pub fn filter(&self) -> LogLevel {
        self.filter
    }

    /// Number of newest matching log lines currently scrolled out of view.
    pub fn scroll(&self) -> usize {
        self.scroll
    }

    /// Whether the help overlay should be drawn.
    pub fn show_help(&self) -> bool {
        self.show_help
    }

    /// True when running as the server.
    pub fn is_server(&self) -> bool {
        self.is_server
    }

    /// Whether the runner should exit the event loop.
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Wall-clock time since the dashboard started.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Handle a key press, updating interactive state.
    pub fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('?') | KeyCode::Char('h') => self.show_help = !self.show_help,
            KeyCode::Char('f') => self.filter = cycle_filter(self.filter),
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageDown => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Char('g') | KeyCode::Home => self.scroll = 0,
            KeyCode::Char('c') => {
                self.scroll = 0;
                self.filter = LogLevel::Info;
            }
            _ => {}
        }
    }
}

/// Push `value`, evicting the oldest sample once the ring is full.
fn push_capped(ring: &mut VecDeque<u64>, value: u64) {
    if ring.len() >= MAX_SAMPLES {
        ring.pop_front();
    }
    ring.push_back(value);
}

/// Advance the log filter one step, wrapping `Error → Trace`.
fn cycle_filter(level: LogLevel) -> LogLevel {
    match level {
        LogLevel::Trace => LogLevel::Debug,
        LogLevel::Debug => LogLevel::Info,
        LogLevel::Info => LogLevel::Warn,
        LogLevel::Warn => LogLevel::Error,
        LogLevel::Error => LogLevel::Trace,
    }
}

/// A `Disconnected`, all-zero snapshot for the pre-first-tick state.
fn empty_snapshot(is_server: bool) -> StatsSnapshot {
    StatsSnapshot {
        state: ConnectionState::Disconnected,
        bytes_up: 0,
        bytes_down: 0,
        packets_up: 0,
        packets_down: 0,
        rtt: None,
        connected_for: None,
        reconnect_attempts: 0,
        peer: None,
        negotiated: None,
        is_server,
        endpoint: None,
    }
}

/// Format a byte count as `B`/`KB`/`MB`/`GB` with two decimals above 1 KiB.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a throughput rate (bytes per second) as a human-readable `…/s`.
pub fn format_rate(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

/// Format a duration as `HH:MM:SS` (or `MM:SS` under an hour).
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    if hours > 0 {
        format!("{hours:02}:{mins:02}:{secs:02}")
    } else {
        format!("{mins:02}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn snapshot_with(bytes_up: u64, bytes_down: u64) -> StatsSnapshot {
        StatsSnapshot {
            state: ConnectionState::Connected,
            bytes_up,
            bytes_down,
            packets_up: 0,
            packets_down: 0,
            rtt: None,
            connected_for: None,
            reconnect_attempts: 0,
            peer: None,
            negotiated: None,
            is_server: false,
            endpoint: None,
        }
    }

    fn log(level: LogLevel, message: &str) -> LogRecord {
        LogRecord {
            at: Instant::now(),
            level,
            target: "test".into(),
            message: message.into(),
        }
    }

    #[test]
    fn on_tick_computes_byte_deltas() {
        let mut app = Dashboard::new(false);
        app.on_tick(snapshot_with(1000, 200), vec![]);
        app.on_tick(snapshot_with(1500, 500), vec![]);

        assert_eq!(
            app.up_history().iter().copied().collect::<Vec<_>>(),
            vec![1000, 500]
        );
        assert_eq!(
            app.down_history().iter().copied().collect::<Vec<_>>(),
            vec![200, 300]
        );
        assert_eq!(app.up_rate(), 500);
        assert_eq!(app.down_rate(), 300);
    }

    #[test]
    fn history_is_capped() {
        let mut app = Dashboard::new(false);
        for i in 1..=(MAX_SAMPLES as u64 + 20) {
            app.on_tick(snapshot_with(i * 10, 0), vec![]);
        }
        assert_eq!(app.up_history().len(), MAX_SAMPLES);
        // Each delta is a constant 10 after the first sample.
        assert_eq!(*app.up_history().back().unwrap(), 10);
    }

    #[test]
    fn counter_reset_does_not_underflow() {
        let mut app = Dashboard::new(false);
        app.on_tick(snapshot_with(5000, 5000), vec![]);
        // A smaller cumulative total (e.g. after a reconnect) must not panic.
        app.on_tick(snapshot_with(100, 100), vec![]);
        assert_eq!(app.up_rate(), 0);
        assert_eq!(app.down_rate(), 0);
    }

    #[test]
    fn filter_only_passes_records_at_or_above_level() {
        let mut app = Dashboard::new(false);
        app.on_tick(
            snapshot_with(0, 0),
            vec![
                log(LogLevel::Debug, "d"),
                log(LogLevel::Info, "i"),
                log(LogLevel::Error, "e"),
            ],
        );
        // Default filter is Info: Debug is hidden.
        let msgs: Vec<_> = app.filtered_logs().map(|r| r.message.as_str()).collect();
        assert_eq!(msgs, vec!["i", "e"]);
    }

    #[test]
    fn filter_cycles_through_all_levels_and_wraps() {
        let mut app = Dashboard::new(false);
        assert_eq!(app.filter(), LogLevel::Info);
        app.on_key(KeyCode::Char('f'));
        assert_eq!(app.filter(), LogLevel::Warn);
        app.on_key(KeyCode::Char('f'));
        assert_eq!(app.filter(), LogLevel::Error);
        app.on_key(KeyCode::Char('f'));
        assert_eq!(app.filter(), LogLevel::Trace);
        app.on_key(KeyCode::Char('f'));
        assert_eq!(app.filter(), LogLevel::Debug);
        app.on_key(KeyCode::Char('f'));
        assert_eq!(app.filter(), LogLevel::Info);
    }

    #[test]
    fn scroll_keys_saturate_and_reset() {
        let mut app = Dashboard::new(false);
        app.on_key(KeyCode::Down); // already 0, stays 0
        assert_eq!(app.scroll(), 0);
        app.on_key(KeyCode::Up);
        app.on_key(KeyCode::Up);
        assert_eq!(app.scroll(), 2);
        app.on_key(KeyCode::PageUp);
        assert_eq!(app.scroll(), 12);
        app.on_key(KeyCode::PageDown);
        assert_eq!(app.scroll(), 2);
        app.on_key(KeyCode::Home);
        assert_eq!(app.scroll(), 0);
    }

    #[test]
    fn clear_resets_scroll_and_filter() {
        let mut app = Dashboard::new(false);
        app.on_key(KeyCode::Char('f')); // Warn
        app.on_key(KeyCode::Up); // scroll 1
        app.on_key(KeyCode::Char('c'));
        assert_eq!(app.filter(), LogLevel::Info);
        assert_eq!(app.scroll(), 0);
    }

    #[test]
    fn quit_and_help_keys() {
        let mut app = Dashboard::new(false);
        assert!(!app.should_quit());
        assert!(!app.show_help());
        app.on_key(KeyCode::Char('?'));
        assert!(app.show_help());
        app.on_key(KeyCode::Char('h'));
        assert!(!app.show_help());
        app.on_key(KeyCode::Char('q'));
        assert!(app.should_quit());
    }

    #[test]
    fn format_helpers() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_rate(2048), "2.00 KB/s");
        assert_eq!(format_duration(Duration::from_secs(65)), "01:05");
        assert_eq!(format_duration(Duration::from_secs(3661)), "01:01:01");
    }

    #[test]
    fn snapshot_carries_peer_and_role() {
        let mut app = Dashboard::new(true);
        let mut snap = snapshot_with(1, 1);
        snap.peer = Some("127.0.0.1:9000".parse::<SocketAddr>().unwrap());
        snap.is_server = true;
        app.on_tick(snap, vec![]);
        assert!(app.is_server());
        assert!(app.snapshot().peer.is_some());
    }
}
