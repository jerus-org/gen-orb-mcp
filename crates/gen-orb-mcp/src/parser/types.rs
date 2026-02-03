//! Core data structures for parsed CircleCI orb definitions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root structure representing a complete orb definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrbDefinition {
    /// Orb schema version (e.g., "2.1")
    #[serde(default)]
    pub version: String,

    /// Human-readable description of the orb
    #[serde(default)]
    pub description: Option<String>,

    /// Display metadata for the orb registry
    #[serde(default)]
    pub display: Option<DisplayInfo>,

    /// Imported orbs (name -> orb reference)
    #[serde(default)]
    pub orbs: HashMap<String, String>,

    /// Command definitions
    #[serde(default)]
    pub commands: HashMap<String, Command>,

    /// Job definitions
    #[serde(default)]
    pub jobs: HashMap<String, Job>,

    /// Executor definitions
    #[serde(default)]
    pub executors: HashMap<String, Executor>,
}

/// Display metadata for orb registry listings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DisplayInfo {
    /// URL to orb's home page
    #[serde(default)]
    pub home_url: Option<String>,

    /// URL to source code repository
    #[serde(default)]
    pub source_url: Option<String>,
}

/// A reusable command definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Command {
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Parameters accepted by this command
    #[serde(default)]
    pub parameters: HashMap<String, Parameter>,

    /// Steps to execute
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Common execution environment configuration shared by jobs and executors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutorConfig {
    /// Docker images for execution
    #[serde(default)]
    pub docker: Option<Vec<DockerImage>>,

    /// Machine image configuration
    #[serde(default)]
    pub machine: Option<MachineConfig>,

    /// macOS configuration
    #[serde(default)]
    pub macos: Option<MacOsConfig>,

    /// Resource class for compute sizing
    #[serde(default)]
    pub resource_class: Option<String>,

    /// Working directory
    #[serde(default)]
    pub working_directory: Option<String>,

    /// Environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Shell to use
    #[serde(default)]
    pub shell: Option<String>,
}

/// A job definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Job {
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Executor to run this job on
    #[serde(default)]
    pub executor: Option<ExecutorRef>,

    /// Execution environment configuration
    #[serde(flatten)]
    pub config: ExecutorConfig,

    /// Parameters accepted by this job
    #[serde(default)]
    pub parameters: HashMap<String, Parameter>,

    /// Steps to execute
    #[serde(default)]
    pub steps: Vec<Step>,

    /// Parallelism level
    #[serde(default)]
    pub parallelism: Option<u32>,

    /// Circleci IP ranges
    #[serde(default)]
    pub circleci_ip_ranges: Option<bool>,
}

/// An executor definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Executor {
    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Execution environment configuration
    #[serde(flatten)]
    pub config: ExecutorConfig,

    /// Parameters accepted by this executor
    #[serde(default)]
    pub parameters: HashMap<String, Parameter>,
}

/// Reference to an executor with optional parameter overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExecutorRef {
    /// Simple executor name
    Name(String),
    /// Executor with parameter overrides
    WithParams {
        /// Executor name
        name: String,
        /// Parameter values to pass
        #[serde(flatten)]
        parameters: HashMap<String, serde_yaml::Value>,
    },
}

impl Default for ExecutorRef {
    fn default() -> Self {
        Self::Name(String::new())
    }
}

/// Parameter definition for commands, jobs, or executors.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Parameter {
    /// Parameter type
    #[serde(rename = "type")]
    pub param_type: ParameterType,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Default value (type matches param_type)
    #[serde(default)]
    pub default: Option<serde_yaml::Value>,

    /// Allowed values for enum type
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}

/// Supported parameter types in CircleCI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// String value
    #[default]
    String,
    /// Boolean value
    Boolean,
    /// Integer value
    Integer,
    /// One of a set of allowed values
    Enum,
    /// Environment variable name
    #[serde(rename = "env_var_name")]
    EnvVarName,
    /// Steps to inject (for macro-like parameters)
    Steps,
    /// Executor reference
    Executor,
}

/// A step in a command or job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Step {
    /// Simple string step (e.g., "checkout")
    Simple(String),
    /// Structured step
    Structured(StructuredStep),
}

impl Default for Step {
    fn default() -> Self {
        Self::Simple(String::new())
    }
}

