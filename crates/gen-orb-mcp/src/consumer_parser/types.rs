//! Types for parsed consumer CircleCI configuration.

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

/// Parsed representation of a consumer's `.circleci/` directory.
/// Provides a job-graph model suitable for conformance checking.
#[derive(Debug, Clone, Default)]
pub struct ConsumerConfig {
    /// All CI files found, keyed by path relative to the `.circleci/`
    /// directory.
    pub files: HashMap<PathBuf, CiFile>,
}

impl ConsumerConfig {
    /// Returns all job invocations across all files and workflows.
    pub fn all_invocations(&self) -> impl Iterator<Item = &JobInvocation> {
        self.files
            .values()
            .flat_map(|f| f.workflows.values())
            .flat_map(|w| w.jobs.iter())
    }

    /// Returns job invocations filtered by orb alias prefix (e.g. `"toolkit"`).
    pub fn invocations_for_orb<'a>(
        &'a self,
        orb_alias: &'a str,
    ) -> impl Iterator<Item = &'a JobInvocation> {
        self.all_invocations()
            .filter(move |inv| inv.orb_alias.as_deref() == Some(orb_alias))
    }

    /// Returns all orb command step invocations across all files and custom
    /// jobs.
    pub fn all_step_invocations(&self) -> impl Iterator<Item = &StepInvocation> {
        self.files
            .values()
            .flat_map(|f| f.custom_jobs.values())
            .flat_map(|j| j.steps.iter())
    }

    /// Returns step invocations filtered by orb alias (e.g. `"toolkit"`).
    pub fn step_invocations_for_orb<'a>(
        &'a self,
        orb_alias: &'a str,
    ) -> impl Iterator<Item = &'a StepInvocation> {
        self.all_step_invocations()
            .filter(move |step| step.orb_alias.as_deref() == Some(orb_alias))
    }
}

/// Parsed representation of a single CI YAML file.
#[derive(Debug, Clone, Default)]
pub struct CiFile {
    /// Full filesystem path to this file.
    pub source_path: std::path::PathBuf,
    /// Orb aliases declared in this file: alias → orb reference.
    pub orb_aliases: HashMap<String, OrbRef>,
    /// Workflows defined in this file.
    pub workflows: HashMap<String, Workflow>,
    /// Consumer-defined jobs declared in this file (top-level `jobs:` key).
    /// Only jobs that contain at least one orb command step are included.
    pub custom_jobs: HashMap<String, CustomJob>,
    /// Pipeline parameter names declared in the top-level `parameters:` block.
    pub pipeline_parameters: Vec<String>,
}

/// Reference to a specific orb at a pinned version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrbRef {
    /// The orb publisher/namespace (e.g. `"jerus-org"`).
    pub org: String,
    /// The orb name (e.g. `"circleci-toolkit"`).
    pub name: String,
    /// The pinned version string (e.g. `"4.8.0"` or `"5.0.0"`).
    pub version: String,
}

impl OrbRef {
    /// Parses an orb reference string of the form `"org/name@version"`.
    ///
    /// Returns `None` if the string doesn't match this pattern.
    pub fn parse(s: &str) -> Option<Self> {
        // Expected form: "org/name@version"
        let (org_name, version) = s.split_once('@')?;
        let (org, name) = org_name.split_once('/')?;
        Some(OrbRef {
            org: org.to_string(),
            name: name.to_string(),
            version: version.to_string(),
        })
    }

    /// Returns the fully-qualified orb reference string `"org/name@version"`.
    pub fn to_ref_string(&self) -> String {
        format!("{}/{}@{}", self.org, self.name, self.version)
    }
}

/// A workflow definition within a CI file.
#[derive(Debug, Clone, Default)]
pub struct Workflow {
    /// The job invocations in this workflow, in declaration order.
    pub jobs: Vec<JobInvocation>,
}

/// A single job invocation within a workflow.
#[derive(Debug, Clone)]
pub struct JobInvocation {
    /// The full job reference as written, e.g. `"toolkit/update_prlog"`.
    pub reference: String,
    /// Resolved orb alias (e.g. `"toolkit"`), or `None` for local jobs.
    pub orb_alias: Option<String>,
    /// Job name within the orb (e.g. `"update_prlog"`), or `None` for local
    /// jobs.
    pub orb_job: Option<String>,
    /// Parameters passed at the invocation site.
    pub parameters: HashMap<String, serde_yaml::Value>,
    /// Job names (or `name:` overrides) that this invocation requires.
    pub requires: Vec<String>,
    /// The `name:` override if present; identifies this invocation in
    /// `requires` lists.
    pub name_override: Option<String>,
    /// Source location for targeted YAML editing.
    pub location: SourceLocation,
}

