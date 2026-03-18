//! Semantic diff between two `OrbDefinition` instances.
//!
//! The `OrbDiffer` computes a `Vec<ConformanceRule>` by comparing an old and a
//! new `OrbDefinition`. It runs a set of detection passes:
//!
//! 1. Collect removed/added jobs and parameters
//! 2. Run `JobAbsorbed` heuristic (removed job + new boolean param)
//! 3. Run `JobRenamed` heuristic (parameter-set fuzzy match)
//! 4. Emit `JobRemoved` for unaccounted removals
//! 5. Emit `ParameterRemoved` for parameters lost from surviving jobs
//! 6. Emit `ParameterEnumValueRemoved` for enum values removed from surviving parameters

pub mod heuristics;

use std::collections::{HashMap, HashSet};

use crate::conformance_rule::ConformanceRule;
use crate::parser::types::{OrbDefinition, ParameterType};

use heuristics::{detect_absorbed_jobs, detect_renamed_jobs};

/// The default Jaccard similarity threshold for rename detection.
const RENAME_THRESHOLD: f64 = 0.7;

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

        // Phase 1: identify removed/added jobs
        let removed_job_names: HashSet<String> = self
            .old
            .jobs
            .keys()
            .filter(|name| !self.new.jobs.contains_key(*name))
            .cloned()
            .collect();

        let removed_jobs: HashMap<String, &crate::parser::types::Job> = removed_job_names
            .iter()
            .filter_map(|name| self.old.jobs.get(name).map(|j| (name.clone(), j)))
            .collect();
        let new_jobs: HashMap<String, &crate::parser::types::Job> =
            self.new.jobs.iter().map(|(k, v)| (k.clone(), v)).collect();
        let old_jobs: HashMap<String, &crate::parser::types::Job> =
            self.old.jobs.iter().map(|(k, v)| (k.clone(), v)).collect();

        // Phase 2: JobAbsorbed detection
        let absorbed = detect_absorbed_jobs(&removed_jobs, &new_jobs, &old_jobs);
        for (absorbed_name, into_name) in &absorbed {
            rules.push(ConformanceRule::JobAbsorbed {
                absorbed: absorbed_name.clone(),
                into: into_name.clone(),
                since_version: self.since_version.clone(),
            });
        }

        // Phase 3: JobRenamed detection (on remaining unaccounted removals)
        let unaccounted: HashSet<String> = removed_job_names
            .difference(&absorbed.keys().cloned().collect())
            .cloned()
            .collect();
        let renamed = detect_renamed_jobs(&unaccounted, &new_jobs, &old_jobs, RENAME_THRESHOLD);
        for (from, to) in &renamed {
            // Compute removed parameters: params in old job but not new job
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
            let removed_parameters: Vec<String> = old_params
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

        // Phase 4: JobRemoved for remaining unaccounted jobs
        let renamed_keys: HashSet<String> = renamed.keys().cloned().collect();
        for name in unaccounted.difference(&renamed_keys) {
            rules.push(ConformanceRule::JobRemoved {
                name: name.clone(),
                since_version: self.since_version.clone(),
            });
        }

        // Phase 5: ParameterRemoved for surviving jobs
        for (job_name, old_job) in &self.old.jobs {
            let Some(new_job) = self.new.jobs.get(job_name.as_str()) else {
                continue; // job was removed — handled above
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

        // Phase 6: ParameterEnumValueRemoved for surviving enum parameters
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

                // Determine fallback: first value in new_values, or empty string
                let fallback = new_param
                    .enum_values
                    .as_deref()
                    .and_then(|v| v.first())
                    .map(|s| s.as_str())
                    .unwrap_or("");

                for removed_value in old_values.difference(&new_values) {
                    rules.push(ConformanceRule::ParameterEnumValueRemoved {
                        job: job_name.clone(),
                        parameter: param_name.clone(),
                        removed_value: removed_value.to_string(),
                        fallback_value: fallback.to_string(),
                        since_version: self.since_version.clone(),
                    });
                }
            }
        }

        rules
    }
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

    fn str_param() -> Parameter {
        Parameter {
            param_type: ParameterType::String,
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
}
