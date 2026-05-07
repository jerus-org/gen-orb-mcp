//! # gen-orb-mcp
//!
//! Generate MCP (Model Context Protocol) servers from CircleCI orb definitions.
//!
//! This tool enables AI coding assistants to understand and work with private
//! CircleCI orbs by generating MCP servers that expose orb commands, jobs,
//! and executors as resources.
//!
//! ## Usage
//!
//! ```bash
//! gen-orb-mcp generate --orb-path ./src/@orb.yml --output ./dist/
//! ```

pub mod conformance_rule;
pub mod consumer_parser;
pub mod differ;
pub mod generator;
pub mod migrator;
pub mod parser;
pub mod primer;

use anyhow::Result;
use clap::{Parser, Subcommand};
use generator::CodeGenerator;
use parser::OrbParser;

/// Generate MCP servers from CircleCI orb definitions.
#[derive(Debug, Parser)]
#[command(name = "gen-orb-mcp")]
#[command(
    author,
    version,
    about,
    long_about = "Generate MCP servers from CircleCI orb definitions, \
        exposing commands, jobs, and executors as AI-accessible resources. \
        Supports migration tooling, prior-version snapshots, and diff-based \
        conformance rules to help consumers keep their CI config in sync with \
        orb updates."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Generate an MCP server from an orb definition
    Generate {
        /// Path to the orb YAML file (e.g., src/@orb.yml)
        #[arg(short = 'p', long)]
        orb_path: std::path::PathBuf,

        /// Output directory for generated server
        #[arg(short = 'o', long, default_value = "./dist")]
        output: std::path::PathBuf,

        /// Output format
        #[arg(short, long, value_enum, default_value = "source")]
        format: OutputFormat,

        /// Name for the generated orb server (defaults to filename)
        #[arg(short, long)]
        name: Option<String>,

        /// Version for the generated MCP server crate (e.g., "1.0.0")
        ///
        /// Required when regenerating an existing output directory.
        /// For CI workflows, this should match the orb release version.
        #[arg(short = 'V', long)]
        version: Option<String>,

        /// Overwrite existing files without confirmation
        ///
        /// Required for non-interactive CI environments when output exists.
        #[arg(long)]
        force: bool,

        /// Directory containing conformance rule JSON files to embed in the
        /// server
        ///
        /// All *.json files in this directory are merged and embedded as
        /// migration tooling in the generated server. When provided,
        /// the server gains plan_migration and apply_migration MCP
        /// Tools in addition to Resources.
        #[arg(long)]
        migrations: Option<std::path::PathBuf>,

        /// Directory of prior orb version YAML snapshots to embed in the server
        ///
        /// Each file should be named `<version>.yml` (e.g., `4.7.1.yml`). The
        /// generated server will expose version-specific resources for each
        /// prior version alongside the current version.
        #[arg(long)]
        prior_versions: Option<std::path::PathBuf>,

        /// Tag prefix used to discover the orb version from git tags
        ///
        /// The git repository is derived automatically from --orb-path.
        /// Defaults to "v" (matches tags like v6.0.0).
        #[arg(long, default_value = "v")]
        tag_prefix: String,
    },
    /// Validate an orb definition without generating
    Validate {
        /// Path to the orb YAML file
        #[arg(short = 'p', long)]
        orb_path: std::path::PathBuf,
    },
    /// Compute conformance rules by diffing two orb versions
    ///
    /// Compares the current orb against a previous version (read from a file)
    /// and emits a JSON array of ConformanceRule values. These rules can be
    /// passed to `generate --migrations` to embed migration tooling in the
    /// generated MCP server.
    Diff {
        /// Path to the current orb YAML (the new version)
        #[arg(long)]
        current: std::path::PathBuf,

        /// Path to the previous orb YAML (the old version to diff against)
        #[arg(long)]
        previous: std::path::PathBuf,

        /// The version string to embed in emitted rules (e.g. "5.0.0")
        #[arg(long)]
        since_version: String,

        /// Optional output file for the JSON rules (default: stdout)
        #[arg(long)]
        output: Option<std::path::PathBuf>,
    },
    /// Apply conformance-based migration to a consumer's .circleci/ directory
    ///
    /// Reads conformance rules from a JSON file (produced by `diff`) and
    /// applies them to the consumer's CI config. Reports planned changes
    /// before applying.
    Migrate {
        /// Path to the consumer's .circleci/ directory
        #[arg(long, default_value = ".circleci")]
        ci_dir: std::path::PathBuf,

        /// The orb alias as used in the consumer's orbs: section (e.g.
        /// "toolkit")
        #[arg(long)]
        orb: String,

        /// Path to the conformance rules JSON file (produced by `diff`)
        #[arg(long)]
        rules: std::path::PathBuf,

        /// Show planned changes without modifying files
        #[arg(long)]
        dry_run: bool,
    },
    /// Populate prior-versions/ and migrations/ from git history
    ///
    /// Discovers version tags in a sliding window (default: last 6 months),
    /// checks out each version, saves a snapshot to
    /// `prior-versions/<version>.yml`, and computes conformance-rule diffs
    /// to `migrations/<version>.json`. Removes files for versions outside
    /// the window to keep binary size bounded. Idempotent.
    Prime {
        /// Path to the orb YAML entry point
        #[arg(short = 'p', long, default_value = "src/@orb.yml")]
        orb_path: std::path::PathBuf,

        /// Path to the git repository root (default: walk up from orb-path to
        /// .git)
        #[arg(long)]
        git_repo: Option<std::path::PathBuf>,

        /// Git tag prefix (e.g. "v" matches tags like "v4.1.0")
        #[arg(long, default_value = "v")]
        tag_prefix: String,

        /// Fixed earliest version anchor (e.g. "4.1.0"); conflicts with --since
        #[arg(long, conflicts_with = "since")]
        earliest_version: Option<String>,

        /// Rolling window duration (e.g. "6 months", "1 year"); default: "6
        /// months"
        #[arg(long)]
        since: Option<String>,

        /// Directory to write prior-version snapshots
        #[arg(long, default_value = "prior-versions")]
        prior_versions_dir: std::path::PathBuf,

        /// Directory to write migration rule JSON files
        #[arg(long, default_value = "migrations")]
        migrations_dir: std::path::PathBuf,

        /// Write to `/tmp/gen-orb-mcp-prime-<pid>/` and print
        /// PRIME_PV_DIR/PRIME_MIG_DIR to stdout
        #[arg(long)]
        ephemeral: bool,

        /// Override git rename detection for a specific job (repeatable).
        /// Format: `OLD=NEW`, e.g. `--rename-map common_tests_rolling=common_tests`.
        /// Manual entries take precedence over git-detected hints for matching
        /// old names.  Use this when commits cannot be restructured to follow
        /// the two-commit rename rule.
        #[arg(long, value_name = "OLD=NEW")]
        rename_map: Vec<String>,

        /// Describe actions without writing any files
        #[arg(long)]
        dry_run: bool,
    },
}

