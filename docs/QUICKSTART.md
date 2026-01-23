# gen-orb-mcp: Quick Start Guide

## Project Status: READY TO BEGIN

All planning complete. Answers finalized. Standards aligned with CLAUDE.md.

---

## Critical Information

**Repository:** jerus-org/gen-orb-mcp  
**Developer:** Solo (you) + Claude Code  
**Time:** 15 hours/week (~2 days/week)  
**MVP Timeline:** 15-20 weeks  
**License:** Dual MIT/Apache-2.0  
**CI/CD:** CircleCI with circleci-toolkit orb  

---

## First Steps (This Week)

### Day 1-2: Create Repository

```bash
# Create repository
gh repo create jerus-org/gen-orb-mcp --public \
  --description "Generate MCP servers from CircleCI orb definitions"

# Clone
git clone https://github.com/jerus-org/gen-orb-mcp.git
cd gen-orb-mcp

# Initialize workspace
mkdir -p crates/gen-orb-mcp/src
```

### Create Initial Files

**Workspace `Cargo.toml`:**
```toml
[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Jeremiah Russell <jrussell@jerus.org>"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/jerus-org/gen-orb-mcp"
homepage = "https://github.com/jerus-org/gen-orb-mcp"

[workspace.dependencies]
# Shared dependencies
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

**Crate `Cargo.toml` (`crates/gen-orb-mcp/Cargo.toml`):**
```toml
[package]
name = "gen-orb-mcp"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Generate MCP servers from CircleCI orb definitions"
keywords = ["circleci", "orb", "mcp", "ai", "codegen"]
categories = ["development-tools", "command-line-utilities"]
readme = "README.md"

[dependencies]
pmcp = "1.8"
serde.workspace = true
serde_yaml = "0.9"
clap = { version = "4.0", features = ["derive"] }
tokio.workspace = true
handlebars = "5.0"
anyhow.workspace = true
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

[dev-dependencies]
tempfile = "3.0"
trycmd = "0.15"

[[bin]]
name = "gen-orb-mcp"
path = "src/main.rs"

[lib]
name = "gen_orb_mcp"
path = "src/lib.rs"
```

**Initial `src/main.rs`:**
```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser, Debug)]
enum Commands {
    /// Generate MCP server from orb definition
    Generate {
        /// Path to orb YAML file
        #[arg(long)]
        orb_path: std::path::PathBuf,
        
        /// Output directory
        #[arg(long)]
        output: std::path::PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    
    match args.command {
        Commands::Generate { orb_path, output } => {
            tracing::info!(?orb_path, ?output, "Generating MCP server");
            // TODO: Implement generation
            Ok(())
        }
    }
}
```

**Initial `src/lib.rs`:**
```rust
//! # gen-orb-mcp
//!
//! Generate Model Context Protocol (MCP) servers from CircleCI orb definitions.

pub mod parser;
pub mod generator;

pub use parser::OrbParser;
pub use generator::Generator;
```

### Day 3-4: CircleCI Configuration

**Copy config files from outputs:**
```bash
# Main validation workflow
cp /path/to/outputs/circleci-config.yml .circleci/config.yml

# Release workflow
cp /path/to/outputs/circleci-release.yml .circleci/release.yml
```

**These configs are based on the pcu repository pattern:**
- Two separate workflows (validation + release)
- Uses `jerus-org/circleci-toolkit@4.2.0` orb
- Automated PRLOG.md updates
- Recovery support for failed releases

**See CIRCLECI_SETUP_GUIDE.md for complete configuration details**

### Additional Files

**`justfile`:**
```make
# Build the project
build:
    cargo build

# Run tests
test: clippy check doc unit-tests

# Run clippy
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Cargo check
check:
    cargo check --all-targets --all-features

# Generate documentation
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# Run unit tests
unit-tests:
    cargo test

# Format code
fmt:
    cargo +nightly fmt
    cargo +stable fmt --check

# Coverage
cov:
    cargo tarpaulin --out Html --output-dir coverage
```

**`rustfmt.toml`:**
```toml
edition = "2021"
max_width = 100
```

**`deny.toml`:**
```toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-3-Clause",
]
```

**`renovate.json`:**
```json
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": [
    "config:base"
  ],
  "packageRules": [
    {
      "matchUpdateTypes": ["minor", "patch"],
      "automerge": true
    }
  ]
}
```

**`PRLOG.md`:**
```markdown
# Pull Request Log

## UNRELEASED

```

**Crate `README.md` (`crates/gen-orb-mcp/README.md`):**
```markdown
# gen-orb-mcp

Generate Model Context Protocol (MCP) servers from CircleCI orb definitions.

## Status

🚧 **Under Development** - MVP in progress

## Roadmap

### Phase 1: MVP (In Progress)
- Parse CircleCI orb YAML
- Generate MCP server (single version)
- Linux binary deployment
- Private registry support

### Phase 2: Enhanced (Planned)
- Multi-version support with delta encoding
- Workspace version detection
- Migration tooling for breaking changes
- Multiple deployment formats

### Phase 3: Advanced (Future)
- Smart router for complex repositories
- Context-aware version routing

## Installation

Coming soon! Name reserved on crates.io.

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

