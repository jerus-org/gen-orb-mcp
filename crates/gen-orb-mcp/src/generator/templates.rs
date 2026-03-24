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

/// Template for a per-version module file (src/versions/v{version_ident}.rs).
pub const VERSION_MODULE_RS: &str = include_str!("../../templates/version_module.rs.hbs");

/// Template for the versions dispatcher module (src/versions/mod.rs).
pub const VERSIONS_MOD_RS: &str = include_str!("../../templates/versions_mod.rs.hbs");

/// Template for the current-version resource lookup module (src/current/mod.rs).
pub const CURRENT_MOD_RS: &str = include_str!("../../templates/current_mod.rs.hbs");