/// Output format for generated MCP server
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Compile to native binary (Linux x86_64)
    Binary,
    /// Generate Rust source code
    Source,
}

/// Optional embedding inputs for `run_generate`.
struct GenerateExtras<'a> {
    migrations: &'a Option<std::path::PathBuf>,
    prior_versions_dir: &'a Option<std::path::PathBuf>,
    tag_prefix: &'a str,
}

impl Cli {
    /// Execute the CLI command
    pub fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Generate {
                orb_path,
                output,
                format,
                name,
                version,
                force,
                migrations,
                prior_versions,
                tag_prefix,
            } => run_generate(
                orb_path,
                output,
                format,
                name,
                version,
                *force,
                GenerateExtras {
                    migrations,
                    prior_versions_dir: prior_versions,
                    tag_prefix,
                },
            ),
            Commands::Validate { orb_path } => run_validate(orb_path),
            Commands::Diff {
                current,
                previous,
                since_version,
                output,
            } => run_diff(current, previous, since_version, output),
            Commands::Migrate {
                ci_dir,
                orb,
                rules: rules_path,
                dry_run,
            } => run_migrate(ci_dir, orb, rules_path, *dry_run),
            Commands::Prime {
                orb_path,
                git_repo,
                tag_prefix,
                earliest_version,
                since,
                prior_versions_dir,
                migrations_dir,
                rename_map,
                ephemeral,
                dry_run,
            } => run_prime(
                orb_path,
                git_repo.as_deref(),
                tag_prefix,
                earliest_version.as_deref(),
                since.as_deref(),
                prior_versions_dir,
                migrations_dir,
                rename_map,
                *ephemeral,
                *dry_run,
            ),
        }
    }
}