/// Structured step definitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredStep {
    /// Run a shell command
    Run(RunStep),
    /// Checkout code
    Checkout(CheckoutStep),
    /// Restore cached files
    #[serde(rename = "restore_cache")]
    RestoreCache(CacheStep),
    /// Save files to cache
    #[serde(rename = "save_cache")]
    SaveCache(SaveCacheStep),
    /// Conditional step
    When(ConditionalStep),
    /// Negative conditional step
    Unless(ConditionalStep),
    /// Persist files to workspace
    #[serde(rename = "persist_to_workspace")]
    PersistToWorkspace(WorkspaceStep),
    /// Attach workspace files
    #[serde(rename = "attach_workspace")]
    AttachWorkspace(AttachWorkspaceStep),
    /// Store test results
    #[serde(rename = "store_test_results")]
    StoreTestResults(StoreTestResultsStep),
    /// Store artifacts
    #[serde(rename = "store_artifacts")]
    StoreArtifacts(StoreArtifactsStep),
    /// Add SSH keys
    #[serde(rename = "add_ssh_keys")]
    AddSshKeys(AddSshKeysStep),
    /// Set up remote Docker
    #[serde(rename = "setup_remote_docker")]
    SetupRemoteDocker(SetupRemoteDockerStep),
    /// Invoke another command or orb command
    #[serde(untagged)]
    CommandInvocation(HashMap<String, serde_yaml::Value>),
}

/// Run step configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunStep {
    /// Simple command string
    Simple(String),
    /// Full run configuration
    Full {
        /// Command to execute
        command: String,
        /// Step name
        #[serde(default)]
        name: Option<String>,
        /// Working directory
        #[serde(default)]
        working_directory: Option<String>,
        /// Environment variables
        #[serde(default)]
        environment: HashMap<String, String>,
        /// Shell to use
        #[serde(default)]
        shell: Option<String>,
        /// Background execution
        #[serde(default)]
        background: Option<bool>,
        /// Timeout in seconds
        #[serde(default)]
        no_output_timeout: Option<String>,
        /// Condition for execution
        #[serde(default)]
        when: Option<String>,
    },
}

/// Checkout step configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckoutStep {
    /// Path to checkout to
    #[serde(default)]
    pub path: Option<String>,
}

/// Cache restore step configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStep {
    /// Cache key or keys
    #[serde(default)]
    pub key: Option<String>,
    /// Fallback keys
    #[serde(default)]
    pub keys: Option<Vec<String>>,
    /// Name for the step
    #[serde(default)]
    pub name: Option<String>,
}

/// Cache save step configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaveCacheStep {
    /// Cache key
    pub key: String,
    /// Paths to cache
    #[serde(default)]
    pub paths: Vec<String>,
    /// Step name
    #[serde(default)]
    pub name: Option<String>,
    /// Condition for execution
    #[serde(default)]
    pub when: Option<String>,
}

/// Conditional step (when/unless).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConditionalStep {
    /// Condition to evaluate
    pub condition: serde_yaml::Value,
    /// Steps to run if condition is met
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Workspace persistence step.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceStep {
    /// Root directory
    pub root: String,
    /// Paths to persist
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Workspace attachment step.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttachWorkspaceStep {
    /// Path to attach at
    pub at: String,
}

/// Store test results step.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoreTestResultsStep {
    /// Path to test results
    pub path: String,
}

/// Store artifacts step.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoreArtifactsStep {
    /// Path to artifacts
    pub path: String,
    /// Destination path
    #[serde(default)]
    pub destination: Option<String>,
}

/// Add SSH keys step.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AddSshKeysStep {
    /// Fingerprints of keys to add
    #[serde(default)]
    pub fingerprints: Vec<String>,
}

/// Setup remote Docker step.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SetupRemoteDockerStep {
    /// Docker version
    #[serde(default)]
    pub version: Option<String>,
    /// Enable Docker layer caching
    #[serde(default)]
    pub docker_layer_caching: Option<bool>,
}

/// Docker image configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DockerImage {
    /// Simple image name
    Simple(String),
    /// Full image configuration with auth, environment, etc.
    Full(Box<DockerImageFull>),
}

/// Full Docker image configuration with all options.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DockerImageFull {
    /// Docker image reference
    pub image: String,
    /// Authentication credentials
    #[serde(default)]
    pub auth: Option<DockerAuth>,
    /// AWS ECR authentication
    #[serde(default)]
    pub aws_auth: Option<AwsAuth>,
    /// Container name
    #[serde(default)]
    pub name: Option<String>,
    /// Entrypoint override
    #[serde(default)]
    pub entrypoint: Option<Vec<String>>,
    /// Command override
    #[serde(default)]
    pub command: Option<Vec<String>>,
    /// User to run as
    #[serde(default)]
    pub user: Option<String>,
    /// Environment variables
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

