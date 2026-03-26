//! Consumer configuration parser.
//!
//! Parses a consumer's `.circleci/` directory into a [`ConsumerConfig`]
//! job-graph model that can be inspected by conformance rules.
//!
//! ## What is parsed
//!
//! - `orbs:` sections → [`OrbRef`] map keyed by alias
//! - `workflows:` sections → [`Workflow`] with [`JobInvocation`] list
//! - Per-invocation: job reference, orb alias, parameters, `requires:`, `name:`
//!   override
//!
//! - `jobs:` top-level definitions → [`CustomJob`] with [`StepInvocation`] list
//!   (only steps that invoke orb commands are captured)
//!
//! ## What is not parsed
//!
//! - `commands:`, `executors:` sections
//! - Pipeline parameters and `when:` conditions

pub mod error;
pub mod graph;
pub mod types;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub use error::ConsumerParserError;
pub use graph::{find_absorbed_candidates, requires_chain, transitively_requires};
pub use types::{
    CiFile, ConsumerConfig, CustomJob, JobInvocation, OrbRef, SourceLocation, StepInvocation,
    StepLocation, Workflow,
};

/// Parses one or more CircleCI YAML files from a directory into a
/// [`ConsumerConfig`].
pub struct ConsumerParser;

impl ConsumerParser {
    /// Parses all `.yml` and `.yaml` files in `ci_dir` and returns the combined
    /// config.
    ///
    /// Files that fail to parse are skipped with a warning rather than failing
    /// the entire parse — a consumer's `.circleci/` may contain non-config
    /// YAML files (e.g. `renovate.json5` is sometimes placed there). A file
    /// is silently skipped if it doesn't look like a CircleCI config (no
    /// `version:` key).
    pub fn parse_directory(ci_dir: &Path) -> Result<ConsumerConfig, ConsumerParserError> {
        if !ci_dir.is_dir() {
            return Err(ConsumerParserError::DirectoryNotFound {
                path: ci_dir.display().to_string(),
            });
        }

        let mut config = ConsumerConfig::default();
        let mut found_any = false;

        let entries = std::fs::read_dir(ci_dir).map_err(|e| ConsumerParserError::IoError {
            path: ci_dir.display().to_string(),
            source: e,
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !is_yaml_file(&path) {
                continue;
            }

            match Self::parse_file(&path) {
                Ok(Some(mut ci_file)) => {
                    ci_file.source_path = path.clone();
                    // Use filename (not full path) as the key
                    let key = path
                        .file_name()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| path.clone());
                    config.files.insert(key, ci_file);
                    found_any = true;
                }
                Ok(None) => {
                    // File doesn't look like a CircleCI config — skip silently
                    tracing::debug!(path = %path.display(), "Skipping non-CircleCI YAML file");
                }
                Err(e) => {
                    // Log but don't fail the entire parse
                    tracing::warn!(path = %path.display(), error = %e, "Failed to parse CI file, skipping");
                }
            }
        }

        if !found_any {
            return Err(ConsumerParserError::NoFilesFound {
                path: ci_dir.display().to_string(),
            });
        }

        Ok(config)
    }

    /// Parses a single CircleCI YAML file.
    ///
    /// Returns `Ok(None)` if the file doesn't contain a CircleCI `version:`
    /// key, indicating it is not a CI config file.
    pub fn parse_file(path: &Path) -> Result<Option<CiFile>, ConsumerParserError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConsumerParserError::IoError {
            path: path.display().to_string(),
            source: e,
        })?;

        Self::parse_str(&content, path)
    }

    /// Parses CircleCI config YAML from a string.
    ///
    /// The `source_path` is used only for `SourceLocation` tracking and error
    /// messages.
    pub fn parse_str(
        content: &str,
        source_path: &Path,
    ) -> Result<Option<CiFile>, ConsumerParserError> {
        let raw: serde_yaml::Value =
            serde_yaml::from_str(content).map_err(|e| ConsumerParserError::YamlError {
                path: source_path.display().to_string(),
                source: e,
            })?;

        let map = match &raw {
            serde_yaml::Value::Mapping(m) => m,
            _ => return Ok(None),
        };

        // Must have a `version:` key to be a CircleCI config
        if !map.contains_key("version") {
            return Ok(None);
        }

        Ok(Some(parse_ci_file_from_map(map, source_path)))
    }
}

