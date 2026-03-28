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

use std::{collections::HashMap, path::Path};

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
        if apply_single_change(&mut result, change) {
            applied_count += 1;
        }
    }

    (result, applied_count)
}

/// Dispatches a single planned change to the appropriate applicator function.
fn apply_single_change(lines: &mut Vec<String>, change: &PlannedChange) -> bool {
    match &change.change_type {
        ChangeType::RemoveJobInvocation { workflow, job_ref } => {
            remove_job_invocation(lines, workflow, job_ref)
        }
        ChangeType::RenameJobInvocation { workflow, from, to } => {
            rename_job_invocation(lines, workflow, from, to)
        }
        ChangeType::RemoveParameter {
            workflow,
            job_ref,
            parameter,
        } => remove_parameter(lines, workflow, job_ref, parameter),
        ChangeType::ReplaceParameterValue {
            workflow,
            job_ref,
            parameter,
            replacement,
        } => replace_parameter_value(lines, workflow, job_ref, parameter, replacement),
        ChangeType::RemoveCommandInvocation { job, command_ref } => {
            remove_command_invocation(lines, job, command_ref)
        }
        ChangeType::RenameCommandInvocation { job, from, to } => {
            rename_command_invocation(lines, job, from, to)
        }
        ChangeType::RemoveCommandParameter {
            job,
            command_ref,
            parameter,
        } => remove_command_parameter(lines, job, command_ref, parameter),
        ChangeType::RemovePipelineParameter { parameter } => {
            remove_pipeline_parameter(lines, parameter)
        }
        ChangeType::UpdateOrbVersion {
            orb_alias,
            from_version,
            to_version,
        } => update_orb_version(lines, orb_alias, from_version, to_version),
        ChangeType::UpdateRequiresEntry {
            workflow,
            job_ref,
            old_req,
            new_req,
        } => update_requires_entry(lines, workflow, job_ref, old_req, new_req),
        ChangeType::RemoveRequiresEntry {
            workflow,
            job_ref,
            entry_name,
        } => remove_requires_entry(lines, workflow, job_ref, entry_name),
        ChangeType::RenameParameter {
            workflow,
            job_ref,
            from,
            to,
        } => rename_parameter(lines, workflow, job_ref, from, to),
    }
}

/// Removes a job invocation block from the YAML lines.
///
/// Finds the line containing the job reference (e.g. `- toolkit/label:`) within
/// the given workflow section and removes it along with all its indented
/// children.
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

    // For a scalar value (`key: value`), remove only that one line.
    // For a block value (`key:` followed by indented children), remove the
    // key line plus all its indented children.
    let param_end = if lines[param_idx].trim_end().ends_with(':') {
        let param_indent = leading_spaces(&lines[param_idx]);
        find_param_value_end(lines, param_idx + 1, param_indent)
    } else {
        param_idx + 1
    };
    lines.drain(param_idx..param_end);

    // Fix #93: if the job line ends with `:` and now has no children,
    // strip the trailing colon to produce valid YAML.
    if lines[job_start].trim_end().ends_with(':')
        && !has_children_after(lines, job_start, job_indent)
    {
        let trimmed = lines[job_start].trim_end_matches(':').to_string();
        lines[job_start] = trimmed;
    }

    true
}

