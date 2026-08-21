/// Integration test: generated MCP server source compiles with cargo build.
///
/// This test exercises the full generation pipeline — template rendering plus
/// dependency resolution — to catch rmcp API or feature-flag mismatches that
/// content-only string assertions cannot detect.
use std::{collections::HashMap, path::Path, process::Command};

use gen_orb_mcp::{
    generator::CodeGenerator,
    parser::{Command as OrbCommand, Job, OrbDefinition, Parameter, ParameterType},
};
use tempfile::TempDir;

fn fixture_orb() -> OrbDefinition {
    let mut orb = OrbDefinition {
        version: "2.1".to_string(),
        description: Some(
            "Fixture orb for compilation tests — exercises commands, jobs, and tools path"
                .to_string(),
        ),
        ..Default::default()
    };

    // Command with a string parameter (exercises has_tools path in template)
    let mut cmd_params = HashMap::new();
    cmd_params.insert(
        "message".to_string(),
        Parameter {
            param_type: ParameterType::String,
            description: Some("Message to print".to_string()),
            default: Some(serde_yaml::Value::String("hello".to_string())),
            enum_values: None,
        },
    );
    orb.commands.insert(
        "print".to_string(),
        OrbCommand {
            description: Some("Print a message".to_string()),
            parameters: cmd_params,
            steps: vec![],
        },
    );

    // Job (exercises resource generation)
    let mut job_params = HashMap::new();
    job_params.insert(
        "tag".to_string(),
        Parameter {
            param_type: ParameterType::String,
            description: Some("Docker image tag".to_string()),
            default: Some(serde_yaml::Value::String("latest".to_string())),
            enum_values: None,
        },
    );
    orb.jobs.insert(
        "run-print".to_string(),
        Job {
            description: Some("Run the print command".to_string()),
            parameters: job_params,
            steps: vec![],
            executor: None,
            config: Default::default(),
            parallelism: None,
            circleci_ip_ranges: None,
        },
    );

    orb
}

/// Run `cargo build` in `dir` and assert it succeeds with zero compiler
/// warnings, printing captured stderr on failure for diagnosability.
fn assert_compiles_without_warnings(dir: &Path) {
    let output = Command::new("cargo")
        .args(["check", "--color", "never"])
        .current_dir(dir)
        .output()
        .expect("failed to run cargo check");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "generated MCP server did not compile — check template rmcp version and import paths:\n{stderr}"
    );

    assert!(
        !stderr.contains("warning:"),
        "generated MCP server compiled with warnings:\n{stderr}"
    );
}

/// Redirect the generated fixture's crates.io dependency on gen-orb-mcp to
/// this local checkout, so the has_tools=true branch compiles against
/// current source instead of fetching a published release. This only proves
/// the generated code and gen-orb-mcp's library API are compatible — it
/// can't prove the exact-pinned version in Cargo.toml is actually published
/// on crates.io, since that can't be known before this crate itself is
/// released.
fn patch_gen_orb_mcp_dependency(crate_dir: &Path) {
    // Cargo accepts forward slashes in paths on every platform including
    // Windows; normalising avoids emitting invalid TOML when
    // CARGO_MANIFEST_DIR contains backslashes.
    let local_path = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let patch = format!("\n[patch.crates-io]\ngen-orb-mcp = {{ path = \"{local_path}\" }}\n");
    let cargo_toml = crate_dir.join("Cargo.toml");
    let mut contents = std::fs::read_to_string(&cargo_toml).expect("read Cargo.toml");
    contents.push_str(&patch);
    std::fs::write(&cargo_toml, contents).expect("write Cargo.toml");
}

#[test]
fn generated_server_compiles() {
    let generator = CodeGenerator::new().expect("CodeGenerator::new");
    let orb = fixture_orb();
    let server = generator
        .generate(&orb, "fixture-orb", "1.0.0")
        .expect("generate");

    let tmp = TempDir::new().expect("TempDir::new");
    server.write_to(tmp.path()).expect("write_to");

    assert_compiles_without_warnings(tmp.path());
}

#[test]
fn generated_server_with_tools_compiles() {
    let generator = CodeGenerator::new()
        .expect("CodeGenerator::new")
        .with_conformance_rules_json("[]".to_string());
    let orb = fixture_orb();
    let server = generator
        .generate(&orb, "fixture-orb", "1.0.0")
        .expect("generate");

    let tmp = TempDir::new().expect("TempDir::new");
    server.write_to(tmp.path()).expect("write_to");
    patch_gen_orb_mcp_dependency(tmp.path());

    assert_compiles_without_warnings(tmp.path());
}
