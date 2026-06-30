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
        #[arg(short = 'p', long, default_value = "src/@orb.yml")]
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
        #[arg(long = "crate-version")]
        crate_version: Option<String>,

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
        #[arg(short = 'p', long, default_value = "src/@orb.yml")]
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
        /// Format: `OLD=NEW`, e.g. `--rename-map
        /// common_tests_rolling=common_tests`. Manual entries take
        /// precedence over git-detected hints for matching old names.
        /// Use this when commits cannot be restructured to follow
        /// the two-commit rename rule.
        #[arg(long, value_name = "OLD=NEW")]
        rename_map: Vec<String>,

        /// Describe actions without writing any files
        #[arg(long)]
        dry_run: bool,
    },
    /// Stage, commit, and push generated artifacts back to the repository
    ///
    /// Idempotent: if the working tree is clean after staging the specified
    /// paths, exits successfully without creating an empty commit.
    /// The default commit message includes [skip ci] to prevent CI
    /// re-triggering.
    Save {
        /// Paths to stage and commit (relative to repository root).
        ///
        /// Repeatable (`--paths a --paths b`) or comma-separated
        /// (`--paths a,b`) so a single orb parameter can carry multiple paths.
        #[arg(long, required = true, value_delimiter = ',')]
        paths: Vec<std::path::PathBuf>,

        /// Commit message
        #[arg(
            short = 'm',
            long,
            default_value = "chore: update generated MCP server artifacts [skip ci]"
        )]
        message: String,

        /// Push after committing (default: true)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        push: bool,

        /// Stage and commit only, do not push
        #[arg(long, conflicts_with = "push")]
        no_push: bool,

        /// Show what would be committed without writing anything
        #[arg(long)]
        dry_run: bool,

        /// Use GPG-signed commit and GitHub App token push.
        ///
        /// Reads the GPG key, ownertrust, commit name/email and signing key id
        /// from env vars whose NAMES default to GPG_KEY / GPG_TRUST /
        /// GIT_USER_NAME / GIT_USER_EMAIL / GPG_SIGN_KEY and are configurable
        /// via gen-orb-mcp.toml ([sign]) or the --*-env flags. Also reads
        /// GITHUB_TOKEN (GitHub App token), CIRCLE_PROJECT_USERNAME,
        /// CIRCLE_PROJECT_REPONAME, CIRCLE_BRANCH.
        #[arg(long)]
        sign: bool,

        /// Path to the config file (default: gen-orb-mcp.toml in cwd).
        #[arg(long)]
        config: Option<std::path::PathBuf>,

        /// Env var NAME for the base64 GPG key (default GPG_KEY;
        /// [sign].gpg_key_env).
        #[arg(long)]
        gpg_key_env: Option<String>,

        /// Env var NAME for the GPG ownertrust (default GPG_TRUST;
        /// [sign].trust_env).
        #[arg(long)]
        trust_env: Option<String>,

        /// Env var NAME for the commit author name (default GIT_USER_NAME;
        /// [sign].user_name_env).
        #[arg(long)]
        user_name_env: Option<String>,

        /// Env var NAME for the commit author email (default GIT_USER_EMAIL;
        /// [sign].user_email_env).
        #[arg(long)]
        user_email_env: Option<String>,

        /// Env var NAME for the GPG signing key id (default GPG_SIGN_KEY;
        /// [sign].sign_key_env).
        #[arg(long)]
        sign_key_env: Option<String>,
    },
    /// Upload a compiled binary to an existing GitHub release as a release
    /// asset
    ///
    /// The GitHub release must already exist before this command is run.
    /// Set GITHUB_TOKEN, CIRCLE_PROJECT_USERNAME, CIRCLE_PROJECT_REPONAME,
    /// and CIRCLE_TAG (or use --tag) in the environment.
    Publish {
        /// Orb binary base name (e.g. "gen-orb-mcp"). Derives the binary path
        /// and asset name when --binary / --asset-name are not given:
        ///   binary = <input>/target/release/<name_underscored>_mcp
        ///   asset  = <name_underscored>_mcp-linux-x86_64
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Directory containing the compiled `target/release` (used with
        /// --name).
        #[arg(short = 'i', long, default_value = "./dist")]
        input: std::path::PathBuf,

        /// Path to the binary file to upload (overrides derivation from --name)
        #[arg(short = 'b', long)]
        binary: Option<std::path::PathBuf>,

        /// Name for the release asset, e.g. my-orb-mcp-linux-x86_64
        /// (overrides derivation from --name)
        #[arg(short = 'a', long)]
        asset_name: Option<String>,

        /// Release tag to publish to. When omitted, read from the env var named
        /// by --tag-env / [publish].tag_env (default CIRCLE_TAG).
        #[arg(long)]
        tag: Option<String>,

        /// Env var NAME holding the release tag when --tag is not given
        /// (default CIRCLE_TAG; config [publish].tag_env).
        #[arg(long)]
        tag_env: Option<String>,

        /// Path to the config file (default: gen-orb-mcp.toml in cwd).
        #[arg(long)]
        config: Option<std::path::PathBuf>,

        /// Describe the upload without performing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Compile generated MCP server source to a native binary
    Build {
        /// Directory containing generated MCP server source
        #[arg(short = 'i', long)]
        input: std::path::PathBuf,

        /// Override the binary name (default: derived from Cargo.toml)
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Rust target triple (default: host)
        #[arg(long)]
        target: Option<String>,

        /// Print the cargo command without running it
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
                crate_version,
                force,
                migrations,
                prior_versions,
                tag_prefix,
            } => run_generate(
                orb_path,
                output,
                format,
                name,
                crate_version,
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
            Commands::Save {
                paths,
                message,
                push,
                no_push,
                dry_run,
                sign,
                config,
                gpg_key_env,
                trust_env,
                user_name_env,
                user_email_env,
                sign_key_env,
            } => {
                let config_path = config
                    .clone()
                    .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_CONFIG_FILE));
                let overrides = SignEnvNameOverrides {
                    gpg_key_env: gpg_key_env.clone(),
                    trust_env: trust_env.clone(),
                    user_name_env: user_name_env.clone(),
                    user_email_env: user_email_env.clone(),
                    sign_key_env: sign_key_env.clone(),
                };
                run_save(
                    paths,
                    message,
                    *push && !*no_push,
                    *dry_run,
                    *sign,
                    &config_path,
                    &overrides,
                )
            }
            Commands::Publish {
                name,
                input,
                binary,
                asset_name,
                tag,
                tag_env,
                config,
                dry_run,
            } => {
                let config_path = config
                    .clone()
                    .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_CONFIG_FILE));
                PublishJob {
                    name: name.as_deref(),
                    input,
                    binary: binary.as_deref(),
                    asset_name: asset_name.as_deref(),
                    tag: tag.as_deref(),
                    dry_run: *dry_run,
                    config_path: &config_path,
                    tag_env_override: tag_env.as_deref(),
                }
                .run()
            }
            Commands::Build {
                input,
                name,
                target,
                dry_run,
            } => run_build(input, name.as_deref(), target.as_deref(), *dry_run),
        }
    }
}

