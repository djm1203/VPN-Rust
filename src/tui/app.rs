//! Application state management for the TUI.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

/// Maximum number of log entries to keep in the buffer.
const MAX_LOG_ENTRIES: usize = 100;

/// Connection state for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl ConnectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting...",
            ConnectionState::Connected => "Connected",
            ConnectionState::Reconnecting => "Reconnecting...",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            ConnectionState::Disconnected => Color::Red,
            ConnectionState::Connecting => Color::Yellow,
            ConnectionState::Connected => Color::Green,
            ConnectionState::Reconnecting => Color::Yellow,
        }
    }
}

/// Events that can be sent to update the TUI.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Update connection state.
    StateChange(ConnectionState),
    /// Add bytes to sent counter.
    BytesSent(u64),
    /// Add bytes to received counter.
    BytesReceived(u64),
    /// Add a log entry.
    Log(LogEntry),
    /// Update peer information.
    PeerInfo(String),
    /// Update client count (server mode).
    ClientCount(usize),
    /// Request to quit the application.
    Quit,
}

/// A log entry with timestamp and level.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: Instant,
    pub level: LogLevel,
    pub message: String,
}

/// Log levels for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            LogLevel::Info => Color::Cyan,
            LogLevel::Warn => Color::Yellow,
            LogLevel::Error => Color::Red,
            LogLevel::Debug => Color::Gray,
        }
    }
}

/// Shared statistics that can be updated from async tasks.
#[derive(Debug, Default)]
pub struct Stats {
    pub bytes_sent: AtomicU64,
    pub bytes_received: AtomicU64,
    pub packets_sent: AtomicU64,
    pub packets_received: AtomicU64,
}

impl Stats {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn add_sent(&self, bytes: u64) {
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_received(&self, bytes: u64) {
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
        self.packets_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    pub fn get_bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    pub fn get_packets_sent(&self) -> u64 {
        self.packets_sent.load(Ordering::Relaxed)
    }

    pub fn get_packets_received(&self) -> u64 {
        self.packets_received.load(Ordering::Relaxed)
    }
}

/// Application state for the TUI.
pub struct App {
    /// Current connection state.
    pub state: ConnectionState,
    /// Connection start time (when connected).
    pub connected_at: Option<Instant>,
    /// Application start time.
    pub started_at: Instant,
    /// Traffic statistics.
    pub stats: Arc<Stats>,
    /// Log entries buffer.
    pub logs: VecDeque<LogEntry>,
    /// Peer information (server address for client, or client list for server).
    pub peer_info: String,
    /// Number of connected clients (server mode).
    pub client_count: usize,
    /// Whether running in server mode.
    pub is_server: bool,
    /// Whether the app should quit.
    pub should_quit: bool,
    /// Event receiver.
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
}

impl App {
    /// Creates a new App with an event channel.
    pub fn new(is_server: bool) -> (Self, mpsc::UnboundedSender<AppEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let stats = Stats::new();

        let app = Self {
            state: ConnectionState::Disconnected,
            connected_at: None,
            started_at: Instant::now(),
            stats,
            logs: VecDeque::with_capacity(MAX_LOG_ENTRIES),
            peer_info: String::new(),
            client_count: 0,
            is_server,
            should_quit: false,
            event_rx: rx,
        };

        (app, tx)
    }

    /// Returns a clone of the stats handle for use in async tasks.
    pub fn stats_handle(&self) -> Arc<Stats> {
        Arc::clone(&self.stats)
    }

    /// Process pending events.
    pub fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            self.handle_event(event);
        }
    }

    /// Handle a single event.
    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::StateChange(state) => {
                self.state = state;
                if state == ConnectionState::Connected {
                    self.connected_at = Some(Instant::now());
                } else if state == ConnectionState::Disconnected {
                    self.connected_at = None;
                }
            }
            AppEvent::BytesSent(bytes) => {
                self.stats.add_sent(bytes);
            }
            AppEvent::BytesReceived(bytes) => {
                self.stats.add_received(bytes);
            }
            AppEvent::Log(entry) => {
                if self.logs.len() >= MAX_LOG_ENTRIES {
                    self.logs.pop_front();
                }
                self.logs.push_back(entry);
            }
            AppEvent::PeerInfo(info) => {
                self.peer_info = info;
            }
            AppEvent::ClientCount(count) => {
                self.client_count = count;
            }
            AppEvent::Quit => {
                self.should_quit = true;
            }
        }
    }

    /// Add a log entry.
    pub fn log(&mut self, level: LogLevel, message: impl Into<String>) {
        let entry = LogEntry {
            timestamp: Instant::now(),
            level,
            message: message.into(),
        };

        if self.logs.len() >= MAX_LOG_ENTRIES {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
    }

    /// Get connection duration as a formatted string.
    pub fn connection_duration(&self) -> String {
        match self.connected_at {
            Some(start) => format_duration(start.elapsed()),
            None => "—".to_string(),
        }
    }

    /// Get uptime as a formatted string.
    pub fn uptime(&self) -> String {
        format_duration(self.started_at.elapsed())
    }
}

/// Format a duration as HH:MM:SS.
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

/// Format bytes as a human-readable string.
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
        format!("{} B", bytes)
    }
}
