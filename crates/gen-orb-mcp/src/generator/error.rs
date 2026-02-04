//! Generator-specific error types.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during code generation.
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Failed to render a template.
    #[error("failed to render template '{name}': {source}")]
    TemplateRender {
        name: String,
        #[source]
        source: handlebars::RenderError,
    },

    /// Failed to register a template.
    #[error("failed to register template '{name}': {source}")]
    TemplateRegister {
        name: String,
        #[source]
        source: handlebars::TemplateError,
    },

    /// Failed to register a helper.
    #[error("failed to register helper: {message}")]
    HelperRegister { message: String },

    /// Failed to serialize data for template context.
    #[error("failed to serialize context: {source}")]
    Serialization {
        #[source]
        source: serde_json::Error,
    },

    /// Failed to write output file.
    #[error("failed to write file '{path}': {source}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to create output directory.
    #[error("failed to create directory '{path}': {source}")]
    DirectoryCreate {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to run rustfmt on generated code.
    #[error("rustfmt failed: {message}")]
    RustfmtFailed { message: String },

    /// Failed to run clippy on generated code.
    #[error("clippy failed: {message}")]
    ClippyFailed { message: String },

    /// Invalid orb name.
    #[error("invalid orb name '{name}': {reason}")]
    InvalidOrbName { name: String, reason: String },
}