fn run_generate(
    orb_path: &std::path::PathBuf,
    output: &std::path::PathBuf,
    format: &OutputFormat,
    name: &Option<String>,
    crate_version: &Option<String>,
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
    let resolved_version =
        resolve_version(output, crate_version.as_deref(), force, git_hint.as_deref())?;
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

/// Config file auto-discovered in the working directory (override with
/// --config).
const DEFAULT_CONFIG_FILE: &str = "gen-orb-mcp.toml";
/// Generic default env-var NAMES for the signing inputs — deliberately free of
/// any org-specific convention. A consumer maps them to their own secret names
/// once via `gen-orb-mcp.toml` ([sign]) or per-call `--*-env` flags.
const DEFAULT_GPG_KEY_ENV: &str = "GPG_KEY";
const DEFAULT_TRUST_ENV: &str = "GPG_TRUST";
const DEFAULT_USER_NAME_ENV: &str = "GIT_USER_NAME";
const DEFAULT_USER_EMAIL_ENV: &str = "GIT_USER_EMAIL";
const DEFAULT_SIGN_KEY_ENV: &str = "GPG_SIGN_KEY";
/// Default env-var NAME holding the release tag for `publish`.
const DEFAULT_TAG_ENV: &str = "CIRCLE_TAG";

#[derive(Debug)]
struct SignEnv {
    gpg_key_b64: String,
    gpg_trust: String,
    user_name: String,
    user_email: String,
    sign_key: String,
}

/// The env-var NAMES (not values) from which `read_sign_env` reads the signing
/// inputs. Resolved by precedence: `--*-env` flag > `gen-orb-mcp.toml` [sign] >
/// generic default.
#[derive(Debug, Clone)]
struct SignEnvNames {
    gpg_key: String,
    trust: String,
    user_name: String,
    user_email: String,
    sign_key: String,
}

/// Per-call CLI overrides for the signing env-var names (highest precedence).
#[derive(Debug, Default, Clone)]
struct SignEnvNameOverrides {
    gpg_key_env: Option<String>,
    trust_env: Option<String>,
    user_name_env: Option<String>,
    user_email_env: Option<String>,
    sign_key_env: Option<String>,
}

/// Resolve the env-var NAMES for the signing inputs. Only names are configured
/// here; the secret/identifier VALUES are read from those vars in
/// `read_sign_env`, so nothing private is committed or passed on the CLI.
fn resolve_sign_env_names(
    config_path: &std::path::Path,
    overrides: &SignEnvNameOverrides,
) -> Result<SignEnvNames> {
    let mut builder = config::Config::builder()
        .set_default("sign.gpg_key_env", DEFAULT_GPG_KEY_ENV)?
        .set_default("sign.trust_env", DEFAULT_TRUST_ENV)?
        .set_default("sign.user_name_env", DEFAULT_USER_NAME_ENV)?
        .set_default("sign.user_email_env", DEFAULT_USER_EMAIL_ENV)?
        .set_default("sign.sign_key_env", DEFAULT_SIGN_KEY_ENV)?
        .add_source(config::File::from(config_path).required(false));
    if let Some(v) = overrides.gpg_key_env.as_deref() {
        builder = builder.set_override("sign.gpg_key_env", v)?;
    }
    if let Some(v) = overrides.trust_env.as_deref() {
        builder = builder.set_override("sign.trust_env", v)?;
    }
    if let Some(v) = overrides.user_name_env.as_deref() {
        builder = builder.set_override("sign.user_name_env", v)?;
    }
    if let Some(v) = overrides.user_email_env.as_deref() {
        builder = builder.set_override("sign.user_email_env", v)?;
    }
    if let Some(v) = overrides.sign_key_env.as_deref() {
        builder = builder.set_override("sign.sign_key_env", v)?;
    }
    let cfg = builder.build()?;
    Ok(SignEnvNames {
        gpg_key: cfg.get_string("sign.gpg_key_env")?,
        trust: cfg.get_string("sign.trust_env")?,
        user_name: cfg.get_string("sign.user_name_env")?,
        user_email: cfg.get_string("sign.user_email_env")?,
        sign_key: cfg.get_string("sign.sign_key_env")?,
    })
}

/// Resolve the env-var NAME holding the release tag (used when `--tag` is not
/// given). Precedence: `--tag-env` flag > `gen-orb-mcp.toml` [publish].tag_env
/// > `CIRCLE_TAG`.
fn resolve_tag_env_name(
    config_path: &std::path::Path,
    override_name: Option<&str>,
) -> Result<String> {
    if let Some(v) = override_name {
        return Ok(v.to_string());
    }
    let cfg = config::Config::builder()
        .set_default("publish.tag_env", DEFAULT_TAG_ENV)?
        .add_source(config::File::from(config_path).required(false))
        .build()?;
    Ok(cfg.get_string("publish.tag_env")?)
}

fn read_sign_env(names: &SignEnvNames) -> Result<SignEnv> {
    let read = |name: &str| -> Result<String> {
        std::env::var(name)
            .map_err(|_| anyhow::anyhow!("{name} env var not set (required with --sign)"))
    };
    Ok(SignEnv {
        gpg_key_b64: read(&names.gpg_key)?,
        gpg_trust: read(&names.trust)?,
        user_name: read(&names.user_name)?,
        user_email: read(&names.user_email)?,
        sign_key: read(&names.sign_key)?,
    })
}

fn build_pcu_config() -> Result<config::Config> {
    // PCU_APP_ID and PCU_PRIVATE_KEY (if present via pcu-app context) are
    // picked up automatically by the PCU_ prefix source and used for GitHub
    // App auth, which carries branch-protection bypass authority.
    // GITHUB_TOKEN is registered as a PAT fallback for environments without
    // App credentials.
    let mut builder = config::Config::builder()
        .set_default("prlog", "PRLOG.md")?
        .set_default("branch", "CIRCLE_BRANCH")?
        .set_default("default_branch", "main")?
        .set_default("username", "CIRCLE_PROJECT_USERNAME")?
        .set_default("reponame", "CIRCLE_PROJECT_REPONAME")?
        .set_override("command", "push")?
        .add_source(config::Environment::with_prefix("PCU"));
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        builder = builder.set_default("pat", token)?;
    }
    Ok(builder.build()?)
}