/// Populates a `CiFile` from the top-level YAML mapping sections.
fn parse_ci_file_from_map(map: &serde_yaml::Mapping, source_path: &Path) -> CiFile {
    let mut ci_file = CiFile::default();

    // Parse pipeline parameters
    if let Some(params_value) = map.get("parameters") {
        ci_file.pipeline_parameters = parse_pipeline_parameter_names(params_value);
    }

    // Parse orb aliases
    if let Some(orbs_value) = map.get("orbs") {
        ci_file.orb_aliases = parse_orb_aliases(orbs_value);
    }

    // Parse workflows
    if let Some(workflows_value) = map.get("workflows") {
        ci_file.workflows = parse_workflows(workflows_value, source_path, &ci_file.orb_aliases);
    }

    // Parse consumer-defined jobs (steps may invoke orb commands)
    if let Some(jobs_value) = map.get("jobs") {
        ci_file.custom_jobs = parse_custom_jobs(jobs_value, source_path, &ci_file.orb_aliases);
    }

    ci_file
}

/// Extracts the key names from the top-level `parameters:` block.
fn parse_pipeline_parameter_names(params_value: &serde_yaml::Value) -> Vec<String> {
    let mut result = Vec::new();
    let Some(map) = params_value.as_mapping() else {
        return result;
    };
    for key in map.keys() {
        if let Some(name) = key.as_str() {
            result.push(name.to_string());
        }
    }
    result
}

fn is_yaml_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("yml") | Some("yaml")
    )
}

/// Parses the `orbs:` section into a map of alias → `OrbRef`.
fn parse_orb_aliases(orbs_value: &serde_yaml::Value) -> HashMap<String, OrbRef> {
    let mut result = HashMap::new();

    let Some(map) = orbs_value.as_mapping() else {
        return result;
    };

    for (key, val) in map {
        let Some(alias) = key.as_str() else { continue };
        let Some(ref_str) = val.as_str() else {
            continue;
        };

        if let Some(orb_ref) = OrbRef::parse(ref_str) {
            result.insert(alias.to_string(), orb_ref);
        } else {
            tracing::debug!(alias, ref_str, "Could not parse orb reference, skipping");
        }
    }

    result
}

/// Parses the `workflows:` section into a map of workflow name → `Workflow`.
fn parse_workflows(
    workflows_value: &serde_yaml::Value,
    source_path: &Path,
    orb_aliases: &HashMap<String, OrbRef>,
) -> HashMap<String, Workflow> {
    let mut result = HashMap::new();

    let Some(map) = workflows_value.as_mapping() else {
        return result;
    };

    for (key, val) in map {
        let Some(workflow_name) = key.as_str() else {
            continue;
        };

        // Skip the `version:` key that sometimes appears inside `workflows:`
        if workflow_name == "version" {
            continue;
        }

        let workflow = parse_workflow(val, workflow_name, source_path, orb_aliases);
        result.insert(workflow_name.to_string(), workflow);
    }

    result
}

/// Parses a single workflow definition.
fn parse_workflow(
    val: &serde_yaml::Value,
    workflow_name: &str,
    source_path: &Path,
    orb_aliases: &HashMap<String, OrbRef>,
) -> Workflow {
    let mut workflow = Workflow::default();

    let Some(map) = val.as_mapping() else {
        return workflow;
    };

    let Some(jobs_value) = map.get("jobs") else {
        return workflow;
    };

    let Some(jobs_seq) = jobs_value.as_sequence() else {
        return workflow;
    };

    for (job_index, job_entry) in jobs_seq.iter().enumerate() {
        if let Some(inv) = parse_job_invocation(
            job_entry,
            workflow_name,
            job_index,
            source_path,
            orb_aliases,
        ) {
            workflow.jobs.push(inv);
        }
    }

    workflow
}

