# gen-orb-mcp: Implementation Plan

## MVP Phase (Weeks 1-8)

### Week 1-2: Project Setup & Core Parser

#### Task 1.1: Repository Setup
**Owner:** Development Lead  
**Duration:** 2 days  
**Dependencies:** None

**Subtasks:**
- [ ] Create GitHub repository: `jerus-org/gen-orb-mcp`
- [ ] Initialize Rust project with cargo
- [ ] Set up directory structure
- [ ] Configure .gitignore and .editorconfig
- [ ] Set up GitHub Actions CI/CD pipeline
- [ ] Add MIT/Apache-2.0 dual license
- [ ] Create initial README

**Acceptance Criteria:**
- Repository accessible and properly configured
- CI pipeline runs on push (lint, test, build)
- README contains project description and goals

**Deliverables:**
```
gen-orb-mcp/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── src/
│   ├── main.rs
│   └── lib.rs
├── Cargo.toml
├── README.md
└── LICENSE-MIT / LICENSE-APACHE
```

---

#### Task 1.2: Orb YAML Parser
**Owner:** Development Lead  
**Duration:** 5 days  
**Dependencies:** 1.1

**Subtasks:**
- [ ] Define Rust structs for orb schema
- [ ] Implement YAML parsing using serde_yaml
- [ ] Add validation logic for required fields
- [ ] Handle orb versions (2.0, 2.1)
- [ ] Write comprehensive tests
- [ ] Document parser API

**Acceptance Criteria:**
- Successfully parse circleci-toolkit orb YAML
- Validate all commands, jobs, executors
- Handle missing optional fields gracefully
- Test coverage > 80%
- Clear error messages for invalid YAML

**Deliverables:**
```rust
// src/parser/mod.rs
pub struct OrbParser;
pub struct OrbDefinition {
    pub version: String,
    pub commands: HashMap<String, Command>,
    pub jobs: HashMap<String, Job>,
    pub executors: HashMap<String, Executor>,
}

// Tests
#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_valid_orb() { }
    
    #[test]
    fn test_parse_invalid_orb() { }
}
```

**Test Data:**
- Valid orb samples (simple, complex)
- Invalid orbs (syntax errors, missing fields)
- Edge cases (empty commands, nested parameters)

---

#### Task 1.3: CLI Interface
**Owner:** Development Lead  
**Duration:** 3 days  
**Dependencies:** 1.2

**Subtasks:**
- [ ] Define CLI structure using clap
- [ ] Implement `generate` command
- [ ] Add `--orb-path` argument
- [ ] Add `--output` argument
- [ ] Add `--version` and `--help`
- [ ] Implement input validation
- [ ] Add progress indicators
- [ ] Write CLI tests

**Acceptance Criteria:**
- `gen-orb-mcp generate --help` shows usage
- Validates input paths exist
- Shows progress during generation
- Handles errors gracefully
- Returns appropriate exit codes

**Deliverables:**
```bash
$ gen-orb-mcp generate \
    --orb-path ./circleci-toolkit/src/@orb.yml \
    --output ./dist/

Parsing orb definition... ✓
Generating MCP server code... ✓
Building binary... ✓
Output: ./dist/circleci-toolkit-mcp
```

---

### Week 3-4: MCP Server Generation

#### Task 2.1: Template System
**Owner:** Development Lead  
**Duration:** 4 days  
**Dependencies:** 1.2

**Subtasks:**
- [ ] Set up Handlebars template engine
- [ ] Create template directory structure
- [ ] Write main.rs.hbs template
- [ ] Write resources.rs.hbs template
- [ ] Write types.rs.hbs template
- [ ] Write Cargo.toml.hbs template
- [ ] Add template rendering logic
- [ ] Test template generation

**Acceptance Criteria:**
- Templates generate valid Rust code
- All orb information accessible in templates
- Generated code compiles successfully
- Templates are readable and maintainable

**Deliverables:**
```
templates/
├── server/
│   ├── main.rs.hbs
│   ├── resources.rs.hbs
│   ├── tools.rs.hbs
│   └── types.rs.hbs
├── cargo/
│   └── Cargo.toml.hbs
└── README.md
```

**Template Example:**
```handlebars
{{!-- main.rs.hbs --}}
use pmcp::*;

#[derive(Clone)]
pub struct {{orb_name}}McpServer {
    definition: OrbDefinition,
}

#[tokio::main]
async fn main() {
    let server = McpServer::new("{{orb_name}}-mcp");
    
    {{#each resources}}
    server.add_resource({{this}});
    {{/each}}
    
    server.run().await.unwrap();
}
```

---

#### Task 2.2: Resource Generation
**Owner:** Development Lead  
**Duration:** 4 days  
**Dependencies:** 2.1