fn run_save(
    paths: &[std::path::PathBuf],
    message: &str,
    push: bool,
    dry_run: bool,
    sign: bool,
    config_path: &std::path::Path,
    sign_overrides: &SignEnvNameOverrides,
) -> Result<()> {
    if sign {
        let names = resolve_sign_env_names(config_path, sign_overrides)?;
        let sign_env = read_sign_env(&names)?;
        pcu::import_gpg_key(&sign_env.gpg_key_b64, &sign_env.gpg_trust)
            .map_err(|e| anyhow::anyhow!("GPG import failed: {e}"))?;
        // The commit identity and signing key are passed explicitly to pcu via
        // SignConfig (below), so no git-config setup is needed — this avoids the
        // CI config-visibility fragility (safe.directory / dubious ownership).
        run_save_signed(paths, message, push, dry_run, &sign_env)
    } else {
        run_save_unsigned(paths, message, push, dry_run)
    }
}

fn run_save_signed(
    paths: &[std::path::PathBuf],
    message: &str,
    push: bool,
    dry_run: bool,
    sign_env: &SignEnv,
) -> Result<()> {
    let pcu_config = build_pcu_config()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let client = rt
        .block_on(pcu::Client::new_with(&pcu_config))
        .map_err(|e| anyhow::anyhow!("Failed to create pcu client: {}", e))?;

    use pcu::GitOps;
    let path_refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
    client
        .stage_paths(&path_refs)
        .map_err(|e| anyhow::anyhow!("Failed to stage paths: {e}"))?;

    // Open a fresh repo handle after staging so the index reflects the
    // changes written to disk by client.stage_paths().
    let repo = git2::Repository::discover(".")
        .map_err(|e| anyhow::anyhow!("Not inside a git repository: {}", e))?;
    let mut index = repo.index()?;
    let head_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let diff = save_compute_diff(&repo, &mut index, head_commit.as_ref())?;

    if diff.deltas().count() == 0 {
        println!("Nothing to commit — working tree clean after staging.");
        return Ok(());
    }
    if dry_run {
        save_print_dry_run(&diff, message, push);
        return Ok(());
    }

    // Supply the commit identity and GPG signing key explicitly so pcu does not
    // read them from git config (which is not reliably visible to its repo
    // handle in CI).
    let sign_config = pcu::SignConfig::new(pcu::Sign::Gpg)
        .with_identity(&sign_env.user_name, &sign_env.user_email)
        .with_signing_key(&sign_env.sign_key);
    client
        .commit_staged(sign_config, message, "", None)
        .map_err(|e| anyhow::anyhow!("Failed to sign and commit: {}", e))?;
    println!("Created signed commit: {message}");
    if push {
        client
            .push_commit("", None, false, &sign_env.user_name)
            .map_err(|e| anyhow::anyhow!("Failed to push: {}", e))?;
        println!("Pushed to remote.");
    }
    Ok(())
}

fn run_save_unsigned(
    paths: &[std::path::PathBuf],
    message: &str,
    push: bool,
    dry_run: bool,
) -> Result<()> {
    let repo = git2::Repository::discover(".")
        .map_err(|e| anyhow::anyhow!("Not inside a git repository: {}", e))?;
    let mut index = repo.index()?;
    let path_strs: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
    index
        .add_all(path_strs.iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| anyhow::anyhow!("Failed to stage paths: {e}"))?;
    index.write()?;
    let head_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let diff = save_compute_diff(&repo, &mut index, head_commit.as_ref())?;

    if diff.deltas().count() == 0 {
        println!("Nothing to commit — working tree clean after staging.");
        return Ok(());
    }
    if dry_run {
        save_print_dry_run(&diff, message, push);
        return Ok(());
    }

    let oid = save_create_commit(&repo, &mut index, message, head_commit.as_ref())?;
    tracing::info!(commit = %oid, "Created commit");
    println!("Created commit {oid}: {message}");
    if push {
        save_git_push(&repo)?;
    }
    Ok(())
}

fn save_compute_diff<'repo>(
    repo: &'repo git2::Repository,
    index: &mut git2::Index,
    head_commit: Option<&git2::Commit<'_>>,
) -> Result<git2::Diff<'repo>> {
    let new_tree_oid = index.write_tree()?;
    let new_tree = repo.find_tree(new_tree_oid)?;
    let head_tree = head_commit.map(|c| c.tree()).transpose()?;
    Ok(repo.diff_tree_to_tree(head_tree.as_ref(), Some(&new_tree), None)?)
}

fn save_print_dry_run(diff: &git2::Diff<'_>, message: &str, push: bool) {
    println!("Would commit the following changes:");
    for delta in diff.deltas() {
        let path = delta
            .new_file()
            .path()
            .and_then(|p| p.to_str())
            .unwrap_or("(unknown)");
        println!("  {path}");
    }
    println!("Commit message: {message}");
    if push {
        println!("Would push after committing.");
    }
}

fn save_create_commit(
    repo: &git2::Repository,
    index: &mut git2::Index,
    message: &str,
    head_commit: Option<&git2::Commit<'_>>,
) -> Result<git2::Oid> {
    let sig = repo.signature()?;
    let new_tree_oid = index.write_tree()?;
    let new_tree = repo.find_tree(new_tree_oid)?;
    let parents: Vec<&git2::Commit> = head_commit.into_iter().collect();
    Ok(repo.commit(Some("HEAD"), &sig, &sig, message, &new_tree, &parents)?)
}