/// Returns `true` if any non-blank line after `after` has indentation greater
/// than `parent_indent`.
fn has_children_after(lines: &[String], after: usize, parent_indent: usize) -> bool {
    for line in lines.iter().skip(after + 1) {
        if line.trim().is_empty() {
            continue;
        }
        return leading_spaces(line) > parent_indent;
    }
    false
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

    // Replace inline: `          parameter: old_value` → `          parameter:
    // new_value`
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

/// Removes an orb command step invocation from a consumer's custom job.
fn remove_command_invocation(lines: &mut Vec<String>, job: &str, command_ref: &str) -> bool {
    let Some(steps_start) = find_custom_job_section(lines, job) else {
        return false;
    };
    let Some(step_start) = find_step_line(lines, steps_start, command_ref) else {
        return false;
    };
    let step_indent = leading_spaces(&lines[step_start]);
    let step_end = find_block_end(lines, step_start + 1, step_indent);
    lines.drain(step_start..step_end);
    true
}

/// Renames an orb command step invocation in a consumer's custom job.
fn rename_command_invocation(lines: &mut [String], job: &str, from: &str, to: &str) -> bool {
    let Some(steps_start) = find_custom_job_section(lines, job) else {
        return false;
    };
    let Some(step_start) = find_step_line(lines, steps_start, from) else {
        return false;
    };
    if lines[step_start].contains(from) {
        lines[step_start] = lines[step_start].replacen(from, to, 1);
        return true;
    }
    false
}

/// Removes a parameter from an orb command step invocation in a consumer's
/// custom job.
fn remove_command_parameter(
    lines: &mut Vec<String>,
    job: &str,
    command_ref: &str,
    parameter: &str,
) -> bool {
    let Some(steps_start) = find_custom_job_section(lines, job) else {
        return false;
    };
    let Some(step_start) = find_step_line(lines, steps_start, command_ref) else {
        return false;
    };
    let step_indent = leading_spaces(&lines[step_start]);
    let step_end = find_block_end(lines, step_start + 1, step_indent);
    let Some(param_idx) = find_param_line(lines, step_start, step_end, parameter) else {
        return false;
    };
    let param_end = if lines[param_idx].trim_end().ends_with(':') {
        let param_indent = leading_spaces(&lines[param_idx]);
        find_param_value_end(lines, param_idx + 1, param_indent)
    } else {
        param_idx + 1
    };
    lines.drain(param_idx..param_end);
    true
}

/// Removes a parameter declaration from the top-level `parameters:` block.
///
/// Finds `parameters:` at indent 0, then finds `  <parameter>:` (indent 2)
/// within the block, and removes it along with its indented children.
/// Updates the orb version pin in the `orbs:` section.
///
/// Finds lines of the form `  <orb_alias>: ...@<from_version>` and replaces
/// `@<from_version>` with `@<to_version>`.
fn update_orb_version(
    lines: &mut [String],
    orb_alias: &str,
    from_version: &str,
    to_version: &str,
) -> bool {
    let old_suffix = format!("@{from_version}");
    let new_suffix = format!("@{to_version}");
    let prefix = format!("{orb_alias}:");
    let mut changed = false;
    for line in lines.iter_mut() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(&prefix) && line.contains(&old_suffix) {
            *line = line.replace(&old_suffix, &new_suffix);
            changed = true;
        }
    }
    changed
}

/// Updates a `requires:` entry in a job invocation from `old_req` to `new_req`.
fn update_requires_entry(
    lines: &mut [String],
    workflow: &str,
    job_ref: &str,
    old_req: &str,
    new_req: &str,
) -> bool {
    let Some(idx) = find_requires_entry_idx(lines, workflow, job_ref, old_req) else {
        return false;
    };
    let entry_indent = leading_spaces(&lines[idx]);
    lines[idx] = format!("{}- {new_req}", " ".repeat(entry_indent));
    true
}

/// Removes a specific entry from a job invocation's `requires:` list.
fn remove_requires_entry(
    lines: &mut Vec<String>,
    workflow: &str,
    job_ref: &str,
    entry_name: &str,
) -> bool {
    let Some(idx) = find_requires_entry_idx(lines, workflow, job_ref, entry_name) else {
        return false;
    };
    lines.remove(idx);
    true
}