**Subtasks:**
- [ ] Implement list_commands resource
- [ ] Implement list_jobs resource
- [ ] Implement list_executors resource
- [ ] Implement get_command_info resource
- [ ] Implement get_job_info resource
- [ ] Add resource metadata
- [ ] Generate resource handlers
- [ ] Test resource generation

**Acceptance Criteria:**
- Resources expose all orb capabilities
- Resource URIs follow MCP conventions
- Resource descriptions are clear
- Generated handlers compile
- Runtime tests pass

**Deliverables:**
```rust
// Generated resources
async fn list_commands() -> Result<Vec<CommandInfo>> {
    // Return all commands from orb
}

async fn get_command_info(name: String) -> Result<CommandInfo> {
    // Return specific command details
}
```

---

#### Task 2.3: pmcp Integration
**Owner:** Development Lead  
**Duration:** 3 days  
**Dependencies:** 2.2

**Subtasks:**
- [ ] Add pmcp dependency to Cargo.toml
- [ ] Integrate pmcp server initialization
- [ ] Map orb resources to pmcp resources
- [ ] Handle resource discovery
- [ ] Implement error handling
- [ ] Test pmcp protocol compliance
- [ ] Document pmcp usage

**Acceptance Criteria:**
- Generated server uses pmcp SDK correctly
- Server responds to MCP protocol messages
- Resources discoverable via list_resources
- Error responses follow MCP format
- Protocol tests pass

---

### Week 5: Binary Build & Testing

#### Task 3.1: Rust Compiler Integration
**Owner:** Development Lead  
**Duration:** 3 days  
**Dependencies:** 2.3

**Subtasks:**
- [ ] Implement build pipeline
- [ ] Create temporary build directory
- [ ] Execute cargo build --release
- [ ] Extract compiled binary
- [ ] Handle compilation errors
- [ ] Add progress reporting
- [ ] Clean up temporary files
- [ ] Test build process

**Acceptance Criteria:**
- Successfully compiles generated code
- Produces runnable binary
- Handles compilation errors gracefully
- Build time < 60 seconds
- Binary size < 20MB

**Deliverables:**
```rust
pub struct RustCompiler {
    cargo_path: PathBuf,
}

impl RustCompiler {
    pub async fn compile(
        &self,
        source: &GeneratedServer
    ) -> Result<PathBuf> {
        // Build and return binary path
    }
}
```

---

#### Task 3.2: End-to-End Testing
**Owner:** Development Lead  
**Duration:** 4 days  
**Dependencies:** 3.1

**Subtasks:**
- [ ] Create test orb samples
- [ ] Write integration tests
- [ ] Test full pipeline (parse → generate → build)
- [ ] Test generated MCP server
- [ ] Verify Claude Code compatibility
- [ ] Performance testing
- [ ] Document test procedures

**Acceptance Criteria:**
- Full pipeline works end-to-end
- Generated server responds correctly
- Claude Code can query server
- All integration tests pass
- Performance within acceptable limits

**Test Scenarios:**
```rust
#[tokio::test]
async fn test_full_pipeline() {
    // Parse orb
    let orb = OrbParser::parse("test-orb.yml")?;
    
    // Generate server
    let server = Generator::generate(&orb)?;
    
    // Build binary
    let binary = Compiler::compile(&server)?;
    
    // Run server
    let handle = spawn_server(binary);
    
    // Query via MCP
    let response = query_mcp("list_commands").await?;
    
    assert!(response.commands.len() > 0);
}
```

---

### Week 6: CI/CD Integration

#### Task 4.1: CircleCI Toolkit Integration
**Owner:** DevOps Engineer  
**Duration:** 3 days  
**Dependencies:** 3.2

**Subtasks:**
- [ ] Add gen-orb-mcp to circleci-toolkit CI
- [ ] Create generation job
- [ ] Configure artifact storage
- [ ] Add version tagging
- [ ] Test in CI environment
- [ ] Document integration

**Acceptance Criteria:**
- Automatic generation on tag push
- Binary artifact uploaded
- Version matches orb version
- CI job completes successfully

**CircleCI Config:**
```yaml
# .circleci/config.yml
jobs:
  generate-mcp-server:
    docker:
      - image: rust:1.75
    steps:
      - checkout
      - run:
          name: Install gen-orb-mcp
          command: cargo install gen-orb-mcp
      - run:
          name: Generate MCP Server
          command: |
            gen-orb-mcp generate \
              --orb-path ./src/@orb.yml \
              --output ./dist/
      - store_artifacts:
          path: ./dist/circleci-toolkit-mcp
          
workflows:
  release:
    jobs:
      - orb-tools/publish:
          filters:
            tags:
              only: /^v.*/
      - generate-mcp-server:
          requires:
            - orb-tools/publish
          filters:
            tags:
              only: /^v.*/
```

