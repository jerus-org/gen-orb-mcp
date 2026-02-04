//! Embedded Handlebars templates for code generation.
//!
//! Templates are embedded at compile time using `include_str!` for simplicity
//! and to ensure the generator is self-contained.

/// Template for the main entry point (main.rs).
pub const MAIN_RS: &str = include_str!("../../templates/main.rs.hbs");

/// Template for the library code (lib.rs).
pub const LIB_RS: &str = include_str!("../../templates/lib.rs.hbs");

/// Template for the Cargo manifest (Cargo.toml).
pub const CARGO_TOML: &str = include_str!("../../templates/Cargo.toml.hbs");
