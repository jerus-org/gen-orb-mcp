//! Template context structs for code generation.
//!
//! These structures are serialized and passed to Handlebars templates
//! to generate the MCP server code.

use serde::Serialize;

use crate::parser::{
    Command, Executor, ExecutorConfig, Job, OrbDefinition, Parameter, ParameterType,
};

/// Root context passed to templates for generating the MCP server.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratorContext {
    /// Name of the orb (e.g., "my-toolkit")
    pub orb_name: String,

    /// Crate name in snake_case (e.g., "my_toolkit_mcp")
    pub crate_name: String,

    /// Struct name in PascalCase (e.g., "MyToolkitMcp")
    pub struct_name: String,

    /// Server version string
    pub version: String,

    /// Optional description of the orb
    pub description: Option<String>,

    /// Command contexts for template rendering
    pub commands: Vec<CommandContext>,

    /// Job contexts for template rendering
    pub jobs: Vec<JobContext>,

    /// Executor contexts for template rendering
    pub executors: Vec<ExecutorContext>,

    /// Whether there are any resources to expose
    pub has_resources: bool,
}

/// Context for a single command.
#[derive(Debug, Clone, Serialize)]
pub struct CommandContext {
    /// Command name as defined in the orb
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Parameters accepted by this command
    pub parameters: Vec<ParameterContext>,

    /// MCP resource URI for this command
    pub uri: String,

    /// JSON representation of the command for embedding
    pub json_content: String,
}

/// Context for a single job.
#[derive(Debug, Clone, Serialize)]
pub struct JobContext {
    /// Job name as defined in the orb
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Parameters accepted by this job
    pub parameters: Vec<ParameterContext>,

    /// Executor reference if specified
    pub executor: Option<String>,

    /// Execution environment configuration
    pub config: ExecutorConfigContext,

    /// MCP resource URI for this job
    pub uri: String,

    /// JSON representation of the job for embedding
    pub json_content: String,
}

/// Context for a single executor.
#[derive(Debug, Clone, Serialize)]
pub struct ExecutorContext {
    /// Executor name as defined in the orb
    pub name: String,

    /// Optional description
    pub description: Option<String>,

    /// Parameters accepted by this executor
    pub parameters: Vec<ParameterContext>,

    /// Execution environment configuration
    pub config: ExecutorConfigContext,

    /// MCP resource URI for this executor
    pub uri: String,

    /// JSON representation of the executor for embedding
    pub json_content: String,
}

/// Context for executor configuration.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ExecutorConfigContext {
    /// Docker images (as strings)
    pub docker_images: Vec<String>,

    /// Resource class
    pub resource_class: Option<String>,

    /// Working directory
    pub working_directory: Option<String>,

    /// Environment variables as key-value pairs
    pub environment: Vec<(String, String)>,

    /// Shell to use
    pub shell: Option<String>,
}

/// Context for a single parameter.
#[derive(Debug, Clone, Serialize)]
pub struct ParameterContext {
    /// Parameter name
    pub name: String,

    /// Parameter type as string
    pub param_type: String,

    /// Optional description
    pub description: Option<String>,

    /// Default value as JSON string
    pub default: Option<String>,

    /// Whether the parameter is required (no default)
    pub required: bool,

    /// Allowed enum values if applicable
    pub enum_values: Option<Vec<String>>,
}

impl GeneratorContext {
    /// Create a GeneratorContext from an OrbDefinition.
    ///
    /// # Arguments
    ///
    /// * `orb` - The parsed orb definition
    /// * `orb_name` - The name to use for the orb (typically derived from filename)
    /// * `version` - The semantic version for the generated MCP server crate
    pub fn from_orb(orb: &OrbDefinition, orb_name: &str, version: &str) -> Self {
        let crate_name = to_snake_case(orb_name).replace('-', "_") + "_mcp";
        let struct_name = to_pascal_case(orb_name) + "Mcp";

        let commands: Vec<CommandContext> = orb
            .commands
            .iter()
            .map(|(name, cmd)| CommandContext::from_command(name, cmd))
            .collect();

        let jobs: Vec<JobContext> = orb
            .jobs
            .iter()
            .map(|(name, job)| JobContext::from_job(name, job))
            .collect();

        let executors: Vec<ExecutorContext> = orb
            .executors
            .iter()
            .map(|(name, exec)| ExecutorContext::from_executor(name, exec))
            .collect();

        let has_resources = !commands.is_empty() || !jobs.is_empty() || !executors.is_empty();

        Self {
            orb_name: orb_name.to_string(),
            crate_name,
            struct_name,
            version: version.to_string(),
            description: orb.description.clone(),
            commands,
            jobs,
            executors,
            has_resources,
        }
    }
}