---

#### Task 4.2: Developer Testing
**Owner:** Development Team  
**Duration:** 2 days  
**Dependencies:** 4.1

**Subtasks:**
- [ ] Distribute binary to test users
- [ ] Configure Claude Code with MCP server
- [ ] Test real-world workflows
- [ ] Collect feedback
- [ ] Document issues
- [ ] Fix critical bugs

**Acceptance Criteria:**
- 3+ developers successfully use server
- Claude Code queries work correctly
- No critical bugs identified
- Positive user feedback

**Test Workflows:**
1. "What parameters does build-job accept?"
2. "Show me an example of using deploy-job"
3. "What executors are available?"

---

### Week 7-8: Polish & Release

#### Task 5.1: Documentation
**Owner:** Technical Writer  
**Duration:** 5 days  
**Dependencies:** 4.2

**Subtasks:**
- [ ] Write user guide
- [ ] Create quick start tutorial
- [ ] Document CLI usage
- [ ] Add architecture documentation
- [ ] Write contributing guide
- [ ] Generate API docs (rustdoc)
- [ ] Create example configurations

**Acceptance Criteria:**
- Complete user documentation
- Clear quick start (< 5 minutes)
- All public APIs documented
- Examples for common use cases

**Documentation Structure:**
```
docs/
├── user-guide/
│   ├── installation.md
│   ├── quick-start.md
│   ├── configuration.md
│   └── troubleshooting.md
├── developer-guide/
│   ├── architecture.md
│   ├── contributing.md
│   └── testing.md
└── examples/
    ├── basic-usage.md
    └── circleci-toolkit.md
```

---

#### Task 5.2: MVP Release
**Owner:** Development Lead  
**Duration:** 3 days  
**Dependencies:** 5.1

**Subtasks:**
- [ ] Final code review
- [ ] Update version to 0.1.0
- [ ] Create release notes
- [ ] Publish to crates.io
- [ ] Tag GitHub release
- [ ] Announce in CircleCI community
- [ ] Monitor for issues

**Acceptance Criteria:**
- Clean crates.io publication
- GitHub release with binaries
- No critical issues in first week
- Positive community response

**Release Checklist:**
- [ ] All tests passing
- [ ] Documentation complete
- [ ] Version numbers updated
- [ ] CHANGELOG.md updated
- [ ] Security audit complete
- [ ] Performance benchmarks run

---

## Enhanced Phase (Weeks 9-18)

### Week 9-11: Multi-Version Support

#### Task 6.1: Git Integration
**Owner:** Development Lead  
**Duration:** 4 days

**Subtasks:**
- [ ] Add git2 dependency
- [ ] Implement repository cloning
- [ ] Extract versions from git tags
- [ ] Checkout specific versions
- [ ] Parse multiple orb versions
- [ ] Cache repository locally

**Acceptance Criteria:**
- Fetch orb history from git
- Parse all tagged versions
- Handle missing tags gracefully
- Cache improves performance

---

#### Task 6.2: Delta Encoding
**Owner:** Development Lead  
**Duration:** 6 days

**Subtasks:**
- [ ] Implement delta calculation
- [ ] Design delta storage format
- [ ] Write reconstruction algorithm
- [ ] Optimize delta size
- [ ] Test reconstruction accuracy
- [ ] Benchmark performance

**Acceptance Criteria:**
- Accurate version reconstruction
- 50%+ space savings vs full storage
- Reconstruction time < 5ms
- Handles all delta types

**Delta Format:**
```rust
pub struct OrbDelta {
    from: Version,
    to: Version,
    commands: CommandDelta,
    jobs: JobDelta,
    executors: ExecutorDelta,
}

pub struct CommandDelta {
    added: Vec<Command>,
    removed: Vec<String>,
    modified: Vec<(String, Diff)>,
}
```

---

#### Task 6.3: Version Detection
**Owner:** Development Lead  
**Duration:** 5 days

**Subtasks:**
- [ ] Implement workspace scanner
- [ ] Parse CircleCI config files
- [ ] Extract orb versions
- [ ] Cache detection results
- [ ] Handle multiple configs
- [ ] Test detection accuracy

**Acceptance Criteria:**
- Detects versions in all configs
- Handles multiple versions
- Cache reduces scan time
- Works with nested configs

---

### Week 12-14: Migration Tooling

#### Task 7.1: Migration Schema
**Owner:** Development Lead  
**Duration:** 3 days

**Subtasks:**
- [ ] Define migration YAML schema
- [ ] Create example migrations
- [ ] Add schema validation
- [ ] Document migration format
- [ ] Create template migrations

