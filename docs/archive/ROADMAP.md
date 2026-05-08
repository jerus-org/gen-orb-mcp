# gen-orb-mcp: Project Roadmap

## Executive Summary

**gen-orb-mcp** is a Rust-based tool that generates Model Context Protocol (MCP) servers from CircleCI orb definitions. This enables AI coding assistants like Claude Code to understand and work with private CircleCI orbs, including handling breaking changes through automated migration tooling.

**Key Innovation:** First tool to convert CircleCI orb YAML into AI-accessible MCP servers with version-aware documentation and migration support.

**Target Users:**
- Organizations with private CircleCI orbs
- Platform teams managing DevOps tooling
- CircleCI community (open source release)

---

## Phase 1: Minimum Viable Product (MVP)
**Timeline:** 6-8 weeks  
**Goal:** Generate basic MCP servers from orb definitions with single-version support

### MVP Scope

**Core Features:**
1. ✅ Parse CircleCI orb YAML (commands, jobs, executors)
2. ✅ Generate MCP server with Resources (read-only orb documentation)
3. ✅ Generate static binary deployment format
4. ✅ Basic CLI interface
5. ✅ Integration with circleci-toolkit orb release workflow

**MVP Deliverables:**
- Rust crate: `gen-orb-mcp` v0.1.0
- CLI tool that generates MCP servers from orb YAML
- Binary MCP server for circleci-toolkit v3.x (current version)
- Documentation for basic usage
- Integration into jerus-org/circleci-toolkit CI/CD

**Non-Goals for MVP:**
- Multiple version support (Phase 2)
- Migration tooling (Phase 2)
- Container deployment (Phase 2)
- Smart router (Phase 3)

### MVP Architecture

```
gen-orb-mcp (CLI Tool)
├── Input: orb YAML files
├── Parser: Extract commands, jobs, executors
├── Generator: Create MCP server Rust code using pmcp SDK
├── Builder: Compile to binary
└── Output: Standalone MCP server binary
```

**Technology Stack:**
- Rust (stable 1.75+)
- pmcp SDK (MCP protocol implementation)
- serde_yaml (YAML parsing)
- clap (CLI interface)

### MVP Success Criteria

1. **Functionality:** Generate working MCP server from circleci-toolkit orb
2. **Usability:** Developers can query orb documentation via Claude Code
3. **Reliability:** MCP server runs stably without crashes
4. **Integration:** Automated generation in circleci-toolkit release workflow
5. **Performance:** Server startup < 100ms, query response < 10ms

### MVP Timeline

**Week 1-2: Project Setup & Core Parser**
- Repository structure
- Orb YAML parser implementation
- Unit tests for parser

**Week 3-4: MCP Server Generation**
- Template system for server code generation
- pmcp SDK integration
- Resource endpoint generation

**Week 5: Binary Build & Testing**
- Rust compilation pipeline
- End-to-end testing with circleci-toolkit
- Documentation

**Week 6: CI/CD Integration**
- Add generation step to circleci-toolkit release workflow
- Automated testing in CI
- Developer testing with Claude Code

**Week 7-8: Polish & Release**
- Bug fixes
- Documentation improvements
- MVP release: gen-orb-mcp v0.1.0

---

## Phase 2: Enhanced Features
**Timeline:** 8-10 weeks (after MVP)  
**Goal:** Multi-version support, migration tooling, multiple deployment formats

### Enhanced Features

**2.1 Multi-Version Support (Weeks 1-3)**

Multi-version support is a prerequisite for migration tooling (2.2) and is critical for
consumers mid-migration who have different orb versions pinned across their CI files.

*Example:* a consumer migrating from toolkit 4.9.6 to 5.0.0 may have `config.yml` still
on 4.9.6 while `release.yml` is already on 5.0.0. Without multi-version awareness, the
MCP server gives wrong documentation for the files still on the old version and cannot
explain why familiar parameters no longer exist under the new version.

Components:

- **Multi-version orb embedding**: The generated binary embeds documentation for the
  current version plus N prior versions (configurable, default: all versions since last
  major boundary). Delta encoding minimises binary size — only changed jobs/parameters
  are stored per version rather than full snapshots.