/// Finds the line index of a specific entry inside a job invocation's
/// `requires:` list.
///
/// Returns `None` if the workflow, job, `requires:` block, or entry is not
/// found.
fn find_requires_entry_idx(
    lines: &[String],
    workflow: &str,
    job_ref: &str,
    entry: &str,
) -> Option<usize> {
    let workflow_start = find_workflow_section(lines, workflow)?;
    let job_start = find_job_line(lines, workflow_start, job_ref)?;
    let job_indent = leading_spaces(&lines[job_start]);
    let job_end = find_block_end(lines, job_start + 1, job_indent);
    let requires_idx = find_requires_block(lines, job_start, job_end)?;
    let requires_indent = leading_spaces(&lines[requires_idx]);
    let target = format!("- {entry}");
    let mut i = requires_idx + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if leading_spaces(line) <= requires_indent {
            break;
        }
        if line.trim() == target {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Finds the index of the `requires:` line within a job's parameter block.
fn find_requires_block(lines: &[String], job_start: usize, job_end: usize) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .skip(job_start + 1)
        .take(job_end - job_start - 1)
        .find(|(_, l)| l.trim() == "requires:")
        .map(|(i, _)| i)
}

/// Renames a parameter key in a job invocation, preserving its value.
fn rename_parameter(
    lines: &mut [String],
    workflow: &str,
    job_ref: &str,
    from: &str,
    to: &str,
) -> bool {
    let Some(workflow_start) = find_workflow_section(lines, workflow) else {
        return false;
    };
    let Some(job_start) = find_job_line(lines, workflow_start, job_ref) else {
        return false;
    };
    let job_indent = leading_spaces(&lines[job_start]);
    let job_end = find_block_end(lines, job_start + 1, job_indent);

    let Some(param_idx) = find_param_line(lines, job_start, job_end, from) else {
        return false;
    };

    // Replace the parameter key prefix, preserving the rest of the line
    // (everything from the colon onwards).
    //
    // Line looks like: `          from: value`
    // We want:         `          to: value`
    let line = &lines[param_idx];
    let from_prefix = format!("{from}:");
    if let Some(colon_pos) = line.find(&from_prefix) {
        let indent = &line[..colon_pos];
        let rest = &line[colon_pos + from_prefix.len()..]; // everything after `from:`
        lines[param_idx] = format!("{indent}{to}:{rest}");
        return true;
    }
    false
}

fn remove_pipeline_parameter(lines: &mut Vec<String>, parameter: &str) -> bool {
    // Find top-level `parameters:` (zero indentation)
    let Some(params_line) = lines
        .iter()
        .position(|l| l.trim_end() == "parameters:" && !l.starts_with(' '))
    else {
        return false;
    };

    // Search within the parameters block for `  <parameter>:` (indent 2)
    let mut i = params_line + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        // Stop if we've left the parameters block (back to zero indentation)
        if !line.starts_with(' ') {
            break;
        }
        let indent = leading_spaces(line);
        let trimmed = line.trim();
        // Match only direct children of `parameters:` at indent 2
        if indent == 2
            && (trimmed == format!("{parameter}:") || trimmed.starts_with(&format!("{parameter}:")))
        {
            // Remove this line and all indented children
            let param_end = find_param_value_end(lines, i + 1, indent);
            lines.drain(i..param_end);
            return true;
        }
        i += 1;
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
            if let Some(jobs_idx) = find_jobs_line_after(lines, i + 1) {
                return Some(jobs_idx);
            }
        }
    }
    None
}

