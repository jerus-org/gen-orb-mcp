//! Migration planner: converts `Vec<ConformanceRule>` × `ConsumerConfig` → `MigrationPlan`.
//!
//! The planner inspects the consumer's actual CI state and applies each conformance
//! rule to identify what needs changing. It is version-agnostic: it checks actual
//! state, not assumed source versions.

use std::collections::HashSet;

use crate::conformance_rule::ConformanceRule;
use crate::consumer_parser::graph::find_absorbed_candidates;
use crate::consumer_parser::types::ConsumerConfig;

use super::types::{ChangeType, MigrationPlan, PlannedChange};

/// Produces a `MigrationPlan` by applying each conformance rule against the consumer config.
///
/// # Arguments
/// * `rules` — conformance rules for the target orb version
/// * `config` — parsed consumer config
/// * `orb_alias` — the alias used in the consumer's `orbs:` section (e.g. `"toolkit"`)
pub fn plan(rules: &[ConformanceRule], config: &ConsumerConfig, orb_alias: &str) -> MigrationPlan {
    let mut changes: Vec<PlannedChange> = Vec::new();

    // Detect the current pinned version (take the first file that references the orb)
    let detected_version = detect_version(config, orb_alias);

    // Determine the target version from the rules (take the first rule's since_version)
    let target_version = rules
        .first()
        .map(|r| r.since_version().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    for rule in rules {
        apply_rule(rule, config, orb_alias, &mut changes);
    }

    // Deduplicate changes that reference the same file/workflow/job to avoid
    // double-applying when multiple rules target the same invocation
    dedup_changes(&mut changes);

    MigrationPlan {
        orb: orb_alias.to_string(),
        target_version,
        detected_version,
        changes,
    }
}

/// Detects the current version of the orb as pinned in the consumer config.
fn detect_version(config: &ConsumerConfig, orb_alias: &str) -> String {
    for ci_file in config.files.values() {
        if let Some(orb_ref) = ci_file.orb_aliases.get(orb_alias) {
            return orb_ref.version.clone();
        }
    }
    "unknown".to_string()
}

/// Applies a single conformance rule to the consumer config and appends any
/// required changes to `changes`.
fn apply_rule(
    rule: &ConformanceRule,
    config: &ConsumerConfig,
    orb_alias: &str,
    changes: &mut Vec<PlannedChange>,
) {
    match rule {
        ConformanceRule::JobRemoved { name, .. } => {
            plan_job_removed(config, orb_alias, name, changes);
        }
        ConformanceRule::JobRenamed {
            from,
            to,
            removed_parameters,
            ..
        } => {
            plan_job_renamed(config, orb_alias, from, to, removed_parameters, changes);
        }
        ConformanceRule::ParameterRemoved { job, parameter, .. } => {
            plan_parameter_removed(config, orb_alias, job, parameter, changes);
        }
        ConformanceRule::JobAbsorbed { absorbed, into, .. } => {
            plan_job_absorbed(config, orb_alias, absorbed, into, changes);
        }
        ConformanceRule::ParameterEnumValueRemoved {
            job,
            parameter,
            removed_value,
            fallback_value,
            ..
        } => {
            plan_enum_value_removed(
                config,
                orb_alias,
                job,
                parameter,
                removed_value,
                fallback_value,
                changes,
            );
        }
        ConformanceRule::ParameterAdded { .. } => {
            // Cannot auto-apply: the value for a new mandatory parameter is
            // context-dependent. The MCP tool layer advises the user to add it.
        }
        ConformanceRule::CommandRemoved { .. }
        | ConformanceRule::CommandRenamed { .. }
        | ConformanceRule::CommandParameterRemoved { .. }
        | ConformanceRule::CommandParameterAdded { .. } => {
            // Command-level changes in consumer custom job steps are not yet
            // parsed by the consumer_parser. The MCP tool layer advises manual review.
        }
    }
}

fn plan_job_removed(
    config: &ConsumerConfig,
    orb_alias: &str,
    job_name: &str,
    changes: &mut Vec<PlannedChange>,
) {
    for ci_file in config.files.values() {
        for (workflow_name, workflow) in &ci_file.workflows {
            for inv in &workflow.jobs {
                if inv.matches(orb_alias, job_name) {
                    changes.push(PlannedChange {
                        file: inv.location.file.clone(),
                        description: format!(
                            "Remove `{orb_alias}/{job_name}` — job was removed with no replacement"
                        ),
                        change_type: ChangeType::RemoveJobInvocation {
                            workflow: workflow_name.clone(),
                            job_ref: inv.effective_name().to_string(),
                        },
                        before: format!("- {orb_alias}/{job_name}"),
                        after: String::new(),
                    });
                }
            }
        }
    }
}

fn plan_job_renamed(
    config: &ConsumerConfig,
    orb_alias: &str,
    from: &str,
    to: &str,
    removed_parameters: &[String],
    changes: &mut Vec<PlannedChange>,
) {
    for ci_file in config.files.values() {
        for (workflow_name, workflow) in &ci_file.workflows {
            for inv in &workflow.jobs {
                if inv.matches(orb_alias, from) {
                    // Rename the job reference
                    changes.push(PlannedChange {
                        file: inv.location.file.clone(),
                        description: format!("Rename `{orb_alias}/{from}` → `{orb_alias}/{to}`"),
                        change_type: ChangeType::RenameJobInvocation {
                            workflow: workflow_name.clone(),
                            from: format!("{orb_alias}/{from}"),
                            to: format!("{orb_alias}/{to}"),
                        },
                        before: format!("{orb_alias}/{from}"),
                        after: format!("{orb_alias}/{to}"),
                    });

                    // Strip removed parameters
                    for param in removed_parameters {
                        if inv.parameters.contains_key(param.as_str()) {
                            changes.push(PlannedChange {
                                file: inv.location.file.clone(),
                                description: format!(
                                    "Remove parameter `{param}` from renamed job `{orb_alias}/{to}`"
                                ),
                                change_type: ChangeType::RemoveParameter {
                                    workflow: workflow_name.clone(),
                                    job_ref: inv.effective_name().to_string(),
                                    parameter: param.clone(),
                                },
                                before: format!("{param}: <value>"),
                                after: String::new(),
                            });
                        }
                    }
                }
            }
        }
    }
}

fn plan_parameter_removed(
    config: &ConsumerConfig,
    orb_alias: &str,
    job_name: &str,
    parameter: &str,
    changes: &mut Vec<PlannedChange>,
) {
    for ci_file in config.files.values() {
        for (workflow_name, workflow) in &ci_file.workflows {
            for inv in &workflow.jobs {
                if inv.matches(orb_alias, job_name) && inv.parameters.contains_key(parameter) {
                    changes.push(PlannedChange {
                        file: inv.location.file.clone(),
                        description: format!(
                            "Remove parameter `{parameter}` from `{orb_alias}/{job_name}`"
                        ),
                        change_type: ChangeType::RemoveParameter {
                            workflow: workflow_name.clone(),
                            job_ref: inv.effective_name().to_string(),
                            parameter: parameter.to_string(),
                        },
                        before: format!("{parameter}: <value>"),
                        after: String::new(),
                    });
                }
            }
        }
    }
}

fn plan_job_absorbed(
    config: &ConsumerConfig,
    orb_alias: &str,
    absorbed: &str,
    into: &str,
    changes: &mut Vec<PlannedChange>,
) {
    for ci_file in config.files.values() {
        for (workflow_name, workflow) in &ci_file.workflows {
            // Find the effective name(s) for the absorbing job so that we can
            // check whether the absorbed job's requires-chain includes it.
            let absorbing_effective_names: Vec<String> = workflow
                .jobs
                .iter()
                .filter(|inv| inv.matches(orb_alias, into))
                .map(|inv| inv.effective_name().to_string())
                .collect();

            for absorbing_name in &absorbing_effective_names {
                let candidates =
                    find_absorbed_candidates(workflow, orb_alias, absorbed, absorbing_name);
                for idx in candidates {
                    let inv = &workflow.jobs[idx];
                    changes.push(PlannedChange {
                        file: inv.location.file.clone(),
                        description: format!(
                            "Remove `{orb_alias}/{absorbed}` — its functionality is now included \
                             in `{orb_alias}/{into}` (requires chain includes `{absorbing_name}`)"
                        ),
                        change_type: ChangeType::RemoveJobInvocation {
                            workflow: workflow_name.clone(),
                            job_ref: inv.effective_name().to_string(),
                        },
                        before: format!("- {orb_alias}/{absorbed}"),
                        after: String::new(),
                    });
                }
            }
        }
    }
}

fn plan_enum_value_removed(
    config: &ConsumerConfig,
    orb_alias: &str,
    job_name: &str,
    parameter: &str,
    removed_value: &str,
    fallback_value: &str,
    changes: &mut Vec<PlannedChange>,
) {
    for ci_file in config.files.values() {
        for (workflow_name, workflow) in &ci_file.workflows {
            for inv in &workflow.jobs {
                if !inv.matches(orb_alias, job_name) {
                    continue;
                }
                let Some(val) = inv.parameters.get(parameter) else {
                    continue;
                };
                let current_value = match val {
                    serde_yaml::Value::String(s) => s.as_str(),
                    _ => continue,
                };
                if current_value == removed_value {
                    changes.push(PlannedChange {
                        file: inv.location.file.clone(),
                        description: format!(
                            "Replace `{parameter}: {removed_value}` with `{parameter}: {fallback_value}` \
                             on `{orb_alias}/{job_name}` — value `{removed_value}` was removed"
                        ),
                        change_type: ChangeType::ReplaceParameterValue {
                            workflow: workflow_name.clone(),
                            job_ref: inv.effective_name().to_string(),
                            parameter: parameter.to_string(),
                            replacement: fallback_value.to_string(),
                        },
                        before: format!("{parameter}: {removed_value}"),
                        after: format!("{parameter}: {fallback_value}"),
                    });
                }
            }
        }
    }
}

/// Removes duplicate `RemoveJobInvocation` changes that target the same
/// workflow + job_ref (can arise when both `JobAbsorbed` and `JobRemoved`
/// fire for the same invocation).
fn dedup_changes(changes: &mut Vec<PlannedChange>) {
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    changes.retain(|c| {
        let key = match &c.change_type {
            ChangeType::RemoveJobInvocation { workflow, job_ref } => (
                c.file.display().to_string(),
                workflow.clone(),
                format!("remove:{job_ref}"),
            ),
            _ => return true, // keep non-remove changes without dedup
        };
        seen.insert(key)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumer_parser::types::{
        CiFile, ConsumerConfig, JobInvocation, OrbRef, SourceLocation, Workflow,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_config_with_label_and_update_prlog() -> ConsumerConfig {
        let mut config = ConsumerConfig::default();
        let mut ci_file = CiFile::default();

        ci_file.orb_aliases.insert(
            "toolkit".to_string(),
            OrbRef {
                org: "jerus-org".to_string(),
                name: "circleci-toolkit".to_string(),
                version: "4.8.0".to_string(),
            },
        );

        let mut workflow = Workflow::default();
        workflow.jobs.push(JobInvocation {
            reference: "toolkit/update_prlog".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_job: Some("update_prlog".to_string()),
            parameters: {
                let mut p = HashMap::new();
                p.insert(
                    "min_rust_version".to_string(),
                    serde_yaml::Value::String("1.85".to_string()),
                );
                p
            },
            requires: vec![],
            name_override: Some("update-prlog-on-main".to_string()),
            location: SourceLocation {
                file: PathBuf::from("update_prlog.yml"),
                workflow: "update_prlog".to_string(),
                job_index: 0,
            },
        });
        workflow.jobs.push(JobInvocation {
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
        });

        ci_file
            .workflows
            .insert("update_prlog".to_string(), workflow);
        config
            .files
            .insert(PathBuf::from("update_prlog.yml"), ci_file);
        config
    }

    #[test]
    fn test_plan_parameter_removed() {
        let config = make_config_with_label_and_update_prlog();
        let rules = vec![ConformanceRule::ParameterRemoved {
            job: "update_prlog".to_string(),
            parameter: "min_rust_version".to_string(),
            since_version: "5.0.0".to_string(),
        }];

        let plan_result = plan(&rules, &config, "toolkit");
        assert_eq!(plan_result.changes.len(), 1);
        matches!(
            &plan_result.changes[0].change_type,
            ChangeType::RemoveParameter { parameter, .. } if parameter == "min_rust_version"
        );
    }

    #[test]
    fn test_plan_job_absorbed() {
        let config = make_config_with_label_and_update_prlog();
        let rules = vec![ConformanceRule::JobAbsorbed {
            absorbed: "label".to_string(),
            into: "update_prlog".to_string(),
            since_version: "5.0.0".to_string(),
        }];

        let plan_result = plan(&rules, &config, "toolkit");
        assert_eq!(
            plan_result.changes.len(),
            1,
            "Expected 1 change, got: {:?}",
            plan_result
                .changes
                .iter()
                .map(|c| &c.description)
                .collect::<Vec<_>>()
        );
        assert!(matches!(
            &plan_result.changes[0].change_type,
            ChangeType::RemoveJobInvocation { job_ref, .. } if job_ref == "toolkit/label"
        ));
    }

    #[test]
    fn test_plan_detects_version() {
        let config = make_config_with_label_and_update_prlog();
        let rules = vec![ConformanceRule::ParameterRemoved {
            job: "update_prlog".to_string(),
            parameter: "min_rust_version".to_string(),
            since_version: "5.0.0".to_string(),
        }];

        let plan_result = plan(&rules, &config, "toolkit");
        assert_eq!(plan_result.detected_version, "4.8.0");
        assert_eq!(plan_result.target_version, "5.0.0");
    }

    #[test]
    fn test_plan_no_changes_when_already_conformant() {
        let config = make_config_with_label_and_update_prlog();
        // Rule targets a parameter that isn't in the consumer's config
        let rules = vec![ConformanceRule::ParameterRemoved {
            job: "update_prlog".to_string(),
            parameter: "nonexistent_param".to_string(),
            since_version: "5.0.0".to_string(),
        }];

        let plan_result = plan(&rules, &config, "toolkit");
        assert!(plan_result.changes.is_empty());
    }

    #[test]
    fn test_plan_job_removed() {
        let config = make_config_with_label_and_update_prlog();
        // Treat label as entirely removed (not absorbed)
        let rules = vec![ConformanceRule::JobRemoved {
            name: "label".to_string(),
            since_version: "5.0.0".to_string(),
        }];

        let plan_result = plan(&rules, &config, "toolkit");
        assert_eq!(plan_result.changes.len(), 1);
        assert!(matches!(
            &plan_result.changes[0].change_type,
            ChangeType::RemoveJobInvocation { .. }
        ));
    }
}