fn save_git_push(repo: &git2::Repository) -> Result<()> {
    // git2 0.21: StringArray::iter() yields Result<Option<&str>, Error>;
    // keep the first valid UTF-8 remote name, defaulting to "origin".
    let remote_name = repo
        .remotes()?
        .iter()
        .filter_map(|r| r.ok().flatten())
        .next()
        .unwrap_or("origin")
        .to_string();

    let mut callbacks = git2::RemoteCallbacks::new();
    let git_config = repo.config()?;
    let mut cred_handler = git2_credentials::CredentialHandler::new(git_config);
    callbacks.credentials(move |url, username, allowed| {
        cred_handler.try_next_credential(url, username, allowed)
    });

    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(callbacks);

    let head_ref = repo.head()?;
    // git2 0.21: Reference::shorthand() returns Result<&str, Error>.
    let branch_name = head_ref
        .shorthand()
        .map_err(|e| anyhow::anyhow!("HEAD has no branch name: {e}"))?;
    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");

    let mut remote = repo.find_remote(&remote_name)?;
    remote
        .push(&[refspec.as_str()], Some(&mut push_opts))
        .map_err(|e| anyhow::anyhow!("Push failed: {}", e))?;

    println!("Pushed to {remote_name}/{branch_name}");
    Ok(())
}

/// Resolve the binary path and release asset name for `publish`.
///
/// Explicit `--binary` / `--asset-name` take precedence; otherwise both are
/// derived from `--name` and the `input` directory:
///   binary = `<input>/target/release/<name_underscored>_mcp`
///   asset  = `<name_underscored>_mcp-linux-x86_64`
fn resolve_publish_target(
    name: Option<&str>,
    input: &std::path::Path,
    binary: Option<&std::path::Path>,
    asset_name: Option<&str>,
) -> Result<(std::path::PathBuf, String)> {
    let derived = name.map(|n| {
        let underscored = n.replace('-', "_");
        let bin = input
            .join("target")
            .join("release")
            .join(format!("{underscored}_mcp"));
        let asset = format!("{underscored}_mcp-linux-x86_64");
        (bin, asset)
    });

    let resolved_binary = binary
        .map(std::path::Path::to_path_buf)
        .or_else(|| derived.as_ref().map(|(bin, _)| bin.clone()))
        .ok_or_else(|| anyhow::anyhow!("publish requires --binary or --name"))?;
    let resolved_asset = asset_name
        .map(str::to_string)
        .or_else(|| derived.as_ref().map(|(_, asset)| asset.clone()))
        .ok_or_else(|| anyhow::anyhow!("publish requires --asset-name or --name"))?;

    Ok((resolved_binary, resolved_asset))
}

/// Inputs for the `publish` command, captured so the run logic is a method on
/// the data rather than a many-argument free function.
struct PublishJob<'a> {
    name: Option<&'a str>,
    input: &'a std::path::Path,
    binary: Option<&'a std::path::Path>,
    asset_name: Option<&'a str>,
    tag: Option<&'a str>,
    dry_run: bool,
    config_path: &'a std::path::Path,
    tag_env_override: Option<&'a str>,
}

impl PublishJob<'_> {
    fn run(self) -> Result<()> {
        let (binary, asset_name) =
            resolve_publish_target(self.name, self.input, self.binary, self.asset_name)?;
        let binary = binary.as_path();
        let asset_name = asset_name.as_str();
        if !binary.exists() {
            anyhow::bail!("Binary not found: {}", binary.display());
        }

        let resolved_tag = match self.tag {
            Some(t) => t.to_string(),
            None => {
                let tag_env_name = resolve_tag_env_name(self.config_path, self.tag_env_override)?;
                std::env::var(&tag_env_name).map_err(|_| {
                    anyhow::anyhow!(
                        "No release tag provided. Set {tag_env_name} or use --tag <TAG>"
                    )
                })?
            }
        };

        if self.dry_run {
            let owner = std::env::var("CIRCLE_PROJECT_USERNAME").unwrap_or_default();
            let repo_name = std::env::var("CIRCLE_PROJECT_REPONAME").unwrap_or_default();
            println!("Would upload release asset (dry run):");
            println!("  Binary:     {}", binary.display());
            println!("  Asset name: {asset_name}");
            println!("  Tag:        {resolved_tag}");
            if !owner.is_empty() && !repo_name.is_empty() {
                println!("  Repo:       {owner}/{repo_name}");
            }
            return Ok(());
        }

        let pcu_config = build_pcu_config()?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let client = pcu::Client::new_with(&pcu_config)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create pcu client: {e}"))?;
                client
                    .upload_release_asset(&resolved_tag, binary, asset_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to upload release asset: {e}"))
            })
    }
}

fn run_build(
    input: &std::path::Path,
    name: Option<&str>,
    target: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    let cargo_toml = input.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!(
            "No Cargo.toml found in input directory: {}",
            input.display()
        );
    }

    let binary_name = match name {
        Some(n) => n.to_string(),
        None => read_crate_name(input)?,
    };

    let mut cargo_args = vec!["build", "--release"];
    if let Some(t) = target {
        cargo_args.extend(["--target", t]);
    }

    let binary_dir = match target {
        Some(t) => input.join("target").join(t).join("release"),
        None => input.join("target").join("release"),
    };
    let binary_path = binary_dir.join(&binary_name);

    if dry_run {
        println!("Would run: cargo {}", cargo_args.join(" "));
        println!("  Input:  {}", input.display());
        println!("  Binary: {}", binary_path.display());
        return Ok(());
    }

    tracing::info!(input = %input.display(), binary = %binary_path.display(), "Compiling MCP server");
    println!("Compiling MCP server...");
    let status = std::process::Command::new("cargo")
        .args(&cargo_args)
        .current_dir(input)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to run cargo: {}", e))?;

    if !status.success() {
        anyhow::bail!(
            "cargo build failed. Source code is available at: {}",
            input.display()
        );
    }

    println!("Successfully compiled MCP server:");
    println!("  Binary: {}", binary_path.display());

    Ok(())
}

fn read_crate_name(input: &std::path::Path) -> Result<String> {
    let content = std::fs::read_to_string(input.join("Cargo.toml"))
        .map_err(|e| anyhow::anyhow!("Failed to read Cargo.toml: {}", e))?;
    parse_package_name(&content)
        .ok_or_else(|| anyhow::anyhow!("Could not find [package] name in Cargo.toml"))
}