impl CommandContext {
    fn from_command(name: &str, cmd: &Command) -> Self {
        let parameters: Vec<ParameterContext> = cmd
            .parameters
            .iter()
            .map(|(pname, param)| ParameterContext::from_parameter(pname, param))
            .collect();

        // Create a serializable representation for JSON embedding
        let json_content = create_command_json(name, cmd);

        Self {
            name: name.to_string(),
            description: cmd.description.clone(),
            parameters,
            uri: format!("orb://commands/{}", name),
            json_content,
        }
    }
}

impl JobContext {
    fn from_job(name: &str, job: &Job) -> Self {
        let parameters: Vec<ParameterContext> = job
            .parameters
            .iter()
            .map(|(pname, param)| ParameterContext::from_parameter(pname, param))
            .collect();

        let executor = job.executor.as_ref().map(|e| match e {
            crate::parser::ExecutorRef::Name(n) => n.clone(),
            crate::parser::ExecutorRef::WithParams { name, .. } => name.clone(),
        });

        let json_content = create_job_json(name, job);

        Self {
            name: name.to_string(),
            description: job.description.clone(),
            parameters,
            executor,
            config: ExecutorConfigContext::from_config(&job.config),
            uri: format!("orb://jobs/{}", name),
            json_content,
        }
    }
}

impl ExecutorContext {
    fn from_executor(name: &str, exec: &Executor) -> Self {
        let parameters: Vec<ParameterContext> = exec
            .parameters
            .iter()
            .map(|(pname, param)| ParameterContext::from_parameter(pname, param))
            .collect();

        let json_content = create_executor_json(name, exec);

        Self {
            name: name.to_string(),
            description: exec.description.clone(),
            parameters,
            config: ExecutorConfigContext::from_config(&exec.config),
            uri: format!("orb://executors/{}", name),
            json_content,
        }
    }
}

impl ExecutorConfigContext {
    fn from_config(config: &ExecutorConfig) -> Self {
        let environment: Vec<(String, String)> = config
            .environment
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Self {
            docker_images: extract_docker_images(config),
            resource_class: config.resource_class.clone(),
            working_directory: config.working_directory.clone(),
            environment,
            shell: config.shell.clone(),
        }
    }
}

impl ParameterContext {
    fn from_parameter(name: &str, param: &Parameter) -> Self {
        let param_type = param_type_to_str(&param.param_type).to_string();

        let default = param
            .default
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));

        Self {
            name: name.to_string(),
            param_type,
            description: param.description.clone(),
            default: default.clone(),
            required: default.is_none(),
            enum_values: param.enum_values.clone(),
        }
    }
}

/// Convert ParameterType to string representation.
fn param_type_to_str(pt: &ParameterType) -> &'static str {
    match pt {
        ParameterType::String => "string",
        ParameterType::Boolean => "boolean",
        ParameterType::Integer => "integer",
        ParameterType::Enum => "enum",
        ParameterType::EnvVarName => "env_var_name",
        ParameterType::Steps => "steps",
        ParameterType::Executor => "executor",
    }
}