/// Docker registry authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DockerAuth {
    /// Username (often environment variable reference)
    pub username: String,
    /// Password (often environment variable reference)
    pub password: String,
}

/// AWS ECR authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwsAuth {
    /// AWS access key ID
    #[serde(default)]
    pub aws_access_key_id: Option<String>,
    /// AWS secret access key
    #[serde(default)]
    pub aws_secret_access_key: Option<String>,
    /// OIDC role ARN
    #[serde(default)]
    pub oidc_role_arn: Option<String>,
}

/// Machine executor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MachineConfig {
    /// Boolean (use default machine)
    Enabled(bool),
    /// Machine image specification
    Image {
        /// Machine image to use
        image: String,
        /// Enable Docker layer caching
        #[serde(default)]
        docker_layer_caching: Option<bool>,
    },
}

/// macOS executor configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MacOsConfig {
    /// Xcode version
    pub xcode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_type_deserialize() {
        let yaml = r#"string"#;
        let pt: ParameterType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(pt, ParameterType::String);

        let yaml = r#"boolean"#;
        let pt: ParameterType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(pt, ParameterType::Boolean);

        let yaml = r#"env_var_name"#;
        let pt: ParameterType = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(pt, ParameterType::EnvVarName);
    }

    #[test]
    fn test_simple_command_deserialize() {
        let yaml = r#"
description: "Run tests"
parameters:
  coverage:
    type: boolean
    default: false
    description: "Enable coverage"
steps:
  - checkout
  - run: cargo test
"#;
        let cmd: Command = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cmd.description, Some("Run tests".to_string()));
        assert!(cmd.parameters.contains_key("coverage"));
        assert_eq!(cmd.steps.len(), 2);
    }

    #[test]
    fn test_docker_image_simple() {
        let yaml = r#""rust:1.75""#;
        let img: DockerImage = serde_yaml::from_str(yaml).unwrap();
        matches!(img, DockerImage::Simple(s) if s == "rust:1.75");
    }

    #[test]
    fn test_docker_image_full() {
        let yaml = r#"
image: rust:1.75
auth:
  username: $DOCKER_USER
  password: $DOCKER_PASS
"#;
        let img: DockerImage = serde_yaml::from_str(yaml).unwrap();
        match img {
            DockerImage::Full(full) => {
                assert_eq!(full.image, "rust:1.75");
                assert!(full.auth.is_some());
            }
            _ => panic!("Expected Full variant"),
        }
    }

    #[test]
    fn test_executor_ref_simple() {
        let yaml = r#""default""#;
        let exec: ExecutorRef = serde_yaml::from_str(yaml).unwrap();
        matches!(exec, ExecutorRef::Name(s) if s == "default");
    }

    #[test]
    fn test_step_simple() {
        let yaml = r#""checkout""#;
        let step: Step = serde_yaml::from_str(yaml).unwrap();
        matches!(step, Step::Simple(s) if s == "checkout");
    }

    #[test]
    fn test_run_step_simple() {
        let yaml = r#"
run: echo hello
"#;
        let step: StructuredStep = serde_yaml::from_str(yaml).unwrap();
        match step {
            StructuredStep::Run(RunStep::Simple(cmd)) => {
                assert_eq!(cmd, "echo hello");
            }
            _ => panic!("Expected Run with Simple variant"),
        }
    }

    #[test]
    fn test_run_step_full() {
        let yaml = r#"
run:
  name: Run tests
  command: cargo test
  working_directory: ~/project
"#;
        let step: StructuredStep = serde_yaml::from_str(yaml).unwrap();
        match step {
            StructuredStep::Run(RunStep::Full { command, name, .. }) => {
                assert_eq!(command, "cargo test");
                assert_eq!(name, Some("Run tests".to_string()));
            }
            _ => panic!("Expected Run with Full variant"),
        }
    }

    #[test]
    fn test_orb_definition_empty() {
        let yaml = r#"
version: "2.1"
"#;
        let orb: OrbDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(orb.version, "2.1");
        assert!(orb.commands.is_empty());
        assert!(orb.jobs.is_empty());
        assert!(orb.executors.is_empty());
    }
}