- **`ConsumerParser` version detection**: Parses `orbs:` sections across all
  `.circleci/*.yml` files to identify which version of the orb each file uses. Builds
  a per-file version map so documentation queries are routed correctly.

- **Version-aware MCP Resources**: Resource handlers inspect the consumer's pinned
  version (passed as context or detected from the active file) and return documentation
  for the correct version. Responses include version context: "In toolkit 4.9.6, job
  `update_prlog` accepts `min_rust_version`. This parameter was removed in 5.0.0."

- **Cross-version query support**: Claude Code can ask "what did `idiomatic_rust` look
  like in 4.9.6?" and get a correct historical answer — essential when explaining to a
  user why their existing config uses a pattern that no longer exists.

*Integration with 2.2:* The `ConsumerParser` version map feeds directly into the
`Migrator` — files already pinned to the target version are excluded from the migration
plan, so the tool only touches what actually needs changing.

**2.2 Migration Tooling (Weeks 4-6)**

*Design: conformance-based, not path-dependent.*

A static diff-based migration (`v4.11.0 → v5.0.0`) fails against a consumer on v4.8.0
because it assumes a source state that does not exist. The correct model is goal-oriented:
"does your config conform to the target version's contract?" The tool inspects the
consumer's actual CI state and fixes all non-conformant patterns regardless of which
intermediate versions were skipped.

Core components:

- **`OrbDiffer`** — diffs two `OrbDefinition`s to produce `Vec<ConformanceRule>`.
  Auto-detects `JobAbsorbed` (when job B is removed and job A gains a `run_B: bool`
  parameter), and `JobRenamed` (fuzzy-match on parameter-set similarity). Runs at
  release time; the orb author never writes migration rules manually.

- **`ConsumerParser`** — parses consumer `.circleci/*.yml` into a job-graph model
  (`ConsumerConfig`). Resolves orb aliases to versions. Provides `requires_chain()`
  for transitive dependency traversal — critical for `JobAbsorbed` detection
  (find any `label` invocation whose requires-chain includes `update_prlog`,
  regardless of file layout).

- **`Migrator`** — applies `Vec<ConformanceRule>` to `ConsumerConfig` to produce a
  `MigrationPlan`; applies the plan via targeted in-place YAML editing that preserves
  comments and formatting.

- **`gen-orb-mcp diff` CLI command** — fetches previous orb version from registry,
  computes conformance rules, writes JSON audit trail. Called in orb release pipeline.

- **`gen-orb-mcp migrate` CLI command** — applies migration locally from rules file
  without needing the MCP server. Useful for bulk scripted migration.

- **MCP Tools** (generated server) — `plan_migration` and `apply_migration` Tools
  embedded in the generated binary. Conformance rules are baked in at generation time
  alongside orb documentation. Claude Code invokes `plan_migration`, shows the user
  the diff, then calls `apply_migration` on approval.

See `/home/gorta/.claude/plans/gen-orb-mcp-phase2-migration.md` for full design.

**2.3 Multiple Deployment Formats (Weeks 7-8)**
- Container image generation
- Source code distribution
- Skill file generation
- Private registry support

**2.4 Advanced Tools (Weeks 9-10)**
- Config validation against orb schema
- Deprecation warnings
- Usage analytics/best practices

### Enhanced Architecture

```
gen-orb-mcp v0.2.0+
│
├── OrbDiffer (new — Phase 2.1 + 2.2)
│   ├── Fetch prior versions from CircleCI registry
│   ├── Semantic diff: OrbDefinition × OrbDefinition → Vec<ConformanceRule>
│   ├── Delta encoding for multi-version storage
│   ├── JobAbsorbed heuristic (removed job + new run_X bool param)
│   └── JobRenamed heuristic (parameter-set fuzzy match)
│
├── ConsumerParser (new — Phase 2.1 + 2.2)
│   ├── Parse .circleci/*.yml → ConsumerConfig (job-graph model)
│   ├── Per-file orb alias → version resolution
│   └── requires_chain() graph traversal
│
├── Migrator (new — Phase 2.2)
│   ├── Planner: ConformanceRules × ConsumerConfig → MigrationPlan
│   │   (skips files already on target version — from ConsumerParser)
│   ├── Applicator: in-place YAML editing (preserves formatting/comments)
│   └── Reporter: human-readable plan display
│
├── Multi-Format Generator (updated — Phase 2.3)
│   ├── Embeds multi-version delta data + ConformanceRules in generated server
│   ├── Binary builder
│   ├── Container builder (Dockerfile generation)
│   ├── Source packager
│   └── Skill file generator
│
└── Generated MCP Server (enhanced — Phase 2.1 + 2.2)
    ├── Resources (version-aware: routes to correct version per consumer file)
    │   └── Cross-version queries: "what did job X look like in v4.9.6?"
    └── Tools: plan_migration, apply_migration
        (conformance rules + version history embedded at generation time)
```

