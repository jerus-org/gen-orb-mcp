//! Migration engine: planning and applying conformance-based migrations.
//!
//! The migrator converts `Vec<ConformanceRule>` + `ConsumerConfig` into a
//! `MigrationPlan`, then applies the plan to files on disk.
//!
//! ## Usage
//!
//! ```ignore
//! use gen_orb_mcp::migrator::Migrator;
//! use gen_orb_mcp::consumer_parser::ConsumerParser;
//!
//! let config = ConsumerParser::parse_directory(".circleci/".as_ref())?;
//! let plan = Migrator::plan(&rules, &config, "toolkit");
//! println!("{}", plan.format_summary());
//! let applied = Migrator::apply(&plan, false)?;  // false = not dry run
//! ```

pub mod applicator;
pub mod planner;
pub mod reporter;
pub mod types;

pub use types::{AppliedChanges, ChangeType, MigrationPlan, PlannedChange};

use crate::conformance_rule::ConformanceRule;
use crate::consumer_parser::types::ConsumerConfig;

/// Entry point for migration planning and application.
pub struct Migrator;

impl Migrator {
    /// Produces a `MigrationPlan` from conformance rules and a consumer config.
    ///
    /// # Arguments
    /// * `rules` — conformance rules for the target version
    /// * `config` — parsed consumer CI config
    /// * `orb_alias` — the orb alias as used in the consumer's config
    pub fn plan(
        rules: &[ConformanceRule],
        config: &ConsumerConfig,
        orb_alias: &str,
    ) -> MigrationPlan {
        planner::plan(rules, config, orb_alias)
    }

    /// Applies a migration plan to files on disk.
    ///
    /// When `dry_run` is `true`, files are not modified.
    pub fn apply(plan: &MigrationPlan, dry_run: bool) -> std::io::Result<AppliedChanges> {
        applicator::apply(plan, dry_run)
    }
}
