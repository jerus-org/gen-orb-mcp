# gen-orb-mcp: Technical Architecture

## System Overview

gen-orb-mcp is a code generation tool that transforms CircleCI orb definitions into MCP (Model Context Protocol) servers, enabling AI assistants to understand and interact with orb capabilities.

```
┌─────────────────────────────────────────────────────────────┐
│                     gen-orb-mcp                             │
│                                                             │
│  Input: Orb YAML + Migration Rules                         │
│         ↓                                                   │
│  Parser → Analyzer → Generator → Builder                   │
│         ↓                                                   │
│  Output: MCP Server (Binary/Container/Source/Skill)        │
└─────────────────────────────────────────────────────────────┘
```

---

## Architecture Layers

### Layer 1: Input Processing

**Purpose:** Parse and validate orb definitions

**Components:**

```rust
// Orb Parser
pub struct OrbParser {
    yaml_parser: YamlParser,
    validator: OrbValidator,
}

pub struct OrbDefinition {
    version: String,
    orb_metadata: OrbMetadata,
    commands: HashMap<String, Command>,
    jobs: HashMap<String, Job>,
    executors: HashMap<String, Executor>,
}

pub struct Command {
    name: String,
    description: String,
    parameters: Vec<Parameter>,
    steps: Vec<Step>,
}

pub struct Parameter {
    name: String,
    param_type: ParameterType,
    default: Option<String>,
    description: String,
    required: bool,
}
```

**Responsibilities:**
- Parse orb YAML using serde_yaml
- Validate structure against CircleCI schema
- Extract commands, jobs, executors
- Build typed Rust representation

**Error Handling:**
- Invalid YAML → clear syntax error
- Missing required fields → validation error with path
- Unknown orb version → warning, proceed with best effort

---

### Layer 2: Multi-Version Analysis

**Purpose:** Handle multiple orb versions efficiently

**Components:**

```rust
pub struct VersionAnalyzer {
    git_repo: Option<GitRepository>,
    versions: BTreeMap<Version, OrbDefinition>,
}

pub struct DeltaCalculator {
    base_version_selector: BaseVersionSelector,
}

pub struct OrbDelta {
    from_version: Version,
    to_version: Version,
    commands_added: Vec<Command>,
    commands_removed: Vec<String>,
    commands_modified: Vec<CommandDelta>,
    jobs_added: Vec<Job>,
    // ... similar for jobs, executors
}

pub enum BaseVersionSelector {
    MajorVersions,    // Keep base for each major version
    EveryNthVersion(usize), // Keep every Nth version as base
    SizeThreshold(usize),   // New base if delta > threshold
}
```

**Delta Encoding Strategy:**

```
v1.0.0 [BASE] ────────────────────┐
  ↓                                │
v1.1.0 [delta: +2 commands]        │
  ↓                                │
v1.2.0 [delta: +1 job]             │
  ↓                                │
v2.0.0 [BASE - breaking changes] ──┘
  ↓
v2.1.0 [delta: +3 commands]
  ↓
v3.0.0 [BASE]
```

**Reconstruction Algorithm:**
```rust
fn reconstruct_version(target: &Version) -> OrbDefinition {
    let base = find_base_version(target);
    let mut definition = base.clone();
    
    let deltas = get_deltas_path(base.version, target);
    for delta in deltas {
        definition.apply_delta(&delta);
    }
    
    definition
}
```

---

### Layer 3: Migration Analysis

**Purpose:** Parse and apply migration rules

**Components:**

```rust
pub struct MigrationParser {
    yaml_parser: YamlParser,
}

pub struct MigrationRule {
    id: String,
    from_version: Version,
    to_version: Version,
    rule_type: RuleType,
    rationale: String,
}

pub enum RuleType {
    ParameterRenamed {
        scope: Scope,
        old_name: String,
        new_name: String,
    },
    ParameterMoved {
        scope: Scope,
        old_location: String,
        new_location: String,
    },
    ParameterRemoved {
        scope: Scope,
        parameter: String,
        replacement: Option<String>,
    },
    JobRestructured {
        job: String,
        changes: Vec<StructuralChange>,
    },
}

pub struct MigrationEngine {
    rules: HashMap<(Version, Version), Vec<MigrationRule>>,
}
```

**Migration YAML Schema:**

```yaml
version_from: "2.5.0"
version_to: "3.0.0"
release_date: "2024-01-15"

breaking_changes:
  - id: rename-node-version
    type: parameter_renamed
    scope:
      commands: [build-job]
    changes:
      old_name: node_version
      new_name: runtime_version
    migration:
      search: "node_version:\\s*(.+)"
      replace: "runtime_version: $1"
    rationale: "Support multiple runtimes beyond Node.js"

deprecations:
  - parameter: cache_ttl
    scope: [build-job]
    deprecated_in: "3.0.0"
    removed_in: "4.0.0"
    replacement: "Use CircleCI's cache.save step"
```

