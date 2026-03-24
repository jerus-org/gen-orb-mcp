//! Code generator module for creating MCP servers from orb definitions.
//!
//! This module transforms a parsed `OrbDefinition` into a working MCP server
//! by rendering Handlebars templates to produce Rust source code.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//!
//! use gen_orb_mcp::{generator::CodeGenerator, parser::OrbParser};
//!
//! let orb = OrbParser::parse(Path::new("./src/@orb.yml")).unwrap();
//! let generator = CodeGenerator::new().unwrap();
//! let server = generator.generate(&orb, "my-orb", "1.0.0").unwrap();
//!
//! // Write to output directory
//! server.write_to(Path::new("./dist")).unwrap();
//! ```

pub mod context;
pub mod error;
pub mod templates;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub use context::GeneratorContext;
pub use error::GeneratorError;
use handlebars::Handlebars;

use crate::parser::OrbDefinition;

/// Generated MCP server output containing all source files.
#[derive(Debug, Clone)]
pub struct GeneratedServer {
    /// Map of relative file paths to their text content.
    pub files: HashMap<PathBuf, String>,

    /// Map of relative file paths to their binary content.
    ///
    /// Used for non-text artefacts such as `data/versions.bin`.
    pub binary_files: HashMap<PathBuf, Vec<u8>>,

    /// The crate name for the generated server.
    pub crate_name: String,

    /// The orb name this server was generated from.
    pub orb_name: String,
}

impl GeneratedServer {
    /// Write all generated files to the specified output directory.
    ///
    /// Creates the directory structure if it doesn't exist.
    pub fn write_to(&self, output_dir: &Path) -> Result<(), GeneratorError> {
        // Create output directory
        fs::create_dir_all(output_dir).map_err(|e| GeneratorError::DirectoryCreate {
            path: output_dir.to_path_buf(),
            source: e,
        })?;

        // Create src subdirectory
        let src_dir = output_dir.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| GeneratorError::DirectoryCreate {
            path: src_dir.clone(),
            source: e,
        })?;

        // Write text files
        for (rel_path, content) in &self.files {
            let full_path = output_dir.join(rel_path);

            // Ensure parent directory exists
            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).map_err(|e| GeneratorError::DirectoryCreate {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }

            fs::write(&full_path, content).map_err(|e| GeneratorError::FileWrite {
                path: full_path.clone(),
                source: e,
            })?;
        }

        // Write binary files
        for (rel_path, content) in &self.binary_files {
            let full_path = output_dir.join(rel_path);

            if let Some(parent) = full_path.parent() {
                fs::create_dir_all(parent).map_err(|e| GeneratorError::DirectoryCreate {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }

            fs::write(&full_path, content).map_err(|e| GeneratorError::FileWrite {
                path: full_path.clone(),
                source: e,
            })?;
        }

        Ok(())
    }

    /// Format the generated Rust files using rustfmt.
    ///
    /// This modifies the files in-place within the GeneratedServer.
    pub fn format(&mut self, output_dir: &Path) -> Result<(), GeneratorError> {
        // Write files first so rustfmt can process them
        self.write_to(output_dir)?;

        // Collect Rust file paths first to avoid borrow issues
        let rs_files: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|p| p.extension().is_some_and(|ext| ext == "rs"))
            .cloned()
            .collect();

        // Run rustfmt on each Rust file
        for rel_path in rs_files {
            let full_path = output_dir.join(&rel_path);
            run_rustfmt(&full_path)?;

            // Read back the formatted content
            let formatted =
                fs::read_to_string(&full_path).map_err(|e| GeneratorError::FileWrite {
                    path: full_path.clone(),
                    source: e,
                })?;

            self.files.insert(rel_path, formatted);
        }

        Ok(())
    }
}

/// Code generator that transforms orb definitions into MCP server source code.
#[derive(Debug)]
pub struct CodeGenerator<'a> {
    handlebars: Handlebars<'a>,
    prior_versions: Vec<(String, OrbDefinition)>,
    conformance_rules_json: Option<String>,
}

impl<'a> CodeGenerator<'a> {
    /// Set prior-version snapshots to embed alongside the current version.
    pub fn with_prior_versions(mut self, versions: Vec<(String, OrbDefinition)>) -> Self {
        self.prior_versions = versions;
        self
    }