/// Extract docker image names from ExecutorConfig.
fn extract_docker_images(config: &ExecutorConfig) -> Vec<String> {
    config
        .docker
        .as_ref()
        .map(|images| {
            images
                .iter()
                .map(|img| match img {
                    crate::parser::DockerImage::Simple(s) => s.clone(),
                    crate::parser::DockerImage::Full(f) => f.image.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Convert a string to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;

    for (i, c) in s.chars().enumerate() {
        if c == '-' || c == '_' || c == ' ' {
            result.push('_');
            prev_is_upper = false;
        } else if c.is_uppercase() {
            if i > 0 && !prev_is_upper && !result.ends_with('_') {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else {
            result.push(c);
            prev_is_upper = false;
        }
    }

    result
}

/// Convert a string to PascalCase.
fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;

    for c in s.chars() {
        if c == '-' || c == '_' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_uppercase().next().unwrap());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// JSON representation of a parameter for embedding in resources.
#[derive(Serialize)]
struct ParameterJson<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    param_type: &'static str,
    description: Option<&'a str>,
    default: Option<&'a serde_yaml::Value>,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    enum_values: Option<&'a Vec<String>>,
}

/// Convert parameters map to JSON-serializable format.
fn params_to_json(params: &std::collections::HashMap<String, Parameter>) -> Vec<ParameterJson<'_>> {
    params
        .iter()
        .map(|(pname, param)| ParameterJson {
            name: pname,
            param_type: param_type_to_str(&param.param_type),
            description: param.description.as_deref(),
            default: param.default.as_ref(),
            required: param.default.is_none(),
            enum_values: param.enum_values.as_ref(),
        })
        .collect()
}

/// Create JSON representation of a command for embedding in resources.
fn create_command_json(name: &str, cmd: &Command) -> String {
    #[derive(Serialize)]
    struct CommandJson<'a> {
        name: &'a str,
        description: Option<&'a str>,
        parameters: Vec<ParameterJson<'a>>,
        steps_count: usize,
    }

    let json = CommandJson {
        name,
        description: cmd.description.as_deref(),
        parameters: params_to_json(&cmd.parameters),
        steps_count: cmd.steps.len(),
    };

    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".to_string())
}

/// Create JSON representation of a job for embedding in resources.
fn create_job_json(name: &str, job: &Job) -> String {
    #[derive(Serialize)]
    struct JobJson<'a> {
        name: &'a str,
        description: Option<&'a str>,
        executor: Option<String>,
        parameters: Vec<ParameterJson<'a>>,
        steps_count: usize,
        docker_images: Vec<String>,
        resource_class: Option<&'a str>,
    }

    let executor = job.executor.as_ref().map(|e| match e {
        crate::parser::ExecutorRef::Name(n) => n.clone(),
        crate::parser::ExecutorRef::WithParams { name, .. } => name.clone(),
    });

    let json = JobJson {
        name,
        description: job.description.as_deref(),
        executor,
        parameters: params_to_json(&job.parameters),
        steps_count: job.steps.len(),
        docker_images: extract_docker_images(&job.config),
        resource_class: job.config.resource_class.as_deref(),
    };

    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".to_string())
}

/// Create JSON representation of an executor for embedding in resources.
fn create_executor_json(name: &str, exec: &Executor) -> String {
    #[derive(Serialize)]
    struct ExecutorJson<'a> {
        name: &'a str,
        description: Option<&'a str>,
        parameters: Vec<ParameterJson<'a>>,
        docker_images: Vec<String>,
        resource_class: Option<&'a str>,
        working_directory: Option<&'a str>,
    }

    let json = ExecutorJson {
        name,
        description: exec.description.as_deref(),
        parameters: params_to_json(&exec.parameters),
        docker_images: extract_docker_images(&exec.config),
        resource_class: exec.config.resource_class.as_deref(),
        working_directory: exec.config.working_directory.as_deref(),
    };

    serde_json::to_string_pretty(&json).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Command, OrbDefinition, Parameter, ParameterType};
    use std::collections::HashMap;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("my-orb"), "my_orb");
        assert_eq!(to_snake_case("MyOrb"), "my_orb");
        assert_eq!(to_snake_case("myOrb"), "my_orb");
        assert_eq!(to_snake_case("my_orb"), "my_orb");
        assert_eq!(to_snake_case("my orb"), "my_orb");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-orb"), "MyOrb");
        assert_eq!(to_pascal_case("my_orb"), "MyOrb");
        assert_eq!(to_pascal_case("my orb"), "MyOrb");
        assert_eq!(to_pascal_case("myOrb"), "MyOrb");
    }

    #[test]
    fn test_generator_context_from_orb() {
        let mut orb = OrbDefinition {
            version: "2.1".to_string(),
            description: Some("Test orb".to_string()),
            ..Default::default()
        };

        let mut params = HashMap::new();
        params.insert(
            "name".to_string(),
            Parameter {
                param_type: ParameterType::String,
                description: Some("Name param".to_string()),
                default: Some(serde_yaml::Value::String("World".to_string())),
                enum_values: None,
            },
        );

        orb.commands.insert(
            "greet".to_string(),
            Command {
                description: Some("Greet command".to_string()),
                parameters: params,
                steps: vec![],
            },
        );

        let ctx = GeneratorContext::from_orb(&orb, "my-toolkit", "1.5.0");

        assert_eq!(ctx.orb_name, "my-toolkit");
        assert_eq!(ctx.crate_name, "my_toolkit_mcp");
        assert_eq!(ctx.struct_name, "MyToolkitMcp");
        assert_eq!(ctx.version, "1.5.0");
        assert_eq!(ctx.description, Some("Test orb".to_string()));
        assert_eq!(ctx.commands.len(), 1);
        assert!(ctx.has_resources);

        let cmd = &ctx.commands[0];
        assert_eq!(cmd.name, "greet");
        assert_eq!(cmd.uri, "orb://commands/greet");
    }

    #[test]
    fn test_parameter_context() {
        let param = Parameter {
            param_type: ParameterType::Boolean,
            description: Some("Enable feature".to_string()),
            default: None,
            enum_values: None,
        };

        let ctx = ParameterContext::from_parameter("enabled", &param);

        assert_eq!(ctx.name, "enabled");
        assert_eq!(ctx.param_type, "boolean");
        assert!(ctx.required);
        assert!(ctx.default.is_none());
    }

    #[test]
    fn test_explicit_version() {
        let orb = OrbDefinition::default();
        let ctx = GeneratorContext::from_orb(&orb, "empty-orb", "2.0.0");

        assert_eq!(ctx.version, "2.0.0");
        assert!(!ctx.has_resources);
    }
}