impl JobInvocation {
    /// Returns the effective name by which other jobs identify this invocation
    /// in their `requires:` lists.
    ///
    /// This is the `name:` override if one was given, otherwise the job
    /// reference.
    pub fn effective_name(&self) -> &str {
        self.name_override
            .as_deref()
            .unwrap_or(self.reference.as_str())
    }

    /// Returns `true` if this invocation calls a job from the named orb.
    pub fn is_from_orb(&self, alias: &str) -> bool {
        self.orb_alias.as_deref() == Some(alias)
    }

    /// Returns `true` if this invocation calls the named job from the named
    /// orb.
    pub fn matches(&self, orb_alias: &str, job_name: &str) -> bool {
        self.orb_alias.as_deref() == Some(orb_alias) && self.orb_job.as_deref() == Some(job_name)
    }
}

/// Source location pointing to a specific job invocation in a YAML file.
///
/// Used by the `applicator` to make targeted in-place edits that preserve
/// formatting, comments, and surrounding content.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// Path to the CI file containing this invocation.
    pub file: PathBuf,
    /// Name of the workflow containing this invocation.
    pub workflow: String,
    /// Zero-based index of this job in the workflow's `jobs:` list.
    pub job_index: usize,
}

/// A consumer-defined job body (under the top-level `jobs:` key).
///
/// Only orb command steps are captured; bare steps (`checkout`, `run:`, etc.)
/// are filtered out as they are not subject to orb conformance rules.
#[derive(Debug, Clone, Default)]
pub struct CustomJob {
    /// Orb command steps in this job, in declaration order.
    pub steps: Vec<StepInvocation>,
}

/// A single orb command step invocation inside a consumer's custom job.
#[derive(Debug, Clone)]
pub struct StepInvocation {
    /// The full command reference as written, e.g. `"toolkit/setup_env"`.
    pub reference: String,
    /// Resolved orb alias (e.g. `"toolkit"`).
    pub orb_alias: Option<String>,
    /// Command name within the orb (e.g. `"setup_env"`).
    pub orb_command: Option<String>,
    /// Parameters passed to this command invocation.
    pub parameters: HashMap<String, serde_yaml::Value>,
    /// Source location for targeted YAML editing.
    pub location: StepLocation,
}

impl StepInvocation {
    /// Returns `true` if this step invokes the named command from the named
    /// orb.
    pub fn matches(&self, orb_alias: &str, command_name: &str) -> bool {
        self.orb_alias.as_deref() == Some(orb_alias)
            && self.orb_command.as_deref() == Some(command_name)
    }
}

/// Source location pointing to a specific step invocation inside a custom job's
/// steps.
///
/// Distinct from `SourceLocation` (which targets workflow job invocations):
/// steps are located by job name + step index, not workflow + job index.
#[derive(Debug, Clone)]
pub struct StepLocation {
    /// Path to the CI file containing this step.
    pub file: PathBuf,
    /// Name of the consumer custom job containing this step.
    pub job: String,
    /// Zero-based index of this step in the job's `steps:` list.
    pub step_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orb_ref_parse_valid() {
        let r = OrbRef::parse("jerus-org/circleci-toolkit@4.8.0").unwrap();
        assert_eq!(r.org, "jerus-org");
        assert_eq!(r.name, "circleci-toolkit");
        assert_eq!(r.version, "4.8.0");
    }

    #[test]
    fn test_orb_ref_parse_invalid() {
        assert!(OrbRef::parse("no-at-sign").is_none());
        assert!(OrbRef::parse("no-slash@1.0.0").is_none());
        assert!(OrbRef::parse("").is_none());
    }

    #[test]
    fn test_orb_ref_round_trip() {
        let original = "digital-prstv/circleci-toolkit@5.0.0";
        let r = OrbRef::parse(original).unwrap();
        assert_eq!(r.to_ref_string(), original);
    }

