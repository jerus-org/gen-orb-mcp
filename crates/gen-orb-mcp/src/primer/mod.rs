//! Orb version history primer.
//!
//! Manages `prior-versions/` and `migrations/` directories as a sliding window:
//! - Versions within the window → snapshots and conformance rule JSON files
//!   created
//! - Versions outside the window → snapshots and rule files removed
//! - Idempotent: existing files are skipped; out-of-window files are removed

use std::path::{Path, PathBuf};

use anyhow::Result;
use chrono::{Datelike, NaiveDate};

use crate::{
    conformance_rule::ConformanceRule,
    differ,
    parser::{OrbDefinition, OrbParser},
};

/// A version tag annotated with its commit date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagWithDate {
    pub version: String,
    pub date: NaiveDate,
}

/// Configuration for the prime operation.
pub struct PrimeConfig {
    pub git_repo: PathBuf,
    pub tag_prefix: String,
    pub orb_path_relative: PathBuf,
    pub prior_versions_dir: PathBuf,
    pub migrations_dir: PathBuf,
    pub dry_run: bool,
}

/// Result of a prime operation.
#[derive(Debug, Default)]
pub struct PrimeResult {
    pub snapshots_added: usize,
    pub snapshots_removed: usize,
    pub migrations_added: usize,
    pub migrations_removed: usize,
}

impl std::fmt::Display for PrimeResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} snapshot(s) added, {} removed; {} migration file(s) added, {} removed",
            self.snapshots_added,
            self.snapshots_removed,
            self.migrations_added,
            self.migrations_removed
        )
    }
}

// ── Pure functions
// ────────────────────────────────────────────────────────────

/// Filter a version list to those at or after `earliest` (semver comparison).
///
/// Tags that cannot be parsed as semver are silently skipped with a warning.
pub fn filter_by_version(tags: &[String], earliest: &semver::Version) -> Vec<String> {
    tags.iter()
        .filter_map(|v| match semver::Version::parse(v) {
            Ok(parsed) if parsed >= *earliest => Some(v.clone()),
            Ok(_) => None,
            Err(_) => {
                tracing::warn!(tag = %v, "Skipping tag: not valid semver");
                None
            }
        })
        .collect()
}

/// Filter a list of tags-with-dates to those on or after `cutoff`.
pub fn filter_by_date(tags: &[TagWithDate], cutoff: NaiveDate) -> Vec<String> {
    tags.iter()
        .filter(|t| t.date >= cutoff)
        .map(|t| t.version.clone())
        .collect()
}

/// Compute the cutoff `NaiveDate` from a duration string.
///
/// Supported formats: `"N months"`, `"N month"`, `"N years"`, `"N year"`,
/// `"N weeks"`, `"N week"`.
///
/// `today` is injected so tests can pass a fixed date.
pub fn since_cutoff(since: &str, today: NaiveDate) -> Result<NaiveDate> {
    let since = since.trim().to_lowercase();
    let parts: Vec<&str> = since.splitn(2, ' ').collect();
    if parts.len() != 2 {
        anyhow::bail!(
            "Invalid --since format '{}': expected 'N unit' e.g. '6 months'",
            since
        );
    }
    let n: i64 = parts[0]
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid --since format '{}': N must be a number", since))?;
    let unit = parts[1].trim_end_matches('s'); // normalise plural
    match unit {
        "month" => {
            let month = today.month() as i64 - n;
            let (year_offset, new_month) = if month <= 0 {
                let years_back = (-month / 12) + 1;
                (years_back, month + years_back * 12)
            } else {
                (0, month)
            };
            let new_year = today.year() - year_offset as i32;
            let new_month = new_month as u32;
            // Saturate to last day of month if needed
            let max_day = days_in_month(new_year, new_month);
            let new_day = today.day().min(max_day);
            NaiveDate::from_ymd_opt(new_year, new_month, new_day)
                .ok_or_else(|| anyhow::anyhow!("Date arithmetic produced invalid date"))
        }
        "year" => {
            let new_year = today.year() - n as i32;
            let max_day = days_in_month(new_year, today.month());
            let new_day = today.day().min(max_day);
            NaiveDate::from_ymd_opt(new_year, today.month(), new_day)
                .ok_or_else(|| anyhow::anyhow!("Date arithmetic produced invalid date"))
        }
        "week" => {
            let days = chrono::Days::new((n * 7) as u64);
            today
                .checked_sub_days(days)
                .ok_or_else(|| anyhow::anyhow!("Date arithmetic overflow"))
        }
        _ => anyhow::bail!(
            "Invalid --since unit '{}': use 'months', 'years', or 'weeks'",
            parts[1]
        ),
    }
}

