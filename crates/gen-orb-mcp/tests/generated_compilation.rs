/// Integration test: generated MCP server source compiles with cargo build.
///
/// This test exercises the full generation pipeline — template rendering plus
/// dependency resolution — to catch rmcp API or feature-flag mismatches that
/// content-only string assertions cannot detect.
use std::{collections::HashMap, process::Command};

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

#[test]
fn generated_server_compiles() {
    let generator = CodeGenerator::new().expect("CodeGenerator::new");
    let orb = fixture_orb();
    let server = generator
        .generate(&orb, "fixture-orb", "1.0.0")
        .expect("generate");

    let tmp = TempDir::new().expect("TempDir::new");
    server.write_to(tmp.path()).expect("write_to");

    let status = Command::new("cargo")
        .args(["build", "--color", "never"])
        .current_dir(tmp.path())
        .status()
        .expect("failed to run cargo build");

    assert!(
        status.success(),
        "generated MCP server did not compile — check template rmcp version and import paths"
    );
}
