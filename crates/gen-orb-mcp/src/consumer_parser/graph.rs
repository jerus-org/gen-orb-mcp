//! Workflow graph traversal for requires-chain resolution.
//!
//! CircleCI workflows form a directed acyclic graph where each job invocation
//! lists its upstream dependencies in `requires:`. This module provides
//! utilities to traverse that graph, which is essential for `JobAbsorbed`
//! detection: a `label` job that `requires` `update_prlog` is redundant once
//! `update_prlog` absorbs `label`'s functionality.

use std::collections::{HashMap, HashSet, VecDeque};

use super::types::Workflow;

/// Builds an index from effective job name to invocation index within the
/// workflow's `jobs` list. Used to resolve `requires:` references.
pub fn build_name_index(workflow: &Workflow) -> HashMap<String, usize> {
    workflow
        .jobs
        .iter()
        .enumerate()
        .map(|(idx, inv)| (inv.effective_name().to_string(), idx))
        .collect()
}

/// Returns the transitive requires-chain of a given job invocation within its
/// workflow.
///
/// The returned vector contains all jobs that the target invocation transitively
/// depends on (i.e. all ancestors in the DAG), in breadth-first order.
/// The target invocation itself is **not** included.
///
/// # Arguments
/// * `target_index` — index of the invocation to start from within `workflow.jobs`
/// * `workflow` — the workflow containing all invocations
///
/// Returns an empty vec if the target has no `requires:` entries or if none of
/// the referenced names can be resolved within the workflow.
pub fn requires_chain(target_index: usize, workflow: &Workflow) -> Vec<usize> {
    let name_index = build_name_index(workflow);
    let mut visited: HashSet<usize> = HashSet::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut result: Vec<usize> = Vec::new();

    // Seed the queue with direct requires of the target
    for req_name in &workflow.jobs[target_index].requires {
        if let Some(&idx) = name_index.get(req_name.as_str()) {
            if visited.insert(idx) {
                queue.push_back(idx);
                result.push(idx);
            }
        }
    }

    // BFS to find all transitive requires
    while let Some(current_idx) = queue.pop_front() {
        for req_name in &workflow.jobs[current_idx].requires {
            if let Some(&idx) = name_index.get(req_name.as_str()) {
                if visited.insert(idx) {
                    queue.push_back(idx);
                    result.push(idx);
                }
            }
        }
    }

    result
}

/// Returns `true` if `target_index` transitively requires `ancestor_name`
/// within the given workflow.
pub fn transitively_requires(
    target_index: usize,
    ancestor_name: &str,
    workflow: &Workflow,
) -> bool {
    let chain = requires_chain(target_index, workflow);
    chain.iter().any(|&idx| {
        workflow.jobs[idx].effective_name() == ancestor_name
            || workflow.jobs[idx].reference == ancestor_name
    })
}

