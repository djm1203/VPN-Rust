//! In-memory capture of `tracing` events for the TUI log viewer.
//!
//! [`LogBuffer`] is a bounded, `Arc`-shared ring of recent log records. The
//! [`LogLayer`] plugs into a `tracing_subscriber` registry and pushes every
//! emitted event into the buffer; the TUI drains a snapshot on each render tick
//! and renders it in a scrolling, filterable panel (milestone M4).
//!
//! This replaces printing `tracing` output to stdout while the TUI owns the
//! alternate screen — logs are diverted into the buffer instead of corrupting
//! the terminal UI.

use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// Severity of a captured log record, mirroring `tracing::Level`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Short, fixed-width label for display.
    pub fn as_str(self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }

    fn from_tracing(level: &Level) -> Self {
        match *level {
            Level::TRACE => LogLevel::Trace,
            Level::DEBUG => LogLevel::Debug,
            Level::INFO => LogLevel::Info,
            Level::WARN => LogLevel::Warn,
            Level::ERROR => LogLevel::Error,
        }
    }
}

/// A single captured log record.
#[derive(Debug, Clone)]
pub struct LogRecord {
    /// When the event was captured (monotonic; render code shows elapsed time).
    pub at: Instant,
    pub level: LogLevel,
    /// Event target (module path), useful for filtering.
    pub target: String,
    /// Rendered message and any additional fields.
    pub message: String,
}

/// A bounded ring of recent [`LogRecord`]s, shared between the tracing layer
/// (writer) and the TUI (reader).
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<VecDeque<LogRecord>>>,
    capacity: usize,
}

impl LogBuffer {
    /// Create a buffer retaining at most `capacity` records.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.min(1024)))),
            capacity: capacity.max(1),
        }
    }

    /// Append a record, evicting the oldest if at capacity.
    pub fn push(&self, record: LogRecord) {
        let mut buf = self.inner.lock().unwrap();
        if buf.len() >= self.capacity {
            buf.pop_front();
        }
        buf.push_back(record);
    }

    /// Number of records currently retained.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().unwrap().is_empty()
    }

    /// Copy out all retained records, oldest first.
    pub fn snapshot(&self) -> Vec<LogRecord> {
        self.inner.lock().unwrap().iter().cloned().collect()
    }

    /// A `tracing_subscriber` layer that feeds this buffer.
    pub fn layer(&self) -> LogLayer {
        LogLayer {
            buffer: self.clone(),
        }
    }
}

/// A `tracing_subscriber::Layer` that captures events into a [`LogBuffer`].
pub struct LogLayer {
    buffer: LogBuffer,
}

impl<S: Subscriber> Layer<S> for LogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        self.buffer.push(LogRecord {
            at: Instant::now(),
            level: LogLevel::from_tracing(meta.level()),
            target: meta.target().to_string(),
            message: visitor.finish(),
        });
    }
}

/// Collects the `message` field plus any other fields into a single string.
#[derive(Default)]
struct MessageVisitor {
    message: String,
    extra: String,
}

impl MessageVisitor {
    fn finish(self) -> String {
        if self.extra.is_empty() {
            self.message
        } else if self.message.is_empty() {
            self.extra.trim_start().to_string()
        } else {
            format!("{}{}", self.message, self.extra)
        }
    }
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.extra, " {}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            let _ = write!(self.extra, " {}={}", field.name(), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::subscriber::with_default;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    #[test]
    fn ring_evicts_oldest_beyond_capacity() {
        let buf = LogBuffer::new(2);
        for i in 0..4 {
            buf.push(LogRecord {
                at: Instant::now(),
                level: LogLevel::Info,
                target: "t".into(),
                message: format!("m{i}"),
            });
        }
        let snap = buf.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].message, "m2");
        assert_eq!(snap[1].message, "m3");
    }

    #[test]
    fn layer_captures_event_message_and_level() {
        let buf = LogBuffer::new(16);
        let subscriber = Registry::default().with(buf.layer());
        with_default(subscriber, || {
            tracing::info!("hello world");
            tracing::warn!(code = 7, "danger");
        });

        let snap = buf.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].level, LogLevel::Info);
        assert_eq!(snap[0].message, "hello world");
        assert_eq!(snap[1].level, LogLevel::Warn);
        assert!(snap[1].message.contains("danger"));
        assert!(snap[1].message.contains("code=7"));
    }
}
