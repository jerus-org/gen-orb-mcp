//! In-place YAML applicator.
//!
//! Applies a `MigrationPlan` to files on disk by making targeted edits that
//! preserve comments, ordering, and indentation as much as possible.
//!
//! ## Strategy
//!
//! Rather than round-tripping through `serde_yaml` (which destroys comments and
//! reorders keys), the applicator uses a line-oriented approach:
//!
//! 1. Read the file as lines.
//! 2. For each change in the plan, locate the target section and apply the
//!    minimal edit (remove lines, replace a value, rename a key).
//! 3. Write the modified lines back.
//!
//! This preserves comments, anchors, and formatting for unchanged sections.

use std::collections::HashMap;
use std::path::Path;

use crate::migrator::types::{AppliedChanges, ChangeType, MigrationPlan, PlannedChange};

/// Applies the migration plan to files on disk.
///
/// When `dry_run` is `true`, the files are not modified; the returned
/// `AppliedChanges` still counts what would have changed.
pub fn apply(plan: &MigrationPlan, dry_run: bool) -> std::io::Result<AppliedChanges> {
    let mut applied = AppliedChanges::default();

    // Group changes by file
    let mut by_file: HashMap<std::path::PathBuf, Vec<&PlannedChange>> = HashMap::new();
    for change in &plan.changes {
        by_file.entry(change.file.clone()).or_default().push(change);
    }

    for (file_path, changes) in &by_file {
        match apply_to_file(file_path, changes, dry_run) {
            Ok(count) if count > 0 => {
                applied.changes_applied += count;
                applied.files_modified += 1;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    path = %file_path.display(),
                    error = %e,
                    "Failed to apply changes to file"
                );
                applied
                    .skipped
                    .push(format!("{}: {}", file_path.display(), e));
            }
        }
    }

    Ok(applied)
}

/// Applies changes to a single file. Returns the number of changes applied.
fn apply_to_file(path: &Path, changes: &[&PlannedChange], dry_run: bool) -> std::io::Result<usize> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    let (new_lines, count) = apply_changes_to_lines(&lines, changes);

    if count > 0 && !dry_run {
        let new_content = new_lines.join("\n") + if content.ends_with('\n') { "\n" } else { "" };
        std::fs::write(path, new_content)?;
    }

    Ok(count)
}

/// Applies a slice of planned changes to a slice of lines.
///
/// Returns the modified lines and the count of changes actually applied.
pub fn apply_changes_to_lines(lines: &[&str], changes: &[&PlannedChange]) -> (Vec<String>, usize) {
    let mut result: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let mut applied_count = 0;

    for change in changes {
        match &change.change_type {
            ChangeType::RemoveJobInvocation { workflow, job_ref } => {
                if remove_job_invocation(&mut result, workflow, job_ref) {
                    applied_count += 1;
                }
            }
            ChangeType::RenameJobInvocation { workflow, from, to } => {
                if rename_job_invocation(&mut result, workflow, from, to) {
                    applied_count += 1;
                }
            }
            ChangeType::RemoveParameter {
                workflow,
                job_ref,
                parameter,
            } => {
                if remove_parameter(&mut result, workflow, job_ref, parameter) {
                    applied_count += 1;
                }
            }
            ChangeType::ReplaceParameterValue {
                workflow,
                job_ref,
                parameter,
                replacement,
            } => {
                if replace_parameter_value(&mut result, workflow, job_ref, parameter, replacement) {
                    applied_count += 1;
                }
            }
        }
    }

    (result, applied_count)
}

/// Removes a job invocation block from the YAML lines.
///
/// Finds the line containing the job reference (e.g. `- toolkit/label:`) within
/// the given workflow section and removes it along with all its indented children.
fn remove_job_invocation(lines: &mut Vec<String>, workflow: &str, job_ref: &str) -> bool {
    let Some(workflow_start) = find_workflow_section(lines, workflow) else {
        return false;
    };

    // Locate the job entry: either `      - toolkit/label` (bare) or
    // `      - toolkit/label:` (with params) or `      - name: toolkit/label`
    let job_line = find_job_line(lines, workflow_start, job_ref);
    let Some(job_start) = job_line else {
        return false;
    };

    // Determine the indentation of the `- ` marker on that line
    let job_indent = leading_spaces(&lines[job_start]);

    // Remove the job entry and all its indented children
    let job_end = find_block_end(lines, job_start + 1, job_indent);
    lines.drain(job_start..job_end);
    true
}