/// Parses a single job entry in a workflow's `jobs:` list.
///
/// A job entry can be either a bare string (`- toolkit/update_prlog`) or
/// a map with the job reference as the sole key and parameters as the value.
fn parse_job_invocation(
    entry: &serde_yaml::Value,
    workflow_name: &str,
    job_index: usize,
    source_path: &Path,
    orb_aliases: &HashMap<String, OrbRef>,
) -> Option<JobInvocation> {
    let location = SourceLocation {
        file: source_path.to_path_buf(),
        workflow: workflow_name.to_string(),
        job_index,
    };

    match entry {
        serde_yaml::Value::String(reference) => {
            // Bare string: `- toolkit/update_prlog`
            let (orb_alias, orb_job) = split_job_reference(reference, orb_aliases);
            Some(JobInvocation {
                reference: reference.clone(),
                orb_alias,
                orb_job,
                parameters: HashMap::new(),
                requires: vec![],
                name_override: None,
                location,
            })
        }
        serde_yaml::Value::Mapping(map) => {
            // Map: `- toolkit/update_prlog: { name: ..., requires: [...], ... }`
            let (reference, params_value) = map
                .iter()
                .next()
                .map(|(k, v)| (k.as_str().unwrap_or("").to_string(), v))?;

            let (orb_alias, orb_job) = split_job_reference(&reference, orb_aliases);

            let (requires, name_override, parameters) = extract_job_params(params_value);

            Some(JobInvocation {
                reference,
                orb_alias,
                orb_job,
                parameters,
                requires,
                name_override,
                location,
            })
        }
        _ => None,
    }
}

/// Splits a job reference like `"toolkit/update_prlog"` into `(Some("toolkit"),
/// Some("update_prlog"))`. Returns `(None, None)` for local job names without a
/// `/`.
fn split_job_reference(
    reference: &str,
    orb_aliases: &HashMap<String, OrbRef>,
) -> (Option<String>, Option<String>) {
    if let Some((alias, job)) = reference.split_once('/') {
        if orb_aliases.contains_key(alias) {
            return (Some(alias.to_string()), Some(job.to_string()));
        }
    }
    (None, None)
}