[Contributing Guide](https://github.com/jerus-org/gen-orb-mcp/blob/main/CONTRIBUTING.md)
```

**Crate `release.toml` (`crates/gen-orb-mcp/release.toml`):**
```toml
pre-release-commit-message = "chore: Release gen-orb-mcp v{{version}}"
tag-message = "{{tag_name}}"
tag-name = "gen-orb-mcp-v{{version}}"
pre-release-hook = ["./release-hook.sh"]
pre-release-replacements = [
    { file = "README.md", search = "gen-orb-mcp = \"\\d+\\.\\d+\\.\\d+\"", replace = "gen-orb-mcp = \"{{version}}\"", exactly = 1 },
]
```

**Crate `release-hook.sh` (`crates/gen-orb-mcp/release-hook.sh`):**
```bash
#!/bin/bash
set -exo pipefail
gen-changelog generate \
    --display-summaries \
    --name "CHANGELOG.md" \
    --package "gen-orb-mcp" \
    --repository-dir "../.." \
    --next-version "${NEW_VERSION:-${1}}"
chmod +x crates/gen-orb-mcp/release-hook.sh
```

**Workspace `release.toml`:**
```toml
sign-tag = true
sign-commit = true
consolidate-commits = true
allow-branch = ["main"]
pre-release-replacements = []
```

### Day 5: Initial Commit

```bash
# Stage all files
git add .

# First commit
git commit -m "chore: Initial project scaffolding

- Workspace structure per organizational standards
- CircleCI configuration with circleci-toolkit
- Basic CLI structure with clap
- License files (MIT/Apache-2.0)
- Release configuration for gen-changelog
- Development tooling (justfile, deny.toml, etc.)"

# Push
git push origin main

# Verify CircleCI runs
```

---

## Development Workflow

### Daily Development

```bash
# Start coding
just build

# Run tests frequently
just test

# Before committing
just fmt
just clippy
just check
```

### With Claude Code

```
"Claude, implement the OrbParser struct in src/parser/mod.rs 
following the architecture in ARCHITECTURE.md"
```

### Testing with circleci-toolkit

```bash
# Clone toolkit for testing
cd ..
git clone https://github.com/jerus-org/circleci-toolkit.git

# Test parser
cd gen-orb-mcp
cargo run -- generate \
  --orb-path ../circleci-toolkit/src/@orb.yml \
  --output ./test-output/
```

---

## MVP Tasks (Next 15-20 Weeks)

### Weeks 1-5: Core Parser
- [x] Repository setup (Week 1)
- [ ] Orb YAML parser (Weeks 2-3)
- [ ] CLI interface (Weeks 4-5)

### Weeks 6-10: MCP Generation
- [ ] Template system (Weeks 6-7)
- [ ] Resource generation (Weeks 8-9)
- [ ] pmcp integration (Week 10)

### Weeks 11-13: Binary Build
- [ ] Rust compiler integration (Weeks 11-12)
- [ ] End-to-end testing (Week 13)

### Weeks 14-17: Integration
- [ ] CircleCI integration (Weeks 14-15)
- [ ] circleci-toolkit integration (Week 16)
- [ ] Developer testing (Week 17)

### Weeks 18-20: Release
- [ ] Documentation (Weeks 18-19)
- [ ] MVP release v0.1.0 (Week 20)

---

## Key Commands

```bash
# Build
cargo build

# Test
just test

# Specific test
cargo test test_name

# Format
just fmt

# Lint
just clippy

# Documentation
just doc

# Coverage
just cov

# Release (when ready)
cd crates/gen-orb-mcp
cargo release patch
```

---

## Critical Requirements (Don't Forget!)

✅ **Private Registry Support** - Must work with Docker Hub + private registries  
✅ **Workspace Structure** - Follow CLAUDE.md standards exactly  
✅ **CircleCI** - Use circleci-toolkit, not GitHub Actions  
✅ **gen-changelog** - Use for CHANGELOG generation  
✅ **Conventional Commits** - For changelog automation  
✅ **Linux Binary** - MVP targets x86_64-unknown-linux-gnu only  
✅ **No Public Data** - Private orb data never exposed  

---

## Reference Documents

- **FINALIZED_PLAN.md** - Complete overview with all answers
- **ARCHITECTURE.md** - Technical design (CircleCI-aligned)
- **IMPLEMENTATION_PLAN.md** - Detailed task breakdown
- **CIRCLECI_INTEGRATION_PLAN.md** - Toolkit integration
- **CLAUDE.md** - Organizational standards (critical!)

---

## Getting Help

**With Claude Code:**
```
"Claude, I'm stuck on [task]. Can you help based on [DOCUMENT.md]?"

"Claude, review my implementation of [feature] against ARCHITECTURE.md"

"Claude, what's the next task per IMPLEMENTATION_PLAN.md?"
```

**References:**
- Architecture questions → ARCHITECTURE.md
- Task questions → IMPLEMENTATION_PLAN.md  
- Standards questions → CLAUDE.md
- Integration questions → CIRCLECI_INTEGRATION_PLAN.md

---

## Success Criteria for Week 1

- [x] Repository created
- [ ] Workspace structure complete
- [ ] CircleCI configured and passing
- [ ] Basic CLI compiles and runs
- [ ] Ready to start parser development

---

**Ready to begin! Let's build gen-orb-mcp! 🚀**

Start with: `gh repo create jerus-org/gen-orb-mcp --public`
