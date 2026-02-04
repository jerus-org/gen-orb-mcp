# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add manual approval step for release verification(pr [#16])
- Add OrbParser for CircleCI orb YAML(pr [#17])
- Add CodeGenerator for MCP server generation(pr [#18])

### Changed

- chore-Use nextsv for version calculation(pr [#15])

### Fixed

- Bootstrap workspace versioning with v0.1.0(pr [#11])
- Add check_tag_exists for release resilience(pr [#12])
- Skip GitHub release if already exists(pr [#13])
- Use GitHub API to check for existing release(pr [#14])

## [0.1.0] - 2026-01-26

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
[Unreleased]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jerus-org/gen-orb-mcp/releases/tag/v0.1.0
