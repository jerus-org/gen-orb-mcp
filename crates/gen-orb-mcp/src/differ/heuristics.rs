//! Heuristics for detecting `JobAbsorbed` and `JobRenamed` conformance rules.
//!
//! These heuristics run on the semantic diff between two `OrbDefinition`s and
//! infer higher-level rules that cannot be detected from removal alone.

use std::collections::{HashMap, HashSet};

use crate::parser::types::{Command, Job, ParameterType};

/// Detects `JobAbsorbed` cases.
///
/// A job B is considered absorbed into job A when:
/// 1. B was removed (no longer exists in the new orb)
/// 2. A exists in the new orb and has a new boolean parameter whose name
///    matches `run_B`, `include_B`, or `B_step` (default `true` or omitted).
///
/// Returns a map of removed job name → absorbing job name.
pub fn detect_absorbed_jobs(
    removed_jobs: &HashMap<String, &Job>,
    new_jobs: &HashMap<String, &Job>,
    old_jobs: &HashMap<String, &Job>,
) -> HashMap<String, String> {
    let mut absorbed = HashMap::new();

    for removed_name in removed_jobs.keys() {
        if let Some(absorbing) = find_absorbing_job(removed_name, new_jobs, old_jobs) {
            absorbed.insert(removed_name.clone(), absorbing);
        }
    }

    absorbed
}

/// Checks whether any job in `new_jobs` has gained a boolean parameter that
/// corresponds to the given `removed_name`.
fn find_absorbing_job(
    removed_name: &str,
    new_jobs: &HashMap<String, &Job>,
    old_jobs: &HashMap<String, &Job>,
) -> Option<String> {
    let candidate_param_names = absorption_candidate_params(removed_name);

    for (job_name, new_job) in new_jobs {
        let old_params: HashSet<&str> = old_jobs
            .get(job_name.as_str())
            .map(|j| j.parameters.keys().map(|s| s.as_str()).collect())
            .unwrap_or_default();

        // Look for a NEW boolean parameter matching one of the candidate names
        for param_name in &candidate_param_names {
            if let Some(param) = new_job.parameters.get(param_name.as_str()) {
                if param.param_type == ParameterType::Boolean
                    && !old_params.contains(param_name.as_str())
                {
                    return Some(job_name.clone());
                }
            }
        }
    }

    None
}

/// Returns the set of parameter name candidates that would indicate absorption
/// of the given job name. For `"label"` this yields `["run_label",
/// "include_label", "label_step"]`.
fn absorption_candidate_params(job_name: &str) -> Vec<String> {
    vec![
        format!("run_{job_name}"),
        format!("include_{job_name}"),
        format!("{job_name}_step"),
    ]
}

/// Detects `JobRenamed` cases using git rename hints with Jaccard fallback.
///
/// Detection strategy (in priority order):
/// 1. **Git hint** — if `git_hints` contains an entry for the removed job,
///    use that mapping unconditionally.  This handles cases where the rename
///    target already existed in the old orb (e.g. `required_builds_rolling` →
///    `required_builds` in circleci-toolkit 6.0.0 where `required_builds` was
///    present in both old and new with different semantics).
/// 2. **Jaccard fallback** — for removed jobs not covered by a hint, compare
///    parameter-name sets against jobs that are *truly new* (absent from the
///    old orb).  If Jaccard similarity ≥ `threshold` (default 0.7), treat the
///    pair as a rename.
///
/// `git_hints` is a map of old job name → new job name derived from
/// `git log --diff-filter=R --name-status` between the two version tags.
///
/// Returns a map of old name → new name.
pub fn detect_renamed_jobs(
    removed_names: &HashSet<String>,
    new_jobs: &HashMap<String, &Job>,
    old_jobs: &HashMap<String, &Job>,
    threshold: f64,
    git_hints: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut renamed = HashMap::new();

    // Pass 1 — apply authoritative git hints first.
    let covered_by_hint = apply_git_hints(removed_names, new_jobs, git_hints, &mut renamed);

    // Pass 2 — Jaccard fallback for removed jobs not covered by a hint.
    // Only consider new jobs that didn't exist before (truly new, not modified).
    let added_jobs: HashMap<&str, &Job> = new_jobs
        .iter()
        .filter(|(name, _)| !old_jobs.contains_key(name.as_str()))
        .map(|(name, job)| (name.as_str(), *job))
        .collect();

    for removed_name in removed_names {
        if covered_by_hint.contains(removed_name) {
            continue;
        }
        let Some(old_job) = old_jobs.get(removed_name.as_str()) else {
            continue;
        };
        let old_params: HashSet<&str> = old_job.parameters.keys().map(|s| s.as_str()).collect();
        if let Some(new_name) = best_jaccard_match(&old_params, &added_jobs, threshold) {
            renamed.insert(removed_name.clone(), new_name.to_string());
        }
    }

    renamed
}

