//! Migration planner: converts `Vec<ConformanceRule>` × `ConsumerConfig` →
//! `MigrationPlan`.
//!
//! The planner inspects the consumer's actual CI state and applies each
//! conformance rule to identify what needs changing. It is version-agnostic: it
//! checks actual state, not assumed source versions.

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use super::types::{ChangeType, MigrationPlan, PlannedChange};
use crate::{
    conformance_rule::ConformanceRule,
    consumer_parser::{graph::find_absorbed_candidates, types::ConsumerConfig},
};

/// Type alias to reduce repetition in planner function signatures.
type Changes = Vec<PlannedChange>;

/// Produces a `MigrationPlan` by applying each conformance rule against the
/// consumer config.
///
/// # Arguments
/// * `rules` — conformance rules for the target orb version
/// * `config` — parsed consumer config
/// * `orb_alias` — the alias used in the consumer's `orbs:` section (e.g.
///   `"toolkit"`)
/// * `current_orb_version` — the version of the orb embedded in this binary
///   (e.g. `"5.3.10"`); used to plan orb version pin updates
pub fn plan(
    rules: &[ConformanceRule],
    config: &ConsumerConfig,
    orb_alias: &str,
    current_orb_version: &str,
) -> MigrationPlan {
    let mut changes: Vec<PlannedChange> = Vec::new();

    // Detect the current pinned version (take the first file that references the
    // orb)
    let detected_version = detect_version(config, orb_alias);

    // Target version is the current orb version embedded in this binary.
    let target_version = if current_orb_version.is_empty() {
        rules
            .first()
            .map(|r| r.since_version().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        current_orb_version.to_string()
    };

    for rule in rules {
        apply_rule(rule, config, orb_alias, &mut changes);
    }

    // Deduplicate changes that reference the same file/workflow/job to avoid
    // double-applying when multiple rules target the same invocation
    dedup_changes(&mut changes);

    // After dedup, detect pipeline parameter declarations that have become
    // orphaned (removed from every remaining invocation in the file).
    plan_orphaned_pipeline_params(config, &changes.clone(), &mut changes);

    // Plan orb version pin updates for files whose current pin differs from
    // the current orb version.
    plan_orb_version_updates(config, orb_alias, current_orb_version, &mut changes);

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
        ConformanceRule::CommandRemoved { name, .. } => {
            plan_command_removed(config, orb_alias, name, changes);
        }
        ConformanceRule::CommandRenamed {
            from,
            to,
            removed_parameters,
            ..
        } => {
            plan_command_renamed(config, orb_alias, from, to, removed_parameters, changes);
        }
        ConformanceRule::CommandParameterRemoved {
            command, parameter, ..
        } => {
            plan_command_parameter_removed(config, orb_alias, command, parameter, changes);
        }
        ConformanceRule::CommandParameterAdded { .. } => {
            // Cannot auto-apply: value for new mandatory command parameter is
            // context-dependent. The MCP tool layer advises the user to add it.
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
                plan_single_job_renamed(
                    inv,
                    workflow_name,
                    orb_alias,
                    from,
                    to,
                    removed_parameters,
                    changes,
                );
            }
        }
    }
}

/// Emits rename and parameter-removal changes for a single matching job
/// invocation.
fn plan_single_job_renamed(
    inv: &crate::consumer_parser::types::JobInvocation,
    workflow_name: &str,
    orb_alias: &str,
    from: &str,
    to: &str,
    removed_parameters: &[String],
    changes: &mut Changes,
) {
    if !inv.matches(orb_alias, from) {
        return;
    }
    // Rename the job reference
    changes.push(PlannedChange {
        file: inv.location.file.clone(),
        description: format!("Rename `{orb_alias}/{from}` → `{orb_alias}/{to}`"),
        change_type: ChangeType::RenameJobInvocation {
            workflow: workflow_name.to_string(),
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
                    workflow: workflow_name.to_string(),
                    job_ref: inv.effective_name().to_string(),
                    parameter: param.clone(),
                },
                before: format!("{param}: <value>"),
                after: String::new(),
            });
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
                plan_single_enum_value(
                    inv,
                    workflow_name,
                    orb_alias,
                    job_name,
                    parameter,
                    removed_value,
                    fallback_value,
                    changes,
                );
            }
        }
    }
}

