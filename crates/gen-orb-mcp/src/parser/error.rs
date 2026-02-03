//! Parser-specific error types.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during orb parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Failed to read file from disk.
    #[error("failed to read file '{path}': {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse YAML content.
    #[error("failed to parse YAML in '{path}': {source}")]
    YamlParse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    /// Missing required file in unpacked orb.
    #[error("missing required file: {path}")]
    MissingFile { path: PathBuf },

    /// Invalid orb structure.
    #[error("invalid orb structure: {message}")]
    InvalidStructure { message: String },

    /// Failed to read directory.
    #[error("failed to read directory '{path}': {source}")]
    DirectoryRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