/// Returns indices of all jobs in the workflow that are candidates for
/// `JobAbsorbed` detection: jobs whose reference matches `absorbed_job_name`
/// AND whose requires-chain includes `absorbing_job_name`.
pub fn find_absorbed_candidates(
    workflow: &Workflow,
    orb_alias: &str,
    absorbed_job_name: &str,
    absorbing_job_name: &str,
) -> Vec<usize> {
    workflow
        .jobs
        .iter()
        .enumerate()
        .filter(|(idx, inv)| {
            inv.matches(orb_alias, absorbed_job_name)
                && transitively_requires(*idx, absorbing_job_name, workflow)
        })
        .map(|(idx, _)| idx)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consumer_parser::types::{JobInvocation, SourceLocation, Workflow};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_job(
        reference: &str,
        orb_alias: Option<&str>,
        orb_job: Option<&str>,
        requires: Vec<&str>,
        name_override: Option<&str>,
    ) -> JobInvocation {
        JobInvocation {
            reference: reference.to_string(),
            orb_alias: orb_alias.map(|s| s.to_string()),
            orb_job: orb_job.map(|s| s.to_string()),
            parameters: HashMap::new(),
            requires: requires.into_iter().map(|s| s.to_string()).collect(),
            name_override: name_override.map(|s| s.to_string()),
            location: SourceLocation {
                file: PathBuf::from("config.yml"),
                workflow: "test".to_string(),
                job_index: 0,
            },
        }
    }

    #[test]
    fn test_requires_chain_direct() {
        let workflow = Workflow {
            jobs: vec![
                make_job(
                    "toolkit/update_prlog",
                    Some("toolkit"),
                    Some("update_prlog"),
                    vec![],
                    Some("update-prlog-on-main"),
                ),
                make_job(
                    "toolkit/label",
                    Some("toolkit"),
                    Some("label"),
                    vec!["update-prlog-on-main"],
                    None,
                ),
            ],
        };

        // label (index 1) should have update_prlog (index 0) in its requires chain
        let chain = requires_chain(1, &workflow);
        assert!(
            chain.contains(&0),
            "Expected index 0 in chain, got {:?}",
            chain
        );
    }

    #[test]
    fn test_requires_chain_empty() {
        let workflow = Workflow {
            jobs: vec![make_job(
                "toolkit/update_prlog",
                Some("toolkit"),
                Some("update_prlog"),
                vec![],
                None,
            )],
        };

        let chain = requires_chain(0, &workflow);
        assert!(chain.is_empty());
    }

    #[test]
    fn test_requires_chain_transitive() {
        // A -> B -> C (A requires B, B requires C)
        let workflow = Workflow {
            jobs: vec![
                make_job("job-c", None, None, vec![], Some("job-c")),
                make_job("job-b", None, None, vec!["job-c"], Some("job-b")),
                make_job("job-a", None, None, vec!["job-b"], Some("job-a")),
            ],
        };

        // job-a (index 2) should transitively require job-c (index 0) via job-b (index 1)
        let chain = requires_chain(2, &workflow);
        assert!(chain.contains(&1), "Expected job-b (1) in chain");
        assert!(
            chain.contains(&0),
            "Expected job-c (0) in chain transitively"
        );
    }

    #[test]
    fn test_transitively_requires_true() {
        let workflow = Workflow {
            jobs: vec![
                make_job(
                    "toolkit/update_prlog",
                    Some("toolkit"),
                    Some("update_prlog"),
                    vec![],
                    Some("update-prlog-on-main"),
                ),
                make_job(
                    "toolkit/label",
                    Some("toolkit"),
                    Some("label"),
                    vec!["update-prlog-on-main"],
                    None,
                ),
            ],
        };
        assert!(transitively_requires(1, "update-prlog-on-main", &workflow));
    }

    #[test]
    fn test_transitively_requires_false() {
        let workflow = Workflow {
            jobs: vec![
                make_job(
                    "toolkit/update_prlog",
                    Some("toolkit"),
                    Some("update_prlog"),
                    vec![],
                    None,
                ),
                make_job(
                    "toolkit/label",
                    Some("toolkit"),
                    Some("label"),
                    vec![],
                    None,
                ), // no requires
            ],
        };
        assert!(!transitively_requires(1, "toolkit/update_prlog", &workflow));
    }

    #[test]
    fn test_find_absorbed_candidates() {
        let workflow = Workflow {
            jobs: vec![
                make_job(
                    "toolkit/update_prlog",
                    Some("toolkit"),
                    Some("update_prlog"),
                    vec![],
                    Some("update-prlog-on-main"),
                ),
                make_job(
                    "toolkit/label",
                    Some("toolkit"),
                    Some("label"),
                    vec!["update-prlog-on-main"],
                    None,
                ),
            ],
        };

        let candidates =
            find_absorbed_candidates(&workflow, "toolkit", "label", "update-prlog-on-main");
        assert_eq!(
            candidates,
            vec![1],
            "Expected label (index 1) to be a candidate"
        );
    }

    #[test]
    fn test_find_absorbed_candidates_not_absorbed_without_requires() {
        let workflow = Workflow {
            jobs: vec![
                make_job(
                    "toolkit/update_prlog",
                    Some("toolkit"),
                    Some("update_prlog"),
                    vec![],
                    None,
                ),
                // label does not require update_prlog — not absorbed
                make_job(
                    "toolkit/label",
                    Some("toolkit"),
                    Some("label"),
                    vec![],
                    None,
                ),
            ],
        };

        let candidates =
            find_absorbed_candidates(&workflow, "toolkit", "label", "toolkit/update_prlog");
        assert!(candidates.is_empty());
    }
}