---

### Layer 4: Code Generation

**Purpose:** Generate MCP server code from orb definition

**Components:**

```rust
pub struct CodeGenerator {
    templates: TemplateRegistry,
    formatter: RustFormatter,
}

pub struct TemplateRegistry {
    server_template: Template,
    resources_template: Template,
    tools_template: Template,
    types_template: Template,
}

pub struct GeneratedServer {
    main_rs: String,
    resources_rs: String,
    tools_rs: String,
    types_rs: String,
    cargo_toml: String,
}
```

**Template System (Handlebars):**

```rust
// templates/server.rs.hbs
use pmcp::*;

pub struct {{orb_name}}McpServer {
    orb_name: String,
    {{#if multi_version}}
    versions: HashMap<Version, OrbDefinition>,
    workspace_path: Option<PathBuf>,
    {{else}}
    definition: OrbDefinition,
    {{/if}}
}

#[tokio::main]
async fn main() {
    let server = McpServer::new("{{orb_name}}-mcp");
    
    {{#each resources}}
    server.add_resource(Resource {
        uri: "{{uri}}",
        name: "{{name}}",
        handler: {{handler}},
    });
    {{/each}}
    
    server.run().await;
}
```

**Generated MCP Resources:**

```rust
// Auto-generated from orb
async fn list_commands() -> Vec<CommandInfo> {
    // Returns all commands in orb with parameters
}

async fn list_jobs() -> Vec<JobInfo> {
    // Returns all jobs with configuration
}

async fn list_executors() -> Vec<ExecutorInfo> {
    // Returns all executors with settings
}

async fn get_changelog() -> ChangelogInfo {
    // Returns version history and breaking changes
}
```

**Generated MCP Tools:**

```rust
// Auto-generated validation
async fn validate_config(config_path: String) -> ValidationResult {
    // Validate CircleCI config against orb schema
}

// Auto-generated migration (if migration rules exist)
async fn generate_migration(
    config_path: String,
    target_version: String
) -> MigrationPlan {
    // Generate specific changes needed
}

// Auto-generated detection
async fn detect_version(workspace_path: String) -> VersionInfo {
    // Detect orb version from configs
}
```

---

### Layer 5: Build System

**Purpose:** Compile generated code into deployable artifacts

**Components:**

```rust
pub struct BuildPipeline {
    rust_compiler: RustCompiler,
    docker_builder: DockerBuilder,
    packager: Packager,
}

pub enum OutputFormat {
    Binary {
        target_triple: String,
    },
    Container {
        registry: Option<String>,
        tag: String,
    },
    Source {
        include_build_scripts: bool,
    },
    Skill {
        format_version: String,
    },
}
```

**Binary Build:**

```rust
impl RustCompiler {
    async fn compile(&self, source: &GeneratedServer) -> Result<Binary> {
        // 1. Write source to temp directory
        let temp_dir = create_temp_project(source)?;
        
        // 2. Run cargo build --release
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&temp_dir)
            .status()?;
        
        // 3. Extract binary
        let binary_path = temp_dir.join("target/release/orb-mcp");
        Ok(Binary::from_path(binary_path))
    }
}
```

**Container Build:**

```dockerfile
# Generated Dockerfile
FROM rust:1.75-alpine AS builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/release/{{orb_name}}-mcp /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/{{orb_name}}-mcp"]
```

**Skill File Generation:**

```rust
impl SkillGenerator {
    fn generate(&self, definition: &OrbDefinition) -> String {
        format!(r#"
# {orb_name} Orb

## Version
Current: {version}

## Commands

### {command_name}
{command_description}

**Parameters:**
{parameters}

**Example:**
```yaml
{example}
```
"#)
    }
}
```

---

### Layer 6: Workspace Detection (Multi-Version)

**Purpose:** Detect orb versions in workspace

**Components:**