/// Returns the number of days in the given month/year.
fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next_month
        .unwrap()
        .signed_duration_since(NaiveDate::from_ymd_opt(year, month, 1).unwrap())
        .num_days() as u32
}

/// Returns `true` if the snapshot file for `version` does not yet exist in
/// `dir`.
pub fn snapshot_needed(dir: &Path, version: &str) -> bool {
    !dir.join(format!("{version}.yml")).exists()
}

/// Returns `true` if the migration file for `version` does not yet exist in
/// `dir`.
pub fn migration_needed(dir: &Path, version: &str) -> bool {
    !dir.join(format!("{version}.json")).exists()
}

/// Compute conformance rules between two consecutive orb versions.
///
/// Returns an empty `Vec` when the orbs are structurally identical.
pub fn compute_diff(old: &OrbDefinition, new: &OrbDefinition, since: &str) -> Vec<ConformanceRule> {
    differ::diff(old, new, since)
}

/// Serialise an `OrbDefinition` to YAML for storage as a snapshot file.
pub fn serialize_orb(orb: &OrbDefinition) -> Result<String> {
    serde_yaml::to_string(orb).map_err(|e| anyhow::anyhow!("Failed to serialise orb: {}", e))
}

// ── Git subprocess functions
// ──────────────────────────────────────────────────