    /// Set serialised conformance rules JSON to embed as MCP Tools in the
    /// generated server.
    pub fn with_conformance_rules_json(mut self, json: String) -> Self {
        self.conformance_rules_json = Some(json);
        self
    }

    /// Optionally set conformance rules JSON; `None` leaves tools disabled.
    pub fn with_conformance_rules_json_opt(mut self, json: Option<String>) -> Self {
        self.conformance_rules_json = json;
        self
    }

    /// Create a new code generator with registered templates.
    pub fn new() -> Result<Self, GeneratorError> {
        let mut handlebars = Handlebars::new();

        // Disable HTML escaping for code generation
        handlebars.register_escape_fn(handlebars::no_escape);

        // Register templates
        handlebars
            .register_template_string("main.rs", templates::MAIN_RS)
            .map_err(|e| GeneratorError::TemplateRegister {
                name: "main.rs".to_string(),
                source: e,
            })?;

        handlebars
            .register_template_string("lib.rs", templates::LIB_RS)
            .map_err(|e| GeneratorError::TemplateRegister {
                name: "lib.rs".to_string(),
                source: e,
            })?;

        handlebars
            .register_template_string("Cargo.toml", templates::CARGO_TOML)
            .map_err(|e| GeneratorError::TemplateRegister {
                name: "Cargo.toml".to_string(),
                source: e,
            })?;

        handlebars
            .register_template_string("version_module.rs", templates::VERSION_MODULE_RS)
            .map_err(|e| GeneratorError::TemplateRegister {
                name: "version_module.rs".to_string(),
                source: e,
            })?;

        handlebars
            .register_template_string("versions_mod.rs", templates::VERSIONS_MOD_RS)
            .map_err(|e| GeneratorError::TemplateRegister {
                name: "versions_mod.rs".to_string(),
                source: e,
            })?;

        handlebars
            .register_template_string("current_mod.rs", templates::CURRENT_MOD_RS)
            .map_err(|e| GeneratorError::TemplateRegister {
                name: "current_mod.rs".to_string(),
                source: e,
            })?;

        // Register custom helpers
        register_helpers(&mut handlebars);

        Ok(Self {
            handlebars,
            prior_versions: vec![],
            conformance_rules_json: None,
        })
    }

    /// Generate an MCP server from an orb definition.
    ///
    /// # Arguments
    ///
    /// * `orb` - The parsed orb definition
    /// * `orb_name` - The name to use for the orb (typically derived from
    ///   filename)
    /// * `version` - The semantic version for the generated MCP server crate
    ///
    /// # Returns
    ///
    /// A `GeneratedServer` containing all source files ready to be written.
    pub fn generate(
        &self,
        orb: &OrbDefinition,
        orb_name: &str,
        version: &str,
    ) -> Result<GeneratedServer, GeneratorError> {
        // Validate orb name
        validate_orb_name(orb_name)?;

        // Build template context
        let context = GeneratorContext::from_orb_with_extras(
            orb,
            orb_name,
            version,
            self.prior_versions.clone(),
            self.conformance_rules_json.clone(),
        );

        // Serialize context for templates
        let ctx_json = serde_json::to_value(&context)
            .map_err(|e| GeneratorError::Serialization { source: e })?;

        // Render templates
        let mut files = HashMap::new();
        let mut binary_files: HashMap<PathBuf, Vec<u8>> = HashMap::new();

        // main.rs
        let main_rs = self.handlebars.render("main.rs", &ctx_json).map_err(|e| {
            GeneratorError::TemplateRender {
                name: "main.rs".to_string(),
                source: e,
            }
        })?;
        files.insert(PathBuf::from("src/main.rs"), main_rs);

        // lib.rs
        let lib_rs = self.handlebars.render("lib.rs", &ctx_json).map_err(|e| {
            GeneratorError::TemplateRender {
                name: "lib.rs".to_string(),
                source: e,
            }
        })?;
        files.insert(PathBuf::from("src/lib.rs"), lib_rs);

        // Cargo.toml
        let cargo_toml = self
            .handlebars
            .render("Cargo.toml", &ctx_json)
            .map_err(|e| GeneratorError::TemplateRender {
                name: "Cargo.toml".to_string(),
                source: e,
            })?;
        files.insert(PathBuf::from("Cargo.toml"), cargo_toml);

        // Current-version resource data
        //
        // Instead of embedding json_content inline in the read_resource match
        // expression (which causes LLVM to OOM for large orbs), all current-
        // version resource content is packed into data/current.bin and looked
        // up at runtime via include_bytes! in src/current/mod.rs.
        if context.has_resources {
            let current_bin =
                build_current_bin(&context.commands, &context.jobs, &context.executors);
            binary_files.insert(PathBuf::from("data/current.bin"), current_bin);

            let current_mod = self
                .handlebars
                .render("current_mod.rs", &ctx_json)
                .map_err(|e| GeneratorError::TemplateRender {
                    name: "current_mod.rs".to_string(),
                    source: e,
                })?;
            files.insert(PathBuf::from("src/current/mod.rs"), current_mod);
        }

        // Prior-version data (when prior versions are present)
        //
        // Instead of generating one .rs file per version (which embeds all
        // JSON as inline string literals and causes LLVM to OOM during release
        // compilation), we pack all content into a single binary blob
        // (`data/versions.bin`) and generate a tiny `src/versions/mod.rs`
        // shim that looks up entries via `include_bytes!`.
        if context.has_prior_versions {
            // data/versions.bin — compact binary lookup table
            let versions_bin = build_versions_bin(&context.prior_versions);
            binary_files.insert(PathBuf::from("data/versions.bin"), versions_bin);

            // src/versions/mod.rs — include_bytes! shim + sequential lookup fn
            let versions_mod = self
                .handlebars
                .render("versions_mod.rs", &ctx_json)
                .map_err(|e| GeneratorError::TemplateRender {
                    name: "versions_mod.rs".to_string(),
                    source: e,
                })?;
            files.insert(PathBuf::from("src/versions/mod.rs"), versions_mod);
        }

        Ok(GeneratedServer {
            files,
            binary_files,
            crate_name: context.crate_name,
            orb_name: orb_name.to_string(),
        })
    }