/// Extracts `requires:`, `name:`, and remaining parameters from a job's
/// parameter map.
fn extract_job_params(
    params_value: &serde_yaml::Value,
) -> (
    Vec<String>,
    Option<String>,
    HashMap<String, serde_yaml::Value>,
) {
    let mut requires = vec![];
    let mut name_override = None;
    let mut parameters = HashMap::new();

    let Some(map) = params_value.as_mapping() else {
        return (requires, name_override, parameters);
    };

    for (key, val) in map {
        let Some(key_str) = key.as_str() else {
            continue;
        };

        match key_str {
            "requires" => {
                if let Some(seq) = val.as_sequence() {
                    requires = seq
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
            "name" => {
                name_override = val.as_str().map(|s| s.to_string());
            }
            _ => {
                parameters.insert(key_str.to_string(), val.clone());
            }
        }
    }

    (requires, name_override, parameters)
}

/// Parses the top-level `jobs:` section into a map of job name → [`CustomJob`].
///
/// Only jobs that contain at least one orb command step produce an entry;
/// jobs with no orb steps are omitted as they have no conformance relevance.
fn parse_custom_jobs(
    jobs_value: &serde_yaml::Value,
    source_path: &Path,
    orb_aliases: &HashMap<String, OrbRef>,
) -> HashMap<String, CustomJob> {
    let mut result = HashMap::new();

    let Some(map) = jobs_value.as_mapping() else {
        return result;
    };

    for (key, val) in map {
        let Some(job_name) = key.as_str() else {
            continue;
        };
        let custom_job = parse_custom_job_steps(val, job_name, source_path, orb_aliases);
        if !custom_job.steps.is_empty() {
            result.insert(job_name.to_string(), custom_job);
        }
    }

    result
}

/// Parses the `steps:` sequence of a single consumer job definition.
fn parse_custom_job_steps(
    job_value: &serde_yaml::Value,
    job_name: &str,
    source_path: &Path,
    orb_aliases: &HashMap<String, OrbRef>,
) -> CustomJob {
    let mut custom_job = CustomJob::default();

    let Some(job_map) = job_value.as_mapping() else {
        return custom_job;
    };
    let Some(steps_value) = job_map.get("steps") else {
        return custom_job;
    };
    let Some(steps_seq) = steps_value.as_sequence() else {
        return custom_job;
    };

    for (step_index, step_entry) in steps_seq.iter().enumerate() {
        if let Some(inv) =
            parse_step_invocation(step_entry, job_name, step_index, source_path, orb_aliases)
        {
            custom_job.steps.push(inv);
        }
    }

    custom_job
}

/// Parses a single step entry from a job's `steps:` list.
///
/// Returns `None` for non-orb steps (`checkout`, `run:`, etc.) — only steps
/// whose orb alias appears in `orb_aliases` are returned.
///
/// Handles two forms:
/// - Bare string: `- toolkit/setup_env`
/// - Map form: `- toolkit/setup_env:\n    param: value`
fn parse_step_invocation(
    entry: &serde_yaml::Value,
    job_name: &str,
    step_index: usize,
    source_path: &Path,
    orb_aliases: &HashMap<String, OrbRef>,
) -> Option<StepInvocation> {
    let location = StepLocation {
        file: source_path.to_path_buf(),
        job: job_name.to_string(),
        step_index,
    };

    match entry {
        serde_yaml::Value::String(reference) => {
            // Bare string form: `- toolkit/setup_env`
            let (orb_alias, orb_command) = split_orb_command(reference, orb_aliases)?;
            Some(StepInvocation {
                reference: reference.clone(),
                orb_alias: Some(orb_alias),
                orb_command: Some(orb_command),
                parameters: HashMap::new(),
                location,
            })
        }
        serde_yaml::Value::Mapping(map) => {
            // Map form: `- toolkit/setup_env:\n    param: value`
            // The single key is the command reference; value is null or param map.
            let (reference, params_value) = map
                .iter()
                .next()
                .map(|(k, v)| (k.as_str().unwrap_or("").to_string(), v))?;

            let (orb_alias, orb_command) = split_orb_command(&reference, orb_aliases)?;

            let parameters = extract_command_params(params_value);

            Some(StepInvocation {
                reference,
                orb_alias: Some(orb_alias),
                orb_command: Some(orb_command),
                parameters,
                location,
            })
        }
        _ => None,
    }
}

/// Splits a command reference like `"toolkit/setup_env"` into `("toolkit",
/// "setup_env")`.
///
/// Returns `None` if the alias part is not a known orb alias or the reference
/// has no `/`.
fn split_orb_command(
    reference: &str,
    orb_aliases: &HashMap<String, OrbRef>,
) -> Option<(String, String)> {
    let (alias, command) = reference.split_once('/')?;
    if orb_aliases.contains_key(alias) {
        Some((alias.to_string(), command.to_string()))
    } else {
        None
    }
}

/// Extracts parameters from a command invocation's value (the map under the
/// command key).
///
/// Unlike job invocations, command steps have no `requires:` or `name:` fields;
/// all keys are parameters.
fn extract_command_params(params_value: &serde_yaml::Value) -> HashMap<String, serde_yaml::Value> {
    let mut parameters = HashMap::new();

    let Some(map) = params_value.as_mapping() else {
        return parameters;
    };

    for (key, val) in map {
        if let Some(key_str) = key.as_str() {
            parameters.insert(key_str.to_string(), val.clone());
        }
    }

    parameters
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const SAMPLE_CONFIG: &str = r#"
version: 2.1

orbs:
  toolkit: jerus-org/circleci-toolkit@4.8.0

workflows:
  validation:
    jobs:
      - toolkit/required_builds_rolling:
          name: required-builds
          context: [bot-check]
      - toolkit/update_prlog:
          name: update-prlog-on-main
          context: [release, bot-check]
          min_rust_version: "1.85"
      - toolkit/label:
          context: [pcu-app]
          requires:
            - update-prlog-on-main
"#;

    const SAMPLE_UPDATE_PRLOG: &str = r#"
version: 2.1

parameters:
  update_pcu:
    type: boolean
    default: false

orbs:
  toolkit: jerus-org/circleci-toolkit@4.8.0

workflows:
  update_prlog:
    jobs:
      - toolkit/update_prlog:
          name: update-prlog-on-main
          context: [release, bot-check, pcu-app]
          min_rust_version: "1.85"
          target_branch: "main"
      - toolkit/label:
          context: [pcu-app]
          requires:
            - update-prlog-on-main
"#;

    #[test]
    fn test_parse_str_basic() {
        let path = Path::new("config.yml");
        let result = ConsumerParser::parse_str(SAMPLE_CONFIG, path).unwrap();
        let ci_file = result.expect("Should parse as a CI file");

        assert!(ci_file.orb_aliases.contains_key("toolkit"));
        let toolkit_ref = &ci_file.orb_aliases["toolkit"];
        assert_eq!(toolkit_ref.org, "jerus-org");
        assert_eq!(toolkit_ref.version, "4.8.0");

        assert!(ci_file.workflows.contains_key("validation"));
        let workflow = &ci_file.workflows["validation"];
        assert_eq!(workflow.jobs.len(), 3);
    }

    #[test]
    fn test_parse_str_job_references() {
        let path = Path::new("config.yml");
        let result = ConsumerParser::parse_str(SAMPLE_CONFIG, path).unwrap();
        let ci_file = result.unwrap();
        let workflow = &ci_file.workflows["validation"];

        let first_job = &workflow.jobs[0];
        assert_eq!(first_job.reference, "toolkit/required_builds_rolling");
        assert_eq!(first_job.orb_alias.as_deref(), Some("toolkit"));
        assert_eq!(
            first_job.orb_job.as_deref(),
            Some("required_builds_rolling")
        );
        assert_eq!(first_job.name_override.as_deref(), Some("required-builds"));
    }

    #[test]
    fn test_parse_str_requires() {
        let path = Path::new("update_prlog.yml");
        let result = ConsumerParser::parse_str(SAMPLE_UPDATE_PRLOG, path).unwrap();
        let ci_file = result.unwrap();
        let workflow = &ci_file.workflows["update_prlog"];

        let label_job = &workflow.jobs[1];
        assert_eq!(label_job.orb_job.as_deref(), Some("label"));
        assert_eq!(label_job.requires, vec!["update-prlog-on-main"]);
    }

    #[test]
    fn test_parse_str_non_ci_file_returns_none() {
        let yaml = "key: value\nother: stuff";
        let path = Path::new("renovate.yaml");
        let result = ConsumerParser::parse_str(yaml, path).unwrap();
        assert!(result.is_none(), "Non-CI YAML should return None");
    }

    #[test]
    fn test_parse_str_parameters_extracted() {
        let path = Path::new("update_prlog.yml");
        let result = ConsumerParser::parse_str(SAMPLE_UPDATE_PRLOG, path).unwrap();
        let ci_file = result.unwrap();
        let workflow = &ci_file.workflows["update_prlog"];
        let update_prlog_job = &workflow.jobs[0];

        // min_rust_version should be in parameters
        assert!(
            update_prlog_job.parameters.contains_key("min_rust_version"),
            "min_rust_version should be parsed as a parameter"
        );
        // name and requires should NOT be in parameters
        assert!(!update_prlog_job.parameters.contains_key("name"));
        assert!(!update_prlog_job.parameters.contains_key("requires"));
    }

    #[test]
    fn test_parse_directory_error_on_missing_dir() {
        let result = ConsumerParser::parse_directory(Path::new("/nonexistent/path/.circleci"));
        assert!(result.is_err());
        matches!(
            result.unwrap_err(),
            ConsumerParserError::DirectoryNotFound { .. }
        );
    }

    #[test]
    fn test_parse_directory_round_trip() {
        use std::fs;

        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("config.yml"), SAMPLE_CONFIG).unwrap();
        fs::write(tmp.path().join("update_prlog.yml"), SAMPLE_UPDATE_PRLOG).unwrap();

        let config = ConsumerParser::parse_directory(tmp.path()).unwrap();
        assert_eq!(config.files.len(), 2);

        let toolkit_invocations: Vec<_> = config.invocations_for_orb("toolkit").collect();
        // 3 from config.yml + 2 from update_prlog.yml = 5
        assert_eq!(toolkit_invocations.len(), 5);
    }

    const SAMPLE_WITH_CUSTOM_JOBS: &str = r#"
version: 2.1

orbs:
  toolkit: jerus-org/circleci-toolkit@4.7.1

jobs:
  my-release-job:
    executor: toolkit/rust_env_rolling
    steps:
      - checkout
      - toolkit/setup_env:
          token: $GITHUB_TOKEN
      - run: cargo build
      - toolkit/publish_crate:
          package: my-crate

workflows:
  release:
    jobs:
      - my-release-job
"#;

    const SAMPLE_WITH_BARE_STEP: &str = r#"
version: 2.1

orbs:
  toolkit: jerus-org/circleci-toolkit@4.7.1

jobs:
  simple-job:
    executor: toolkit/rust_env_rolling
    steps:
      - checkout
      - toolkit/setup_env
      - run: cargo test

workflows:
  ci:
    jobs:
      - simple-job
"#;

    #[test]
    fn test_parse_custom_jobs_basic() {
        let path = Path::new("config.yml");
        let ci_file = ConsumerParser::parse_str(SAMPLE_WITH_CUSTOM_JOBS, path)
            .unwrap()
            .unwrap();

        assert!(
            ci_file.custom_jobs.contains_key("my-release-job"),
            "Should parse custom job"
        );
        let job = &ci_file.custom_jobs["my-release-job"];
        // checkout and run: should be filtered out; only toolkit steps remain
        assert_eq!(job.steps.len(), 2, "Only orb command steps should be kept");
    }

    #[test]
    fn test_parse_step_map_form() {
        let path = Path::new("config.yml");
        let ci_file = ConsumerParser::parse_str(SAMPLE_WITH_CUSTOM_JOBS, path)
            .unwrap()
            .unwrap();

        let job = &ci_file.custom_jobs["my-release-job"];
        let setup = &job.steps[0];
        assert_eq!(setup.reference, "toolkit/setup_env");
        assert_eq!(setup.orb_alias.as_deref(), Some("toolkit"));
        assert_eq!(setup.orb_command.as_deref(), Some("setup_env"));
        assert!(setup.parameters.contains_key("token"));

        let publish = &job.steps[1];
        assert_eq!(publish.reference, "toolkit/publish_crate");
        assert!(publish.parameters.contains_key("package"));
    }

    #[test]
    fn test_parse_step_bare_form() {
        let path = Path::new("config.yml");
        let ci_file = ConsumerParser::parse_str(SAMPLE_WITH_BARE_STEP, path)
            .unwrap()
            .unwrap();

        let job = &ci_file.custom_jobs["simple-job"];
        assert_eq!(job.steps.len(), 1);
        let step = &job.steps[0];
        assert_eq!(step.reference, "toolkit/setup_env");
        assert_eq!(step.orb_alias.as_deref(), Some("toolkit"));
        assert_eq!(step.orb_command.as_deref(), Some("setup_env"));
        assert!(step.parameters.is_empty());
    }

    #[test]
    fn test_parse_step_non_orb_skipped() {
        let path = Path::new("config.yml");
        let ci_file = ConsumerParser::parse_str(SAMPLE_WITH_CUSTOM_JOBS, path)
            .unwrap()
            .unwrap();

        let job = &ci_file.custom_jobs["my-release-job"];
        // Neither "checkout" nor "run: cargo build" should appear in steps
        assert!(
            job.steps.iter().all(|s| s.reference.contains('/')),
            "All parsed steps should be orb command references"
        );
    }

    #[test]
    fn test_parse_step_location_fields() {
        let path = Path::new("config.yml");
        let ci_file = ConsumerParser::parse_str(SAMPLE_WITH_CUSTOM_JOBS, path)
            .unwrap()
            .unwrap();

        let job = &ci_file.custom_jobs["my-release-job"];
        // toolkit/setup_env is the 2nd step overall (index 1 in the original YAML,
        // but step_index tracks position in the full steps array including non-orb
        // steps)
        let setup_step = job
            .steps
            .iter()
            .find(|s| s.reference == "toolkit/setup_env")
            .unwrap();
        assert_eq!(setup_step.location.job, "my-release-job");
        assert_eq!(setup_step.location.step_index, 1); // index in full steps
                                                       // list
    }

    #[test]
    fn test_parse_full_config_jobs_and_workflows() {
        let path = Path::new("config.yml");
        let ci_file = ConsumerParser::parse_str(SAMPLE_WITH_CUSTOM_JOBS, path)
            .unwrap()
            .unwrap();

        // Workflows are also parsed
        assert!(ci_file.workflows.contains_key("release"));
        assert_eq!(ci_file.workflows["release"].jobs.len(), 1);

        // Custom jobs are parsed
        assert_eq!(ci_file.custom_jobs.len(), 1);
    }

    #[test]
    fn test_label_absorbed_detection() {
        let path = Path::new("update_prlog.yml");
        let result = ConsumerParser::parse_str(SAMPLE_UPDATE_PRLOG, path).unwrap();
        let ci_file = result.unwrap();
        let workflow = &ci_file.workflows["update_prlog"];

        // label (index 1) requires update-prlog-on-main (which is update_prlog at index
        // 0)
        let candidates =
            find_absorbed_candidates(workflow, "toolkit", "label", "update-prlog-on-main");
        assert_eq!(
            candidates,
            vec![1],
            "label should be detected as absorbed candidate"
        );
    }

    #[test]
    fn test_parse_pipeline_parameters() {
        // SAMPLE_UPDATE_PRLOG already has a `parameters:` block with `update_pcu`
        let path = Path::new("update_prlog.yml");
        let ci_file = ConsumerParser::parse_str(SAMPLE_UPDATE_PRLOG, path)
            .unwrap()
            .unwrap();
        assert!(
            ci_file
                .pipeline_parameters
                .contains(&"update_pcu".to_string()),
            "update_pcu should be in pipeline_parameters"
        );
    }
}
