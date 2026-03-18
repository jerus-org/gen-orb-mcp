//! Types for migration planning and application.

use std::path::PathBuf;

/// A complete migration plan for a consumer repo.
#[derive(Debug, Clone, Default)]
pub struct MigrationPlan {
    /// The orb alias as used in the consumer's config (e.g. `"toolkit"`).
    pub orb: String,
    /// The target orb version to migrate to (e.g. `"5.0.0"`).
    pub target_version: String,
    /// The detected current version in the consumer's config, or `"unknown"`.
    pub detected_version: String,
    /// Individual changes to apply, in the order they should be applied.
    pub changes: Vec<PlannedChange>,
}

/// A single targeted change to a consumer's CI file.
#[derive(Debug, Clone)]
pub struct PlannedChange {
    /// Path to the file to modify.
    pub file: PathBuf,
    /// Human-readable description of the change.
    pub description: String,
    /// Structured change type for the applicator.
    pub change_type: ChangeType,
    /// Human-readable representation of the content before the change.
    pub before: String,
    /// Human-readable representation of the content after the change.
    pub after: String,
}

/// The type of change to apply to a CI file.
#[derive(Debug, Clone)]
pub enum ChangeType {
    /// Remove an entire job invocation from a workflow.
    RemoveJobInvocation {
        workflow: String,
        /// The effective name (name_override or reference) of the job to remove.
        job_ref: String,
    },
    /// Rename a job invocation's reference within a workflow.
    RenameJobInvocation {
        workflow: String,
        from: String,
        to: String,
    },
    /// Remove a parameter from a job invocation.
    RemoveParameter {
        workflow: String,
        job_ref: String,
        parameter: String,
    },
    /// Replace a parameter's value in a job invocation.
    ReplaceParameterValue {
        workflow: String,
        job_ref: String,
        parameter: String,
        replacement: String,
    },
}

/// Summary of changes that were actually applied to disk.
#[derive(Debug, Clone, Default)]
pub struct AppliedChanges {
    /// Number of files modified.
    pub files_modified: usize,
    /// Number of individual changes applied.
    pub changes_applied: usize,
    /// Any changes that were skipped (e.g. already in the target state).
    pub skipped: Vec<String>,
}

impl AppliedChanges {
    /// Returns a human-readable summary.
    pub fn format_summary(&self) -> String {
        if self.changes_applied == 0 {
            return "No changes applied — config already conforms.".to_string();
        }
        let mut lines = vec![format!(
            "Applied {} change(s) across {} file(s).",
            self.changes_applied, self.files_modified
        )];
        if !self.skipped.is_empty() {
            lines.push(format!("Skipped {} change(s):", self.skipped.len()));
            for s in &self.skipped {
                lines.push(format!("  • {}", s));
            }
        }
        lines.join("\n")
    }
}