/// Extract the `name` field from the `[package]` section of a Cargo.toml
/// string.
fn parse_package_name(toml: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
        } else if trimmed.starts_with('[') {
            in_package = false;
        } else if in_package {
            if let Some(name) = parse_name_assignment(trimmed) {
                return Some(name);
            }
        }
    }
    None
}

/// Parse a `name = "value"` assignment line, returning the unquoted value.
fn parse_name_assignment(line: &str) -> Option<String> {
    let rest = line.strip_prefix("name")?;
    let rest = rest.trim().strip_prefix('=')?;
    let name = rest.trim().trim_matches('"').trim_matches('\'').to_string();
    (!name.is_empty()).then_some(name)
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
             \x20   gen-orb-mcp generate --orb-path <PATH> --output {} --crate-version <VERSION> --force\n\n\
             Or ensure --orb-path is inside a git repository with version tags (e.g. v6.0.0).\n\
             Use --tag-prefix if your tags use a non-standard prefix.",
            output.display(),
            output.display()
        )
    } else {
        format!(
            "No version could be determined for the generated MCP server.\n\
             Provide the version explicitly:\n\n\
             \x20   gen-orb-mcp generate --orb-path <PATH> --output {} --crate-version <VERSION>\n\n\
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
    fn test_cli_parse_generate_default_orb_path() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "generate"]);
        assert!(
            cli.is_ok(),
            "generate should work without --orb-path (default: src/@orb.yml)"
        );
        if let Ok(Cli {
            command: Commands::Generate { orb_path, .. },
        }) = cli
        {
            assert_eq!(orb_path, std::path::PathBuf::from("src/@orb.yml"));
        }
    }

    #[test]
    fn test_cli_parse_validate_default_orb_path() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "validate"]);
        assert!(
            cli.is_ok(),
            "validate should work without --orb-path (default: src/@orb.yml)"
        );
        if let Ok(Cli {
            command: Commands::Validate { orb_path },
        }) = cli
        {
            assert_eq!(orb_path, std::path::PathBuf::from("src/@orb.yml"));
        }
    }

    #[test]
    fn test_cli_parse_generate_with_crate_version_legacy() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--crate-version",
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
            "--crate-version",
            "1.2.3",
            "--force",
        ]);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_cli_parse_generate_with_crate_version() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "generate",
            "--orb-path",
            "test.yml",
            "--output",
            "./out",
            "--crate-version",
            "1.2.3",
        ]);
        assert!(cli.is_ok(), "--crate-version should be accepted");
    }

    #[test]
    fn test_cli_parse_generate_version_flag_rejected() {
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
        assert!(
            cli.is_err(),
            "--version should be rejected (conflicts with clap built-in)"
        );
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
        assert!(err.contains("--crate-version"));
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
    /// path.  `worktree.join(absolute_path)` then ignores the worktree and
    /// reads the current working copy — producing snapshots with
    /// current-version content for every historical tag.
    ///
    /// The fix: canonicalise `start` at the top of `find_git_root` so the
    /// walk-up always operates on absolute paths and returns an absolute
    /// result.
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

    // --- save subcommand tests ---

    fn init_git_repo(dir: &std::path::Path) {
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
        // Initial commit so HEAD exists
        std::fs::write(dir.join("README.md"), "test").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn test_save_clean_tree_exits_without_commit() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        let _cwd_guard = CWD_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        // Stage the path we already committed — tree is clean after staging
        let result = run_save(
            &[std::path::PathBuf::from("README.md")],
            "chore: test",
            false,
            false,
            false,
            std::path::Path::new("gen-orb-mcp.toml"),
            &SignEnvNameOverrides::default(),
        );
        std::env::set_current_dir(&original).unwrap();
        assert!(
            result.is_ok(),
            "clean tree should exit 0 without creating a commit: {result:?}"
        );
    }

    #[test]
    fn test_save_changed_path_creates_commit() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        std::fs::write(dir.path().join("new-file.txt"), "hello").unwrap();
        let _cwd_guard = CWD_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = run_save(
            &[std::path::PathBuf::from("new-file.txt")],
            "chore: add generated file",
            false,
            false,
            false,
            std::path::Path::new("gen-orb-mcp.toml"),
            &SignEnvNameOverrides::default(),
        );
        std::env::set_current_dir(&original).unwrap();
        assert!(
            result.is_ok(),
            "changed path should commit successfully: {result:?}"
        );
        // Verify a commit was created beyond the initial one
        let log = std::process::Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let log_str = String::from_utf8_lossy(&log.stdout);
        assert!(
            log_str.lines().count() >= 2,
            "expected at least 2 commits, got: {log_str}"
        );
    }

    #[test]
    fn test_save_directory_path_stages_contents() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        // Create a directory with files inside — mirrors the prior-versions/ and
        // migrations/ case
        let subdir = dir.path().join("generated");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("a.json"), r#"{"v": 1}"#).unwrap();
        std::fs::write(subdir.join("b.json"), r#"{"v": 2}"#).unwrap();
        let _cwd_guard = CWD_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = run_save(
            &[std::path::PathBuf::from("generated")],
            "chore: add generated dir",
            false,
            false,
            false,
            std::path::Path::new("gen-orb-mcp.toml"),
            &SignEnvNameOverrides::default(),
        );
        std::env::set_current_dir(&original).unwrap();
        assert!(
            result.is_ok(),
            "directory path should stage all contents and commit: {result:?}"
        );
        let log = std::process::Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let log_str = String::from_utf8_lossy(&log.stdout);
        assert!(
            log_str.lines().count() >= 2,
            "expected at least 2 commits after staging directory, got: {log_str}"
        );
    }

    #[test]
    fn test_save_dry_run_does_not_commit() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());
        std::fs::write(dir.path().join("artifact.txt"), "generated").unwrap();
        let _cwd_guard = CWD_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = run_save(
            &[std::path::PathBuf::from("artifact.txt")],
            "chore: generated",
            false,
            true,
            false,
            std::path::Path::new("gen-orb-mcp.toml"),
            &SignEnvNameOverrides::default(),
        );
        std::env::set_current_dir(&original).unwrap();
        assert!(result.is_ok(), "dry_run should succeed: {result:?}");
        // Only the initial commit should exist
        let log = std::process::Command::new("git")
            .args(["log", "--oneline"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let log_str = String::from_utf8_lossy(&log.stdout);
        assert_eq!(
            log_str.lines().count(),
            1,
            "dry_run must not create a commit, got: {log_str}"
        );
    }

    #[test]
    fn test_cli_parse_save_required_paths() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "save",
            "--paths",
            "prior-versions",
            "--paths",
            "migrations",
        ]);
        assert!(cli.is_ok(), "save with --paths should parse");
    }

    #[test]
    fn test_cli_parse_save_sign_flag() {
        let cli =
            Cli::try_parse_from(["gen-orb-mcp", "save", "--paths", "prior-versions", "--sign"]);
        assert!(
            cli.is_ok(),
            "--sign flag should be accepted on save command"
        );
        if let Commands::Save { sign, .. } = cli.unwrap().command {
            assert!(sign, "--sign should be true when flag is passed");
        } else {
            panic!("expected Save variant");
        }
    }

    #[test]
    fn read_sign_env_missing_var_errors_with_resolved_name() {
        // Use a unique, definitely-absent var name so this is parallel-safe and
        // independent of the ambient environment.
        let names = SignEnvNames {
            gpg_key: "T185_MISSING_GPG_KEY".to_string(),
            trust: "T185_MISSING_TRUST".to_string(),
            user_name: "T185_MISSING_UN".to_string(),
            user_email: "T185_MISSING_UE".to_string(),
            sign_key: "T185_MISSING_SK".to_string(),
        };
        for k in [
            "T185_MISSING_GPG_KEY",
            "T185_MISSING_TRUST",
            "T185_MISSING_UN",
            "T185_MISSING_UE",
            "T185_MISSING_SK",
        ] {
            std::env::remove_var(k);
        }
        let result = read_sign_env(&names);
        assert!(
            result.is_err(),
            "should fail when the resolved var is absent"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("T185_MISSING_GPG_KEY"),
            "error should mention the resolved var name, got: {msg}"
        );
    }

    #[test]
    fn test_cli_parse_save_all_flags() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "save",
            "--paths",
            "prior-versions",
            "--message",
            "custom message",
            "--no-push",
            "--dry-run",
        ]);
        assert!(cli.is_ok(), "save with all flags should parse");
        if let Commands::Save {
            paths,
            message,
            no_push,
            dry_run,
            ..
        } = cli.unwrap().command
        {
            assert_eq!(paths, vec![std::path::PathBuf::from("prior-versions")]);
            assert_eq!(message, "custom message");
            assert!(no_push);
            assert!(dry_run);
        } else {
            panic!("expected Save variant");
        }
    }

    // --- publish subcommand tests ---

    #[test]
    fn test_publish_missing_binary_returns_error() {
        let dir = TempDir::new().unwrap();
        let result = PublishJob {
            name: None,
            input: std::path::Path::new("."),
            binary: Some(&dir.path().join("missing-binary")),
            asset_name: Some("asset.tar.gz"),
            tag: None,
            dry_run: false,
            config_path: std::path::Path::new("no-such-config-185.toml"),
            tag_env_override: None,
        }
        .run();
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Binary not found"),
            "error should mention missing binary, got: {msg}"
        );
    }

    #[test]
    fn test_publish_dry_run_succeeds_without_token() {
        let dir = TempDir::new().unwrap();
        let binary = dir.path().join("my-binary");
        std::fs::write(&binary, b"fake binary").unwrap();
        // dry_run must succeed without credentials — no API call is made
        std::env::remove_var("GITHUB_TOKEN");
        let result = PublishJob {
            name: None,
            input: std::path::Path::new("."),
            binary: Some(&binary),
            asset_name: Some("my-asset"),
            tag: Some("v1.0.0"),
            dry_run: true,
            config_path: std::path::Path::new("no-such-config-185.toml"),
            tag_env_override: None,
        }
        .run();
        assert!(
            result.is_ok(),
            "dry_run should not require credentials: {result:?}"
        );
    }

    #[test]
    fn test_publish_dry_run_missing_tag_returns_error() {
        let dir = TempDir::new().unwrap();
        let binary = dir.path().join("my-binary");
        std::fs::write(&binary, b"fake binary").unwrap();
        std::env::set_var("GITHUB_TOKEN", "fake-token");
        std::env::remove_var("CIRCLE_TAG");
        // no --tag and no CIRCLE_TAG — should fail with a clear message
        let result = PublishJob {
            name: None,
            input: std::path::Path::new("."),
            binary: Some(&binary),
            asset_name: Some("my-asset"),
            tag: None,
            dry_run: true,
            config_path: std::path::Path::new("no-such-config-185.toml"),
            tag_env_override: None,
        }
        .run();
        std::env::remove_var("GITHUB_TOKEN");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("tag") || msg.contains("CIRCLE_TAG"),
            "error should mention tag or CIRCLE_TAG, got: {msg}"
        );
    }

    #[test]
    fn test_publish_dry_run_prints_parameters() {
        let dir = TempDir::new().unwrap();
        let binary = dir.path().join("my-binary");
        std::fs::write(&binary, b"fake binary").unwrap();
        std::env::set_var("GITHUB_TOKEN", "fake-token");
        std::env::set_var("CIRCLE_PROJECT_USERNAME", "jerus-org");
        std::env::set_var("CIRCLE_PROJECT_REPONAME", "my-orb");
        let result = PublishJob {
            name: None,
            input: std::path::Path::new("."),
            binary: Some(&binary),
            asset_name: Some("my-asset-linux-x86_64"),
            tag: Some("v1.0.0"),
            dry_run: true,
            config_path: std::path::Path::new("no-such-config-185.toml"),
            tag_env_override: None,
        }
        .run();
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("CIRCLE_PROJECT_USERNAME");
        std::env::remove_var("CIRCLE_PROJECT_REPONAME");
        assert!(
            result.is_ok(),
            "dry_run with all params should succeed: {result:?}"
        );
    }

    #[test]
    fn test_cli_parse_publish_required_args() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "publish",
            "--binary",
            "/tmp/my-binary",
            "--asset-name",
            "my-binary-linux-x86_64",
        ]);
        assert!(cli.is_ok(), "publish with required args should parse");
    }

    #[test]
    fn test_cli_parse_publish_all_flags() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "publish",
            "--binary",
            "/tmp/my-binary",
            "--asset-name",
            "my-binary-linux-x86_64",
            "--tag",
            "v2.0.0",
            "--dry-run",
        ]);
        assert!(cli.is_ok(), "publish with all flags should parse");
        if let Commands::Publish {
            binary,
            asset_name,
            tag,
            dry_run,
            ..
        } = cli.unwrap().command
        {
            assert_eq!(
                binary.as_deref().and_then(|p| p.to_str()),
                Some("/tmp/my-binary")
            );
            assert_eq!(asset_name.as_deref(), Some("my-binary-linux-x86_64"));
            assert_eq!(tag.as_deref(), Some("v2.0.0"));
            assert!(dry_run);
        } else {
            panic!("expected Publish variant");
        }
    }

    #[test]
    fn test_resolve_publish_target_derives_from_name() {
        let (binary, asset) = resolve_publish_target(
            Some("gen-orb-mcp"),
            std::path::Path::new("/tmp/mcp-server"),
            None,
            None,
        )
        .expect("derivation from name should succeed");
        assert_eq!(
            binary,
            std::path::PathBuf::from("/tmp/mcp-server/target/release/gen_orb_mcp_mcp")
        );
        assert_eq!(asset, "gen_orb_mcp_mcp-linux-x86_64");
    }

    #[test]
    fn test_resolve_publish_target_explicit_overrides_name() {
        let (binary, asset) = resolve_publish_target(
            Some("gen-orb-mcp"),
            std::path::Path::new("/tmp/mcp-server"),
            Some(std::path::Path::new("/custom/bin")),
            Some("custom-asset"),
        )
        .expect("explicit values should win");
        assert_eq!(binary, std::path::PathBuf::from("/custom/bin"));
        assert_eq!(asset, "custom-asset");
    }

    #[test]
    fn test_resolve_publish_target_requires_name_or_binary() {
        let result = resolve_publish_target(None, std::path::Path::new("./dist"), None, None);
        assert!(
            result.is_err(),
            "must error when neither --name nor --binary is given"
        );
    }

    #[test]
    fn test_cli_parse_publish_with_name() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "publish",
            "--name",
            "gen-orb-mcp",
            "--input",
            "/tmp/mcp-server",
        ]);
        assert!(cli.is_ok(), "publish with --name should parse");
        if let Commands::Publish {
            name,
            input,
            binary,
            ..
        } = cli.unwrap().command
        {
            assert_eq!(name.as_deref(), Some("gen-orb-mcp"));
            assert_eq!(input, std::path::PathBuf::from("/tmp/mcp-server"));
            assert!(binary.is_none());
        } else {
            panic!("expected Publish variant");
        }
    }

    #[test]
    fn test_cli_parse_save_comma_separated_paths() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "save",
            "--paths",
            "prior-versions,migrations",
        ]);
        assert!(cli.is_ok(), "comma-separated --paths should parse");
        if let Commands::Save { paths, .. } = cli.unwrap().command {
            assert_eq!(
                paths,
                vec![
                    std::path::PathBuf::from("prior-versions"),
                    std::path::PathBuf::from("migrations"),
                ]
            );
        } else {
            panic!("expected Save variant");
        }
    }

    // --- build subcommand tests ---

    fn write_cargo_toml(dir: &std::path::Path, name: &str) {
        std::fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        )
        .unwrap();
    }

    #[test]
    fn test_build_missing_cargo_toml_returns_error() {
        let dir = TempDir::new().unwrap();
        let result = run_build(dir.path(), None, None, false);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Cargo.toml"),
            "error should mention Cargo.toml, got: {msg}"
        );
    }

    #[test]
    fn test_build_dry_run_does_not_invoke_cargo() {
        let dir = TempDir::new().unwrap();
        write_cargo_toml(dir.path(), "my-server");
        // Not a valid Rust project — cargo would fail if invoked.
        // With dry_run=true the function must succeed without running cargo.
        let result = run_build(dir.path(), None, None, true);
        assert!(
            result.is_ok(),
            "dry_run should succeed without invoking cargo: {result:?}"
        );
    }

    #[test]
    fn test_build_name_override_accepted_in_dry_run() {
        let dir = TempDir::new().unwrap();
        write_cargo_toml(dir.path(), "my-server");
        let result = run_build(dir.path(), Some("custom-name"), None, true);
        assert!(
            result.is_ok(),
            "name override + dry_run should succeed: {result:?}"
        );
    }

    #[test]
    fn test_build_target_triple_accepted_in_dry_run() {
        let dir = TempDir::new().unwrap();
        write_cargo_toml(dir.path(), "my-server");
        let result = run_build(dir.path(), None, Some("x86_64-unknown-linux-musl"), true);
        assert!(
            result.is_ok(),
            "target + dry_run should succeed: {result:?}"
        );
    }

    #[test]
    fn test_parse_package_name_extracts_name() {
        let toml = "[package]\nname = \"my-orb-mcp\"\nversion = \"0.1.0\"\n";
        assert_eq!(
            parse_package_name(toml),
            Some("my-orb-mcp".to_string()),
            "should extract package name"
        );
    }

    #[test]
    fn test_parse_package_name_stops_at_next_section() {
        let toml = "[package]\nname = \"my-orb-mcp\"\n[dependencies]\nname = \"ignored\"\n";
        assert_eq!(parse_package_name(toml), Some("my-orb-mcp".to_string()));
    }

    #[test]
    fn test_parse_package_name_returns_none_when_absent() {
        let toml = "[dependencies]\nanyhow = \"1\"\n";
        assert_eq!(parse_package_name(toml), None);
    }

    #[test]
    fn test_read_crate_name_from_file() {
        let dir = TempDir::new().unwrap();
        write_cargo_toml(dir.path(), "test-crate");
        let result = read_crate_name(dir.path());
        assert!(result.is_ok(), "read_crate_name should succeed: {result:?}");
        assert_eq!(result.unwrap(), "test-crate");
    }

    #[test]
    fn test_cli_parse_build_required_input() {
        let cli = Cli::try_parse_from(["gen-orb-mcp", "build", "--input", "/tmp/my-server"]);
        assert!(cli.is_ok(), "build --input should parse");
    }

    #[test]
    fn test_cli_parse_build_all_flags() {
        let cli = Cli::try_parse_from([
            "gen-orb-mcp",
            "build",
            "--input",
            "/tmp/my-server",
            "--name",
            "my_server",
            "--target",
            "x86_64-unknown-linux-musl",
            "--dry-run",
        ]);
        assert!(cli.is_ok(), "build with all flags should parse");
        if let Commands::Build {
            input,
            name,
            target,
            dry_run,
        } = cli.unwrap().command
        {
            assert_eq!(input.to_str().unwrap(), "/tmp/my-server");
            assert_eq!(name.as_deref(), Some("my_server"));
            assert_eq!(target.as_deref(), Some("x86_64-unknown-linux-musl"));
            assert!(dry_run);
        } else {
            panic!("expected Build variant");
        }
    }

    // --- #185: configurable signing / publish env-var names ---

    #[test]
    fn sign_env_names_default_to_generic() {
        let names = resolve_sign_env_names(
            std::path::Path::new("does-not-exist-185.toml"),
            &SignEnvNameOverrides::default(),
        )
        .unwrap();
        assert_eq!(names.gpg_key, "GPG_KEY");
        assert_eq!(names.trust, "GPG_TRUST");
        assert_eq!(names.user_name, "GIT_USER_NAME");
        assert_eq!(names.user_email, "GIT_USER_EMAIL");
        assert_eq!(names.sign_key, "GPG_SIGN_KEY");
    }

    #[test]
    fn sign_env_names_from_config_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("gen-orb-mcp.toml");
        std::fs::write(
            &path,
            "[sign]\n\
             gpg_key_env = \"BOT_GPG_KEY\"\n\
             trust_env = \"BOT_TRUST\"\n\
             user_name_env = \"BOT_USER_NAME\"\n\
             user_email_env = \"BOT_USER_EMAIL\"\n\
             sign_key_env = \"BOT_SIGN_KEY\"\n",
        )
        .unwrap();
        let names = resolve_sign_env_names(&path, &SignEnvNameOverrides::default()).unwrap();
        assert_eq!(names.gpg_key, "BOT_GPG_KEY");
        assert_eq!(names.trust, "BOT_TRUST");
        assert_eq!(names.user_name, "BOT_USER_NAME");
        assert_eq!(names.user_email, "BOT_USER_EMAIL");
        assert_eq!(names.sign_key, "BOT_SIGN_KEY");
    }

    #[test]
    fn sign_env_cli_override_beats_config() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("gen-orb-mcp.toml");
        std::fs::write(&path, "[sign]\ngpg_key_env = \"BOT_GPG_KEY\"\n").unwrap();
        let overrides = SignEnvNameOverrides {
            gpg_key_env: Some("CLI_GPG".to_string()),
            ..Default::default()
        };
        let names = resolve_sign_env_names(&path, &overrides).unwrap();
        assert_eq!(names.gpg_key, "CLI_GPG", "CLI override wins over config");
        assert_eq!(
            names.trust, "GPG_TRUST",
            "unspecified falls back to default"
        );
    }

    #[test]
    fn read_sign_env_reads_resolved_names() {
        let names = SignEnvNames {
            gpg_key: "T185_GPG_KEY".to_string(),
            trust: "T185_TRUST".to_string(),
            user_name: "T185_UN".to_string(),
            user_email: "T185_UE".to_string(),
            sign_key: "T185_SK".to_string(),
        };
        for (k, v) in [
            ("T185_GPG_KEY", "key"),
            ("T185_TRUST", "trust"),
            ("T185_UN", "Bot"),
            ("T185_UE", "bot@example.com"),
            ("T185_SK", "ABCD"),
        ] {
            std::env::set_var(k, v);
        }
        let se = read_sign_env(&names).unwrap();
        assert_eq!(se.gpg_key_b64, "key");
        assert_eq!(se.user_email, "bot@example.com");
        for k in [
            "T185_GPG_KEY",
            "T185_TRUST",
            "T185_UN",
            "T185_UE",
            "T185_SK",
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn read_sign_env_error_names_the_resolved_var() {
        let names = SignEnvNames {
            gpg_key: "T185_ABSENT_KEY".to_string(),
            trust: "x".to_string(),
            user_name: "x".to_string(),
            user_email: "x".to_string(),
            sign_key: "x".to_string(),
        };
        std::env::remove_var("T185_ABSENT_KEY");
        let err = read_sign_env(&names).unwrap_err().to_string();
        assert!(
            err.contains("T185_ABSENT_KEY"),
            "error should name the resolved var, got: {err}"
        );
    }

    #[test]
    fn tag_env_name_resolution() {
        assert_eq!(
            resolve_tag_env_name(std::path::Path::new("nope-185.toml"), None).unwrap(),
            "CIRCLE_TAG"
        );
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("gen-orb-mcp.toml");
        std::fs::write(&path, "[publish]\ntag_env = \"MY_TAG\"\n").unwrap();
        assert_eq!(resolve_tag_env_name(&path, None).unwrap(), "MY_TAG");
        assert_eq!(
            resolve_tag_env_name(&path, Some("CLI_TAG")).unwrap(),
            "CLI_TAG"
        );
    }
}

#[cfg(test)]
mod git2_build_features {
    //! Guards the linked libgit2 build features. `git2` 0.21 changed its
    //! default features to `[]` (0.20 defaulted to `["ssh", "https"]`), so a
    //! bare `git2 = "0.21"` builds libgit2 WITHOUT a TLS backend — the `save`
    //! command's HTTPS push then fails at runtime with
    //! `there is no TLS stream available; class=Ssl (16)`. These tests fail
    //! fast at test time if the features regress again.

    #[test]
    fn libgit2_has_https_support() {
        assert!(
            git2::Version::get().https(),
            "linked libgit2 has no HTTPS/TLS backend — `save`'s git push will fail \
             with 'there is no TLS stream available'. Ensure the `git2` dependency \
             enables the `https` feature (git2 0.21 dropped it from the defaults)."
        );
    }

    #[test]
    fn libgit2_has_ssh_support() {
        assert!(
            git2::Version::get().ssh(),
            "linked libgit2 has no SSH backend — SSH git remotes will fail. Ensure \
             the `git2` dependency enables the `ssh` feature (git2 0.21 dropped it \
             from the defaults)."
        );
    }
}