/// Finds the `jobs:` key within the block starting at `start`.
///
/// Stops searching when it encounters a non-indented line (a new top-level
/// key).
fn find_jobs_line_after(lines: &[String], start: usize) -> Option<usize> {
    for (j, jline) in lines.iter().enumerate().skip(start) {
        let t = jline.trim();
        if t == "jobs:" {
            return Some(j);
        }
        if !jline.starts_with(' ') {
            break;
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

        // Bare string or with-params form: `- toolkit/label` or `- toolkit/label:`
        if is_direct_job_match(trimmed, job_ref) {
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

/// Returns `true` if `trimmed` is a bare or map-keyed job entry for `job_ref`.
fn is_direct_job_match(trimmed: &str, job_ref: &str) -> bool {
    trimmed == format!("- {job_ref}") || trimmed == format!("- {job_ref}:")
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

/// Returns the line index one past the end of a parameter's block value.
///
/// Unlike [`find_block_end`], which stops only at list items (`-`) or
/// top-level keys, this stops at ANY non-empty line whose indentation is
/// ≤ `param_indent`. That correctly handles sibling parameters in a job
/// invocation, which are at the same indent level as the parameter being
/// removed but are not list-item markers.
fn find_param_value_end(lines: &[String], start: usize, param_indent: usize) -> usize {
    for (i, line) in lines.iter().enumerate().skip(start) {
        if line.trim().is_empty() {
            continue;
        }
        if leading_spaces(line) <= param_indent {
            return i;
        }
    }
    lines.len()
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

/// Returns the line index of the `steps:` key inside the named consumer custom
/// job.
///
/// Scans for the top-level `jobs:` key, then finds `  job_name:` beneath it,
/// then finds `    steps:` within that job block.
fn find_custom_job_section(lines: &[String], job_name: &str) -> Option<usize> {
    // Find top-level `jobs:` (zero indentation)
    let jobs_line = lines
        .iter()
        .position(|l| l.trim_end() == "jobs:" && !l.starts_with(' '))?;

    let job_marker = format!("{job_name}:");

    // Find `  job_name:` (exactly two leading spaces) within the jobs block
    let mut i = jobs_line + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        // Stop if we've returned to zero indentation (next top-level key)
        if !line.starts_with(' ') {
            break;
        }
        let trimmed = line.trim();
        if trimmed == job_marker || trimmed.starts_with(&format!("{job_name}: ")) {
            // Found the job — now look for `steps:` within it
            let job_indent = leading_spaces(line);
            return find_steps_in_job_block(lines, i, job_indent);
        }
        i += 1;
    }
    None
}

/// Finds the `steps:` key within a consumer custom job block.
///
/// Scans forward from `job_start + 1`, stopping when the indentation returns
/// to `job_indent` or below (i.e. the job block ends).
fn find_steps_in_job_block(lines: &[String], job_start: usize, job_indent: usize) -> Option<usize> {
    let mut j = job_start + 1;
    while j < lines.len() {
        let jline = &lines[j];
        if jline.trim().is_empty() {
            j += 1;
            continue;
        }
        let jindent = leading_spaces(jline);
        if jindent <= job_indent && !jline.trim().is_empty() {
            break;
        }
        if jline.trim() == "steps:" {
            return Some(j);
        }
        j += 1;
    }
    None
}

/// Finds the line index of a step entry within a job's `steps:` list.
///
/// Matches:
/// - `      - toolkit/cmd` (bare string form)
/// - `      - toolkit/cmd:` (map form with parameters)
fn find_step_line(lines: &[String], steps_start: usize, command_ref: &str) -> Option<usize> {
    let steps_indent = leading_spaces(&lines[steps_start]);
    let entry_indent = steps_indent + 2; // `- ` entries indented under `steps:`

    let mut i = steps_start + 1;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        let indent = leading_spaces(line);
        // Stop if we've left the steps section
        if indent < entry_indent && !line.trim().is_empty() {
            break;
        }
        let trimmed = line.trim();
        if trimmed == format!("- {command_ref}") || trimmed == format!("- {command_ref}:") {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::migrator::types::{ChangeType, PlannedChange};

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

    // A more realistic sample where the removed parameter is NOT the last one —
    // there are sibling parameters after it in the same job invocation.
    const SAMPLE_WITH_TRAILING_PARAMS: &str = r#"version: 2.1

orbs:
  toolkit: jerus-org/circleci-toolkit@4.7.1

workflows:
  update_prlog:
    jobs:
      - toolkit/update_prlog:
          name: update-prlog-on-main
          context:
            - release
            - bot-check
            - pcu-app
          min_rust_version: "1.82"
          target_branch: "main"
          pcu_from_merge: --from-merge
          update_pcu: false
          pcu_verbosity: "-vvv"
      - toolkit/label:
          min_rust_version: "1.82"
          context: pcu-app
          requires:
            - update-prlog-on-main"#;

    #[test]
    fn test_remove_parameter_preserves_sibling_params() {
        // Removing a non-last scalar parameter must NOT drain subsequent siblings.
        let lines: Vec<&str> = SAMPLE_WITH_TRAILING_PARAMS.lines().collect();
        let change =
            remove_param_change("update_prlog", "update-prlog-on-main", "min_rust_version");
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        // Sibling parameters of update_prlog must survive
        assert!(
            output.contains("target_branch:"),
            "target_branch should remain after min_rust_version removal"
        );
        assert!(
            output.contains("pcu_from_merge:"),
            "pcu_from_merge should remain after min_rust_version removal"
        );
        assert!(
            output.contains("update_pcu:"),
            "update_pcu should remain after min_rust_version removal"
        );
        assert!(
            output.contains("pcu_verbosity:"),
            "pcu_verbosity should remain after min_rust_version removal"
        );
        // The label job must survive intact
        assert!(
            output.contains("toolkit/label"),
            "toolkit/label should remain"
        );
        // update_prlog invocation must still be present
        assert!(
            output.contains("toolkit/update_prlog"),
            "toolkit/update_prlog should remain"
        );
    }

    #[test]
    fn test_remove_block_parameter_preserves_siblings() {
        // A block-valued parameter (context: with list children) should be
        // fully removed but must not consume the sibling that follows.
        const BLOCK_SAMPLE: &str = r#"version: 2.1
orbs:
  toolkit: jerus-org/circleci-toolkit@4.8.0
workflows:
  update_prlog:
    jobs:
      - toolkit/update_prlog:
          name: update-prlog-on-main
          some_block_param:
            - item1
            - item2
          target_branch: "main"
      - toolkit/label:
          context: [pcu-app]"#;
        let lines: Vec<&str> = BLOCK_SAMPLE.lines().collect();
        let change =
            remove_param_change("update_prlog", "update-prlog-on-main", "some_block_param");
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            !output.contains("some_block_param"),
            "some_block_param should be removed"
        );
        assert!(!output.contains("item1"), "list children should be removed");
        assert!(
            output.contains("target_branch:"),
            "sibling target_branch should remain"
        );
    }

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

    const SAMPLE_WITH_JOBS: &str = r#"version: 2.1

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
      - my-release-job"#;

    fn remove_cmd_change(job: &str, command_ref: &str) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RemoveCommandInvocation {
                job: job.to_string(),
                command_ref: command_ref.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    fn rename_cmd_change(job: &str, from: &str, to: &str) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RenameCommandInvocation {
                job: job.to_string(),
                from: from.to_string(),
                to: to.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    fn remove_cmd_param_change(job: &str, command_ref: &str, parameter: &str) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RemoveCommandParameter {
                job: job.to_string(),
                command_ref: command_ref.to_string(),
                parameter: parameter.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    #[test]
    fn test_remove_command_invocation() {
        let lines: Vec<&str> = SAMPLE_WITH_JOBS.lines().collect();
        let change = remove_cmd_change("my-release-job", "toolkit/setup_env");
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            !output.contains("toolkit/setup_env"),
            "setup_env should be removed"
        );
        assert!(
            !output.contains("token:"),
            "token param should be removed with the step"
        );
        assert!(
            output.contains("toolkit/publish_crate"),
            "publish_crate should remain"
        );
        assert!(output.contains("checkout"), "checkout should remain");
    }

    #[test]
    fn test_rename_command_invocation() {
        let lines: Vec<&str> = SAMPLE_WITH_JOBS.lines().collect();
        let change = rename_cmd_change(
            "my-release-job",
            "toolkit/setup_env",
            "toolkit/configure_env",
        );
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            output.contains("toolkit/configure_env"),
            "should be renamed"
        );
        assert!(
            !output.contains("toolkit/setup_env"),
            "old name should be gone"
        );
        assert!(output.contains("token:"), "params should be preserved");
    }

    #[test]
    fn test_remove_command_parameter() {
        let lines: Vec<&str> = SAMPLE_WITH_JOBS.lines().collect();
        let change = remove_cmd_param_change("my-release-job", "toolkit/publish_crate", "package");
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            !output.contains("package:"),
            "package param should be removed"
        );
        assert!(
            output.contains("toolkit/publish_crate"),
            "step header should remain"
        );
    }

    #[test]
    fn test_remove_command_invocation_noop_if_not_found() {
        let lines: Vec<&str> = SAMPLE_WITH_JOBS.lines().collect();
        let change = remove_cmd_change("my-release-job", "toolkit/nonexistent");
        let (_, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_remove_command_invocation_noop_wrong_job() {
        let lines: Vec<&str> = SAMPLE_WITH_JOBS.lines().collect();
        let change = remove_cmd_change("other-job", "toolkit/setup_env");
        let (_, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_no_change_if_not_found() {
        let lines: Vec<&str> = SAMPLE.lines().collect();
        let change = remove_change("update_prlog", "toolkit/nonexistent");
        let (_, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 0);
    }

    const SAMPLE_WITH_PIPELINE_PARAMS: &str = r#"version: 2.1
parameters:
  min_rust_version:
    type: string
    default: "1.82"
  update_pcu:
    type: boolean
    default: false
orbs:
  toolkit: jerus-org/circleci-toolkit@4.7.1
workflows:
  update_prlog:
    jobs:
      - toolkit/update_prlog:
          context: [pcu-app]
          target_branch: "main""#;

    #[test]
    fn test_remove_pipeline_parameter() {
        let lines: Vec<&str> = SAMPLE_WITH_PIPELINE_PARAMS.lines().collect();
        let change = PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RemovePipelineParameter {
                parameter: "min_rust_version".to_string(),
            },
            before: String::new(),
            after: String::new(),
        };
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            !output.contains("min_rust_version"),
            "declaration should be removed"
        );
        assert!(
            output.contains("update_pcu:"),
            "sibling param should remain"
        );
        assert!(output.contains("target_branch:"), "job params unaffected");
    }

    // ── Fix #93: Bare mapping key after last parameter removal ──────────────

    #[test]
    fn test_remove_last_parameter_strips_trailing_colon() {
        const YAML: &str = r#"version: 2.1
orbs:
  toolkit: jerus-org/circleci-toolkit@4.9.5
workflows:
  validation:
    jobs:
      - toolkit/test_doc_build:
          min_rust_version: "1.85"
      - toolkit/required_builds:
          min_rust_version: "1.85"
          other_param: value"#;
        let lines: Vec<&str> = YAML.lines().collect();
        let change =
            remove_param_change("validation", "toolkit/test_doc_build", "min_rust_version");
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            output.contains("- toolkit/test_doc_build\n"),
            "job line should have no trailing colon when last param removed, got:\n{output}"
        );
        assert!(
            output.contains("other_param: value"),
            "sibling job must be untouched"
        );
    }

    // ── Fix #94: `requires` entries updated on job rename ──────────────────

    fn update_requires_change(
        workflow: &str,
        job_ref: &str,
        old_req: &str,
        new_req: &str,
    ) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::UpdateRequiresEntry {
                workflow: workflow.to_string(),
                job_ref: job_ref.to_string(),
                old_req: old_req.to_string(),
                new_req: new_req.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    #[test]
    fn test_update_requires_entry() {
        const YAML: &str = r#"version: 2.1
orbs:
  toolkit: jerus-org/circleci-toolkit@5.3.10
workflows:
  validation:
    jobs:
      - toolkit/common_tests:
          min_rust_version: "1.85"
      - toolkit/idiomatic_rust:
          requires:
            - toolkit/common_tests_rolling
            - security with sonarcloud"#;
        let lines: Vec<&str> = YAML.lines().collect();
        let change = update_requires_change(
            "validation",
            "toolkit/idiomatic_rust",
            "toolkit/common_tests_rolling",
            "toolkit/common_tests",
        );
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            output.contains("- toolkit/common_tests\n"),
            "requires entry should be updated"
        );
        assert!(
            !output.contains("toolkit/common_tests_rolling"),
            "old requires entry should be gone"
        );
        assert!(
            output.contains("security with sonarcloud"),
            "other requires entry must remain"
        );
    }

    // ── Fix #92: Dangling requires cleanup after job removal ────────────────

    fn remove_requires_change(workflow: &str, job_ref: &str, entry_name: &str) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RemoveRequiresEntry {
                workflow: workflow.to_string(),
                job_ref: job_ref.to_string(),
                entry_name: entry_name.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    #[test]
    fn test_remove_requires_entry() {
        const YAML: &str = r#"version: 2.1
orbs:
  toolkit: digital-prstv/circleci-toolkit@5.3.10
workflows:
  execute:
    jobs:
      - toolkit/run_data_retention_rules:
          execute: true
          context: [cull-gmail]
          requires:
            - toolkit/no_unreleased_changes
            - other-job"#;
        let lines: Vec<&str> = YAML.lines().collect();
        let change = remove_requires_change(
            "execute",
            "toolkit/run_data_retention_rules",
            "toolkit/no_unreleased_changes",
        );
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            !output.contains("toolkit/no_unreleased_changes"),
            "requires entry should be removed"
        );
        assert!(
            output.contains("other-job"),
            "other requires entry must remain"
        );
        assert!(
            output.contains("toolkit/run_data_retention_rules"),
            "job itself must remain"
        );
    }

    // ── Fix #95: ParameterRenamed conformance rule ──────────────────────────

    fn rename_param_change(workflow: &str, job_ref: &str, from: &str, to: &str) -> PlannedChange {
        PlannedChange {
            file: PathBuf::from("test.yml"),
            description: "test".to_string(),
            change_type: ChangeType::RenameParameter {
                workflow: workflow.to_string(),
                job_ref: job_ref.to_string(),
                from: from.to_string(),
                to: to.to_string(),
            },
            before: String::new(),
            after: String::new(),
        }
    }

    #[test]
    fn test_rename_parameter() {
        const YAML: &str = r#"version: 2.1
orbs:
  toolkit: jerus-org/circleci-toolkit@6.0.0
workflows:
  validation:
    jobs:
      - toolkit/test_features:
          name: test-unique
          min_rust_version: "1.85"
          cargo_args: --package mockd --features unique"#;
        let lines: Vec<&str> = YAML.lines().collect();
        let change = rename_param_change(
            "validation",
            "test-unique",
            "min_rust_version",
            "rust_version",
        );
        let (new_lines, count) = apply_changes_to_lines(&lines, &[&change]);
        assert_eq!(count, 1);
        let output = new_lines.join("\n");
        assert!(
            output.contains("rust_version: \"1.85\""),
            "parameter should be renamed with value preserved"
        );
        assert!(
            !output.contains("min_rust_version"),
            "old parameter name should be gone"
        );
        assert!(
            output.contains("cargo_args:"),
            "other params should be untouched"
        );
    }
}