fn run_generate(
    orb_path: &std::path::PathBuf,
    output: &std::path::PathBuf,
    format: &OutputFormat,
    name: &Option<String>,
    version: &Option<String>,
    force: bool,
    extras: GenerateExtras<'_>,
) -> Result<()> {
    tracing::info!(?orb_path, ?output, ?format, "Generating MCP server");

    let orb = OrbParser::parse(orb_path).map_err(|e| anyhow::anyhow!("{}", e))?;
    tracing::info!(
        commands = orb.commands.len(),
        jobs = orb.jobs.len(),
        executors = orb.executors.len(),
        "Parsed orb definition"
    );

    let orb_name = name.clone().unwrap_or_else(|| derive_orb_name(orb_path));

    // Auto-discover version from the git repo containing orb_path
    let git_hint: Option<String> = match find_git_root(orb_path) {
        Ok(repo) => discover_latest_version(&repo, extras.tag_prefix)?,
        Err(_) => None,
    };
    let resolved_version = resolve_version(output, version.as_deref(), force, git_hint.as_deref())?;
    tracing::info!(version = %resolved_version, "Using version");

    let conformance_rules = if let Some(migrations_dir) = extras.migrations {
        load_conformance_rules(migrations_dir)?
    } else {
        vec![]
    };
    if !conformance_rules.is_empty() {
        tracing::info!(rules = conformance_rules.len(), "Loaded conformance rules");
    }

    let prior_versions_data = if let Some(dir) = extras.prior_versions_dir {
        load_prior_versions(dir)?
    } else {
        vec![]
    };
    if !prior_versions_data.is_empty() {
        tracing::info!(
            versions = prior_versions_data.len(),
            "Loaded prior versions"
        );
    }

    let conformance_rules_json = if !conformance_rules.is_empty() {
        Some(serde_json::to_string(&conformance_rules)?)
    } else {
        None
    };

    let generator = CodeGenerator::new()
        .map_err(|e| anyhow::anyhow!("{}", e))?
        .with_prior_versions(prior_versions_data)
        .with_conformance_rules_json_opt(conformance_rules_json);
    let server = generator
        .generate(&orb, &orb_name, &resolved_version)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match format {
        OutputFormat::Source => {
            server
                .write_to(output)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("Generated MCP server source code:");
            println!("  Output: {}", output.display());
            println!("  Crate: {}", server.crate_name);
            println!("  Version: {}", resolved_version);
            println!("  Commands: {}", orb.commands.len());
            println!("  Jobs: {}", orb.jobs.len());
            println!("  Executors: {}", orb.executors.len());
            println!();
            println!("To build: cd {} && cargo build --release", output.display());
        }
        OutputFormat::Binary => {
            server
                .write_to(output)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("Compiling MCP server...");
            let status = std::process::Command::new("cargo")
                .args(["build", "--release"])
                .current_dir(output)
                .status();
            match status {
                Ok(s) if s.success() => {
                    let binary_path = output.join("target/release").join(&server.crate_name);
                    println!("Successfully compiled MCP server:");
                    println!("  Binary: {}", binary_path.display());
                    println!("  Version: {}", resolved_version);
                }
                Ok(_) => {
                    anyhow::bail!(
                        "Compilation failed. Source code is available at: {}",
                        output.display()
                    );
                }
                Err(e) => {
                    anyhow::bail!(
                        "Failed to run cargo: {}. Source code is available at: {}",
                        e,
                        output.display()
                    );
                }
            }
        }
    }

    Ok(())
}

fn run_validate(orb_path: &std::path::PathBuf) -> Result<()> {
    tracing::info!(?orb_path, "Validating orb definition");
    let orb = OrbParser::parse(orb_path).map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("Orb validation successful!");
    println!("  Version: {}", orb.version);
    if let Some(desc) = &orb.description {
        println!("  Description: {}", desc);
    }
    println!("  Commands: {}", orb.commands.len());
    for name in orb.commands.keys() {
        println!("    - {}", name);
    }
    println!("  Jobs: {}", orb.jobs.len());
    for name in orb.jobs.keys() {
        println!("    - {}", name);
    }
    println!("  Executors: {}", orb.executors.len());
    for name in orb.executors.keys() {
        println!("    - {}", name);
    }
    Ok(())
}

