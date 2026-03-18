//! Semantic diff between two `OrbDefinition` instances.
//!
//! The `OrbDiffer` computes a `Vec<ConformanceRule>` by comparing an old and a
//! new `OrbDefinition`. It runs a set of detection passes:
//!
//! **Job-level passes**
//! 1. Collect removed/added jobs
//! 2. Run `JobAbsorbed` heuristic (removed job + new boolean param)
//! 3. Run `JobRenamed` heuristic (parameter-set fuzzy match)
//! 4. Emit `JobRemoved` for unaccounted removals
//! 5. Emit `ParameterRemoved` for parameters lost from surviving jobs
//! 6. Emit `ParameterAdded` for mandatory params added to surviving jobs
//! 7. Emit `ParameterEnumValueRemoved` for enum values removed from surviving parameters
//!
//! **Command-level passes**
//! 8. Run `CommandRenamed` heuristic (parameter-set fuzzy match)
//! 9. Emit `CommandRemoved` for unaccounted removals
//! 10. Emit `CommandParameterRemoved` for parameters lost from surviving commands
//! 11. Emit `CommandParameterAdded` for mandatory params added to surviving commands

pub mod heuristics;

use std::collections::{HashMap, HashSet};

use crate::conformance_rule::ConformanceRule;
use crate::parser::types::{Command, Job, OrbDefinition, Parameter, ParameterType};

use heuristics::{detect_absorbed_jobs, detect_renamed_commands, detect_renamed_jobs};

/// The default Jaccard similarity threshold for rename detection.
const RENAME_THRESHOLD: f64 = 0.7;

/// Type alias for the four job maps returned by `build_job_sets`.
type JobSets<'a> = (
    HashSet<String>,
    HashMap<String, &'a Job>,
    HashMap<String, &'a Job>,
    HashMap<String, &'a Job>,
);

/// Type alias for the three command maps returned by `build_command_sets`.
type CommandSets<'a> = (
    HashSet<String>,
    HashMap<String, &'a Command>,
    HashMap<String, &'a Command>,
);

/// Computes the `Vec<ConformanceRule>` representing breaking changes between
/// `old` and `new` versions of an orb.
///
/// # Arguments
/// * `old` — the previous orb version (e.g. `4.11.0`)
/// * `new_orb` — the current orb version (e.g. `5.0.0`)
/// * `since_version` — the version string to embed in emitted rules (typically the new version)
pub fn diff(
    old: &OrbDefinition,
    new_orb: &OrbDefinition,
    since_version: &str,
) -> Vec<ConformanceRule> {
    OrbDiffer::new(old, new_orb, since_version).diff()
}

/// Computes conformance rules between two orb versions.
pub struct OrbDiffer<'a> {
    old: &'a OrbDefinition,
    new: &'a OrbDefinition,
    since_version: String,
}

impl<'a> OrbDiffer<'a> {
    /// Creates a new differ.
    pub fn new(old: &'a OrbDefinition, new_orb: &'a OrbDefinition, since_version: &str) -> Self {
        Self {
            old,
            new: new_orb,
            since_version: since_version.to_string(),
        }
    }

    /// Runs all detection passes and returns the combined conformance rules.
    pub fn diff(&self) -> Vec<ConformanceRule> {
        let mut rules = Vec::new();
        self.diff_jobs(&mut rules);
        self.diff_commands(&mut rules);
        rules
    }

    // ── Job-level passes ──────────────────────────────────────────────────────

    fn diff_jobs(&self, rules: &mut Vec<ConformanceRule>) {
        let (removed_names, removed_jobs, new_jobs, old_jobs) = self.build_job_sets();

        let absorbed = detect_absorbed_jobs(&removed_jobs, &new_jobs, &old_jobs);
        self.emit_absorbed(&absorbed, rules);

        let unaccounted = subtract_keys(&removed_names, absorbed.keys());
        let renamed = detect_renamed_jobs(&unaccounted, &new_jobs, &old_jobs, RENAME_THRESHOLD);
        self.emit_renamed_jobs(&renamed, rules);

        let still_unaccounted = subtract_keys(&unaccounted, renamed.keys());
        self.emit_jobs_removed(&still_unaccounted, rules);

        self.emit_parameter_removed(rules);
        self.emit_parameter_added(rules);
        self.emit_enum_value_removed(rules);
    }

