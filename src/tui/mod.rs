//! Terminal User Interface for VPN-Rust.
//!
//! This module provides a real-time terminal UI using ratatui,
//! displaying connection status, traffic statistics, and logs.

mod app;
mod runner;
mod ui;

pub use app::{App, AppEvent, ConnectionState, LogEntry, LogLevel, Stats};
pub use runner::{log_entry, send_log, Terminal, TuiRunner};