/// Emits a parameter-value replacement change for a single matching job
/// invocation.
#[allow(clippy::too_many_arguments)]
fn plan_single_enum_value(
    inv: &crate::consumer_parser::types::JobInvocation,
    workflow_name: &str,
    orb_alias: &str,
    job_name: &str,
    parameter: &str,
    removed_value: &str,
    fallback_value: &str,
    changes: &mut Changes,
) {
    if !inv.matches(orb_alias, job_name) {
        return;
    }
    let Some(val) = inv.parameters.get(parameter) else {
        return;
    };
    let current_value = match val {
        serde_yaml::Value::String(s) => s.as_str(),
        _ => return,
    };
    if current_value == removed_value {
        changes.push(PlannedChange {
            file: inv.location.file.clone(),
            description: format!(
                "Replace `{parameter}: {removed_value}` with `{parameter}: {fallback_value}` \
                 on `{orb_alias}/{job_name}` — value `{removed_value}` was removed"
            ),
            change_type: ChangeType::ReplaceParameterValue {
                workflow: workflow_name.to_string(),
                job_ref: inv.effective_name().to_string(),
                parameter: parameter.to_string(),
                replacement: fallback_value.to_string(),
            },
            before: format!("{parameter}: {removed_value}"),
            after: format!("{parameter}: {fallback_value}"),
        });
    }
}

fn plan_command_removed(
    config: &ConsumerConfig,
    orb_alias: &str,
    command_name: &str,
    changes: &mut Changes,
) {
    for ci_file in config.files.values() {
        for (job_name, custom_job) in &ci_file.custom_jobs {
            for step in &custom_job.steps {
                if step.matches(orb_alias, command_name) {
                    changes.push(PlannedChange {
                        file: step.location.file.clone(),
                        description: format!(
                            "Remove `{orb_alias}/{command_name}` from job `{job_name}` \
                             — command was removed with no replacement"
                        ),
                        change_type: ChangeType::RemoveCommandInvocation {
                            job: job_name.clone(),
                            command_ref: step.reference.clone(),
                        },
                        before: format!("- {orb_alias}/{command_name}"),
                        after: String::new(),
                    });
                }
            }
        }
    }
}

fn plan_command_renamed(
    config: &ConsumerConfig,
    orb_alias: &str,
    from: &str,
    to: &str,
    removed_parameters: &[String],
    changes: &mut Changes,
) {
    for ci_file in config.files.values() {
        for (job_name, custom_job) in &ci_file.custom_jobs {
            for step in &custom_job.steps {
                plan_single_command_renamed(
                    step,
                    job_name,
                    orb_alias,
                    from,
                    to,
                    removed_parameters,
                    changes,
                );
            }
        }
    }
}

/// Emits rename and parameter-removal changes for a single matching command
/// step.
fn plan_single_command_renamed(
    step: &crate::consumer_parser::types::StepInvocation,
    job_name: &str,
    orb_alias: &str,
    from: &str,
    to: &str,
    removed_parameters: &[String],
    changes: &mut Changes,
) {
    if !step.matches(orb_alias, from) {
        return;
    }
    // Rename the command reference
    changes.push(PlannedChange {
        file: step.location.file.clone(),
        description: format!(
            "Rename `{orb_alias}/{from}` → `{orb_alias}/{to}` in job `{job_name}`"
        ),
        change_type: ChangeType::RenameCommandInvocation {
            job: job_name.to_string(),
            from: format!("{orb_alias}/{from}"),
            to: format!("{orb_alias}/{to}"),
        },
        before: format!("{orb_alias}/{from}"),
        after: format!("{orb_alias}/{to}"),
    });
    // Strip removed parameters
    for param in removed_parameters {
        if step.parameters.contains_key(param.as_str()) {
            changes.push(PlannedChange {
                file: step.location.file.clone(),
                description: format!(
                    "Remove parameter `{param}` from renamed command \
                     `{orb_alias}/{to}` in job `{job_name}`"
                ),
                change_type: ChangeType::RemoveCommandParameter {
                    job: job_name.to_string(),
                    command_ref: format!("{orb_alias}/{to}"),
                    parameter: param.clone(),
                },
                before: format!("{param}: <value>"),
                after: String::new(),
            });
        }
    }
}