    /// Generate an MCP server and format the output.
    ///
    /// This is a convenience method that generates and formats in one step.
    pub fn generate_formatted(
        &self,
        orb: &OrbDefinition,
        orb_name: &str,
        version: &str,
        output_dir: &Path,
    ) -> Result<GeneratedServer, GeneratorError> {
        let mut server = self.generate(orb, orb_name, version)?;
        server.format(output_dir)?;
        Ok(server)
    }
}

/// Register custom Handlebars helpers.
fn register_helpers(handlebars: &mut Handlebars) {
    // Helper to get array length
    handlebars.register_helper(
        "length",
        Box::new(
            |h: &handlebars::Helper,
             _: &Handlebars,
             _: &handlebars::Context,
             _: &mut handlebars::RenderContext,
             out: &mut dyn handlebars::Output|
             -> handlebars::HelperResult {
                let param = h.param(0).ok_or_else(|| {
                    handlebars::RenderErrorReason::ParamNotFoundForIndex("length", 0)
                })?;

                let len = match param.value() {
                    serde_json::Value::Array(arr) => arr.len(),
                    serde_json::Value::Object(obj) => obj.len(),
                    serde_json::Value::String(s) => s.len(),
                    _ => 0,
                };

                out.write(&len.to_string())?;
                Ok(())
            },
        ),
    );
}

/// Validate that the orb name is valid for use in generated code.
fn validate_orb_name(name: &str) -> Result<(), GeneratorError> {
    if name.is_empty() {
        return Err(GeneratorError::InvalidOrbName {
            name: name.to_string(),
            reason: "name cannot be empty".to_string(),
        });
    }

    // Check for valid characters (alphanumeric, hyphens, underscores)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(GeneratorError::InvalidOrbName {
            name: name.to_string(),
            reason: "name can only contain alphanumeric characters, hyphens, and underscores"
                .to_string(),
        });
    }

    // Must start with a letter
    if !name.chars().next().is_some_and(|c| c.is_alphabetic()) {
        return Err(GeneratorError::InvalidOrbName {
            name: name.to_string(),
            reason: "name must start with a letter".to_string(),
        });
    }

    Ok(())
}

/// Run rustfmt on a file.
fn run_rustfmt(path: &Path) -> Result<(), GeneratorError> {
    let output = Command::new("rustfmt").arg(path).output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // If rustfmt is not installed or fails, we continue without formatting
            tracing::warn!("rustfmt warning for {}: {}", path.display(), stderr);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // rustfmt not installed, skip formatting
            tracing::debug!("rustfmt not found, skipping formatting");
            Ok(())
        }
        Err(e) => Err(GeneratorError::RustfmtFailed {
            message: e.to_string(),
        }),
    }
}