fn run_diff(
    current: &std::path::PathBuf,
    previous: &std::path::PathBuf,
    since_version: &str,
    output: &Option<std::path::PathBuf>,
) -> Result<()> {
    tracing::info!(?current, ?previous, "Diffing orb versions");

    let new_orb = OrbParser::parse(current).map_err(|e| anyhow::anyhow!("{}", e))?;
    let old_orb = OrbParser::parse(previous).map_err(|e| anyhow::anyhow!("{}", e))?;

    let rules = differ::diff(&old_orb, &new_orb, since_version);
    println!("Computed {} conformance rule(s):", rules.len());
    for rule in &rules {
        println!("  • {}", rule.description());
    }

    let json = serde_json::to_string_pretty(&rules)?;

    if let Some(out_path) = output {
        std::fs::write(out_path, &json)?;
        println!("\nRules written to: {}", out_path.display());
    } else {
        println!("\n{}", json);
    }

    Ok(())
}

fn run_migrate(
    ci_dir: &std::path::PathBuf,
    orb: &str,
    rules_path: &std::path::PathBuf,
    dry_run: bool,
) -> Result<()> {
    tracing::info!(?ci_dir, orb, "Migrating consumer config");

    let rules_json = std::fs::read_to_string(rules_path)
        .map_err(|e| anyhow::anyhow!("Failed to read rules file: {}", e))?;
    let rules: Vec<conformance_rule::ConformanceRule> = serde_json::from_str(&rules_json)
        .map_err(|e| anyhow::anyhow!("Failed to parse rules JSON: {}", e))?;

    let config = consumer_parser::ConsumerParser::parse_directory(ci_dir)
        .map_err(|e| anyhow::anyhow!("Failed to parse CI config: {}", e))?;

    let plan = migrator::Migrator::plan(&rules, &config, orb, "");
    println!("{}", plan.format_summary());

    if plan.changes.is_empty() {
        return Ok(());
    }

    if dry_run {
        println!("\n(Dry run — no files modified)");
        return Ok(());
    }

    let applied = migrator::Migrator::apply(&plan, false)?;
    println!("\n{}", applied.format_summary());

    Ok(())
}

/// Loads prior orb version snapshots from a directory of `<version>.yml` files.
fn load_prior_versions(dir: &std::path::Path) -> Result<Vec<(String, parser::OrbDefinition)>> {
    if !dir.is_dir() {
        anyhow::bail!("Prior versions directory does not exist: {}", dir.display());
    }
    let mut versions = Vec::new();
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
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
        let orb_def = OrbParser::parse(&path)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))?;
        tracing::debug!(path = %path.display(), version = %version, "Loaded prior version");
        versions.push((version, orb_def));
    }
    Ok(versions)
}

/// Loads and merges conformance rules from all `*.json` files in a directory.
fn load_conformance_rules(dir: &std::path::Path) -> Result<Vec<conformance_rule::ConformanceRule>> {
    if !dir.is_dir() {
        anyhow::bail!("Migrations directory does not exist: {}", dir.display());
    }
    let mut all_rules = Vec::new();
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
        let rules: Vec<conformance_rule::ConformanceRule> = serde_json::from_str(&json)
            .map_err(|e| anyhow::anyhow!("Failed to parse {}: {}", path.display(), e))?;
        tracing::debug!(path = %path.display(), count = rules.len(), "Loaded rules file");
        all_rules.extend(rules);
    }
    Ok(all_rules)
}