**Schema Example:**
```yaml
version_from: "2.5.0"
version_to: "3.0.0"

breaking_changes:
  - id: rename-param
    type: parameter_renamed
    scope:
      commands: [build-job]
    changes:
      old: node_version
      new: runtime_version
```

---

#### Task 7.2: Migration Engine
**Owner:** Development Lead  
**Duration:** 6 days

**Subtasks:**
- [ ] Parse migration YAML
- [ ] Implement rule types
- [ ] Write rule application logic
- [ ] Test migration accuracy
- [ ] Handle edge cases
- [ ] Add dry-run mode

**Acceptance Criteria:**
- Applies all rule types correctly
- Generates accurate migrations
- Handles complex transformations
- Dry-run shows changes without applying

---

#### Task 7.3: Migration Tool Integration
**Owner:** Development Lead  
**Duration:** 4 days

**Subtasks:**
- [ ] Add migration tool to MCP server
- [ ] Implement generate_migration tool
- [ ] Add validation of migrated config
- [ ] Test with real migrations
- [ ] Document migration workflow

**Acceptance Criteria:**
- Claude can generate migrations
- Validation catches errors
- Real-world migrations work
- Clear user feedback

---

### Week 15-16: Multiple Deployment Formats

#### Task 8.1: Container Builder
**Owner:** DevOps Engineer  
**Duration:** 5 days

**Subtasks:**
- [ ] Generate Dockerfile
- [ ] Implement Docker build
- [ ] Add registry push
- [ ] Support private registries
- [ ] Test container deployment
- [ ] Document container usage

---

#### Task 8.2: Source Distribution
**Owner:** Development Lead  
**Duration:** 3 days

**Subtasks:**
- [ ] Package generated source
- [ ] Add build instructions
- [ ] Test manual compilation
- [ ] Document source distribution

---

#### Task 8.3: Skill File Generator
**Owner:** Development Lead  
**Duration:** 3 days

**Subtasks:**
- [ ] Design skill file format
- [ ] Generate SKILL.md
- [ ] Test with Claude Code
- [ ] Document skill usage

---

### Week 17-18: Advanced Features

#### Task 9.1: Config Validation Tool
**Owner:** Development Lead  
**Duration:** 4 days

**Subtasks:**
- [ ] Implement validation logic
- [ ] Check against orb schema
- [ ] Report detailed errors
- [ ] Test validation accuracy

---

#### Task 9.2: Enhanced Release
**Owner:** Development Lead  
**Duration:** 3 days

**Subtasks:**
- [ ] Update to version 0.2.0
- [ ] Create release notes
- [ ] Publish enhanced version
- [ ] Update documentation
- [ ] Announce new features

---

## Testing Strategy

### Unit Tests
```bash
cargo test --lib
```
- Parser tests: 100+ test cases
- Delta calculation: 50+ test cases
- Migration engine: 75+ test cases

### Integration Tests
```bash
cargo test --test integration
```
- End-to-end generation
- Multi-version scenarios
- Migration workflows

### Performance Tests
```bash
cargo bench
```
- Parse time < 100ms
- Generation time < 5s
- Binary build time < 60s
- Runtime query < 10ms

---

## Risk Mitigation

### Technical Risks

**Risk:** pmcp SDK API changes  
**Mitigation:** Pin to specific version, monitor releases, test before upgrading

**Risk:** Complex orb edge cases  
**Mitigation:** Extensive test suite, gradual rollout, community feedback

**Risk:** Performance issues  
**Mitigation:** Benchmark early, optimize hot paths, profile regularly

### Schedule Risks

**Risk:** Scope creep  
**Mitigation:** Strict MVP definition, defer non-critical features

**Risk:** Unforeseen complexity  
**Mitigation:** Buffer time in schedule, agile iteration

---

## Definition of Done

### MVP
- [ ] All MVP tasks complete
- [ ] Test coverage > 80%
- [ ] Documentation complete
- [ ] Published to crates.io
- [ ] Integrated with circleci-toolkit
- [ ] 3+ successful user tests
- [ ] No critical bugs

### Enhanced
- [ ] All enhanced tasks complete
- [ ] Test coverage > 85%
- [ ] Advanced features working
- [ ] Performance benchmarks met
- [ ] Community feedback positive

---

## Success Metrics

**MVP Success:**
- ✅ gen-orb-mcp v0.1.0 published
- ✅ circleci-toolkit using generated server
- ✅ 5+ developers successfully using server
- ✅ < 3 critical bugs in first month

**Enhanced Success:**
- ✅ gen-orb-mcp v0.2.0 published
- ✅ Multi-version support working
- ✅ 2+ organizations adopting tool
- ✅ 50+ crates.io downloads