```rust
pub struct WorkspaceScanner {
    cache: VersionCache,
    patterns: Vec<String>,
}

pub struct VersionDetector {
    orb_name: String,
}

impl VersionDetector {
    async fn detect_versions(&self, workspace: &Path) -> Vec<ConfigVersion> {
        let configs = self.find_configs(workspace).await?;
        
        let mut versions = Vec::new();
        for config_path in configs {
            if let Some(version) = self.extract_version(&config_path).await? {
                versions.push(ConfigVersion {
                    config_path,
                    version,
                });
            }
        }
        
        versions
    }
    
    async fn extract_version(&self, config: &Path) -> Option<Version> {
        let content = fs::read_to_string(config).await?;
        let yaml: Value = serde_yaml::from_str(&content)?;
        
        // Look for: orbs:
        //             toolkit: org/circleci-toolkit@2.5.0
        yaml["orbs"]
            .as_mapping()?
            .iter()
            .find_map(|(name, spec)| {
                if self.matches_orb_name(name) {
                    self.parse_version_from_spec(spec)
                } else {
                    None
                }
            })
    }
}
```

---

## Data Flow

### MVP (Single Version)

```
Orb YAML
  ↓
[Parser] → OrbDefinition
  ↓
[Generator] → GeneratedServer (Rust code)
  ↓
[Compiler] → Binary
  ↓
Deployed MCP Server
```

### Enhanced (Multi-Version)

```
Orb Git Repo
  ↓
[Version Analyzer] → Fetch versions (v1.0.0, v2.0.0, v3.0.0)
  ↓
[Delta Calculator] → Compute deltas between versions
  ↓
[Generator] → GeneratedServer with embedded versions
  ↓
[Compiler] → Binary with all versions
  ↓
[Runtime] MCP Server
  ↓
[Workspace Scanner] → Detect project uses v2.0.0
  ↓
[Version Selector] → Serve v2.0.0 documentation
```

### Migration Workflow

```
User Request: "Upgrade to v3.0.0"
  ↓
[Workspace Scanner] → Current version: v2.5.0
  ↓
[Migration Engine] → Load rules: v2.5.0 → v3.0.0
  ↓
[Config Parser] → Parse current .circleci/config.yml
  ↓
[Rule Applier] → Generate specific changes
  ↓
[Validator] → Validate against v3.0.0 schema
  ↓
Return MigrationPlan to user
```

---

## Technology Stack

### Core Dependencies

```toml
[dependencies]
# MCP Protocol
pmcp = "1.8"                    # MCP SDK

# YAML Processing
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"

# CLI
clap = { version = "4.0", features = ["derive"] }

# Async Runtime
tokio = { version = "1.0", features = ["full"] }

# Template Engine
handlebars = "5.0"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Git Integration (Phase 2)
git2 = "0.18"

# Docker (Phase 2)
bollard = "0.16"                # Docker API

# Testing
[dev-dependencies]
tempfile = "3.0"
trycmd = "0.15"                 # CLI testing per organizational standards
criterion = "0.5"               # Benchmarks
```

### Supporting Tools (Per Organizational Standards)

- **gen-changelog:** CHANGELOG generation from conventional commits
- **cargo release:** Version management and publishing
- **nextsv:** Semantic version calculation
- **pcu:** Program change updater (if needed for advanced features)

### Build Tools

- **Rust:** 1.87+ (or MSRV as needed)
- **Cargo:** For building and testing
- **Docker:** For container builds (optional)
- **CircleCI:** CI/CD platform (mandatory per organizational standards)
- **circleci-toolkit orb:** Reusable CI/CD jobs
- **jerusdp/ci-rust:1.87:** Docker image for CI builds

---

## Configuration Schema

### Generator Configuration

```yaml
# gen-orb-mcp.yaml
orb:
  name: circleci-toolkit
  namespace: your-org
  repository: https://github.com/jerus-org/circleci-toolkit.git
  
versions:
  strategy: last-n
  count: 10
  base_selector: major_versions
  
output:
  formats:
    - binary
    - container
    - skill
  binary:
    targets:
      - x86_64-unknown-linux-gnu
      - x86_64-apple-darwin
  container:
    registry: ghcr.io/jerus-org
    base_image: alpine:latest
  
migration:
  rules_dir: ./migrations
  validate: true
```

### Runtime Configuration (for generated server)

```json
{
  "orb_name": "circleci-toolkit",
  "workspace_path": "/workspace",
  "cache_ttl": 60,
  "version_detection": {
    "enabled": true,
    "patterns": [".circleci/*.yml", ".circleci/*.yaml"]
  }
}
```

---

## Performance Considerations

### Generation Time
- Parse orb YAML: < 100ms
- Generate server code: < 1s
- Compile binary: 30-60s (one-time)

### Runtime Performance
- Server startup: < 100ms
- Memory usage (single version): ~20MB
- Memory usage (10 versions, delta-encoded): ~50MB
- Query response: < 10ms
- Version detection: < 50ms (cached), < 200ms (scan)

### Optimization Strategies
- **Delta encoding:** Reduce memory for multiple versions
- **Lazy loading:** Load old versions on-demand
- **Caching:** Cache workspace scans
- **Async I/O:** Non-blocking file operations