/// Renames a job reference in the YAML lines.
fn rename_job_invocation(lines: &mut [String], workflow: &str, from: &str, to: &str) -> bool {
    let Some(workflow_start) = find_workflow_section(lines, workflow) else {
        return false;
    };

    let Some(job_start) = find_job_line(lines, workflow_start, from) else {
        return false;
    };

    // Replace the job reference on that line
    if lines[job_start].contains(from) {
        lines[job_start] = lines[job_start].replacen(from, to, 1);
        return true;
    }
    false
}

/// Removes a parameter key-value pair from a job invocation block.
fn remove_parameter(
    lines: &mut Vec<String>,
    workflow: &str,
    job_ref: &str,
    parameter: &str,
) -> bool {
    let Some(workflow_start) = find_workflow_section(lines, workflow) else {
        return false;
    };

    let Some(job_start) = find_job_line(lines, workflow_start, job_ref) else {
        return false;
    };

    let job_indent = leading_spaces(&lines[job_start]);
    let job_end = find_block_end(lines, job_start + 1, job_indent);
    let param_line = find_param_line(lines, job_start, job_end, parameter);

    let Some(param_idx) = param_line else {
        return false;
    };

    let param_indent = leading_spaces(&lines[param_idx]);
    let param_end = find_block_end(lines, param_idx + 1, param_indent);
    lines.drain(param_idx..param_end);
    true
}

/// Replaces the value of a parameter in a job invocation.
fn replace_parameter_value(
    lines: &mut Vec<String>,
    workflow: &str,
    job_ref: &str,
    parameter: &str,
    replacement: &str,
) -> bool {
    let Some(workflow_start) = find_workflow_section(lines, workflow) else {
        return false;
    };

    let Some(job_start) = find_job_line(lines, workflow_start, job_ref) else {
        return false;
    };

    let job_indent = leading_spaces(&lines[job_start]);
    let job_end = find_block_end(lines, job_start + 1, job_indent);

    let Some(param_idx) = find_param_line(lines, job_start, job_end, parameter) else {
        return false;
    };

    // Replace inline: `          parameter: old_value` → `          parameter: new_value`
    let line = &lines[param_idx];
    if let Some(colon_pos) = line.find(": ") {
        let indent_and_key = &line[..colon_pos + 2];
        lines[param_idx] = format!("{indent_and_key}{replacement}");
        return true;
    } else if line.trim_end().ends_with(':') {
        // Multi-line value — replace the entire key line with inline value
        let key_part = line.trim_end().to_string();
        lines[param_idx] = format!("{key_part} {replacement}");
        // Remove following indented value lines
        let param_indent = leading_spaces(&lines[param_idx]);
        let param_end = find_block_end(lines, param_idx + 1, param_indent);
        lines.drain((param_idx + 1)..param_end);
        return true;
    }
    false
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Returns the line index where the given workflow's `jobs:` section begins.
fn find_workflow_section(lines: &[String], workflow: &str) -> Option<usize> {
    // Find `  workflow_name:` then `    jobs:`
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == format!("{workflow}:") || trimmed.starts_with(&format!("{workflow}:")) {
            // Look for jobs: within the next few lines
            for (j, jline) in lines.iter().enumerate().skip(i + 1) {
                let t = jline.trim();
                if t == "jobs:" {
                    return Some(j);
                }
                // Stop if we hit another top-level or workflow-level key
                if !jline.starts_with(' ') && j > i {
                    break;
                }
            }
        }
    }
    None
}

