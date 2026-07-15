//! Terminal User Interface for VPN-Rust.
//!
//! An event-driven, live control cockpit built on ratatui: [`Dashboard`] holds
//! rendering-free state (in `app`), `ui` draws it, and [`run_dashboard`] (in
//! `runner`) owns the terminal and event loop. Log capture lives in
//! [`logbuf`], which the engine feeds via a `tracing` layer and the dashboard
//! samples each tick.

mod app;
pub mod logbuf;
mod runner;
mod ui;

pub use app::Dashboard;
pub use logbuf::{LogBuffer, LogLayer, LogRecord};
pub use runner::run_dashboard;
