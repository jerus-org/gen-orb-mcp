//! Code generator module for creating MCP servers from orb definitions.
//!
//! This module transforms a parsed `OrbDefinition` into a working MCP server
//! by rendering Handlebars templates to produce Rust source code.
//!
//! # Example
//!
//! ```no_run
//! use gen_orb_mcp::generator::CodeGenerator;
//! use gen_orb_mcp::parser::OrbParser;
//! use std::path::Path;
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

pub use context::GeneratorContext;
pub use error::GeneratorError;

use crate::parser::OrbDefinition;
use handlebars::Handlebars;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Generated MCP server output containing all source files.
#[derive(Debug, Clone)]
pub struct GeneratedServer {
    /// Map of relative file paths to their content.
    pub files: HashMap<PathBuf, String>,

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

        // Write all files
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
}

impl<'a> CodeGenerator<'a> {
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

        // Register custom helpers
        register_helpers(&mut handlebars);

        Ok(Self { handlebars })
    }

    /// Generate an MCP server from an orb definition.
    ///
    /// # Arguments
    ///
    /// * `orb` - The parsed orb definition
    /// * `orb_name` - The name to use for the orb (typically derived from filename)
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
        let context = GeneratorContext::from_orb(orb, orb_name, version);

        // Serialize context for templates
        let ctx_json = serde_json::to_value(&context)
            .map_err(|e| GeneratorError::Serialization { source: e })?;

        // Render templates
        let mut files = HashMap::new();

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

        Ok(GeneratedServer {
            files,
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
    use super::*;
    use crate::parser::{Command, OrbDefinition, Parameter, ParameterType};
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_test_orb() -> OrbDefinition {
        let mut orb = OrbDefinition::default();
        orb.version = "2.1".to_string();
        orb.description = Some("Test orb for unit tests".to_string());

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
        assert!(main_rs.contains("test_orb_mcp::create_server"));
    }

    #[test]
    fn test_generated_lib_contains_resources() {
        let generator = CodeGenerator::new().unwrap();
        let orb = create_test_orb();

        let server = generator.generate(&orb, "test-orb", "1.0.0").unwrap();
        let lib_rs = server.files.get(&PathBuf::from("src/lib.rs")).unwrap();

        assert!(lib_rs.contains("ResourceCollection"));
        assert!(lib_rs.contains("StaticResource"));
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
        assert!(cargo.contains("pmcp = "));
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
}