/// Encode a list of (key, value) string pairs into the compact binary format.
///
/// # Format
///
/// ```text
/// [u32 count (LE)]
/// For each entry:
///   [u32 key_len (LE)] [key bytes (UTF-8 URI)]
///   [u32 val_len (LE)] [val bytes (UTF-8 JSON)]
/// ```
fn encode_bin_entries(entries: &[(&str, &str)]) -> Vec<u8> {
    let count = entries.len() as u32;
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&count.to_le_bytes());
    for (key, val) in entries {
        let kb = key.as_bytes();
        let vb = val.as_bytes();
        data.extend_from_slice(&(kb.len() as u32).to_le_bytes());
        data.extend_from_slice(kb);
        data.extend_from_slice(&(vb.len() as u32).to_le_bytes());
        data.extend_from_slice(vb);
    }
    data
}

/// Build a compact binary data blob from all prior-version snapshots.
///
/// The generated `src/versions/mod.rs` contains an identical sequential-scan
/// lookup that reads from this blob via `include_bytes!`.  Using binary data
/// avoids embedding the content as Rust string literals, which causes LLVM to
/// run out of memory when compiling large orbs with many historical versions.
fn build_versions_bin(prior_versions: &[context::VersionSnapshot]) -> Vec<u8> {
    let mut entries: Vec<(&str, &str)> = Vec::new();
    for snap in prior_versions {
        for item in &snap.commands {
            entries.push((&item.uri, &item.json_content));
        }
        for item in &snap.jobs {
            entries.push((&item.uri, &item.json_content));
        }
        for item in &snap.executors {
            entries.push((&item.uri, &item.json_content));
        }
    }
    encode_bin_entries(&entries)
}

/// Build a compact binary data blob from the current-version resources.
///
/// The generated `src/current/mod.rs` contains an identical sequential-scan
/// lookup that reads from this blob via `include_bytes!`.  This avoids
/// embedding large JSON strings as inline Rust string literals inside the
/// `read_resource` match expression, which causes LLVM to run out of memory
/// when compiling large orbs with many commands/jobs/executors.
fn build_current_bin(
    commands: &[context::CommandContext],
    jobs: &[context::JobContext],
    executors: &[context::ExecutorContext],
) -> Vec<u8> {
    let mut entries: Vec<(&str, &str)> = Vec::new();
    for item in commands {
        entries.push((&item.uri, &item.json_content));
    }
    for item in jobs {
        entries.push((&item.uri, &item.json_content));
    }
    for item in executors {
        entries.push((&item.uri, &item.json_content));
    }
    encode_bin_entries(&entries)
}