    fn build_job_sets(&self) -> JobSets<'_> {
        let removed_names: HashSet<String> = self
            .old
            .jobs
            .keys()
            .filter(|name| !self.new.jobs.contains_key(*name))
            .cloned()
            .collect();
        let removed_jobs: HashMap<String, &Job> = removed_names
            .iter()
            .filter_map(|name| self.old.jobs.get(name).map(|j| (name.clone(), j)))
            .collect();
        let new_jobs: HashMap<String, &Job> =
            self.new.jobs.iter().map(|(k, v)| (k.clone(), v)).collect();
        let old_jobs: HashMap<String, &Job> =
            self.old.jobs.iter().map(|(k, v)| (k.clone(), v)).collect();
        (removed_names, removed_jobs, new_jobs, old_jobs)
    }

    fn emit_absorbed(&self, absorbed: &HashMap<String, String>, rules: &mut Vec<ConformanceRule>) {
        for (absorbed_name, into_name) in absorbed {
            rules.push(ConformanceRule::JobAbsorbed {
                absorbed: absorbed_name.clone(),
                into: into_name.clone(),
                since_version: self.since_version.clone(),
            });
        }
    }

    fn emit_renamed_jobs(
        &self,
        renamed: &HashMap<String, String>,
        rules: &mut Vec<ConformanceRule>,
    ) {
        for (from, to) in renamed {
            let old_params: HashSet<&str> = self
                .old
                .jobs
                .get(from.as_str())
                .map(|j| j.parameters.keys().map(|s| s.as_str()).collect())
                .unwrap_or_default();
            let new_params: HashSet<&str> = self
                .new
                .jobs
                .get(to.as_str())
                .map(|j| j.parameters.keys().map(|s| s.as_str()).collect())
                .unwrap_or_default();
            let removed_parameters = old_params
                .difference(&new_params)
                .map(|s| s.to_string())
                .collect();
            rules.push(ConformanceRule::JobRenamed {
                from: from.clone(),
                to: to.clone(),
                removed_parameters,
                since_version: self.since_version.clone(),
            });
        }
    }

    fn emit_jobs_removed(&self, names: &HashSet<String>, rules: &mut Vec<ConformanceRule>) {
        for name in names {
            rules.push(ConformanceRule::JobRemoved {
                name: name.clone(),
                since_version: self.since_version.clone(),
            });
        }
    }

    fn emit_parameter_removed(&self, rules: &mut Vec<ConformanceRule>) {
        for (job_name, old_job) in &self.old.jobs {
            let Some(new_job) = self.new.jobs.get(job_name.as_str()) else {
                continue; // job removed — handled above
            };
            for param_name in old_job.parameters.keys() {
                if !new_job.parameters.contains_key(param_name.as_str()) {
                    rules.push(ConformanceRule::ParameterRemoved {
                        job: job_name.clone(),
                        parameter: param_name.clone(),
                        since_version: self.since_version.clone(),
                    });
                }
            }
        }
    }

    fn emit_parameter_added(&self, rules: &mut Vec<ConformanceRule>) {
        for (job_name, new_job) in &self.new.jobs {
            let Some(old_job) = self.old.jobs.get(job_name.as_str()) else {
                continue; // new job entirely — not a breaking change for existing consumers
            };
            for (param_name, param) in &new_job.parameters {
                if !old_job.parameters.contains_key(param_name.as_str()) && is_mandatory(param) {
                    rules.push(ConformanceRule::ParameterAdded {
                        job: job_name.clone(),
                        parameter: param_name.clone(),
                        since_version: self.since_version.clone(),
                    });
                }
            }
        }
    }

    fn emit_enum_value_removed(&self, rules: &mut Vec<ConformanceRule>) {
        for (job_name, old_job) in &self.old.jobs {
            let Some(new_job) = self.new.jobs.get(job_name.as_str()) else {
                continue;
            };
            for (param_name, old_param) in &old_job.parameters {
                if old_param.param_type != ParameterType::Enum {
                    continue;
                }
                let Some(new_param) = new_job.parameters.get(param_name.as_str()) else {
                    continue; // parameter removed — handled above
                };
                self.emit_removed_enum_values(job_name, param_name, old_param, new_param, rules);
            }
        }
    }

    fn emit_removed_enum_values(
        &self,
        job_name: &str,
        param_name: &str,
        old_param: &Parameter,
        new_param: &Parameter,
        rules: &mut Vec<ConformanceRule>,
    ) {
        let old_values: HashSet<&str> = old_param
            .enum_values
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|s| s.as_str())
            .collect();
        let new_values: HashSet<&str> = new_param
            .enum_values
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|s| s.as_str())
            .collect();
        let fallback = new_param
            .enum_values
            .as_deref()
            .and_then(|v| v.first())
            .map(|s| s.as_str())
            .unwrap_or("");
        for removed_value in old_values.difference(&new_values) {
            rules.push(ConformanceRule::ParameterEnumValueRemoved {
                job: job_name.to_string(),
                parameter: param_name.to_string(),
                removed_value: removed_value.to_string(),
                fallback_value: fallback.to_string(),
                since_version: self.since_version.clone(),
            });
        }
    }

    // ── Command-level passes ──────────────────────────────────────────────────

    fn diff_commands(&self, rules: &mut Vec<ConformanceRule>) {
        let (removed_names, new_cmds, old_cmds) = self.build_command_sets();

        let renamed =
            detect_renamed_commands(&removed_names, &new_cmds, &old_cmds, RENAME_THRESHOLD);
        self.emit_renamed_commands(&renamed, rules);

        let still_unaccounted = subtract_keys(&removed_names, renamed.keys());
        self.emit_commands_removed(&still_unaccounted, rules);

        self.emit_command_parameter_removed(rules);
        self.emit_command_parameter_added(rules);
    }

    fn build_command_sets(&self) -> CommandSets<'_> {
        let removed_names: HashSet<String> = self
            .old
            .commands
            .keys()
            .filter(|name| !self.new.commands.contains_key(*name))
            .cloned()
            .collect();
        let new_cmds: HashMap<String, &Command> = self
            .new
            .commands
            .iter()
            .map(|(k, v)| (k.clone(), v))
            .collect();
        let old_cmds: HashMap<String, &Command> = self
            .old
            .commands
            .iter()
            .map(|(k, v)| (k.clone(), v))
            .collect();
        (removed_names, new_cmds, old_cmds)
    }

    fn emit_commands_removed(&self, names: &HashSet<String>, rules: &mut Vec<ConformanceRule>) {
        for name in names {
            rules.push(ConformanceRule::CommandRemoved {
                name: name.clone(),
                since_version: self.since_version.clone(),
            });
        }
    }

    fn emit_renamed_commands(
        &self,
        renamed: &HashMap<String, String>,
        rules: &mut Vec<ConformanceRule>,
    ) {
        for (from, to) in renamed {
            let old_params: HashSet<&str> = self
                .old
                .commands
                .get(from.as_str())
                .map(|c| c.parameters.keys().map(|s| s.as_str()).collect())
                .unwrap_or_default();
            let new_params: HashSet<&str> = self
                .new
                .commands
                .get(to.as_str())
                .map(|c| c.parameters.keys().map(|s| s.as_str()).collect())
                .unwrap_or_default();
            let removed_parameters = old_params
                .difference(&new_params)
                .map(|s| s.to_string())
                .collect();
            rules.push(ConformanceRule::CommandRenamed {
                from: from.clone(),
                to: to.clone(),
                removed_parameters,
                since_version: self.since_version.clone(),
            });
        }
    }

    fn emit_command_parameter_removed(&self, rules: &mut Vec<ConformanceRule>) {
        for (cmd_name, old_cmd) in &self.old.commands {
            let Some(new_cmd) = self.new.commands.get(cmd_name.as_str()) else {
                continue; // command removed — handled above
            };
            for param_name in old_cmd.parameters.keys() {
                if !new_cmd.parameters.contains_key(param_name.as_str()) {
                    rules.push(ConformanceRule::CommandParameterRemoved {
                        command: cmd_name.clone(),
                        parameter: param_name.clone(),
                        since_version: self.since_version.clone(),
                    });
                }
            }
        }
    }

    fn emit_command_parameter_added(&self, rules: &mut Vec<ConformanceRule>) {
        for (cmd_name, new_cmd) in &self.new.commands {
            let Some(old_cmd) = self.old.commands.get(cmd_name.as_str()) else {
                continue; // new command entirely — not a breaking change
            };
            for (param_name, param) in &new_cmd.parameters {
                if !old_cmd.parameters.contains_key(param_name.as_str()) && is_mandatory(param) {
                    rules.push(ConformanceRule::CommandParameterAdded {
                        command: cmd_name.clone(),
                        parameter: param_name.clone(),
                        since_version: self.since_version.clone(),
                    });
                }
            }
        }
    }
}

