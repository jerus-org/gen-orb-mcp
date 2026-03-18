//! Human-readable formatting of migration plans.

use super::types::{ChangeType, MigrationPlan, PlannedChange};

impl MigrationPlan {
    /// Returns a human-readable summary of the migration plan.
    pub fn format_summary(&self) -> String {
        if self.changes.is_empty() {
            return format!(
                "No migration required: all files already conform to {}@{}",
                self.orb, self.target_version
            );
        }

        let mut lines = Vec::new();
        lines.push(format!(
            "Migration plan: {} → {}",
            self.orb, self.target_version
        ));
        if self.detected_version != "unknown" {
            lines.push(format!("  Detected version: {}", self.detected_version));
        }
        lines.push(format!("  Changes: {}", self.changes.len()));
        lines.push(String::new());

        // Group changes by file
        let mut by_file: std::collections::BTreeMap<String, Vec<&PlannedChange>> =
            std::collections::BTreeMap::new();
        for change in &self.changes {
            by_file
                .entry(change.file.display().to_string())
                .or_default()
                .push(change);
        }

        for (file, changes) in &by_file {
            lines.push(format!("  {}:", file));
            for change in changes {
                lines.push(format!("    • {}", change.description));
                if !change.before.is_empty() || !change.after.is_empty() {
                    lines.push(format!("      Before: {}", change.before));
                    lines.push(format!("      After:  {}", change.after));
                }
            }
        }

        lines.join("\n")
    }
}

impl PlannedChange {
    /// Returns a short one-line description of this change.
    pub fn short_description(&self) -> String {
        match &self.change_type {
            ChangeType::RemoveJobInvocation { workflow, job_ref } => {
                format!("Remove `{job_ref}` from workflow `{workflow}`")
            }
            ChangeType::RenameJobInvocation { workflow, from, to } => {
                format!("Rename `{from}` → `{to}` in workflow `{workflow}`")
            }
            ChangeType::RemoveParameter {
                workflow,
                job_ref,
                parameter,
            } => {
                format!("Remove parameter `{parameter}` from `{job_ref}` in `{workflow}`")
            }
            ChangeType::ReplaceParameterValue {
                workflow,
                job_ref,
                parameter,
                replacement,
            } => {
                format!(
                    "Replace value of `{parameter}` on `{job_ref}` with `{replacement}` in `{workflow}`"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::migrator::types::{ChangeType, MigrationPlan, PlannedChange};
    use std::path::PathBuf;

    #[test]
    fn test_format_summary_no_changes() {
        let plan = MigrationPlan {
            orb: "toolkit".to_string(),
            target_version: "5.0.0".to_string(),
            detected_version: "5.0.0".to_string(),
            changes: vec![],
        };
        let summary = plan.format_summary();
        assert!(summary.contains("No migration required"));
    }

    #[test]
    fn test_format_summary_with_changes() {
        let plan = MigrationPlan {
            orb: "toolkit".to_string(),
            target_version: "5.0.0".to_string(),
            detected_version: "4.8.0".to_string(),
            changes: vec![PlannedChange {
                file: PathBuf::from("update_prlog.yml"),
                description: "Remove label job".to_string(),
                change_type: ChangeType::RemoveJobInvocation {
                    workflow: "update_prlog".to_string(),
                    job_ref: "toolkit/label".to_string(),
                },
                before: "- toolkit/label".to_string(),
                after: String::new(),
            }],
        };
        let summary = plan.format_summary();
        assert!(summary.contains("Migration plan"));
        assert!(summary.contains("5.0.0"));
        assert!(summary.contains("update_prlog.yml"));
    }
}