---

## Security Considerations

### Input Validation
- Validate all YAML input against schema
- Sanitize user-provided paths
- Prevent path traversal attacks

### Generated Code Security
- No arbitrary code execution
- No shell injection vulnerabilities
- Safe string interpolation in templates

### Container Security
- Non-root user in containers
- Minimal base images (Alpine)
- Security scanning in CI

### Private Orb Protection
- Support private registries
- No telemetry or phone-home
- Offline operation capability

---

## Testing Strategy

### Unit Tests
- Parser tests: All orb constructs
- Delta calculator: Version diffing accuracy
- Generator: Template rendering correctness
- Migration engine: Rule application

### Integration Tests
- End-to-end: Orb → Binary → MCP server
- Version detection: Multiple configs
- Migration: Real-world breaking changes

### Property-Based Tests
- Delta reconstruction equivalence
- Migration idempotency
- Parser robustness (fuzzing)

### Performance Tests
- Benchmark generation time
- Benchmark server response time
- Memory usage profiling

---

## Error Handling

### Error Categories

```rust
#[derive(thiserror::Error, Debug)]
pub enum GenOrbMcpError {
    #[error("Failed to parse orb YAML: {0}")]
    ParseError(#[from] serde_yaml::Error),
    
    #[error("Invalid orb structure: {0}")]
    ValidationError(String),
    
    #[error("Version {0} not found in repository")]
    VersionNotFound(Version),
    
    #[error("Failed to generate code: {0}")]
    GenerationError(String),
    
    #[error("Build failed: {0}")]
    BuildError(String),
    
    #[error("Git operation failed: {0}")]
    GitError(#[from] git2::Error),
}
```

### Error Recovery
- Graceful degradation (skip invalid versions)
- Clear error messages with suggestions
- Partial success (generate what's possible)

---

## Extensibility

### Plugin Architecture (Future)

```rust
pub trait OrbExtension {
    fn name(&self) -> &str;
    fn transform(&self, definition: &mut OrbDefinition) -> Result<()>;
}

// Example: Add custom validation
struct CustomValidator;
impl OrbExtension for CustomValidator {
    fn transform(&self, definition: &mut OrbDefinition) -> Result<()> {
        // Add custom logic
        Ok(())
    }
}
```

### Custom Templates

Users can provide custom Handlebars templates:

```bash
gen-orb-mcp generate \
  --orb-path ./orb.yml \
  --template-dir ./my-templates
```

---

## Monitoring & Observability

### Logging

```rust
use tracing::{info, warn, error};

info!(
    orb = %orb_name,
    version = %version,
    "Parsing orb definition"
);

warn!(
    command = %cmd_name,
    "Deprecated parameter detected"
);
```

### Metrics (Runtime MCP Server)

- Request count
- Response time distribution
- Error rate
- Version detection cache hits/misses
- Memory usage

---

## Deployment Architecture

### Binary Deployment

```
Developer Machine
├── gen-orb-mcp (CLI)
├── Generated MCP binary
└── .claude/config.json → points to binary
```

### Container Deployment

```
Container Registry
├── circleci-toolkit-mcp:v3.0.0
└── circleci-toolkit-loader:latest (router)

Developer Machine
├── Docker
└── .claude/config.json → points to container
```

### Multi-Version Architecture

```
┌────────────────────────────────────┐
│  Single MCP Server Binary          │
│                                    │
│  Embedded Versions (delta-encoded):│
│  ├── v2.5.0 (base)                 │
│  ├── v2.6.0 (delta)                │
│  ├── v2.7.0 (delta)                │
│  ├── v3.0.0 (base)                 │
│  └── v3.1.0 (delta)                │
│                                    │
│  Runtime:                          │
│  ├── Scan workspace                │
│  ├── Detect versions needed        │
│  └── Serve appropriate version     │
└────────────────────────────────────┘
```

---

## Future Enhancements

### Potential Features
1. **Web UI:** Visual orb documentation browser
2. **Orb Marketplace:** Integration with CircleCI registry
3. **Smart Suggestions:** AI-powered best practices
4. **Cross-Orb Dependencies:** Handle orb dependencies
5. **OpenAPI Export:** Generate OpenAPI specs from orbs
6. **GitHub Actions Support:** Extend to other CI platforms

### Research Areas
1. **Incremental Compilation:** Faster binary builds
2. **WASM Target:** Run MCP server in browser
3. **Language Server Protocol:** IDE integration
4. **Semantic Versioning:** Auto-detect breaking changes

---

This architecture provides a solid foundation for both the MVP and future enhancements while maintaining flexibility for extension and adaptation.