#[allow(clippy::too_many_arguments)]
fn run_prime(
    orb_path: &std::path::Path,
    git_repo: Option<&std::path::Path>,
    tag_prefix: &str,
    earliest_version: Option<&str>,
    since: Option<&str>,
    prior_versions_dir: &std::path::Path,
    migrations_dir: &std::path::Path,
    rename_map: &[String],
    ephemeral: bool,
    dry_run: bool,
) -> Result<()> {
    use chrono::Local;
    use primer::{
        discover_tags, filter_by_date, filter_by_version, since_cutoff, tag_date, PrimeConfig,
    };

    // Resolve git repo path: either provided, or walk up from orb_path
    let repo_path = if let Some(r) = git_repo {
        r.to_path_buf()
    } else {
        find_git_root(orb_path)?
    };

    // Relative orb path from repo root
    let orb_abs = orb_path
        .canonicalize()
        .unwrap_or_else(|_| orb_path.to_path_buf());
    let repo_abs = repo_path
        .canonicalize()
        .unwrap_or_else(|_| repo_path.to_path_buf());
    let orb_rel = orb_abs
        .strip_prefix(&repo_abs)
        .unwrap_or(orb_path)
        .to_path_buf();

    // Resolve output dirs
    let (pv_dir, mig_dir) = if ephemeral {
        let base =
            std::path::PathBuf::from(format!("/tmp/gen-orb-mcp-prime-{}", std::process::id()));
        (base.join("prior-versions"), base.join("migrations"))
    } else {
        (
            prior_versions_dir.to_path_buf(),
            migrations_dir.to_path_buf(),
        )
    };

    // Discover and filter tags
    let all_tags = discover_tags(&repo_path, tag_prefix)?;
    tracing::info!(count = all_tags.len(), "Discovered version tags");

    let window_versions: Vec<String> = if let Some(ver_str) = earliest_version {
        let earliest = semver::Version::parse(ver_str)
            .map_err(|e| anyhow::anyhow!("Invalid version '{}': {}", ver_str, e))?;
        filter_by_version(&all_tags, &earliest)
    } else {
        let since_str = since.unwrap_or("6 months");
        let today = Local::now().date_naive();
        let cutoff = since_cutoff(since_str, today)?;
        // Need dates for each tag
        let tags_with_dates: Vec<primer::TagWithDate> = all_tags
            .iter()
            .filter_map(|v| match tag_date(&repo_path, tag_prefix, v) {
                Ok(d) => Some(primer::TagWithDate {
                    version: v.clone(),
                    date: d,
                }),
                Err(e) => {
                    tracing::warn!(version = %v, error = %e, "Could not get tag date, skipping");
                    None
                }
            })
            .collect();
        filter_by_date(&tags_with_dates, cutoff)
    };

    tracing::info!(count = window_versions.len(), "Versions in window");

    // Parse --rename-map OLD=NEW entries into (from, to) pairs.
    let extra_rename_hints: Vec<(String, String)> = rename_map
        .iter()
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, '=');
            let from = parts.next()?.trim().to_string();
            let to = parts.next()?.trim().to_string();
            if from.is_empty() || to.is_empty() {
                tracing::warn!(entry, "--rename-map entry is malformed; skipping");
                return None;
            }
            Some((from, to))
        })
        .collect();

    let config = PrimeConfig {
        git_repo: repo_path,
        tag_prefix: tag_prefix.to_string(),
        orb_path_relative: orb_rel,
        prior_versions_dir: pv_dir.clone(),
        migrations_dir: mig_dir.clone(),
        dry_run,
        extra_rename_hints,
    };

    let result = primer::prime(&config, &window_versions)?;

    if ephemeral {
        println!("PRIME_PV_DIR={}", pv_dir.display());
        println!("PRIME_MIG_DIR={}", mig_dir.display());
    }

    println!(
        "prime: +{} snapshots, -{} snapshots, +{} migrations, -{} migrations",
        result.snapshots_added,
        result.snapshots_removed,
        result.migrations_added,
        result.migrations_removed,
    );

    Ok(())
}

/// Walk up from `start` looking for a `.git` directory.
fn find_git_root(start: &std::path::Path) -> Result<std::path::PathBuf> {
    // Canonicalise first: a relative path like "src/@orb.yml" would otherwise
    // produce Path("") when walking up past "src", and "" cannot be
    // canonicalised.  That propagates as an absolute orb_path_relative which
    // makes worktree.join() ignore the worktree entirely.
    let start = start
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Cannot access orb path '{}': {}", start.display(), e))?;
    let mut dir = if start.is_file() {
        start.parent().unwrap_or(&start).to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if dir.join(".git").exists() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(p) => dir = p.to_path_buf(),
            None => anyhow::bail!(
                "Could not find git repository root starting from '{}'",
                start.display()
            ),
        }
    }
}

