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
    #[error("failed to parse TOML config file '{path}'")]
    TomlFile {
        /// The offending file path.
        path: PathBuf,
        /// The underlying TOML parse error.
        #[source]
        source: toml::de::Error,
    },

    /// TOML content failed to parse with no associated file path.
    #[error("invalid TOML configuration")]
    Toml(#[from] toml::de::Error),

    /// A required directive was missing from an OpenVPN configuration.
    #[error("missing '{directive}' directive in OpenVPN configuration")]
    MissingDirective {
        /// The name of the missing directive.
        directive: &'static str,
    },
}

/// Convenience alias for results within the configuration module.
pub type Result<T> = std::result::Result<T, ConfigError>;
