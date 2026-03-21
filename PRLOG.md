# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2026-03-21

### Fixed

- custom Serialize for StructuredStep to avoid YAML tags(pr [#70])

## [0.1.3] - 2026-03-20

### Added

- add prime command(pr [#67])

### Changed

- ci-revert inline release_prlog to standard toolkit job(pr [#66])

### Fixed

- deps: update rust crate chrono to 0.4.44(pr [#68])
- deps: update rust crate semver to 1.0.27(pr [#69])

## [0.1.2] - 2026-03-19

### Added

- Phase 2.2 conformance-based migration tooling(pr [#60])
- migration tools in generated MCP server(pr [#61])

### Changed

- docs-update README and QUICKSTART for current features(pr [#62])
- ci-validate GPG-signed workspace tag via inline release job(pr [#65])

### Fixed

- deps: update rust crate tracing-subscriber to 0.3.23(pr [#63])
- deps: update rust crate pmcp to 1.20.0(pr [#64])

## [0.1.1] - 2026-03-13

### Added

- add cargo-binstall support(pr [#50])

### Changed

- chore-reset version overrides to auto-detect(pr [#28])
- docs-add AI Diligence Statement(pr [#29])
- docs-update CLAUDE.md with learnings and current status(pr [#30])
- docs-add project logo and article cover graphics(pr [#31])
- docs-add CI integration guide(pr [#33])
- ci-migrate to 3-file pipeline model at toolkit 4.9.6(pr [#56])

### Fixed

- deps: update dependency toolkit to v4.4.2(pr [#36])
- deps: update rust crate anyhow to 1.0.101(pr [#37])
- deps: update rust crate clap to 4.5.57(pr [#38])
- deps: update rust crate handlebars to 5.1.2(pr [#39])
- deps: update rust crate thiserror to 1.0.69(pr [#40])
- deps: update rust crate thiserror to v2(pr [#49])
- deps: update rust crate tokio to 1.49.0(pr [#41])
- deps: update rust crate trycmd to 0.15.11(pr [#42])
- deps: update serde packages(pr [#43])
- deps: update rust crate pmcp to 1.10.2(pr [#46])
- deps: update rust crate tempfile to 3.25.0(pr [#47])
- deps: update rust crate handlebars to v6(pr [#48])
- deps: update rust crate pmcp to 1.10.3(pr [#54])
- ci: remove trigger_pipeline from config.yml(pr [#59])
- deps: update rust crate anyhow to 1.0.102(pr [#52])
- deps: update rust crate clap to 4.6.0(pr [#53])
- deps: update rust crate trycmd to v1(pr [#55])
- deps: update rust crate tempfile to 3.27.0(pr [#57])
- deps: update rust crate tokio to 1.50.0(pr [#58])

## [0.1.0] - 2026-02-05

### Added

- Add manual approval step for release verification(pr [#16])
- Add OrbParser for CircleCI orb YAML(pr [#17])
- Add CodeGenerator for MCP server generation(pr [#18])

### Changed

- chore-Use nextsv for version calculation(pr [#15])
- refactor-migrate MCP SDK from pmcp to rmcp(pr [#20])
- docs-explain MCP resources vs tools(pr [#22])
- chore-set explicit version 0.1.0 for release(pr [#24])

### Fixed

- Bootstrap workspace versioning with v0.1.0(pr [#11])
- Add check_tag_exists for release resilience(pr [#12])
- Skip GitHub release if already exists(pr [#13])
- Use GitHub API to check for existing release(pr [#14])
- Handle multi-line descriptions in generated code(pr [#19])
- update templates for rmcp 0.14 API compatibility(pr [#21])
- improve default orb name derivation(pr [#23])
- align calculate-versions with release-crate(pr [#25])
- add workspace version override for PRLOG release(pr [#26])
- pull latest main in release-prlog before pushing(pr [#27])

## [0.1.0-alpha.1] - 2026-01-26

### Changed

- chore-Update Cargo.lock to match version 0.1.0-alpha.1(pr [#10])

### Fixed

- Correct CircleCI config for release(pr [#4])
- Correct release config for pre-release(pr [#5])
- Simplify release config for package releases(pr [#6])
- Repair release after partial publish(pr [#7])
- Correct pcu release command and update pcu version(pr [#8])
- Re-enable cargo release and add release-prlog script(pr [#9])

[#4]: https://github.com/jerus-org/gen-orb-mcp/pull/4
[#5]: https://github.com/jerus-org/gen-orb-mcp/pull/5
[#6]: https://github.com/jerus-org/gen-orb-mcp/pull/6
[#7]: https://github.com/jerus-org/gen-orb-mcp/pull/7
[#8]: https://github.com/jerus-org/gen-orb-mcp/pull/8
[#9]: https://github.com/jerus-org/gen-orb-mcp/pull/9
[#10]: https://github.com/jerus-org/gen-orb-mcp/pull/10
[#11]: https://github.com/jerus-org/gen-orb-mcp/pull/11
[#12]: https://github.com/jerus-org/gen-orb-mcp/pull/12
[#13]: https://github.com/jerus-org/gen-orb-mcp/pull/13
[#14]: https://github.com/jerus-org/gen-orb-mcp/pull/14
[#15]: https://github.com/jerus-org/gen-orb-mcp/pull/15
[#16]: https://github.com/jerus-org/gen-orb-mcp/pull/16
[#17]: https://github.com/jerus-org/gen-orb-mcp/pull/17
[#18]: https://github.com/jerus-org/gen-orb-mcp/pull/18
[#19]: https://github.com/jerus-org/gen-orb-mcp/pull/19
[#20]: https://github.com/jerus-org/gen-orb-mcp/pull/20
[#21]: https://github.com/jerus-org/gen-orb-mcp/pull/21
[#22]: https://github.com/jerus-org/gen-orb-mcp/pull/22
[#23]: https://github.com/jerus-org/gen-orb-mcp/pull/23
[#24]: https://github.com/jerus-org/gen-orb-mcp/pull/24
[#25]: https://github.com/jerus-org/gen-orb-mcp/pull/25
[#26]: https://github.com/jerus-org/gen-orb-mcp/pull/26
[#27]: https://github.com/jerus-org/gen-orb-mcp/pull/27
[#28]: https://github.com/jerus-org/gen-orb-mcp/pull/28
[#29]: https://github.com/jerus-org/gen-orb-mcp/pull/29
[#30]: https://github.com/jerus-org/gen-orb-mcp/pull/30
[#31]: https://github.com/jerus-org/gen-orb-mcp/pull/31
[#33]: https://github.com/jerus-org/gen-orb-mcp/pull/33
[#36]: https://github.com/jerus-org/gen-orb-mcp/pull/36
[#37]: https://github.com/jerus-org/gen-orb-mcp/pull/37
[#38]: https://github.com/jerus-org/gen-orb-mcp/pull/38
[#39]: https://github.com/jerus-org/gen-orb-mcp/pull/39
[#40]: https://github.com/jerus-org/gen-orb-mcp/pull/40
[#49]: https://github.com/jerus-org/gen-orb-mcp/pull/49
[#41]: https://github.com/jerus-org/gen-orb-mcp/pull/41
[#42]: https://github.com/jerus-org/gen-orb-mcp/pull/42
[#43]: https://github.com/jerus-org/gen-orb-mcp/pull/43
[#46]: https://github.com/jerus-org/gen-orb-mcp/pull/46
[#47]: https://github.com/jerus-org/gen-orb-mcp/pull/47
[#48]: https://github.com/jerus-org/gen-orb-mcp/pull/48
[#50]: https://github.com/jerus-org/gen-orb-mcp/pull/50
[#54]: https://github.com/jerus-org/gen-orb-mcp/pull/54
[#56]: https://github.com/jerus-org/gen-orb-mcp/pull/56
[#59]: https://github.com/jerus-org/gen-orb-mcp/pull/59
[#52]: https://github.com/jerus-org/gen-orb-mcp/pull/52
[#53]: https://github.com/jerus-org/gen-orb-mcp/pull/53
[#55]: https://github.com/jerus-org/gen-orb-mcp/pull/55
[#57]: https://github.com/jerus-org/gen-orb-mcp/pull/57
[#58]: https://github.com/jerus-org/gen-orb-mcp/pull/58
[#60]: https://github.com/jerus-org/gen-orb-mcp/pull/60
[#61]: https://github.com/jerus-org/gen-orb-mcp/pull/61
[#62]: https://github.com/jerus-org/gen-orb-mcp/pull/62
[#63]: https://github.com/jerus-org/gen-orb-mcp/pull/63
[#64]: https://github.com/jerus-org/gen-orb-mcp/pull/64
[#65]: https://github.com/jerus-org/gen-orb-mcp/pull/65
[#66]: https://github.com/jerus-org/gen-orb-mcp/pull/66
[#67]: https://github.com/jerus-org/gen-orb-mcp/pull/67
[#68]: https://github.com/jerus-org/gen-orb-mcp/pull/68
[#69]: https://github.com/jerus-org/gen-orb-mcp/pull/69
[#70]: https://github.com/jerus-org/gen-orb-mcp/pull/70
[0.1.4]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.0-alpha.1...v0.1.0
[0.1.0-alpha.1]: https://github.com/jerus-org/gen-orb-mcp/releases/tag/v0.1.0-alpha.1