/// Derive orb name from the orb path.
///
/// For unpacked orbs (`@orb.yml`), uses the project directory name.
/// Handles the common `project/src/@orb.yml` structure by skipping the `src`
/// directory. For packed orbs, uses the file stem (filename without extension).
fn derive_orb_name(path: &std::path::Path) -> String {
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("orb");

    if filename == "@orb.yml" {
        // Get parent directory
        let parent = path.parent();
        let parent_name = parent.and_then(|p| p.file_name()).and_then(|s| s.to_str());

        // If parent is "src", go up one more level to get project name
        if parent_name == Some("src") {
            parent
                .and_then(|p| p.parent())
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("orb")
                .to_string()
        } else {
            parent_name.unwrap_or("orb").to_string()
        }
    } else {
        // Use filename without extension
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("orb")
            .to_string()
    }
}

/// Discover the latest version tag in a git repository with the given prefix.
///
/// Returns `None` when no matching tags exist. On error (e.g. not a git repo),
/// returns `Ok(None)` rather than propagating so callers fall through to the
/// next resolution strategy.
fn discover_latest_version(repo: &std::path::Path, tag_prefix: &str) -> Result<Option<String>> {
    use primer::discover_tags;
    let tags = discover_tags(repo, tag_prefix).unwrap_or_default();
    // discover_tags returns versions sorted ascending; highest is last
    Ok(tags.into_iter().last())
}

