//! Conformance rules for orb migration tooling.
//!
//! A `ConformanceRule` describes a single breaking change that occurred between
//! orb versions and how to detect and remediate it in a consumer's CI configuration.
//! Rules are generated automatically by the `OrbDiffer` and embedded in the generated
//! MCP server binary at release time — the orb author never writes them manually.

use serde::{Deserialize, Serialize};

/// A single breaking-change rule embedded in the generated MCP server.
///
/// Describes something that is no longer valid in the target orb version,
/// along with how to remediate it in a consumer config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum ConformanceRule {
    /// A job was completely removed with no replacement.
    JobRemoved {
        /// The job name that was removed.
        name: String,
        /// The version in which this job was removed.
        since_version: String,
    },

    /// A job was renamed; the old name no longer exists.
    JobRenamed {
        /// The old job name.
        from: String,
        /// The new job name.
        to: String,
        /// Parameters that exist on `from` but not on `to`; strip them during migration.
        removed_parameters: Vec<String>,
        /// The version in which this rename occurred.
        since_version: String,
    },

    /// A parameter was removed from a job.
    ParameterRemoved {
        /// The job from which the parameter was removed.
        job: String,
        /// The parameter name that was removed.
        parameter: String,
        /// The version in which this parameter was removed.
        since_version: String,
    },

    /// A job's functionality was absorbed into another job.
    ///
    /// Any invocation of `absorbed` whose requires-chain includes `into`
    /// is redundant and should be removed.
    JobAbsorbed {
        /// The job that was absorbed (removed).
        absorbed: String,
        /// The job that absorbed it.
        into: String,
        /// The version in which this absorption occurred.
        since_version: String,
    },

    /// An enum parameter lost one of its allowed values.
    ParameterEnumValueRemoved {
        /// The job whose parameter lost a value.
        job: String,
        /// The parameter name.
        parameter: String,
        /// The value that was removed.
        removed_value: String,
        /// The value to substitute when the removed value is encountered.
        fallback_value: String,
        /// The version in which this value was removed.
        since_version: String,
    },
}

impl ConformanceRule {
    /// Returns the version in which this breaking change was introduced.
    pub fn since_version(&self) -> &str {
        match self {
            ConformanceRule::JobRemoved { since_version, .. } => since_version,
            ConformanceRule::JobRenamed { since_version, .. } => since_version,
            ConformanceRule::ParameterRemoved { since_version, .. } => since_version,
            ConformanceRule::JobAbsorbed { since_version, .. } => since_version,
            ConformanceRule::ParameterEnumValueRemoved { since_version, .. } => since_version,
        }
    }

    /// Returns a human-readable description of this rule.
    pub fn description(&self) -> String {
        match self {
            ConformanceRule::JobRemoved {
                name,
                since_version,
            } => {
                format!("Job `{name}` was removed in {since_version} with no replacement")
            }
            ConformanceRule::JobRenamed {
                from,
                to,
                since_version,
                ..
            } => {
                format!("Job `{from}` was renamed to `{to}` in {since_version}")
            }
            ConformanceRule::ParameterRemoved {
                job,
                parameter,
                since_version,
            } => {
                format!("Parameter `{parameter}` was removed from job `{job}` in {since_version}")
            }
            ConformanceRule::JobAbsorbed {
                absorbed,
                into,
                since_version,
            } => {
                format!(
                    "Job `{absorbed}` was absorbed into `{into}` in {since_version}; \
                     invocations of `{absorbed}` that require `{into}` are redundant"
                )
            }
            ConformanceRule::ParameterEnumValueRemoved {
                job,
                parameter,
                removed_value,
                fallback_value,
                since_version,
            } => {
                format!(
                    "Value `{removed_value}` was removed from parameter `{parameter}` \
                     of job `{job}` in {since_version}; use `{fallback_value}` instead"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_removed_round_trip() {
        let rule = ConformanceRule::JobRemoved {
            name: "choose_pipeline".to_string(),
            since_version: "5.0.0".to_string(),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let back: ConformanceRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, back);
    }

    #[test]
    fn test_job_renamed_since_version() {
        let rule = ConformanceRule::JobRenamed {
            from: "idiomatic_rust".to_string(),
            to: "idiomatic_rust_rolling".to_string(),
            removed_parameters: vec!["min_rust_version".to_string()],
            since_version: "5.0.0".to_string(),
        };
        assert_eq!(rule.since_version(), "5.0.0");
    }

    #[test]
    fn test_all_rules_serialize() {
        let rules = vec![
            ConformanceRule::JobRemoved {
                name: "choose_pipeline".to_string(),
                since_version: "5.0.0".to_string(),
            },
            ConformanceRule::JobRenamed {
                from: "idiomatic_rust".to_string(),
                to: "idiomatic_rust_rolling".to_string(),
                removed_parameters: vec!["min_rust_version".to_string()],
                since_version: "5.0.0".to_string(),
            },
            ConformanceRule::ParameterRemoved {
                job: "update_prlog".to_string(),
                parameter: "min_rust_version".to_string(),
                since_version: "5.0.0".to_string(),
            },
            ConformanceRule::JobAbsorbed {
                absorbed: "label".to_string(),
                into: "update_prlog".to_string(),
                since_version: "5.0.0".to_string(),
            },
            ConformanceRule::ParameterEnumValueRemoved {
                job: "update_changelog".to_string(),
                parameter: "update_log_option".to_string(),
                removed_value: "pipeline".to_string(),
                fallback_value: "halt".to_string(),
                since_version: "5.0.0".to_string(),
            },
        ];

        let json = serde_json::to_string_pretty(&rules).unwrap();
        let back: Vec<ConformanceRule> = serde_json::from_str(&json).unwrap();
        assert_eq!(rules, back);
    }

    #[test]
    fn test_descriptions_are_non_empty() {
        let rules = vec![
            ConformanceRule::JobRemoved {
                name: "choose_pipeline".to_string(),
                since_version: "5.0.0".to_string(),
            },
            ConformanceRule::JobAbsorbed {
                absorbed: "label".to_string(),
                into: "update_prlog".to_string(),
                since_version: "5.0.0".to_string(),
            },
        ];
        for rule in &rules {
            assert!(!rule.description().is_empty());
        }
    }
}