/// Finds the line index of a job entry within a workflow's jobs list.
///
/// Matches the following patterns:
/// - `- job_ref` (bare string)
/// - `- job_ref:` (map with params, job ref is key)
/// - A job block whose `name:` child equals `job_ref`
fn find_job_line(lines: &[String], jobs_section: usize, job_ref: &str) -> Option<usize> {
    let jobs_indent = leading_spaces(&lines[jobs_section]);
    let entry_indent = jobs_indent + 2; // `- ` entries are indented under `jobs:`

    let mut i = jobs_section + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        let indent = leading_spaces(line);

        // Stop if we've left the jobs section (dedented past entry level)
        if indent < entry_indent && !line.trim().is_empty() {
            break;
        }

        let trimmed = line.trim();

        // Bare string: `      - toolkit/label`
        if trimmed == format!("- {job_ref}") {
            return Some(i);
        }
        // With params: `      - toolkit/label:`
        if trimmed == format!("- {job_ref}:") {
            return Some(i);
        }

        // Job block: `      - some/job:` where a child line has `name: job_ref`
        if trimmed.starts_with("- ") && trimmed.ends_with(':') {
            let (found, next_i) = scan_job_block_for_name(lines, i, indent, job_ref);
            if found {
                return Some(i);
            }
            i = next_i;
            continue;
        }

        i += 1;
    }
    None
}

/// Scans the child lines of a job block looking for `name: <job_ref>`.
///
/// Returns `(found, next_i)` where `next_i` is the index of the first line
/// that belongs to the next block (for advancing the outer loop).
fn scan_job_block_for_name(
    lines: &[String],
    block_start: usize,
    block_indent: usize,
    job_ref: &str,
) -> (bool, usize) {
    let name_pattern = format!("name: {job_ref}");
    let mut j = block_start + 1;
    while j < lines.len() {
        let child = &lines[j];
        if child.trim().is_empty() {
            j += 1;
            continue;
        }
        if leading_spaces(child) <= block_indent {
            break; // left the block
        }
        if child.trim() == name_pattern {
            return (true, j);
        }
        j += 1;
    }
    (false, j)
}

/// Finds the line index of a parameter within a job's parameter block.
fn find_param_line(lines: &[String], start: usize, end: usize, parameter: &str) -> Option<usize> {
    for (i, line) in lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .take(end - start - 1)
    {
        if line.trim().starts_with(&format!("{parameter}:")) {
            return Some(i);
        }
    }
    None
}

/// Returns the line index one past the end of the block starting at `start`
/// whose parent indentation level is `parent_indent`.
fn find_block_end(lines: &[String], start: usize, parent_indent: usize) -> usize {
    for (i, line) in lines.iter().enumerate().skip(start) {
        if line.trim().is_empty() {
            continue; // skip blank lines
        }
        let indent = leading_spaces(line);
        if indent <= parent_indent && (line.trim().starts_with('-') || !line.starts_with(' ')) {
            return i;
        }
    }
    lines.len()
}

/// Returns the number of leading space characters in a line.
fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrator::types::{ChangeType, PlannedChange};
    use std::path::PathBuf;

    fn remove_change(workflow: &str, job_ref: &str) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RemoveJobInvocation {
                workflow: workflow.to_string(),
                job_ref: job_ref.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    fn remove_param_change(workflow: &str, job_ref: &str, parameter: &str) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RemoveParameter {
                workflow: workflow.to_string(),
                job_ref: job_ref.to_string(),
                parameter: parameter.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    const SAMPLE: &str = r#"version: 2.1

orbs:
  toolkit: jerus-org/circleci-toolkit@4.8.0

workflows:
  update_prlog:
    jobs:
      - toolkit/update_prlog:
          name: update-prlog-on-main
          context: [release, bot-check]
          min_rust_version: "1.85"
      - toolkit/label:
          context: [pcu-app]
          requires:
            - update-prlog-on-main"#;

    #[test]
    fn test_remove_job_invocation() {
        let lines: Vec<&str> = SAMPLE.lines().collect();
        let change = remove_change("update_prlog", "toolkit/label");
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(!output.contains("toolkit/label"), "label should be removed");
        assert!(
            output.contains("toolkit/update_prlog"),
            "update_prlog should remain"
        );
    }

    #[test]
    fn test_remove_parameter() {
        let lines: Vec<&str> = SAMPLE.lines().collect();
        let change =
            remove_param_change("update_prlog", "update-prlog-on-main", "min_rust_version");
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            !output.contains("min_rust_version"),
            "min_rust_version should be removed"
        );
        assert!(output.contains("context:"), "context should remain");
    }

    #[test]
    fn test_no_change_if_not_found() {
        let lines: Vec<&str> = SAMPLE.lines().collect();
        let change = remove_change("update_prlog", "toolkit/nonexistent");
        let (_, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 0);
    }
}