/// Resolve the version to use for the generated MCP server.
///
/// # Version Resolution Rules (priority order)
///
/// 1. Explicit `--version` — always wins
/// 2. `git_hint` — version auto-discovered from git tags via `--git-repo`
/// 3. Fresh generation with no hints — `DEFAULT_VERSION`
/// 4. Existing output with no version — error (must specify `--version`)
///
/// The `--force` flag is required when overwriting existing output.
fn resolve_version(
    output: &std::path::Path,
    version: Option<&str>,
    force: bool,
    git_hint: Option<&str>,
) -> Result<String> {
    let cargo_toml = output.join("Cargo.toml");
    let output_exists = cargo_toml.exists();

    // Explicit version always wins (with force check if output exists)
    if let Some(v) = version {
        if output_exists && !force {
            anyhow::bail!(
                "Output directory '{}' already exists. Use --force to overwrite.",
                output.display()
            );
        }
        tracing::debug!("Using provided version");
        return Ok(v.to_string());
    }

    // Git-discovered version
    if let Some(v) = git_hint {
        if output_exists && !force {
            anyhow::bail!(
                "Output directory '{}' already exists. Use --force to overwrite.",
                output.display()
            );
        }
        tracing::debug!(version = %v, "Using git-discovered version");
        return Ok(v.to_string());
    }

    // No version available — refuse to generate with an unknown version
    let msg = if output_exists {
        format!(
            "Output directory '{}' already exists and no version could be determined.\n\
             Provide the version explicitly:\n\n\
             \x20   gen-orb-mcp generate --orb-path <PATH> --output {} --version <VERSION> --force\n\n\
             Or ensure --orb-path is inside a git repository with version tags (e.g. v6.0.0).\n\
             Use --tag-prefix if your tags use a non-standard prefix.",
            output.display(),
            output.display()
        )
    } else {
        format!(
            "No version could be determined for the generated MCP server.\n\
             Provide the version explicitly:\n\n\
             \x20   gen-orb-mcp generate --orb-path <PATH> --output {} --version <VERSION>\n\n\
             Or ensure --orb-path is inside a git repository with version tags (e.g. v6.0.0).\n\
             Use --tag-prefix if your tags use a non-standard prefix.",
            output.display()
        )
    };
    anyhow::bail!(msg)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_cli_parse_generate() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_generate_with_version() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--version",
            "1.2.3",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_generate_with_force() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--version",
            "1.2.3",
            "--force",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_validate() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "validate", "--orb-path", "test.yml"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_derive_orb_name_from_orb_yml() {
        use std::path::Path;
        // Standard orb structure: project/src/@orb.yml -> "project"
        let path = Path::new("/path/to/my-toolkit/src/@orb.yml");
        assert_eq!(derive_orb_name(path), "my-toolkit");

        // Non-standard structure without src: my-orb/@orb.yml -> "my-orb"
        let path = Path::new("my-orb/@orb.yml");
        assert_eq!(derive_orb_name(path), "my-orb");

        // Edge case: src/@orb.yml at root -> "orb" (no grandparent, falls back to
        // default)
        let path = Path::new("src/@orb.yml");
        assert_eq!(derive_orb_name(path), "orb");
    }

    #[test]
    fn test_derive_orb_name_from_packed() {
        use std::path::Path;
        let path = Path::new("/path/to/my-toolkit.yml");
        assert_eq!(derive_orb_name(path), "my-toolkit");

        let path = Path::new("orb.yml");
        assert_eq!(derive_orb_name(path), "orb");
    }

    #[test]
    fn test_resolve_version_fresh_with_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let result = resolve_version(temp_dir.path(), Some("2.0.0"), false, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2.0.0");
    }

    #[test]
    fn test_resolve_version_fresh_no_version_errors() {
        let temp_dir = TempDir::new().unwrap();
        let result = resolve_version(temp_dir.path(), None, false, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_version_existing_without_version_fails() {
        let temp_dir = TempDir::new().unwrap();
        // Create a Cargo.toml to simulate existing output
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();

        let result = resolve_version(temp_dir.path(), None, false, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("already exists"));
        assert!(err.contains("--version"));
    }

    #[test]
    fn test_resolve_version_existing_with_version_no_force_fails() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();

        let result = resolve_version(temp_dir.path(), Some("1.5.0"), false, None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("--force"));
    }

    #[test]
    fn test_resolve_version_existing_with_version_and_force_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(
            temp_dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();

        let result = resolve_version(temp_dir.path(), Some("1.5.0"), true, None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1.5.0");
    }

    #[test]
    fn test_cli_parse_generate_with_prior_versions() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--prior-versions",
            "./prior",
        ]);
        assert!(cli.is_ok(), "expected --prior-versions flag to be accepted");
    }

    // Tests 11-15: prime command CLI parsing

    #[test]
    fn test_cli_parse_prime_defaults() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "prime"]);
        assert!(cli.is_ok(), "prime with all defaults should parse");
        if let Commands::Prime {
            orb_path,
            tag_prefix,
            earliest_version,
            since,
            prior_versions_dir,
            migrations_dir,
            rename_map,
            ephemeral,
            dry_run,
            git_repo,
        } = cli.unwrap().command
        {
            assert_eq!(orb_path.to_str().unwrap(), "src/@orb.yml");
            assert_eq!(tag_prefix, "v");
            assert!(earliest_version.is_none());
            assert!(since.is_none());
            assert_eq!(prior_versions_dir.to_str().unwrap(), "prior-versions");
            assert_eq!(migrations_dir.to_str().unwrap(), "migrations");
            assert!(rename_map.is_empty());
            assert!(!ephemeral);
            assert!(!dry_run);
            assert!(git_repo.is_none());
        } else {
            panic!("expected Prime variant");
        }
    }

    #[test]
    fn test_cli_parse_prime_earliest_version() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "prime", "--earliest-version", "4.1.0"]);
        assert!(cli.is_ok(), "prime --earliest-version should parse");
        if let Commands::Prime {
            earliest_version, ..
        } = cli.unwrap().command
        {
            assert_eq!(earliest_version.as_deref(), Some("4.1.0"));
        } else {
            panic!("expected Prime variant");
        }
    }

    #[test]
    fn test_cli_parse_prime_since() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "prime", "--since", "3 months"]);
        assert!(cli.is_ok(), "prime --since should parse");
        if let Commands::Prime { since, .. } = cli.unwrap().command {
            assert_eq!(since.as_deref(), Some("3 months"));
        } else {
            panic!("expected Prime variant");
        }
    }

    #[test]
    fn test_cli_parse_prime_exclusive_flags() {
        // --earliest-version and --since are mutually exclusive
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "prime",
            "--earliest-version",
            "4.1.0",
            "--since",
            "6 months",
        ]);
        assert!(
            cli.is_err(),
            "prime with both --earliest-version and --since should be rejected"
        );
    }

    #[test]
    fn test_cli_parse_prime_rename_map() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "prime",
            "--rename-map",
            "common_tests_rolling=common_tests",
            "--rename-map",
            "required_builds_rolling=required_builds",
        ]);
        assert!(cli.is_ok(), "prime --rename-map should parse");
        if let Commands::Prime { rename_map, .. } = cli.unwrap().command {
            assert_eq!(rename_map.len(), 2);
            assert!(rename_map.contains(&"common_tests_rolling=common_tests".to_string()));
            assert!(rename_map.contains(&"required_builds_rolling=required_builds".to_string()));
        } else {
            panic!("expected Prime variant");
        }
    }

    #[test]
    fn test_cli_parse_prime_ephemeral() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "prime", "--ephemeral"]);
        assert!(cli.is_ok(), "prime --ephemeral should parse");
        if let Commands::Prime { ephemeral, .. } = cli.unwrap().command {
            assert!(ephemeral);
        } else {
            panic!("expected Prime variant");
        }
    }

    // Serialises tests that mutate the global CWD.
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Regression test: `find_git_root` with a *relative* orb path must return
    /// an **absolute** path.
    ///
    /// When the user runs `gen-orb-mcp prime --orb-path src/@orb.yml` (the
    /// default), `orb_path` is relative.  `find_git_root` walks up from
    /// `src/@orb.yml` → `src` → `""` (Rust `Path::parent` of `"src"` is `""`).
    /// If the function returns `""`, `repo_abs` cannot be canonicalised, so
    /// `strip_prefix("")` on the absolute `orb_abs` returns the full absolute
    /// path.  `worktree.join(absolute_path)` then ignores the worktree and reads
    /// the current working copy — producing snapshots with current-version
    /// content for every historical tag.
    ///
    /// The fix: canonicalise `start` at the top of `find_git_root` so the
    /// walk-up always operates on absolute paths and returns an absolute result.
    #[test]
    fn test_find_git_root_returns_absolute_path_for_relative_input() {
        let _cwd_guard = CWD_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();

        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(
            tmp.path().join("src").join("@orb.yml"),
            "version: 2.1\ndescription: test",
        )
        .unwrap();

        // Change to the fake repo root so that "src/@orb.yml" is a valid
        // relative path.
        std::env::set_current_dir(tmp.path()).unwrap();

        let result = find_git_root(std::path::Path::new("src/@orb.yml"));

        // Always restore CWD before asserting so a failure doesn't leave the
        // process in the tmp directory.
        std::env::set_current_dir(&original).unwrap();

        let result = result.expect("find_git_root should succeed");
        assert!(
            result.is_absolute(),
            "find_git_root must return an absolute path, got: {:?}",
            result
        );
        assert_eq!(
            result.canonicalize().unwrap(),
            tmp.path().canonicalize().unwrap(),
        );
    }

    // --- Tests for discover_latest_version ---

    #[test]
    fn test_discover_latest_version_returns_none_for_no_tags() {
        let tmp = TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        let result = discover_latest_version(tmp.path(), "v");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_discover_latest_version_returns_highest_semver_tag() {
        let tmp = TempDir::new().unwrap();
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("README.md"), "test").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        for tag in ["v1.0.0", "v2.0.0", "v1.5.0"] {
            std::process::Command::new("git")
                .args(["tag", tag])
                .current_dir(tmp.path())
                .output()
                .unwrap();
        }
        let result = discover_latest_version(tmp.path(), "v");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("2.0.0".to_string()));
    }

    #[test]
    fn test_resolve_version_uses_git_hint_when_no_explicit_version() {
        let temp_dir = TempDir::new().unwrap();
        let result = resolve_version(temp_dir.path(), None, false, Some("3.1.0"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3.1.0");
    }

    #[test]
    fn test_resolve_version_explicit_overrides_git_hint() {
        let temp_dir = TempDir::new().unwrap();
        let result = resolve_version(temp_dir.path(), Some("5.0.0"), false, Some("3.1.0"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "5.0.0");
    }

    #[test]
    fn test_resolve_version_errors_without_version_or_hint() {
        let temp_dir = TempDir::new().unwrap();
        let result = resolve_version(temp_dir.path(), None, false, None);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("No version could be determined"), "got: {msg}");
    }

    #[test]
    fn test_cli_parse_generate_with_tag_prefix() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--tag-prefix",
            "orb-v",
        ]);
        assert!(cli.is_ok(), "generate --tag-prefix should parse");
        if let Commands::Generate { tag_prefix, .. } = cli.unwrap().command {
            assert_eq!(tag_prefix, "orb-v");
        } else {
            panic!("expected Generate variant");
        }
    }

    #[test]
    fn test_cli_parse_generate_tag_prefix_defaults_to_v() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
        ]);
        assert!(cli.is_ok());
        if let Commands::Generate { tag_prefix, .. } = cli.unwrap().command {
            assert_eq!(tag_prefix, "v");
        } else {
            panic!("expected Generate variant");
        }
    }
}