/// Apply git rename hints for removed jobs that have an authoritative hint.
///
/// Inserts matching entries into `renamed` and returns the set of removed job
/// names that were successfully resolved by a hint (so callers can skip them
/// in the Jaccard fallback pass).
fn apply_git_hints<'a>(
    removed_names: &'a HashSet<String>,
    new_jobs: &HashMap<String, &Job>,
    git_hints: &HashMap<String, String>,
    renamed: &mut HashMap<String, String>,
) -> HashSet<&'a String> {
    let mut covered = HashSet::new();
    for removed_name in removed_names {
        if let Some(new_name) = git_hints.get(removed_name) {
            // Only accept the hint if the new job actually exists in the new orb.
            if new_jobs.contains_key(new_name.as_str()) {
                renamed.insert(removed_name.clone(), new_name.clone());
                covered.insert(removed_name);
            }
        }
    }
    covered
}

/// Find the best Jaccard similarity match for `old_params` among `added_jobs`.
///
/// Returns the job name of the best match if its similarity meets `threshold`,
/// or `None` if no candidate qualifies.
fn best_jaccard_match<'a>(
    old_params: &HashSet<&str>,
    added_jobs: &HashMap<&'a str, &Job>,
    threshold: f64,
) -> Option<&'a str> {
    let mut best: Option<(&str, f64)> = None;
    for (new_name, new_job) in added_jobs {
        let new_params: HashSet<&str> = new_job.parameters.keys().map(|s| s.as_str()).collect();
        let sim = jaccard_similarity(old_params, &new_params);
        if sim >= threshold {
            match best {
                None => best = Some((new_name, sim)),
                Some((_, best_sim)) if sim > best_sim => best = Some((new_name, sim)),
                _ => {}
            }
        }
    }
    best.map(|(name, _)| name)
}

/// Detects `CommandRenamed` cases using parameter-set fuzzy matching.
///
/// Mirrors `detect_renamed_jobs` but operates on `Command` definitions.
/// A command is considered renamed when a removed command name has
/// parameter-set Jaccard similarity ≥ `threshold` against a newly added
/// command.
///
/// Returns a map of old name → new name.
pub fn detect_renamed_commands(
    removed_names: &HashSet<String>,
    new_commands: &HashMap<String, &Command>,
    old_commands: &HashMap<String, &Command>,
    threshold: f64,
) -> HashMap<String, String> {
    let mut renamed = HashMap::new();

    // Only consider commands that didn't exist before (truly new, not modified)
    let added_commands: HashMap<&str, &Command> = new_commands
        .iter()
        .filter(|(name, _)| !old_commands.contains_key(name.as_str()))
        .map(|(name, cmd)| (name.as_str(), *cmd))
        .collect();

    for removed_name in removed_names {
        let Some(old_cmd) = old_commands.get(removed_name.as_str()) else {
            continue;
        };
        let old_params: HashSet<&str> = old_cmd.parameters.keys().map(|s| s.as_str()).collect();

        let mut best_match: Option<(&str, f64)> = None;

        for (new_name, new_cmd) in &added_commands {
            let new_params: HashSet<&str> = new_cmd.parameters.keys().map(|s| s.as_str()).collect();
            let sim = jaccard_similarity(&old_params, &new_params);
            if sim >= threshold {
                match best_match {
                    None => best_match = Some((new_name, sim)),
                    Some((_, best_sim)) if sim > best_sim => best_match = Some((new_name, sim)),
                    _ => {}
                }
            }
        }

        if let Some((new_name, _)) = best_match {
            renamed.insert(removed_name.clone(), new_name.to_string());
        }
    }

    renamed
}