fn plan_command_parameter_removed(
    config: &ConsumerConfig,
    orb_alias: &str,
    command_name: &str,
    parameter: &str,
    changes: &mut Changes,
) {
    for ci_file in config.files.values() {
        for (job_name, custom_job) in &ci_file.custom_jobs {
            for step in &custom_job.steps {
                if step.matches(orb_alias, command_name) && step.parameters.contains_key(parameter)
                {
                    changes.push(PlannedChange {
                        file: step.location.file.clone(),
                        description: format!(
                            "Remove parameter `{parameter}` from `{orb_alias}/{command_name}` \
                             in job `{job_name}`"
                        ),
                        change_type: ChangeType::RemoveCommandParameter {
                            job: job_name.clone(),
                            command_ref: step.reference.clone(),
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

/// After the main rules pass, check for pipeline parameter declarations that
/// are no longer referenced in any remaining job invocation in the file.
///
/// For each `RemoveParameter` change grouped by file, if the parameter is also
/// declared in the file's `pipeline_parameters` block and no remaining
/// invocation (not covered by a `RemoveParameter` or `RemoveJobInvocation` for
/// this file) still uses it, emit a `RemovePipelineParameter` change.
fn plan_orphaned_pipeline_params(
    config: &ConsumerConfig,
    existing_changes: &[PlannedChange],
    changes: &mut Vec<PlannedChange>,
) {
    let params_removed_by_file = collect_removed_params_by_file(existing_changes);
    let jobs_removed = collect_removed_jobs(existing_changes);

    for (file_path, removed_params) in &params_removed_by_file {
        // config.files is keyed by filename only (as the consumer parser
        // inserts it), but file_path here is the full path from
        // SourceLocation.file.  Normalise to filename for the lookup.
        let lookup_key = file_path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| file_path.to_path_buf());
        let Some(ci_file) = config.files.get(&lookup_key) else {
            continue;
        };
        for param_name in removed_params {
            if !ci_file.pipeline_parameters.contains(param_name) {
                continue;
            }
            if !param_still_in_use(
                ci_file,
                file_path,
                param_name,
                &jobs_removed,
                existing_changes,
            ) {
                changes.push(make_remove_pipeline_param_change(file_path, param_name));
            }
        }
    }
}

/// Builds a map of file path → set of parameter names being stripped via
/// `RemoveParameter` changes.
fn collect_removed_params_by_file(
    changes: &[PlannedChange],
) -> HashMap<std::path::PathBuf, HashSet<String>> {
    let mut map: HashMap<std::path::PathBuf, HashSet<String>> = HashMap::new();
    for change in changes {
        if let ChangeType::RemoveParameter { parameter, .. } = &change.change_type {
            map.entry(change.file.clone())
                .or_default()
                .insert(parameter.clone());
        }
    }
    map
}

/// Builds a set of `(file, effective_job_ref)` pairs being entirely removed
/// via `RemoveJobInvocation` changes.
fn collect_removed_jobs(changes: &[PlannedChange]) -> HashSet<(std::path::PathBuf, String)> {
    changes
        .iter()
        .filter_map(|c| {
            if let ChangeType::RemoveJobInvocation { job_ref, .. } = &c.change_type {
                Some((c.file.clone(), job_ref.clone()))
            } else {
                None
            }
        })
        .collect()
}

/// Returns `true` if `param_name` is still referenced in at least one
/// invocation in `ci_file` that is neither being entirely removed nor having
/// `param_name` stripped by an existing `RemoveParameter` change.
fn param_still_in_use(
    ci_file: &crate::consumer_parser::types::CiFile,
    file_path: &std::path::Path,
    param_name: &str,
    jobs_removed: &HashSet<(std::path::PathBuf, String)>,
    existing_changes: &[PlannedChange],
) -> bool {
    ci_file
        .workflows
        .values()
        .flat_map(|w| w.jobs.iter())
        .filter(|inv| {
            !jobs_removed.contains(&(file_path.to_path_buf(), inv.effective_name().to_string()))
        })
        .any(|inv| {
            inv.parameters.contains_key(param_name)
                && !param_removed_from_inv(
                    existing_changes,
                    file_path,
                    inv.effective_name(),
                    param_name,
                )
        })
}

/// Returns `true` if an existing `RemoveParameter` change targets `param_name`
/// on `job_ref` in `file_path`.
fn param_removed_from_inv(
    changes: &[PlannedChange],
    file_path: &std::path::Path,
    job_ref: &str,
    param_name: &str,
) -> bool {
    changes.iter().any(|c| {
        c.file == file_path
            && matches!(
                &c.change_type,
                ChangeType::RemoveParameter { job_ref: jr, parameter: p, .. }
                if jr == job_ref && p == param_name
            )
    })
}

/// Constructs a `RemovePipelineParameter` planned change.
fn make_remove_pipeline_param_change(
    file_path: &std::path::Path,
    param_name: &str,
) -> PlannedChange {
    PlannedChange {
        file: file_path.to_path_buf(),
        description: format!(
            "Remove orphaned pipeline parameter `{param_name}` from `parameters:` block"
        ),
        change_type: ChangeType::RemovePipelineParameter {
            parameter: param_name.to_string(),
        },
        before: format!("{param_name}: <declaration>"),
        after: String::new(),
    }
}

/// For each file in `config` that has the orb alias pinned to a version other
/// than `current_orb_version`, emits an `UpdateOrbVersion` change.
fn plan_orb_version_updates(
    config: &ConsumerConfig,
    orb_alias: &str,
    current_orb_version: &str,
    changes: &mut Vec<PlannedChange>,
) {
    if current_orb_version.is_empty() {
        return;
    }
    for (file_path, ci_file) in &config.files {
        let Some(orb_ref) = ci_file.orb_aliases.get(orb_alias) else {
            continue;
        };
        if orb_ref.version == current_orb_version {
            continue;
        }
        // Use the full path stored at parse time; fall back to the filename-only
        // map key when source_path was not set (e.g. in unit tests).
        let resolved = if ci_file.source_path.is_absolute() {
            ci_file.source_path.clone()
        } else {
            file_path.clone()
        };
        changes.push(PlannedChange {
            file: resolved,
            description: format!(
                "Update orb pin `{orb_alias}` from `{}` to `{current_orb_version}`",
                orb_ref.version
            ),
            change_type: ChangeType::UpdateOrbVersion {
                orb_alias: orb_alias.to_string(),
                from_version: orb_ref.version.clone(),
                to_version: current_orb_version.to_string(),
            },
            before: format!(
                "{orb_alias}: {}/{orb_alias}@{}",
                orb_ref.org, orb_ref.version
            ),
            after: format!(
                "{orb_alias}: {}/{orb_alias}@{current_orb_version}",
                orb_ref.org
            ),
        });
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
                format!("remove-job:{job_ref}"),
            ),
            ChangeType::RemoveCommandInvocation { job, command_ref } => (
                c.file.display().to_string(),
                job.clone(),
                format!("remove-cmd:{command_ref}"),
            ),
            _ => return true, // keep other change types without dedup
        };
        seen.insert(key)
    });
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::*;
    use crate::consumer_parser::types::{
        CiFile, ConsumerConfig, JobInvocation, OrbRef, SourceLocation, Workflow,
    };

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

        let plan_result = plan(&rules, &config, "toolkit", "");
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

        let plan_result = plan(&rules, &config, "toolkit", "");
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

        let plan_result = plan(&rules, &config, "toolkit", "");
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

        let plan_result = plan(&rules, &config, "toolkit", "");
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

        let plan_result = plan(&rules, &config, "toolkit", "");
        assert_eq!(plan_result.changes.len(), 1);
        assert!(matches!(
            &plan_result.changes[0].change_type,
            ChangeType::RemoveJobInvocation { .. }
        ));
    }

    // ── Command-level planner tests ───────────────────────────────────────────

    fn make_config_with_custom_job() -> ConsumerConfig {
        use crate::consumer_parser::types::{CustomJob, StepInvocation, StepLocation};

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

        let mut custom_job = CustomJob::default();
        custom_job.steps.push(StepInvocation {
            reference: "toolkit/setup_env".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_command: Some("setup_env".to_string()),
            parameters: {
                let mut p = HashMap::new();
                p.insert(
                    "token".to_string(),
                    serde_yaml::Value::String("$GITHUB_TOKEN".to_string()),
                );
                p
            },
            location: StepLocation {
                file: PathBuf::from("config.yml"),
                job: "my-release-job".to_string(),
                step_index: 1,
            },
        });
        custom_job.steps.push(StepInvocation {
            reference: "toolkit/publish_crate".to_string(),
            orb_alias: Some("toolkit".to_string()),
            orb_command: Some("publish_crate".to_string()),
            parameters: {
                let mut p = HashMap::new();
                p.insert(
                    "package".to_string(),
                    serde_yaml::Value::String("my-crate".to_string()),
                );
                p
            },
            location: StepLocation {
                file: PathBuf::from("config.yml"),
                job: "my-release-job".to_string(),
                step_index: 3,
            },
        });

        ci_file
            .custom_jobs
            .insert("my-release-job".to_string(), custom_job);
        config.files.insert(PathBuf::from("config.yml"), ci_file);
        config
    }

    #[test]
    fn test_plan_command_removed() {
        let config = make_config_with_custom_job();
        let rules = vec![ConformanceRule::CommandRemoved {
            name: "setup_env".to_string(),
            since_version: "5.0.0".to_string(),
        }];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert_eq!(plan_result.changes.len(), 1);
        assert!(matches!(
            &plan_result.changes[0].change_type,
            ChangeType::RemoveCommandInvocation { job, command_ref }
            if job == "my-release-job" && command_ref == "toolkit/setup_env"
        ));
    }

    #[test]
    fn test_plan_command_renamed() {
        let config = make_config_with_custom_job();
        let rules = vec![ConformanceRule::CommandRenamed {
            from: "setup_env".to_string(),
            to: "configure_env".to_string(),
            removed_parameters: vec![],
            since_version: "5.0.0".to_string(),
        }];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert_eq!(plan_result.changes.len(), 1);
        assert!(matches!(
            &plan_result.changes[0].change_type,
            ChangeType::RenameCommandInvocation { job, from, to }
            if job == "my-release-job"
                && from == "toolkit/setup_env"
                && to == "toolkit/configure_env"
        ));
    }

    #[test]
    fn test_plan_command_renamed_strips_removed_params() {
        let config = make_config_with_custom_job();
        let rules = vec![ConformanceRule::CommandRenamed {
            from: "setup_env".to_string(),
            to: "configure_env".to_string(),
            removed_parameters: vec!["token".to_string()],
            since_version: "5.0.0".to_string(),
        }];
        let plan_result = plan(&rules, &config, "toolkit", "");
        // Rename + remove param = 2 changes
        assert_eq!(plan_result.changes.len(), 2);
        assert!(plan_result
            .changes
            .iter()
            .any(|c| matches!(&c.change_type, ChangeType::RenameCommandInvocation { .. })));
        assert!(plan_result.changes.iter().any(|c| matches!(
            &c.change_type,
            ChangeType::RemoveCommandParameter { parameter, .. } if parameter == "token"
        )));
    }

    #[test]
    fn test_plan_command_parameter_removed() {
        let config = make_config_with_custom_job();
        let rules = vec![ConformanceRule::CommandParameterRemoved {
            command: "publish_crate".to_string(),
            parameter: "package".to_string(),
            since_version: "5.0.0".to_string(),
        }];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert_eq!(plan_result.changes.len(), 1);
        assert!(matches!(
            &plan_result.changes[0].change_type,
            ChangeType::RemoveCommandParameter { job, command_ref, parameter }
            if job == "my-release-job"
                && command_ref == "toolkit/publish_crate"
                && parameter == "package"
        ));
    }

    #[test]
    fn test_plan_command_parameter_added_emits_no_change() {
        let config = make_config_with_custom_job();
        let rules = vec![ConformanceRule::CommandParameterAdded {
            command: "setup_env".to_string(),
            parameter: "region".to_string(),
            since_version: "5.0.0".to_string(),
        }];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert!(
            plan_result.changes.is_empty(),
            "CommandParameterAdded should produce no automated changes"
        );
    }

    #[test]
    fn test_plan_command_no_match() {
        let config = make_config_with_custom_job();
        let rules = vec![ConformanceRule::CommandRemoved {
            name: "nonexistent_cmd".to_string(),
            since_version: "5.0.0".to_string(),
        }];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert!(plan_result.changes.is_empty());
    }

    // ── Pipeline parameter orphan-detection tests ─────────────────────────────

    /// Builds a config where both `update_prlog` and `label` invocations in
    /// `update_prlog.yml` pass `min_rust_version`, and the file declares it as
    /// a pipeline parameter.
    fn make_config_with_pipeline_param() -> ConsumerConfig {
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

        // Declare min_rust_version as a pipeline parameter
        ci_file
            .pipeline_parameters
            .push("min_rust_version".to_string());

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
            parameters: {
                let mut p = HashMap::new();
                p.insert(
                    "min_rust_version".to_string(),
                    serde_yaml::Value::String("1.85".to_string()),
                );
                p
            },
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
    fn test_plan_updates_orb_version_pin() {
        let config = make_config_with_pipeline_param(); // has toolkit@4.8.0
        let rules: Vec<ConformanceRule> = vec![];
        let plan_result = plan(&rules, &config, "toolkit", "5.3.10");
        assert!(
            plan_result.changes.iter().any(|c| matches!(
                &c.change_type,
                ChangeType::UpdateOrbVersion { orb_alias, from_version, to_version }
                    if orb_alias == "toolkit"
                    && from_version == "4.8.0"
                    && to_version == "5.3.10"
            )),
            "should plan an UpdateOrbVersion change when pin differs from current version"
        );
        assert_eq!(plan_result.target_version, "5.3.10");
    }

    #[test]
    fn test_plan_skips_orb_version_update_when_already_current() {
        let mut config = make_config_with_pipeline_param();
        // Update the orb version to already be 5.3.10
        if let Some(ci_file) = config.files.values_mut().next() {
            if let Some(orb_ref) = ci_file.orb_aliases.get_mut("toolkit") {
                orb_ref.version = "5.3.10".to_string();
            }
        }
        let rules: Vec<ConformanceRule> = vec![];
        let plan_result = plan(&rules, &config, "toolkit", "5.3.10");
        assert!(
            !plan_result
                .changes
                .iter()
                .any(|c| matches!(&c.change_type, ChangeType::UpdateOrbVersion { .. })),
            "should not plan UpdateOrbVersion when pin already matches current version"
        );
    }

    #[test]
    fn test_plan_removes_orphaned_pipeline_param() {
        let config = make_config_with_pipeline_param();
        let rules = vec![
            ConformanceRule::ParameterRemoved {
                job: "update_prlog".to_string(),
                parameter: "min_rust_version".to_string(),
                since_version: "5.0.0".to_string(),
            },
            ConformanceRule::ParameterRemoved {
                job: "label".to_string(),
                parameter: "min_rust_version".to_string(),
                since_version: "5.0.0".to_string(),
            },
        ];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert!(
            plan_result.changes.iter().any(|c| matches!(
                &c.change_type,
                ChangeType::RemovePipelineParameter { parameter } if parameter == "min_rust_version"
            )),
            "orphaned pipeline parameter should be scheduled for removal"
        );
    }

    /// Same as `test_plan_removes_orphaned_pipeline_param` but uses a full
    /// filesystem path in `SourceLocation.file` (as the real consumer parser
    /// does) while `config.files` is keyed by filename only.  This exercises
    /// the path-normalisation in `plan_orphaned_pipeline_params`.
    #[test]
    fn test_plan_removes_orphaned_pipeline_param_full_path() {
        let mut config = make_config_with_pipeline_param();

        // Re-key config.files to filename only (as the parser does) and update
        // all SourceLocation.file entries to the full path.
        let full_path = PathBuf::from("/home/user/project/.circleci/update_prlog.yml");
        let ci_file = config
            .files
            .remove(&PathBuf::from("update_prlog.yml"))
            .unwrap();
        config.files.insert(PathBuf::from("update_prlog.yml"), {
            let mut f = ci_file;
            // Update every job's source location to use the full path
            for wf in f.workflows.values_mut() {
                for inv in wf.jobs.iter_mut() {
                    inv.location.file = full_path.clone();
                }
            }
            f
        });

        let rules = vec![
            ConformanceRule::ParameterRemoved {
                job: "update_prlog".to_string(),
                parameter: "min_rust_version".to_string(),
                since_version: "5.0.0".to_string(),
            },
            ConformanceRule::ParameterRemoved {
                job: "label".to_string(),
                parameter: "min_rust_version".to_string(),
                since_version: "5.0.0".to_string(),
            },
        ];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert!(
            plan_result.changes.iter().any(|c| matches!(
                &c.change_type,
                ChangeType::RemovePipelineParameter { parameter }
                    if parameter == "min_rust_version"
            )),
            "orphaned pipeline parameter should be removed even when SourceLocation uses a full path"
        );
    }

    #[test]
    fn test_plan_does_not_remove_still_used_pipeline_param() {
        // Only remove min_rust_version from update_prlog but not from label.
        // label still uses it, so the declaration must NOT be removed.
        let config = make_config_with_pipeline_param();
        let rules = vec![ConformanceRule::ParameterRemoved {
            job: "update_prlog".to_string(),
            parameter: "min_rust_version".to_string(),
            since_version: "5.0.0".to_string(),
        }];
        let plan_result = plan(&rules, &config, "toolkit", "");
        assert!(
            !plan_result.changes.iter().any(|c| matches!(
                &c.change_type,
                ChangeType::RemovePipelineParameter { parameter } if parameter == "min_rust_version"
            )),
            "declaration should NOT be removed when label still uses the param"
        );
    }
}