// ── Free helpers ─────────────────────────────────────────────────────────────

/// Returns `true` if a parameter has no default value (must be supplied by caller).
fn is_mandatory(param: &Parameter) -> bool {
    param.default.is_none()
}

/// Returns the elements of `set` that are not keys in `to_remove`.
fn subtract_keys<'k, I: Iterator<Item = &'k String>>(
    set: &HashSet<String>,
    to_remove: I,
) -> HashSet<String> {
    let remove: HashSet<&String> = to_remove.collect();
    set.iter()
        .filter(|k| !remove.contains(k))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{Job, OrbDefinition, Parameter, ParameterType};
    use std::collections::HashMap;

    fn make_orb_with_jobs(jobs: HashMap<String, Job>) -> OrbDefinition {
        OrbDefinition {
            jobs,
            ..Default::default()
        }
    }

    fn make_orb(jobs: HashMap<String, Job>, commands: HashMap<String, Command>) -> OrbDefinition {
        OrbDefinition {
            jobs,
            commands,
            ..Default::default()
        }
    }

    fn str_param() -> Parameter {
        Parameter {
            param_type: ParameterType::String,
            ..Default::default()
        }
    }

    fn str_param_with_default() -> Parameter {
        Parameter {
            param_type: ParameterType::String,
            default: Some(serde_yaml::Value::String("default".to_string())),
            ..Default::default()
        }
    }

    fn bool_param() -> Parameter {
        Parameter {
            param_type: ParameterType::Boolean,
            ..Default::default()
        }
    }

    fn enum_param(values: &[&str]) -> Parameter {
        Parameter {
            param_type: ParameterType::Enum,
            enum_values: Some(values.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        }
    }

    fn job(params: &[(&str, Parameter)]) -> Job {
        Job {
            parameters: params
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            ..Default::default()
        }
    }

    fn command(params: &[(&str, Parameter)]) -> Command {
        Command {
            parameters: params
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_job_removed() {
        let old = make_orb_with_jobs(
            [("choose_pipeline".to_string(), job(&[]))]
                .into_iter()
                .collect(),
        );
        let new = make_orb_with_jobs(HashMap::new());

        let rules = diff(&old, &new, "5.0.0");
        assert!(rules.iter().any(|r| matches!(r,
            ConformanceRule::JobRemoved { name, since_version }
            if name == "choose_pipeline" && since_version == "5.0.0"
        )));
    }

    #[test]
    fn test_parameter_removed() {
        let old = make_orb_with_jobs(
            [(
                "update_prlog".to_string(),
                job(&[("min_rust_version", str_param())]),
            )]
            .into_iter()
            .collect(),
        );
        let new = make_orb_with_jobs(
            [("update_prlog".to_string(), job(&[]))]
                .into_iter()
                .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(rules.iter().any(|r| matches!(r,
            ConformanceRule::ParameterRemoved { job, parameter, .. }
            if job == "update_prlog" && parameter == "min_rust_version"
        )));
    }

    #[test]
    fn test_parameter_added_mandatory() {
        // Surviving job gains a new parameter with no default → ParameterAdded
        let old = make_orb_with_jobs(
            [("deploy".to_string(), job(&[("env", str_param())]))]
                .into_iter()
                .collect(),
        );
        let new = make_orb_with_jobs(
            [(
                "deploy".to_string(),
                job(&[("env", str_param()), ("region", str_param())]),
            )]
            .into_iter()
            .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::ParameterAdded { job, parameter, .. }
                if job == "deploy" && parameter == "region"
            )),
            "Expected ParameterAdded, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_parameter_added_with_default_not_emitted() {
        // New parameter with a default is not a breaking change
        let old = make_orb_with_jobs(
            [("deploy".to_string(), job(&[("env", str_param())]))]
                .into_iter()
                .collect(),
        );
        let new = make_orb_with_jobs(
            [(
                "deploy".to_string(),
                job(&[("env", str_param()), ("region", str_param_with_default())]),
            )]
            .into_iter()
            .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            !rules
                .iter()
                .any(|r| matches!(r, ConformanceRule::ParameterAdded { .. })),
            "Expected no ParameterAdded for defaulted param, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_job_absorbed() {
        let old = make_orb_with_jobs(
            [
                ("label".to_string(), job(&[("context", str_param())])),
                (
                    "update_prlog".to_string(),
                    job(&[("min_rust_version", str_param())]),
                ),
            ]
            .into_iter()
            .collect(),
        );
        let new = make_orb_with_jobs(
            [(
                "update_prlog".to_string(),
                job(&[("context", str_param()), ("run_label", bool_param())]),
            )]
            .into_iter()
            .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::JobAbsorbed { absorbed, into, .. }
                if absorbed == "label" && into == "update_prlog"
            )),
            "Expected JobAbsorbed rule, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_job_renamed() {
        let shared = [
            ("context", str_param()),
            ("cargo_all_features", bool_param()),
            ("cache_version", str_param()),
        ];
        let old = make_orb_with_jobs(
            [("idiomatic_rust".to_string(), job(&shared))]
                .into_iter()
                .collect(),
        );
        let new = make_orb_with_jobs(
            [("idiomatic_rust_rolling".to_string(), job(&shared))]
                .into_iter()
                .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::JobRenamed { from, to, .. }
                if from == "idiomatic_rust" && to == "idiomatic_rust_rolling"
            )),
            "Expected JobRenamed rule, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_enum_value_removed() {
        let old = make_orb_with_jobs(
            [(
                "update_changelog".to_string(),
                job(&[("update_log_option", enum_param(&["halt", "pipeline"]))]),
            )]
            .into_iter()
            .collect(),
        );
        let new = make_orb_with_jobs(
            [(
                "update_changelog".to_string(),
                job(&[("update_log_option", enum_param(&["halt"]))]),
            )]
            .into_iter()
            .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::ParameterEnumValueRemoved {
                    job, parameter, removed_value, fallback_value, ..
                }
                if job == "update_changelog"
                    && parameter == "update_log_option"
                    && removed_value == "pipeline"
                    && fallback_value == "halt"
            )),
            "Expected ParameterEnumValueRemoved, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_no_changes_produces_no_rules() {
        let jobs = [("some_job".to_string(), job(&[("p", str_param())]))]
            .into_iter()
            .collect();
        let old = make_orb_with_jobs(jobs);
        let new_jobs = [("some_job".to_string(), job(&[("p", str_param())]))]
            .into_iter()
            .collect();
        let new = make_orb_with_jobs(new_jobs);

        let rules = diff(&old, &new, "5.0.0");
        assert!(rules.is_empty(), "Expected no rules, got: {:?}", rules);
    }

    #[test]
    fn test_command_removed() {
        let old = make_orb(
            HashMap::new(),
            [("setup_env".to_string(), command(&[]))]
                .into_iter()
                .collect(),
        );
        let new = make_orb(HashMap::new(), HashMap::new());

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::CommandRemoved { name, .. }
                if name == "setup_env"
            )),
            "Expected CommandRemoved, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_command_renamed() {
        let shared = [("token", str_param()), ("env", str_param())];
        let old = make_orb(
            HashMap::new(),
            [("setup_env".to_string(), command(&shared))]
                .into_iter()
                .collect(),
        );
        let new = make_orb(
            HashMap::new(),
            [("configure_env".to_string(), command(&shared))]
                .into_iter()
                .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::CommandRenamed { from, to, .. }
                if from == "setup_env" && to == "configure_env"
            )),
            "Expected CommandRenamed, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_command_parameter_removed() {
        let old = make_orb(
            HashMap::new(),
            [(
                "build".to_string(),
                command(&[("target", str_param()), ("strip", bool_param())]),
            )]
            .into_iter()
            .collect(),
        );
        let new = make_orb(
            HashMap::new(),
            [("build".to_string(), command(&[("target", str_param())]))]
                .into_iter()
                .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::CommandParameterRemoved { command, parameter, .. }
                if command == "build" && parameter == "strip"
            )),
            "Expected CommandParameterRemoved, got: {:?}",
            rules
        );
    }

    #[test]
    fn test_command_parameter_added_mandatory() {
        let old = make_orb(
            HashMap::new(),
            [("publish".to_string(), command(&[("token", str_param())]))]
                .into_iter()
                .collect(),
        );
        let new = make_orb(
            HashMap::new(),
            [(
                "publish".to_string(),
                command(&[("token", str_param()), ("registry", str_param())]),
            )]
            .into_iter()
            .collect(),
        );

        let rules = diff(&old, &new, "5.0.0");
        assert!(
            rules.iter().any(|r| matches!(r,
                ConformanceRule::CommandParameterAdded { command, parameter, .. }
                if command == "publish" && parameter == "registry"
            )),
            "Expected CommandParameterAdded, got: {:?}",
            rules
        );
    }
}