/// Run clippy --fix on a project directory.
#[allow(dead_code)]
fn run_clippy_fix(project_dir: &Path) -> Result<(), GeneratorError> {
    let output = Command::new("cargo")
        .args(["clippy", "--fix", "--allow-dirty", "--allow-staged"])
        .current_dir(project_dir)
        .output();

    match output {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!("clippy warning: {}", stderr);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!("cargo not found, skipping clippy");
            Ok(())
        }
        Err(e) => Err(GeneratorError::ClippyFailed {
            message: e.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;
    use crate::parser::{Command, OrbDefinition, Parameter, ParameterType};

    fn create_test_orb() -> OrbDefinition {
        let mut orb = OrbDefinition {
            version: "2.1".to_string(),
            description: Some("Test orb for unit tests".to_string()),
            ..Default::default()
        };

        // Add a command
        let mut params = HashMap::new();
        params.insert(
            "name".to_string(),
            Parameter {
                param_type: ParameterType::String,
                description: Some("Name to greet".to_string()),
                default: Some(serde_yaml::Value::String("World".to_string())),
                enum_values: None,
            },
        );

        orb.commands.insert(
            "greet".to_string(),
            Command {
                description: Some("Greet someone".to_string()),
                parameters: params,
                steps: vec![],
            },
        );

        orb
    }

    #[test]
    fn test_code_generator_new() {
        let generator = CodeGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_generate_produces_files() {
        let generator = CodeGenerator::new().unwrap();
        let orb = create_test_orb();

        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();

        assert!(server.files.contains_key(&PathBuf::from("src/main.rs")));
        assert!(server.files.contains_key(&PathBuf::from("src/lib.rs")));
        assert!(server.files.contains_key(&PathBuf::from("Cargo.toml")));
        assert_eq!(server.crate_name, "test_orb_mcp");
        assert_eq!(server.orb_name, "test-orb");
    }

    #[test]
    fn test_generated_main_contains_tokio() {
        let generator = CodeGenerator::new().unwrap();
        let orb = create_test_orb();

        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let main_rs = server.files.get(&PathBuf::from("src/main.rs")).unwrap();

        assert!(main_rs.contains("#[tokio::main]"));
        assert!(main_rs.contains("test_orb_mcp::OrbServer::new"));
    }

    #[test]
    fn test_generated_lib_contains_resources() {
        let generator = CodeGenerator::new().unwrap();
        let orb = create_test_orb();

        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(lib_rs.contains("ServerHandler"));
        assert!(lib_rs.contains("RawResource"));
        assert!(lib_rs.contains("orb://commands/greet"));
        assert!(lib_rs.contains("orb://overview"));
    }

    #[test]
    fn test_generated_cargo_toml() {
        let generator = CodeGenerator::new().unwrap();
        let orb = create_test_orb();

        let server = generator.generate(&orb, "test-orb", "2.5.0").unwrap();
        let cargo = server.files.get(&PathBuf::from("Cargo.toml")).unwrap();

        assert!(cargo.contains("name = \"test_orb_mcp\""));
        assert!(cargo.contains("version = \"2.5.0\""));
        assert!(cargo.contains("rmcp = "));
        assert!(cargo.contains("tokio = "));
    }

    #[test]
    fn test_write_to_directory() {
        let generator = CodeGenerator::new().unwrap();
        let orb = create_test_orb();
        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();

        let temp_dir = TempDir::new().unwrap();
        server.write_to(temp_dir.path()).unwrap();

        assert!(temp_dir.path().join("src/main.rs").exists());
        assert!(temp_dir.path().join("src/lib.rs").exists());
        assert!(temp_dir.path().join("Cargo.toml").exists());
    }

    #[test]
    fn test_validate_orb_name() {
        assert!(validate_orb_name("my-orb").is_ok());
        assert!(validate_orb_name("my_orb").is_ok());
        assert!(validate_orb_name("myOrb").is_ok());
        assert!(validate_orb_name("myOrb123").is_ok());

        assert!(validate_orb_name("").is_err());
        assert!(validate_orb_name("123orb").is_err());
        assert!(validate_orb_name("my orb").is_err());
        assert!(validate_orb_name("my.orb").is_err());
    }

    #[test]
    fn test_empty_orb_generates_valid_code() {
        let generator = CodeGenerator::new().unwrap();
        let orb = OrbDefinition::default();

        let server = generator.generate(&orb, "empty-orb", "0.1.0").unwrap();

        // Should still generate valid files even with no commands/jobs/executors
        assert!(server.files.contains_key(&PathBuf::from("src/main.rs")));
        assert!(server.files.contains_key(&PathBuf::from("src/lib.rs")));
    }

    #[test]
    fn test_generate_with_prior_versions_includes_version_resources() {
        let mut prior_orb = OrbDefinition::default();
        prior_orb.commands.insert(
            "old-cmd".to_string(),
            Command {
                description: Some("An old command".to_string()),
                parameters: HashMap::new(),
                steps: vec![],
            },
        );

        let current_orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_prior_versions(vec![("1.0.0".to_string(), prior_orb)]);

        let server = generator
            .generate(&current_orb, "test-orb", "2.0.0")
            .unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        // Should expose the version list resource
        assert!(
            lib_rs.contains("orb://versions"),
            "expected orb://versions resource"
        );
        // Prior-version URIs must NOT be inline in lib.rs — they live in
        // versions.bin and are served at runtime via versions::get(uri).
        // Inlining them causes the list_resources function to grow to hundreds
        // of KB, exhausting LLVM during release compilation.
        assert!(
            !lib_rs.contains("orb://v1.0.0/commands/old-cmd"),
            "prior-version URIs must not be inline in lib.rs (causes LLVM OOM)"
        );
    }

    #[test]
    fn test_generate_with_conformance_rules_includes_tools() {
        let rules_json =
            r#"[{"type":"JobRenamed","from":"old","to":"new","since_version":"2.0.0","description":"renamed"}]"#
                .to_string();
        let orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_conformance_rules_json(rules_json.clone());

        let server = generator.generate(&orb, "test-orb", "2.0.0").unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(
            lib_rs.contains("plan_migration"),
            "expected plan_migration tool"
        );
        assert!(
            lib_rs.contains("apply_migration"),
            "expected apply_migration tool"
        );
        assert!(
            lib_rs.contains("CONFORMANCE_RULES_JSON"),
            "expected embedded rules const"
        );
    }

    #[test]
    fn test_generate_without_tools_has_no_tool_methods() {
        let orb = create_test_orb();
        let generator = CodeGenerator::new().unwrap();

        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(
            !lib_rs.contains("plan_migration"),
            "should not contain tool methods without rules"
        );
        assert!(
            !lib_rs.contains("CONFORMANCE_RULES_JSON"),
            "should not embed rules const without rules"
        );
    }

    #[test]
    fn test_generate_with_tools_cargo_toml_includes_gen_orb_mcp_dep() {
        let rules_json = r#"[]"#.to_string();
        let orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_conformance_rules_json(rules_json);

        let server = generator.generate(&orb, "test-orb", "2.0.0").unwrap();
        let cargo_toml = server.files.get(&PathBuf::from("Cargo.toml")).unwrap();

        // Check for a dependency entry (not just the comment "Generated by
        // gen-orb-mcp")
        assert!(
            cargo_toml.contains("gen-orb-mcp = {"),
            "expected gen-orb-mcp dependency entry when has_tools"
        );
    }

    #[test]
    fn test_generate_without_tools_cargo_toml_excludes_gen_orb_mcp_dep() {
        let orb = create_test_orb();
        let generator = CodeGenerator::new().unwrap();

        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let cargo_toml = server.files.get(&PathBuf::from("Cargo.toml")).unwrap();

        // Check absence of a dependency entry (the comment "Generated by gen-orb-mcp"
        // is allowed)
        assert!(
            !cargo_toml.contains("gen-orb-mcp = {"),
            "should not include gen-orb-mcp dep entry without tools"
        );
    }

    #[test]
    fn test_generate_with_prior_versions_produces_versions_bin_not_rs_files() {
        // After the binary-data refactor, per-version .rs files must NOT be
        // generated.  Instead the content lives in `data/versions.bin` (a
        // binary blob embedded via include_bytes!).  This prevents the Rust
        // compiler from running out of memory when processing thousands of
        // large inline string literals with release optimisations.
        let mut prior_orb = OrbDefinition::default();
        prior_orb.commands.insert(
            "old-cmd".to_string(),
            Command {
                description: Some("An old command".to_string()),
                parameters: HashMap::new(),
                steps: vec![],
            },
        );

        let current_orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_prior_versions(vec![("1.0.0".to_string(), prior_orb)]);

        let server = generator
            .generate(&current_orb, "test-orb", "2.0.0")
            .unwrap();

        // Binary data file must be present
        assert!(
            server
                .binary_files
                .contains_key(&PathBuf::from("data/versions.bin")),
            "expected data/versions.bin in binary_files"
        );
        // versions/mod.rs (the lookup shim) must be present
        assert!(
            server
                .files
                .contains_key(&PathBuf::from("src/versions/mod.rs")),
            "expected src/versions/mod.rs"
        );
        // Per-version .rs files must NOT be generated
        assert!(
            !server
                .files
                .contains_key(&PathBuf::from("src/versions/v1_0_0.rs")),
            "per-version rs files must not be generated (causes OOM at compile)"
        );
    }

    #[test]
    fn test_generate_without_prior_versions_has_no_versions_dir() {
        let orb = create_test_orb();
        let generator = CodeGenerator::new().unwrap();

        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();

        assert!(
            !server
                .files
                .contains_key(&PathBuf::from("src/versions/mod.rs")),
            "should not have versions/mod.rs without prior versions"
        );
    }

    #[test]
    fn test_generate_with_prior_versions_lib_has_mod_declaration() {
        let mut prior_orb = OrbDefinition::default();
        prior_orb.commands.insert(
            "old-cmd".to_string(),
            Command {
                description: None,
                parameters: HashMap::new(),
                steps: vec![],
            },
        );

        let current_orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_prior_versions(vec![("1.0.0".to_string(), prior_orb)]);

        let server = generator
            .generate(&current_orb, "test-orb", "2.0.0")
            .unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(
            lib_rs.contains("mod versions;"),
            "lib.rs should declare mod versions;"
        );
    }

    #[test]
    fn test_generate_with_prior_versions_versions_mod_uses_include_bytes() {
        // versions/mod.rs must embed data via include_bytes! (not inline
        // string literals) and expose a `get(uri) -> Option<String>` lookup.
        let mut prior_orb = OrbDefinition::default();
        prior_orb.commands.insert(
            "old-cmd".to_string(),
            Command {
                description: None,
                parameters: HashMap::new(),
                steps: vec![],
            },
        );

        let current_orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_prior_versions(vec![("1.0.0".to_string(), prior_orb)]);

        let server = generator
            .generate(&current_orb, "test-orb", "2.0.0")
            .unwrap();
        let versions_mod = server
            .files
            .get(&PathBuf::from("src/versions/mod.rs"))
            .unwrap();

        assert!(
            versions_mod.contains("include_bytes!"),
            "versions/mod.rs must use include_bytes! not inline string literals"
        );
        assert!(
            versions_mod.contains("pub(crate) fn get"),
            "versions/mod.rs should expose pub(crate) fn get"
        );
        // The URI from the prior version must NOT be inline in mod.rs
        // (it lives in the binary blob instead)
        assert!(
            !versions_mod.contains("orb://v1.0.0/commands/old-cmd"),
            "URI must not be inline in versions/mod.rs — it belongs in the binary blob"
        );
    }

    #[test]
    fn test_versions_bin_contains_correct_entries() {
        // The binary blob must round-trip: every URI present in the prior
        // version snapshots must be retrievable by the lookup function.
        let mut prior_orb = OrbDefinition::default();
        prior_orb.commands.insert(
            "old-cmd".to_string(),
            Command {
                description: Some("An old command".to_string()),
                parameters: HashMap::new(),
                steps: vec![],
            },
        );

        let current_orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_prior_versions(vec![("1.0.0".to_string(), prior_orb)]);

        let server = generator
            .generate(&current_orb, "test-orb", "2.0.0")
            .unwrap();

        let blob = server
            .binary_files
            .get(&PathBuf::from("data/versions.bin"))
            .expect("data/versions.bin must exist");

        // The blob must contain at least one entry (the old-cmd command)
        assert!(blob.len() >= 4, "blob must have at least a count header");
        let count = u32::from_le_bytes(blob[0..4].try_into().unwrap());
        assert!(count >= 1, "blob must contain at least one entry");

        // Round-trip: look up the expected URI
        let found = lookup_versions_bin(blob, "orb://v1.0.0/commands/old-cmd");
        assert!(
            found.is_some(),
            "blob must contain entry for orb://v1.0.0/commands/old-cmd"
        );
    }

    /// Standalone reimplementation of the runtime lookup used in the generated
    /// `versions/mod.rs`, used here to verify the binary format round-trips.
    fn lookup_versions_bin(data: &[u8], uri: &str) -> Option<String> {
        if data.len() < 4 {
            return None;
        }
        let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let mut pos = 4usize;
        for _ in 0..count {
            if pos + 4 > data.len() {
                return None;
            }
            let key_len =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;
            if pos + key_len > data.len() {
                return None;
            }
            let key = std::str::from_utf8(&data[pos..pos + key_len]).ok()?;
            pos += key_len;
            if pos + 4 > data.len() {
                return None;
            }
            let val_len =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;
            if key == uri {
                return data
                    .get(pos..pos + val_len)
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .map(str::to_owned);
            }
            if pos + val_len > data.len() {
                return None;
            }
            pos += val_len;
        }
        None
    }

    #[test]
    fn test_list_resources_does_not_inline_prior_version_entries() {
        // list_resources must NOT contain a Self::resource() call for every
        // prior-version command/job/executor — for large orbs with many
        // historical versions this creates a function body that exhausts LLVM
        // during release compilation.  Prior-version URIs are discoverable via
        // the orb://versions resource; individual entries must NOT be listed.
        let mut prior_orb = OrbDefinition::default();
        for name in ["cmd-a", "cmd-b", "cmd-c"] {
            prior_orb.commands.insert(
                name.to_string(),
                Command {
                    description: None,
                    parameters: HashMap::new(),
                    steps: vec![],
                },
            );
        }
        let current_orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_prior_versions(vec![("1.0.0".to_string(), prior_orb)]);

        let server = generator
            .generate(&current_orb, "test-orb", "2.0.0")
            .unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        // Prior-version URIs must NOT appear in list_resources
        assert!(
            !lib_rs.contains("orb://v1.0.0/commands/cmd-a"),
            "list_resources must not inline prior-version URIs — causes LLVM OOM"
        );
    }

    #[test]
    fn test_current_version_json_not_inline_in_read_resource() {
        // The json_content for current-version resources must NOT be inline
        // inside the read_resource function body — doing so causes LLVM to
        // exhaust memory when compiling large orbs with release optimisations.
        // Resources must be served via current::get(uri) backed by
        // data/current.bin (embedded via include_bytes!).
        let orb = create_test_orb(); // has "greet" command
        let generator = CodeGenerator::new().unwrap();
        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(
            !lib_rs.contains("\"orb://commands/greet\" => r##"),
            "read_resource must not inline json_content as r## literal — use current::get"
        );
    }

    #[test]
    fn test_current_bin_generated_for_orb_with_resources() {
        let orb = create_test_orb();
        let generator = CodeGenerator::new().unwrap();
        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();

        assert!(
            server
                .binary_files
                .contains_key(&PathBuf::from("data/current.bin")),
            "data/current.bin must be generated for orbs with resources"
        );
    }

    #[test]
    fn test_current_bin_not_generated_for_empty_orb() {
        let orb = OrbDefinition::default();
        let generator = CodeGenerator::new().unwrap();
        let server = generator.generate(&orb, "empty-orb", "1.0.0").unwrap();

        assert!(
            !server
                .binary_files
                .contains_key(&PathBuf::from("data/current.bin")),
            "data/current.bin must not be generated for empty orbs"
        );
    }

    #[test]
    fn test_current_mod_uses_include_bytes() {
        let orb = create_test_orb();
        let generator = CodeGenerator::new().unwrap();
        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();

        let current_mod = server
            .files
            .get(&PathBuf::from("src/current/mod.rs"))
            .expect("src/current/mod.rs must be generated when resources exist");

        assert!(
            current_mod.contains("include_bytes!"),
            "current/mod.rs must use include_bytes! not inline string literals"
        );
        assert!(
            current_mod.contains("pub(crate) fn get"),
            "current/mod.rs must expose pub(crate) fn get"
        );
    }

    #[test]
    fn test_current_bin_round_trips() {
        // Every current-version resource URI must be retrievable from current.bin.
        let orb = create_test_orb(); // has "greet" command
        let generator = CodeGenerator::new().unwrap();
        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();

        let blob = server
            .binary_files
            .get(&PathBuf::from("data/current.bin"))
            .expect("data/current.bin must exist");

        let found = lookup_versions_bin(blob, "orb://commands/greet");
        assert!(
            found.is_some(),
            "current.bin must contain entry for orb://commands/greet"
        );
    }

    #[test]
    fn test_lib_declares_mod_current_when_resources_exist() {
        let orb = create_test_orb();
        let generator = CodeGenerator::new().unwrap();
        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(
            lib_rs.contains("mod current;"),
            "lib.rs must declare mod current; when resources exist"
        );
    }

    #[test]
    fn test_lib_delegates_current_resources_to_current_module() {
        let orb = create_test_orb();
        let generator = CodeGenerator::new().unwrap();
        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(
            lib_rs.contains("current::get(uri)"),
            "lib.rs must delegate current-version resource lookups to current::get(uri)"
        );
    }

    #[test]
    fn test_generate_with_prior_versions_lib_delegates_to_versions_module() {
        let mut prior_orb = OrbDefinition::default();
        prior_orb.commands.insert(
            "old-cmd".to_string(),
            Command {
                description: Some("An old command".to_string()),
                parameters: HashMap::new(),
                steps: vec![],
            },
        );

        let current_orb = create_test_orb();
        let generator = CodeGenerator::new()
            .unwrap()
            .with_prior_versions(vec![("1.0.0".to_string(), prior_orb)]);

        let server = generator
            .generate(&current_orb, "test-orb", "2.0.0")
            .unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        // lib.rs should delegate to versions::get, not inline the JSON
        assert!(
            lib_rs.contains("versions::get(uri)"),
            "lib.rs should delegate to versions::get(uri)"
        );
        // The JSON content for prior versions should NOT be inline in lib.rs
        assert!(
            !lib_rs.contains("\"An old command\""),
            "prior version JSON content should not be inline in lib.rs"
        );
    }
}