/// Computes the Jaccard similarity between two sets of strings.
///
/// `|A ∩ B| / |A ∪ B|`. Returns 0.0 when both sets are empty.
fn jaccard_similarity(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return 0.0;
    }
    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{Parameter, ParameterType};

    fn job_with_params(param_names: &[(&str, ParameterType)]) -> Job {
        let mut parameters = HashMap::new();
        for (name, ptype) in param_names {
            parameters.insert(
                name.to_string(),
                Parameter {
                    param_type: *ptype,
                    description: None,
                    default: None,
                    enum_values: None,
                },
            );
        }
        Job {
            parameters,
            ..Default::default()
        }
    }

    #[test]
    fn test_absorption_candidate_params() {
        let candidates = absorption_candidate_params("label");
        assert!(candidates.contains(&"run_label".to_string()));
        assert!(candidates.contains(&"include_label".to_string()));
        assert!(candidates.contains(&"label_step".to_string()));
    }

    #[test]
    fn test_detect_absorbed_jobs_toolkit_case() {
        // Simulate: label removed, update_prlog gains run_label: bool
        let old_label = job_with_params(&[("context", ParameterType::String)]);
        let old_update_prlog = job_with_params(&[("min_rust_version", ParameterType::String)]);
        let new_update_prlog = job_with_params(&[
            ("context", ParameterType::String),
            ("run_label", ParameterType::Boolean), // newly added boolean
        ]);

        let removed_jobs: HashMap<String, &Job> =
            [("label".to_string(), &old_label)].into_iter().collect();
        let new_jobs: HashMap<String, &Job> = [("update_prlog".to_string(), &new_update_prlog)]
            .into_iter()
            .collect();
        let old_jobs: HashMap<String, &Job> = [
            ("label".to_string(), &old_label),
            ("update_prlog".to_string(), &old_update_prlog),
        ]
        .into_iter()
        .collect();

        let absorbed = detect_absorbed_jobs(&removed_jobs, &new_jobs, &old_jobs);
        assert_eq!(absorbed.get("label"), Some(&"update_prlog".to_string()));
    }

    #[test]
    fn test_detect_absorbed_jobs_no_match() {
        // label removed, no job gains run_label boolean
        let old_label = job_with_params(&[]);
        let new_other = job_with_params(&[("some_string", ParameterType::String)]);

        let removed_jobs: HashMap<String, &Job> =
            [("label".to_string(), &old_label)].into_iter().collect();
        let new_jobs: HashMap<String, &Job> =
            [("other".to_string(), &new_other)].into_iter().collect();
        let old_jobs: HashMap<String, &Job> =
            [("label".to_string(), &old_label)].into_iter().collect();

        let absorbed = detect_absorbed_jobs(&removed_jobs, &new_jobs, &old_jobs);
        assert!(absorbed.is_empty());
    }

    #[test]
    fn test_jaccard_similarity_identical() {
        let a: HashSet<&str> = ["foo", "bar", "baz"].into_iter().collect();
        let b = a.clone();
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_similarity_disjoint() {
        let a: HashSet<&str> = ["foo"].into_iter().collect();
        let b: HashSet<&str> = ["bar"].into_iter().collect();
        assert!((jaccard_similarity(&a, &b)).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_similarity_partial() {
        let a: HashSet<&str> = ["foo", "bar", "baz"].into_iter().collect();
        let b: HashSet<&str> = ["foo", "bar", "qux"].into_iter().collect();
        // intersection = {foo, bar}, union = {foo, bar, baz, qux}
        let expected = 2.0 / 4.0;
        assert!((jaccard_similarity(&a, &b) - expected).abs() < 1e-9);
    }

    #[test]
    fn test_detect_renamed_jobs() {
        // idiomatic_rust removed; idiomatic_rust_rolling added with similar params
        let shared_params = [
            ("context", ParameterType::String),
            ("cargo_all_features", ParameterType::Boolean),
            ("cache_version", ParameterType::String),
        ];
        let old_job = job_with_params(&shared_params);
        let new_job = job_with_params(&shared_params); // same params → high Jaccard

        let removed: HashSet<String> = ["idiomatic_rust".to_string()].into_iter().collect();
        let new_jobs: HashMap<String, &Job> = [("idiomatic_rust_rolling".to_string(), &new_job)]
            .into_iter()
            .collect();
        let old_jobs: HashMap<String, &Job> = [("idiomatic_rust".to_string(), &old_job)]
            .into_iter()
            .collect();

        let no_hints = HashMap::new();
        let renamed = detect_renamed_jobs(&removed, &new_jobs, &old_jobs, 0.7, &no_hints);
        assert_eq!(
            renamed.get("idiomatic_rust"),
            Some(&"idiomatic_rust_rolling".to_string())
        );
    }

    #[test]
    fn test_detect_renamed_jobs_with_git_hint_when_target_existed() {
        // Scenario mirrors the circleci-toolkit 6.0.0 rename:
        //   required_builds_rolling → required_builds
        // The catch: required_builds already existed in old_jobs (it was the
        // pinned variant). The Jaccard-only heuristic excludes it from
        // candidates because it is not "truly new". A git hint must override.
        let shared = [
            ("min_rust_version", ParameterType::String),
            ("cargo_all_features", ParameterType::Boolean),
            ("cache_version", ParameterType::String),
        ];
        let old_rolling = job_with_params(&shared); // required_builds_rolling (old)
        let old_pinned = job_with_params(&shared); // required_builds (old, pinned variant)
        let new_standard = job_with_params(&shared); // required_builds (new, was rolling)

        let removed: HashSet<String> = ["required_builds_rolling".to_string()]
            .into_iter()
            .collect();
        let new_jobs: HashMap<String, &Job> = [("required_builds".to_string(), &new_standard)]
            .into_iter()
            .collect();
        let old_jobs: HashMap<String, &Job> = [
            ("required_builds_rolling".to_string(), &old_rolling),
            ("required_builds".to_string(), &old_pinned),
        ]
        .into_iter()
        .collect();

        // Without hint: Jaccard cannot detect this rename because
        // required_builds is not "truly new" (it existed in old_jobs).
        let no_hints = HashMap::new();
        let without_hint = detect_renamed_jobs(&removed, &new_jobs, &old_jobs, 0.7, &no_hints);
        assert!(
            without_hint.is_empty(),
            "Without git hint, Jaccard should NOT detect rename when target existed before"
        );

        // With git hint: must detect the rename regardless.
        let mut hints = HashMap::new();
        hints.insert(
            "required_builds_rolling".to_string(),
            "required_builds".to_string(),
        );
        let with_hint = detect_renamed_jobs(&removed, &new_jobs, &old_jobs, 0.7, &hints);
        assert_eq!(
            with_hint.get("required_builds_rolling"),
            Some(&"required_builds".to_string()),
            "With git hint, rename must be detected even when target existed before"
        );
    }

    #[test]
    fn test_detect_renamed_jobs_below_threshold() {
        let old_job =
            job_with_params(&[("a", ParameterType::String), ("b", ParameterType::String)]);
        let new_job =
            job_with_params(&[("x", ParameterType::String), ("y", ParameterType::String)]);

        let removed: HashSet<String> = ["old_job".to_string()].into_iter().collect();
        let new_jobs: HashMap<String, &Job> = [("completely_different".to_string(), &new_job)]
            .into_iter()
            .collect();
        let old_jobs: HashMap<String, &Job> =
            [("old_job".to_string(), &old_job)].into_iter().collect();

        let no_hints = HashMap::new();
        let renamed = detect_renamed_jobs(&removed, &new_jobs, &old_jobs, 0.7, &no_hints);
        assert!(renamed.is_empty());
    }
}