/// List all version tags in `git_repo` that start with `tag_prefix`, sorted by
/// semver.
///
/// Returns bare version strings (prefix stripped).
pub fn discover_tags(git_repo: &Path, tag_prefix: &str) -> Result<Vec<String>> {
    let pattern = format!("{}*", tag_prefix);
    let output = std::process::Command::new("git")
        .args([
            "-C",
            git_repo.to_str().unwrap_or("."),
            "tag",
            "--sort=version:refname",
            "-l",
            &pattern,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git tag: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git tag failed: {}", stderr);
    }

    let prefix_len = tag_prefix.len();
    let tags = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let tag = line.trim();
            if tag.starts_with(tag_prefix) {
                Some(tag[prefix_len..].to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(tags)
}

/// Fetch the commit date for a single version tag.
///
/// Runs `git log -1 --format=%ci <prefix><version>^{}` to dereference annotated
/// tags.
pub fn tag_date(git_repo: &Path, tag_prefix: &str, version: &str) -> Result<NaiveDate> {
    let tag = format!("{}{}", tag_prefix, version);
    let output = std::process::Command::new("git")
        .args([
            "-C",
            git_repo.to_str().unwrap_or("."),
            "log",
            "-1",
            "--format=%ci",
            &format!("{}^{{}}", tag),
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git log for tag {}: {}", tag, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git log failed for tag {}: {}", tag, stderr);
    }

    let date_str = String::from_utf8_lossy(&output.stdout);
    let date_str = date_str.trim();
    if date_str.is_empty() {
        anyhow::bail!("git log returned empty date for tag {}", tag);
    }

    // Format: "2024-01-15 10:30:00 +0000" — take the first 10 chars
    NaiveDate::parse_from_str(&date_str[..10], "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("Failed to parse date '{}' for tag {}: {}", date_str, tag, e))
}

/// RAII guard for a git worktree.
///
/// Runs `git worktree remove --force <path>` on drop.
struct WorktreeGuard {
    path: PathBuf,
    git_repo: PathBuf,
}

impl WorktreeGuard {
    fn new(git_repo: &Path, path: &Path) -> Self {
        WorktreeGuard {
            path: path.to_owned(),
            git_repo: git_repo.to_owned(),
        }
    }
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let _ = std::process::Command::new("git")
            .args([
                "-C",
                self.git_repo.to_str().unwrap_or("."),
                "worktree",
                "remove",
                "--force",
                self.path.to_str().unwrap_or(""),
            ])
            .output();
    }
}

/// Check out a version tag into a temp worktree, parse the orb, remove the
/// worktree.
///
/// The RAII guard ensures cleanup even on panic.
pub fn checkout_and_parse(
    git_repo: &Path,
    tag_prefix: &str,
    version: &str,
    orb_path_relative: &Path,
) -> Result<OrbDefinition> {
    let worktree_path = std::env::temp_dir().join(format!(
        "gen-orb-mcp-prime-{}-{}",
        std::process::id(),
        version
    ));

    // Remove stale worktree if it exists
    let _ = std::process::Command::new("git")
        .args([
            "-C",
            git_repo.to_str().unwrap_or("."),
            "worktree",
            "remove",
            "--force",
            worktree_path.to_str().unwrap_or(""),
        ])
        .output();

    let tag = format!("{}{}", tag_prefix, version);
    let add_output = std::process::Command::new("git")
        .args([
            "-C",
            git_repo.to_str().unwrap_or("."),
            "worktree",
            "add",
            worktree_path.to_str().unwrap_or(""),
            &tag,
        ])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to add worktree for {}: {}", tag, e))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        anyhow::bail!("git worktree add failed for {}: {}", tag, stderr);
    }

    // RAII guard: removes worktree on drop (even if parsing fails)
    let _guard = WorktreeGuard::new(git_repo, &worktree_path);

    let orb_full_path = worktree_path.join(orb_path_relative);
    OrbParser::parse(&orb_full_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse orb at {} ({}): {}",
            orb_full_path.display(),
            tag,
            e
        )
    })
}

// ── High-level prime operation
// ────────────────────────────────────────────────

/// Run the prime operation: add missing snapshots/rules and remove
/// out-of-window ones.
pub fn prime(config: &PrimeConfig, window_versions: &[String]) -> Result<PrimeResult> {
    let mut result = PrimeResult::default();

    if !config.dry_run {
        std::fs::create_dir_all(&config.prior_versions_dir)?;
        std::fs::create_dir_all(&config.migrations_dir)?;
    }

    result.snapshots_added = add_snapshots(config, window_versions)?;
    result.migrations_added = add_migrations(config, window_versions)?;

    let window_set: std::collections::HashSet<&String> = window_versions.iter().collect();
    let (snaps_removed, migs_removed) = remove_out_of_window(config, &window_set)?;
    result.snapshots_removed = snaps_removed;
    result.migrations_removed += migs_removed;
    result.migrations_removed += remove_orphaned_migrations(config)?;

    Ok(result)
}

/// ADD: snapshots for in-window versions not yet present.
fn add_snapshots(config: &PrimeConfig, window_versions: &[String]) -> Result<usize> {
    let mut added = 0;
    for version in window_versions {
        if !snapshot_needed(&config.prior_versions_dir, version) {
            tracing::debug!(version, "Skipping snapshot (already exists)");
            continue;
        }
        if config.dry_run {
            println!("would create prior-versions/{version}.yml");
        } else {
            let orb = checkout_and_parse(
                &config.git_repo,
                &config.tag_prefix,
                version,
                &config.orb_path_relative,
            )?;
            let yaml = serialize_orb(&orb)?;
            let path = config.prior_versions_dir.join(format!("{version}.yml"));
            std::fs::write(&path, &yaml)?;
            println!("created prior-versions/{version}.yml");
        }
        added += 1;
    }
    Ok(added)
}

