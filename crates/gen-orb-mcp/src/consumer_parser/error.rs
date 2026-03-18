//! Error types for the consumer parser.

use thiserror::Error;

/// Errors that can occur while parsing a consumer's `.circleci/` directory.
#[derive(Debug, Error)]
pub enum ConsumerParserError {
    /// Failed to read a CI file.
    #[error("Failed to read CI file '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse a CI file as YAML.
    #[error("Failed to parse YAML in '{path}': {source}")]
    YamlError {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },

    /// The specified directory does not exist or is not a directory.
    #[error("CI directory does not exist or is not accessible: '{path}'")]
    DirectoryNotFound { path: String },

    /// No CI files were found in the directory.
    #[error("No YAML CI files found in directory: '{path}'")]
    NoFilesFound { path: String },
}
