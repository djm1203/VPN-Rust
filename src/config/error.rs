//! Error types for the configuration module.
//!
//! Configuration parsing is a stable, library-style boundary, so it exposes a
//! concrete [`ConfigError`] (via `thiserror`) rather than an opaque `anyhow`
//! error. Binaries can still aggregate these with `anyhow` at the top level.

use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur while loading or parsing configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// A configuration file could not be read from disk.
    #[error("failed to read config file '{path}'")]
    Read {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// A TOML configuration file failed to parse (path is known).
    ///
    /// The parse error is boxed because `toml::de::Error` is large; keeping it
    /// inline would bloat every `Result<_, ConfigError>` (clippy
    /// `result_large_err`).
    #[error("failed to parse TOML config file '{path}'")]
    TomlFile {
        /// The offending file path.
        path: PathBuf,
        /// The underlying TOML parse error.
        #[source]
        source: Box<toml::de::Error>,
    },

    /// TOML content failed to parse with no associated file path.
    #[error("invalid TOML configuration")]
    Toml(#[source] Box<toml::de::Error>),

    /// A required directive was missing from an OpenVPN configuration.
    #[error("missing '{directive}' directive in OpenVPN configuration")]
    MissingDirective {
        /// The name of the missing directive.
        directive: &'static str,
    },

    /// A parsed configuration value failed validation.
    ///
    /// This is produced by [`crate::config::Config::validate`] and is meant to
    /// be actionable: `field` names the offending key (e.g. `server.server_ip`),
    /// `value` echoes what was supplied, and `reason` states what is allowed.
    #[error("invalid config value for '{field}' = '{value}': {reason}")]
    Invalid {
        /// The dotted field name that failed validation (e.g. `server.subnet`).
        field: &'static str,
        /// The offending value, echoed back to the user.
        value: String,
        /// A human-readable explanation of what is allowed / how to fix it.
        reason: String,
    },
}

impl From<toml::de::Error> for ConfigError {
    fn from(source: toml::de::Error) -> Self {
        ConfigError::Toml(Box::new(source))
    }
}

/// Convenience alias for results within the configuration module.
pub type Result<T> = std::result::Result<T, ConfigError>;