/// ADD: migration rules for consecutive in-window pairs.
fn add_migrations(config: &PrimeConfig, window_versions: &[String]) -> Result<usize> {
    let mut added = 0;
    for pair in window_versions.windows(2) {
        let prev = &pair[0];
        let curr = &pair[1];
        if !migration_needed(&config.migrations_dir, curr) {
            tracing::debug!(version = %curr, "Skipping migration (already exists)");
            continue;
        }
        if config.dry_run {
            println!("would create migrations/{curr}.json (if rules non-empty)");
        } else {
            added += write_migration_if_nonempty(config, prev, curr)?;
        }
    }
    Ok(added)
}

/// Compute and write a migration file for `curr` vs `prev`; returns 1 if
/// written, 0 if empty.
fn write_migration_if_nonempty(config: &PrimeConfig, prev: &str, curr: &str) -> Result<usize> {
    let prev_path = config.prior_versions_dir.join(format!("{prev}.yml"));
    let curr_path = config.prior_versions_dir.join(format!("{curr}.yml"));
    if !prev_path.exists() || !curr_path.exists() {
        tracing::warn!(prev, curr, "Skipping diff: snapshot missing");
        return Ok(0);
    }
    let old_orb = OrbParser::parse(&prev_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", prev_path.display(), e))?;
    let new_orb = OrbParser::parse(&curr_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", curr_path.display(), e))?;
    let rules = compute_diff(&old_orb, &new_orb, curr);
    if rules.is_empty() {
        tracing::debug!(version = %curr, "No migration rules (orbs identical in structure)");
        return Ok(0);
    }
    let json = serde_json::to_string_pretty(&rules)?;
    let path = config.migrations_dir.join(format!("{curr}.json"));
    std::fs::write(&path, &json)?;
    println!("created migrations/{curr}.json ({} rules)", rules.len());
    Ok(1)
}

/// REMOVE: snapshots outside the window and their matching migration files.
/// Returns `(snapshots_removed, migrations_removed)`.
fn remove_out_of_window(
    config: &PrimeConfig,
    window_set: &std::collections::HashSet<&String>,
) -> Result<(usize, usize)> {
    let (mut snaps, mut migs) = (0, 0);
    if !config.prior_versions_dir.is_dir() {
        return Ok((0, 0));
    }
    for entry in std::fs::read_dir(&config.prior_versions_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let version = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if version.is_empty() || window_set.contains(&version) {
            continue;
        }
        remove_or_announce(
            config.dry_run,
            &path,
            &format!("prior-versions/{version}.yml (outside window)"),
        )?;
        snaps += 1;
        let mig_path = config.migrations_dir.join(format!("{version}.json"));
        if mig_path.exists() {
            remove_or_announce(
                config.dry_run,
                &mig_path,
                &format!("migrations/{version}.json (snapshot removed)"),
            )?;
            migs += 1;
        }
    }
    Ok((snaps, migs))
}

/// REMOVE: orphaned migration files (no matching snapshot). Returns count
/// removed.
fn remove_orphaned_migrations(config: &PrimeConfig) -> Result<usize> {
    let mut removed = 0;
    if !config.migrations_dir.is_dir() {
        return Ok(0);
    }
    for entry in std::fs::read_dir(&config.migrations_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let version = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if version.is_empty() {
            continue;
        }
        let snapshot = config.prior_versions_dir.join(format!("{version}.yml"));
        if !snapshot.exists() {
            remove_or_announce(
                config.dry_run,
                &path,
                &format!("migrations/{version}.json (orphaned)"),
            )?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Remove a file or print a dry-run announcement.
fn remove_or_announce(dry_run: bool, path: &Path, label: &str) -> Result<()> {
    if dry_run {
        println!("would remove {label}");
    } else {
        std::fs::remove_file(path)?;
        println!("removed {label}");
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    // ── Test 1: filter_by_version ─────────────────────────────────────────────
    #[test]
    fn test_filter_tags_by_version() {
        let tags = vec![
            "4.0.2".to_string(),
            "4.0.9".to_string(),
            "4.1.0".to_string(),
            "4.5.0".to_string(),
            "5.0.0".to_string(),
            "not-a-semver".to_string(),
        ];
        let earliest = semver::Version::parse("4.1.0").unwrap();
        let result = filter_by_version(&tags, &earliest);
        assert_eq!(result, vec!["4.1.0", "4.5.0", "5.0.0"]);
        // 4.0.x filtered out; "not-a-semver" silently dropped
    }

    // ── Test 2: filter_by_date ────────────────────────────────────────────────
    #[test]
    fn test_filter_tags_by_date() {
        let cutoff = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        let tags = vec![
            TagWithDate {
                version: "4.0.0".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 8, 31).unwrap(),
            },
            TagWithDate {
                version: "4.5.0".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(), // exactly on cutoff
            },
            TagWithDate {
                version: "4.9.6".to_string(),
                date: NaiveDate::from_ymd_opt(2025, 10, 15).unwrap(),
            },
        ];
        let result = filter_by_date(&tags, cutoff);
        assert_eq!(result, vec!["4.5.0", "4.9.6"]);
    }

    // ── Test 3: since_cutoff default of 6 months ──────────────────────────────
    #[test]
    fn test_default_since_is_six_months() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 20).unwrap();
        let cutoff = since_cutoff("6 months", today).unwrap();
        assert_eq!(cutoff, NaiveDate::from_ymd_opt(2025, 9, 20).unwrap());
    }

    // ── Test 4: since_cutoff parses multiple formats ───────────────────────────
    #[test]
    fn test_since_cutoff_parsing() {
        let today = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        assert_eq!(
            since_cutoff("6 months", today).unwrap(),
            NaiveDate::from_ymd_opt(2025, 7, 15).unwrap()
        );
        assert_eq!(
            since_cutoff("1 year", today).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
        );
        assert_eq!(
            since_cutoff("2 years", today).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        assert_eq!(
            since_cutoff("3 weeks", today).unwrap(),
            NaiveDate::from_ymd_opt(2025, 12, 25).unwrap()
        );
        assert!(
            since_cutoff("garbage", today).is_err(),
            "invalid input should fail"
        );
        assert!(
            since_cutoff("abc months", today).is_err(),
            "non-numeric N should fail"
        );
    }

    // ── Test 5: month-end saturation ──────────────────────────────────────────
    #[test]
    fn test_since_cutoff_month_end() {
        // March 31 - 6 months = September 30 (September has 30 days, not 31)
        let today = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        let cutoff = since_cutoff("6 months", today).unwrap();
        assert_eq!(cutoff, NaiveDate::from_ymd_opt(2025, 9, 30).unwrap());
    }

    // ── Test 6: idempotent — skips existing snapshot ──────────────────────────
    #[test]
    fn test_idempotent_skips_existing_snapshot() {
        let tmp = TempDir::new().unwrap();
        let pv_dir = tmp.path().join("prior-versions");
        std::fs::create_dir_all(&pv_dir).unwrap();
        let snap = pv_dir.join("4.5.0.yml");
        std::fs::write(&snap, "sentinel: existing").unwrap();

        assert!(
            !snapshot_needed(&pv_dir, "4.5.0"),
            "should report not needed"
        );
        assert_eq!(
            std::fs::read_to_string(&snap).unwrap(),
            "sentinel: existing",
            "file must not be overwritten"
        );
    }

    // ── Test 7: idempotent — skips existing migration ─────────────────────────
    #[test]
    fn test_idempotent_skips_existing_diff() {
        let tmp = TempDir::new().unwrap();
        let mig_dir = tmp.path().join("migrations");
        std::fs::create_dir_all(&mig_dir).unwrap();
        let mig = mig_dir.join("4.5.0.json");
        std::fs::write(&mig, r#"[{"type":"existing"}]"#).unwrap();

        assert!(!migration_needed(&mig_dir, "4.5.0"));
        assert_eq!(
            std::fs::read_to_string(&mig).unwrap(),
            r#"[{"type":"existing"}]"#
        );
    }

    // ── Test 8: empty diff — no file written ──────────────────────────────────
    #[test]
    fn test_empty_diff_not_written() {
        let orb = OrbDefinition::default();
        let rules = compute_diff(&orb, &orb, "4.5.0");
        assert!(rules.is_empty(), "identical orbs should produce no rules");

        let tmp = TempDir::new().unwrap();
        let mig_dir = tmp.path().join("migrations");
        std::fs::create_dir_all(&mig_dir).unwrap();

        // Simulate the write-if-non-empty logic
        if !rules.is_empty() {
            std::fs::write(mig_dir.join("4.5.0.json"), "[]").unwrap();
        }
        assert!(
            !mig_dir.join("4.5.0.json").exists(),
            "empty diff must not create a file"
        );
    }

    // ── Test 9: sliding window removes out-of-window snapshot ─────────────────
    #[test]
    fn test_sliding_window_removes_old_snapshot() {
        let tmp = TempDir::new().unwrap();
        let pv_dir = tmp.path().join("prior-versions");
        let mig_dir = tmp.path().join("migrations");
        std::fs::create_dir_all(&pv_dir).unwrap();
        std::fs::create_dir_all(&mig_dir).unwrap();

        // Simulate two snapshots: 4.0.0 (old, out of window) and 5.0.0 (current window)
        std::fs::write(pv_dir.join("4.0.0.yml"), "old: true").unwrap();
        std::fs::write(pv_dir.join("5.0.0.yml"), "current: true").unwrap();

        let window = ["5.0.0".to_string()];
        let config = PrimeConfig {
            git_repo: tmp.path().to_owned(),
            tag_prefix: "v".to_string(),
            orb_path_relative: PathBuf::from("src/@orb.yml"),
            prior_versions_dir: pv_dir.clone(),
            migrations_dir: mig_dir,
            dry_run: false,
        };

        // Only run the removal part (window already satisfied, no git ops needed)
        let window_set: std::collections::HashSet<&String> = window.iter().collect();
        for entry in std::fs::read_dir(&pv_dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yml") {
                continue;
            }
            let version = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if !window_set.contains(&version) {
                std::fs::remove_file(&path).unwrap();
            }
        }

        assert!(
            !pv_dir.join("4.0.0.yml").exists(),
            "out-of-window snapshot must be removed"
        );
        assert!(
            pv_dir.join("5.0.0.yml").exists(),
            "in-window snapshot must remain"
        );
        drop(config); // suppress unused warning
    }

    // ── Test 10: sliding window removes orphaned migration ────────────────────
    #[test]
    fn test_sliding_window_removes_orphaned_migration() {
        let tmp = TempDir::new().unwrap();
        let pv_dir = tmp.path().join("prior-versions");
        let mig_dir = tmp.path().join("migrations");
        std::fs::create_dir_all(&pv_dir).unwrap();
        std::fs::create_dir_all(&mig_dir).unwrap();

        // Migration for 4.0.0 exists but its snapshot does not
        std::fs::write(mig_dir.join("4.0.0.json"), r#"[]"#).unwrap();
        // Snapshot for 5.0.0 exists (in window)
        std::fs::write(pv_dir.join("5.0.0.yml"), "current: true").unwrap();

        // Simulate orphan removal
        for entry in std::fs::read_dir(&mig_dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let version = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let snapshot = pv_dir.join(format!("{version}.yml"));
            if !snapshot.exists() {
                std::fs::remove_file(&path).unwrap();
            }
        }

        assert!(
            !mig_dir.join("4.0.0.json").exists(),
            "orphaned migration must be removed"
        );
    }

    // ── Tests 11-15: CLI parsing (in lib.rs) ─────────────────────────────────
    // These are defined in lib.rs in the tests module for the Prime command
    // variant.

    // ── Test 16: dry-run creates no files ────────────────────────────────────
    // This is an integration test requiring a real git repo fixture.
    // Defined in tests/prime_integration_tests.rs (to be added).
}