**Key dependency**: 2.1 (multi-version) must precede 2.2 (migration). The `ConsumerParser`
version map and cross-version documentation are load-bearing for migration correctness.

### Enhanced Success Criteria

1. **Multi-Version:** Handle repos with multiple orb versions correctly
2. **Migration:** Generate accurate migrations for breaking changes
3. **Deployment:** Support all deployment formats (binary, container, source, skill)
4. **Privacy:** Work with private registries and no-internet scenarios
5. **Performance:** Serve 10 versions with < 500KB memory overhead

---

## Phase 3: Smart Router (Optional)
**Timeline:** 4-6 weeks (after Phase 2)  
**Goal:** Context-aware version routing for complex multi-version repositories

### Smart Router Features

**3.1 Workspace Context Detection**
- Multi-config scanning (.circleci/*.yml)
- Per-config version detection
- Context caching

**3.2 Dynamic Version Server Management**
- Lazy loading of version-specific servers
- Subprocess lifecycle management
- Health checks and auto-restart

**3.3 Intelligent Request Routing**
- Route based on file context
- Handle multi-version queries
- Clarification prompts when ambiguous

### Smart Router Architecture

```
Smart Router MCP Server
├── Workspace Scanner
│   ├── Config discovery
│   ├── Version extraction
│   └── Cache management
├── Server Orchestrator
│   ├── Binary spawning
│   ├── Container orchestration
│   └── Lifecycle management
└── Request Router
    ├── Context inference
    ├── Server selection
    └── Response aggregation
```

**Decision Point:** Build only if real-world usage shows need  
**Alternative:** Embedded delta encoding may be sufficient

---

## Phase 4: Open Source Release
**Timeline:** 2-4 weeks (parallel to Phase 2-3)  
**Goal:** Package and release to Rust community

### Open Source Tasks

**4.1 Code Quality (Week 1)**
- Code review and refactoring
- Comprehensive test coverage (>80%)
- CI/CD pipeline (GitHub Actions)
- Security audit

**4.2 Documentation (Week 2)**
- README with quick start
- User guide
- API documentation (rustdoc)
- Example configurations
- Architecture diagrams

**4.3 Community Setup (Week 3)**
- CONTRIBUTING.md
- CODE_OF_CONDUCT.md
- Issue templates
- PR templates
- License selection (Apache-2.0 OR MIT recommended)

**4.4 Release (Week 4)**
- Publish to crates.io
- GitHub release with binaries
- Announcement (blog post, social media)
- Integration examples (CircleCI community orbs)

### Open Source Success Criteria

1. **Adoption:** 50+ downloads in first month
2. **Engagement:** 5+ GitHub stars, 2+ external contributors
3. **Quality:** No critical bugs, <24hr issue response time
4. **Documentation:** Users can get started without assistance

---

## Milestones & Dependencies

### M1: MVP Complete (Week 8)
- ✅ gen-orb-mcp v0.1.0 published to crates.io
- ✅ circleci-toolkit using generated MCP server
- ✅ Basic documentation

### M2: Enhanced Features (Week 18)
- ✅ gen-orb-mcp v0.2.0 with multi-version support
- ✅ Migration tooling functional
- ✅ Multiple deployment formats

### M3: Community Release (Week 22)
- ✅ Open source project active
- ✅ Community adoption starting
- ✅ Example integrations published

### M4: Smart Router (Optional, Week 28)
- ✅ gen-orb-mcp v0.3.0 with router capability
- ✅ Advanced use cases supported

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| pmcp SDK API changes | Medium | High | Pin to stable version, monitor releases |
| MCP protocol evolution | Medium | Medium | Design for extensibility, version protocol separately |
| Complex orb edge cases | High | Medium | Extensive testing with multiple orbs |
| Low adoption | Medium | Low | Focus on jerus-org first, then expand |
| Performance issues with many versions | Low | Medium | Delta encoding, lazy loading |
| Security concerns with private orbs | Low | High | Multiple deployment formats, privacy-first design |

---

## Resource Requirements

### Development Team
- 1 Senior Rust Engineer (full-time, Phases 1-2)
- 1 DevOps Engineer (part-time, CI/CD integration)
- 1 Technical Writer (part-time, documentation)

### Infrastructure
- GitHub repository (hosting, CI/CD)
- crates.io account (package publishing)
- Container registry (optional, for container deployment format)

### Time Investment
- MVP: ~120 hours (3 weeks @ 40hr/week)
- Enhanced: ~160 hours (4 weeks @ 40hr/week)
- Smart Router: ~80 hours (2 weeks @ 40hr/week)
- Open Source: ~40 hours (1 week @ 40hr/week)
- **Total: ~400 hours (~10 weeks)**

---

## Next Steps

### Immediate (This Week)
1. **Create gen-orb-mcp repository**
2. **Define project structure**
3. **Set up development environment**
4. **Begin orb YAML parser implementation**

### Short Term (Weeks 1-4)
1. **Implement core parser and generator**
2. **Create initial templates**
3. **Build first working prototype**
4. **Test with circleci-toolkit**

### Medium Term (Weeks 5-12)
1. **Complete MVP**
2. **Integrate into circleci-toolkit CI/CD**
3. **Begin enhanced features**
4. **Prepare open source release**

### Long Term (Weeks 13+)
1. **Release enhanced version**
2. **Community engagement**
3. **Evaluate smart router need**
4. **Expand to other organizations' orbs**

---

## Success Metrics

### Technical Metrics
- MCP server generation time: < 5 seconds
- Binary size: < 20MB
- Startup time: < 100ms
- Memory usage: < 100MB (10 versions embedded)
- Test coverage: > 80%

### Business Metrics
- Developer satisfaction: 8/10+ (internal survey)
- Time saved on orb migrations: 2+ hours per migration
- Adoption: 3+ other orbs using gen-orb-mcp within 6 months
- Community engagement: 50+ GitHub stars within 3 months

### Community Metrics
- crates.io downloads: 100+ in first 3 months
- GitHub issues: <5 open issues at any time
- External contributions: 3+ PRs from community
- Documentation quality: <10% of users need support beyond docs

---

## Governance & Maintenance

### Post-Release Maintenance
- Bug fixes: Within 48 hours for critical issues
- Feature requests: Reviewed monthly
- Dependency updates: Quarterly security reviews
- Breaking changes: Major version bumps with migration guides

### Long-Term Vision
- Integration with other CI/CD platforms (GitHub Actions, GitLab CI)
- OpenAPI → MCP generator (generalize beyond orbs)
- Visual orb documentation generator
- Orb marketplace integration

---

## Clarifying Questions

Before proceeding with detailed implementation plans, please clarify:

1. **Team Size:** Is this a solo project or do you have a team? This affects timeline.

2. **MVP Priority:** Which is more important for MVP - working with circleci-toolkit quickly, or having a more general solution? This affects scope.

3. **Versioning Strategy:** For circleci-toolkit, how many versions back do you typically need to support simultaneously? This affects delta encoding strategy.

4. **Privacy Requirements:** Are there hard requirements around private registries or can initial releases use public GitHub/crates.io? This affects Phase 1 scope.

5. **Smart Router:** Is multi-config, multi-version support a hard requirement (build router in Phase 2) or nice-to-have (defer to Phase 3)? This affects priority.

6. **Open Source Timing:** Should open source release wait until enhanced features (Phase 2), or release MVP early to gather community feedback? This affects strategy.

7. **Repository Location:** Should gen-orb-mcp live in jerus-org or a separate organization? This affects governance.

8. **License Preference:** Any preference between Apache-2.0, MIT, or dual-license? This affects legal aspects.

Please review and provide answers to these questions so I can refine the subsequent detailed planning documents.