    #[test]
    fn test_job_invocation_effective_name() {
        let inv = JobInvocation {
            reference: "toolkit/update_prlog".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_job: Some("update_prlog".to_string()),
            parameters: HashMap::new(),
            requires: vec![],
            name_override: Some("update-prlog-on-main".to_string()),
            location: SourceLocation {
                file: PathBuf::from("config.yml"),
                workflow: "update_prlog".to_string(),
                job_index: 0,
            },
        };
        assert_eq!(inv.effective_name(), "update-prlog-on-main");
    }

    #[test]
    fn test_job_invocation_effective_name_no_override() {
        let inv = JobInvocation {
            reference: "toolkit/label".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_job: Some("label".to_string()),
            parameters: HashMap::new(),
            requires: vec!["update-prlog-on-main".to_string()],
            name_override: None,
            location: SourceLocation {
                file: PathBuf::from("update_prlog.yml"),
                workflow: "update_prlog".to_string(),
                job_index: 1,
            },
        };
        assert_eq!(inv.effective_name(), "toolkit/label");
        assert!(inv.matches("toolkit", "label"));
    }

    #[test]
    fn test_consumer_config_invocations_for_orb() {
        let mut config = ConsumerConfig::default();
        let mut file = CiFile::default();
        let mut workflow = Workflow::default();

        workflow.jobs.push(JobInvocation {
            reference: "toolkit/update_prlog".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_job: Some("update_prlog".to_string()),
            parameters: HashMap::new(),
            requires: vec![],
            name_override: None,
            location: SourceLocation {
                file: PathBuf::from("config.yml"),
                workflow: "validation".to_string(),
                job_index: 0,
            },
        });
        workflow.jobs.push(JobInvocation {
            reference: "my-local-job".to_string(),
            orb_alias: None,
            orb_job: None,
            parameters: HashMap::new(),
            requires: vec![],
            name_override: None,
            location: SourceLocation {
                file: PathBuf::from("config.yml"),
                workflow: "validation".to_string(),
                job_index: 1,
            },
        });

        file.workflows.insert("validation".to_string(), workflow);
        config.files.insert(PathBuf::from("config.yml"), file);

        let toolkit_invocations: Vec<_> = config.invocations_for_orb("toolkit").collect();
        assert_eq!(toolkit_invocations.len(), 1);
        assert_eq!(
            toolkit_invocations[0].orb_job.as_deref(),
            Some("update_prlog")
        );
    }

    #[test]
    fn test_consumer_config_step_invocations_for_orb() {
        let mut config = ConsumerConfig::default();
        let mut file = CiFile::default();
        let mut custom_job = CustomJob::default();

        custom_job.steps.push(StepInvocation {
            reference: "toolkit/setup_env".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_command: Some("setup_env".to_string()),
            parameters: HashMap::new(),
            location: StepLocation {
                file: PathBuf::from("config.yml"),
                job: "my-release-job".to_string(),
                step_index: 0,
            },
        });
        custom_job.steps.push(StepInvocation {
            reference: "other/cmd".to_string(),
            orb_alias: Some("other".to_string()),
            orb_command: Some("cmd".to_string()),
            parameters: HashMap::new(),
            location: StepLocation {
                file: PathBuf::from("config.yml"),
                job: "my-release-job".to_string(),
                step_index: 1,
            },
        });

        file.custom_jobs
            .insert("my-release-job".to_string(), custom_job);
        config.files.insert(PathBuf::from("config.yml"), file);

        let toolkit_steps: Vec<_> = config.step_invocations_for_orb("toolkit").collect();
        assert_eq!(toolkit_steps.len(), 1);
        assert_eq!(toolkit_steps[0].orb_command.as_deref(), Some("setup_env"));
    }

    #[test]
    fn test_step_invocation_matches() {
        let step = StepInvocation {
            reference: "toolkit/publish_crate".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_command: Some("publish_crate".to_string()),
            parameters: HashMap::new(),
            location: StepLocation {
                file: PathBuf::from("config.yml"),
                job: "release-job".to_string(),
                step_index: 2,
            },
        };
        assert!(step.matches("toolkit", "publish_crate"));
        assert!(!step.matches("toolkit", "other_cmd"));
        assert!(!step.matches("other", "publish_crate"));
    }
}
