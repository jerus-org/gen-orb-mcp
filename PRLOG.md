# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- chore-inherit renovate pin managers from shared config(pr [#248])

### Fixed

- deps: update dependency gen-orb-mcp to v0.1.51(pr [#239])
- deps: update dependency gen-circleci-orb to v0.0.62(pr [#238])
- deps: update spin to 0.9.9 (drop yanked 0.9.8)(pr [#243])
- deps: update dependency toolkit to v6.6.1(pr [#242])
- deps: update rust crate pmcp to 2.15.0(pr [#240])
- deps: lock file maintenance(pr [#241])
- deps: update rust crate clap to 4.6.2(pr [#245])
- deps: update rust crate handlebars to 6.4.3(pr [#246])
- deps: update rust crate tokio to 1.52.4(pr [#247])
- deps: update pinned containers(pr [#244])
- deps: update rust crate tokio to 1.53.0(pr [#249])

## [0.1.59] - 2026-07-09

### Fixed

- deps: update rust crate pmcp to 2.13.0(pr [#230])
- deps: bump crossbeam-epoch to 0.9.20 (RUSTSEC-2026-0204)(pr [#232])
- deps: update dependency gen-circleci-orb to v0.0.60(pr [#231])
- deps: update dependency toolkit to v6.5.1(pr [#233])
- deps: update dependency gen-circleci-orb to v0.0.61(pr [#234])
- rust_image for build binary jobs (libclang)(pr [#236])
- deps: lock file maintenance(pr [#235])
- deps: lock file maintenance(pr [#237])

## [0.1.58] - 2026-07-04

### Fixed

- clang+cmake for the MCP build image on trixie(pr [#223])
- pin orb image digests in config(pr [#224])
- deps: update rust crate anyhow to 1.0.103(pr [#226])
- deps: update dependency gen-orb-mcp to v0.1.49(pr [#225])
- deps: update rust crate config to 0.15.25(pr [#227])
- deps: update rust crate pcu to 0.6.28(pr [#228])
- deps: update rust crate pmcp to 2.11.0(pr [#229])

## [0.1.57] - 2026-07-01

### Fixed

- repair build-mcp-server signed save-back(pr [#222])

## [0.1.56] - 2026-06-30

### Added

- configurable signing/publish env-var names (#185)(pr [#220])

### Changed

- ci-dogfood gen-orb-mcp orb + refresh orb pins (Stage 1)(pr [#219])
- ci-dogfood gen-orb-mcp/build_mcp_server (adopt cede)(pr [#221])

### Fixed

- lift RUSTSEC-2026-0173 + prune stale ignore(pr [#218])

## [0.1.55] - 2026-06-25

### Added

- adopt gen-circleci-orb auto-record(pr [#207])
- adopt gen-circleci-orb 0.0.50 (Model B auto-record)(pr [#208])

### Fixed

- deps: update dependency toolkit to v6.4.2(pr [#206])
- deps: update pinned containers(pr [#202])
- deps: update dependency gen-orb-mcp to v0.1.46(pr [#203])
- deps: update rust crate config to 0.15.24(pr [#204])
- deps: update rust crate pcu to 0.6.25(pr [#205])
- enable git2 https+ssh features; MSRV 1.91(pr [#201])
- curated run-step names in committed orb command files(pr [#200])
- deps: update rust crate pcu to 0.6.27(pr [#209])
- deps: update rust crate pmcp to 2.10.0(pr [#212])
- deps: update rust crate handlebars to 6.4.2(pr [#213])
- deps: pin dependencies(pr [#214])
- ci: push-orb auths via release App; restrict to non-main(pr [#215])
- deps: update dependency gen-circleci-orb to v0.0.52(pr [#211])
- deps: pin dependencies(pr [#216])
- deps: update dependency gen-circleci-orb to v0.0.53(pr [#217])
- deps: bump quinn-proto for RUSTSEC-2026-0185(pr [#210])

## [0.1.54] - 2026-06-15

### Fixed

- build_mcp_server runs stable image, not /tmp binary(pr [#199])

## [0.1.53] - 2026-06-14

### Added

- curated run-step labels for orb commands(pr [#198])

### Changed

- chore-pin build_mcp_server orb to gen-orb-mcp@0.1.44(pr [#197])

## [0.1.52] - 2026-06-13

### Added

- pass identity + signing key explicitly to pcu(pr [#195])

### Fixed

- deps: update dependency gen-circleci-orb to v0.0.47(pr [#191])
- deps: update dependency orb-tools to v12.4.0(pr [#193])
- deps: update rust crate pcu to 0.6.22(pr [#196])
- deps: pin dependencies(pr [#190])
- deps: update dependency gen-orb-mcp to v0.1.43(pr [#192])
- deps: update git2 packages(pr [#194])

## [0.1.51] - 2026-06-12

### Changed

- chore-pin build_mcp_server orb to gen-orb-mcp@0.1.42(pr [#189])

### Fixed

- build_mcp_server generate uses crate version(pr [#188])

## [0.1.50] - 2026-06-12

### Fixed

- write git identity to global config in save --sign(pr [#187])

## [0.1.49] - 2026-06-11

### Fixed

- set git identity via git config in save --sign(pr [#186])

## [0.1.48] - 2026-06-11

### Changed

- chore-self-host build_mcp_server + CI corrections(pr [#184])

## [0.1.47] - 2026-06-11

### Added

- export build_mcp_server as a composed job_group(pr [#183])

## [0.1.46] - 2026-06-10

### Added

- publish --name derive, save comma paths(pr [#181])

### Changed

- chore-simplify mcp executor config via gen-circleci-orb 0.0.44(pr [#180])
- chore-toolkit 6.3.0 + ignore RUSTSEC-2026-0173(pr [#182])

## [0.1.45] - 2026-06-04

### Fixed

- orb: add gnupg to executor image for save --sign(pr [#178])
- deps: update rust crate chrono to 0.4.45(pr [#179])

## [0.1.44] - 2026-06-04

### Added

- rust executor image via gen-circleci-orb 0.0.43(pr [#176])

### Fixed

- deps: update rust crate pmcp to 2.9.0(pr [#177])

## [0.1.43] - 2026-06-03

### Added

- initialise orb wiring with gen-circleci-orb v0.0.38(pr [#175])

### Changed

- chore-strip orb source and CI wiring for v0.0.38 init(pr [#174])

## [0.1.42] - 2026-05-28

### Fixed

- add libssl-dev and pkg-config to Dockerfile(pr [#172])

## [0.1.41] - 2026-05-26

### Changed

- chore(ci)-bump gen-circleci-orb orb to 0.0.29(pr [#171])

## [0.1.40] - 2026-05-26

### Changed

- refactor(ci)-replace inline orb-release jobs with orb references(pr [#170])

## [0.1.39] - 2026-05-25

### Changed

- chore(ci)-bump gen-circleci-orb orb to 0.0.26(pr [#168])

## [0.1.38] - 2026-05-25

### Added

- add set_https_remote command and use in save job(pr [#156])
- default --orb-path to src/@orb.yml in generate and validate(pr [#167])

### Changed

- chore-bump gen-circleci-orb to 0.0.25, add git_push_subcommand(pr [#157])

### Fixed

- rename generate --version to --crate-version(pr [#165])
- deps: pin dependencies(pr [#158])
- deps: update rust crate config to 0.15.23(pr [#159])
- deps: update rust crate handlebars to 6.4.1(pr [#160])
- deps: update rust crate serde_json to 1.0.150(pr [#161])
- deps: update rust crate tokio to 1.52.3(pr [#162])
- deps: update rust crate pmcp to 2.8.1(pr [#164])

## [0.1.37] - 2026-05-22

### Fixed

- ci: set HTTPS push URL in build-mcp-server(pr [#155])

## [0.1.36] - 2026-05-22

### Fixed

- logging: add log→tracing bridge without SetLoggerError panic(pr [#154])

## [0.1.35] - 2026-05-21

### Changed

- chore(logging)-bridge log→tracing + CI push diagnostics(pr [#153])

## [0.1.34] - 2026-05-21

### Fixed

- save: reduce complexity and fix SSH push failure(pr [#152])

## [0.1.33] - 2026-05-21

### Fixed

- save: restore client.stage_paths + fix stale container binary(pr [#151])

### Security

- Dependencies: bump pcu to 0.6.21(pr [#150])

## [0.1.32] - 2026-05-21

### Fixed

- save: use index.add_all for directory staging(pr [#149])

## [0.1.31] - 2026-05-21

### Added

- replace hand-rolled ops with pcu library APIs(pr [#148])

## [0.1.30] - 2026-05-19

### Fixed

- use shell subprocess for ownertrust import(pr [#147])

## [0.1.29] - 2026-05-19

### Fixed

- expand \n sequences in BOT_TRUST before ownertrust import(pr [#146])

## [0.1.28] - 2026-05-19

### Fixed

- use --ignore-garbage and --allow-secret-key-import for GPG import(pr [#145])

## [0.1.27] - 2026-05-18

### Fixed

- use uploads.github.com base URL for release asset upload(pr [#144])

## [0.1.26] - 2026-05-18

### Fixed

- use crate tag prefix for prime in build-mcp-server(pr [#143])

## [0.1.25] - 2026-05-18

### Fixed

- add openssh-client to executor Dockerfile(pr [#142])

## [0.1.24] - 2026-05-18

### Added

- add --sign flag for GPG-signed commits via pcu library(pr [#139])
- add build-mcp-server to orb-release workflow(pr [#140])

### Fixed

- save: use PCU_ App credentials for bypass push with --sign(pr [#141])

## [0.1.23] - 2026-05-15

### Fixed

- use rust:slim base so build and format:binary work(pr [#138])

## [0.1.22] - 2026-05-14

### Changed

- ci-disable enable_pr_comment on orb-release pack and publish(pr [#135])

### Fixed

- orb: use gen-orb-mcp from PATH in scripts(pr [#136])

## [0.1.21] - 2026-05-14

### Fixed

- match orb-tools workspace contract in orb-release-pack(pr [#134])

## [0.1.20] - 2026-05-14

### Changed

- ci-tag-triggered orb release pipeline(pr [#133])

## [0.1.19] - 2026-05-14

### Fixed

- exit 0 after step halt to stop script execution(pr [#131])

## [0.1.18] - 2026-05-14

### Fixed

- halt container build and orb publish when no crate version(pr [#130])

## [0.1.17] - 2026-05-13

### Fixed

- accept exit 255 from circleci orb info as registered(pr [#129])

## [0.1.16] - 2026-05-13

### Fixed

- use set +e to tolerate circleci setup exit 255(pr [#128])

## [0.1.15] - 2026-05-13

### Fixed

- handle circleci setup exit 255 in ensure-orb-registered(pr [#127])

## [0.1.14] - 2026-05-13

### Fixed

- correct docker context and regenerate orb via init(pr [#125])
- correct docker orb version to registry-available 3.0.1(pr [#126])

## [0.1.13] - 2026-05-13

### Fixed

- make ensure-orb-registered idempotent on already-exists error(pr [#124])

## [0.1.12] - 2026-05-12

### Fixed

- rewrite orb Dockerfile to install pre-built binary(pr [#123])

## [0.1.11] - 2026-05-12

### Added

- wire orb generation via gen-circleci-orb init v0.0.10(pr [#113])
- add build, publish, and save subcommands (tier 2)(pr [#119])

### Changed

- test-add compilation test for generated MCP server(pr [#114])
- docs-add CircleCI orb section to README(pr [#115])
- docs-add next phase plan for tier 2 CLI completion(pr [#117])
- docs-update docs and orb for tier 2 (build/publish/save)(pr [#120])

### Fixed

- deps: update rust crate semver to 1.0.28(pr [#104])
- deps: update rust crate pmcp to 2.6.0(pr [#106])
- deps: update rust crate clap to 4.6.1(pr [#103])
- deps: update dependency toolkit to v6.2.0(pr [#105])
- deps: update rust crate tokio to 1.52.1(pr [#107])
- remove unused circleci/docker orb from release(pr [#121])
- break circular dependency in release workflow(pr [#122])

## [0.1.10] - 2026-04-07

### Added

- add --rename-map override to prime command(pr [#101])
- auto-infer orb version and add get_version tool(pr [#102])

### Changed

- docs-add orb author guide for job renames and migration quality(pr [#90])
- chore-migrate CI to circleci-toolkit 6.0.0(pr [#91])

### Fixed

- use git rename history for JobRenamed detection(pr [#88])
- remove jq from tools verification(pr [#96])
- strip trailing colon after last param removed (#93)(pr [#97])
- update requires entries when job is renamed (#94)(pr [#98])
- remove dangling requires entries after job removal (#92)(pr [#99])
- add ParameterRenamed conformance rule (#95)(pr [#100])

## [0.1.9] - 2026-03-26

### Changed

- test(migrator)-add planner tests for JobRenamed rule(pr [#86])

## [0.1.8] - 2026-03-26

### Added

- add UpdateOrbVersion to update orb pin during migration(pr [#83])

### Fixed

- remove orphaned pipeline param declarations(pr [#81])
- normalise file_path to filename for config.files lookup(pr [#82])
- use CiFile.source_path for UpdateOrbVersion full path(pr [#84])

## [0.1.7] - 2026-03-25

### Fixed

- remove_parameter drains sibling params(pr [#78])
- deps: update rust crate pmcp to 2.0.2(pr [#79])

## [0.1.6] - 2026-03-24

### Fixed

- eliminate inline literals to prevent LLVM OOM on release build(pr [#77])
- deps: update rust crate trycmd to 1.2.0(pr [#74])
- deps: update rust crate pmcp to v2(pr [#76])

## [0.1.5] - 2026-03-23

### Added

- generate per-version module files(pr [#72])

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
[#72]: https://github.com/jerus-org/gen-orb-mcp/pull/72
[#77]: https://github.com/jerus-org/gen-orb-mcp/pull/77
[#74]: https://github.com/jerus-org/gen-orb-mcp/pull/74
[#76]: https://github.com/jerus-org/gen-orb-mcp/pull/76
[#78]: https://github.com/jerus-org/gen-orb-mcp/pull/78
[#79]: https://github.com/jerus-org/gen-orb-mcp/pull/79
[#81]: https://github.com/jerus-org/gen-orb-mcp/pull/81
[#82]: https://github.com/jerus-org/gen-orb-mcp/pull/82
[#83]: https://github.com/jerus-org/gen-orb-mcp/pull/83
[#84]: https://github.com/jerus-org/gen-orb-mcp/pull/84
[#86]: https://github.com/jerus-org/gen-orb-mcp/pull/86
[#88]: https://github.com/jerus-org/gen-orb-mcp/pull/88
[#90]: https://github.com/jerus-org/gen-orb-mcp/pull/90
[#91]: https://github.com/jerus-org/gen-orb-mcp/pull/91
[#96]: https://github.com/jerus-org/gen-orb-mcp/pull/96
[#97]: https://github.com/jerus-org/gen-orb-mcp/pull/97
[#98]: https://github.com/jerus-org/gen-orb-mcp/pull/98
[#99]: https://github.com/jerus-org/gen-orb-mcp/pull/99
[#100]: https://github.com/jerus-org/gen-orb-mcp/pull/100
[#101]: https://github.com/jerus-org/gen-orb-mcp/pull/101
[#102]: https://github.com/jerus-org/gen-orb-mcp/pull/102
[#104]: https://github.com/jerus-org/gen-orb-mcp/pull/104
[#106]: https://github.com/jerus-org/gen-orb-mcp/pull/106
[#103]: https://github.com/jerus-org/gen-orb-mcp/pull/103
[#105]: https://github.com/jerus-org/gen-orb-mcp/pull/105
[#107]: https://github.com/jerus-org/gen-orb-mcp/pull/107
[#114]: https://github.com/jerus-org/gen-orb-mcp/pull/114
[#113]: https://github.com/jerus-org/gen-orb-mcp/pull/113
[#115]: https://github.com/jerus-org/gen-orb-mcp/pull/115
[#117]: https://github.com/jerus-org/gen-orb-mcp/pull/117
[#119]: https://github.com/jerus-org/gen-orb-mcp/pull/119
[#120]: https://github.com/jerus-org/gen-orb-mcp/pull/120
[#121]: https://github.com/jerus-org/gen-orb-mcp/pull/121
[#122]: https://github.com/jerus-org/gen-orb-mcp/pull/122
[#123]: https://github.com/jerus-org/gen-orb-mcp/pull/123
[#124]: https://github.com/jerus-org/gen-orb-mcp/pull/124
[#125]: https://github.com/jerus-org/gen-orb-mcp/pull/125
[#126]: https://github.com/jerus-org/gen-orb-mcp/pull/126
[#127]: https://github.com/jerus-org/gen-orb-mcp/pull/127
[#128]: https://github.com/jerus-org/gen-orb-mcp/pull/128
[#129]: https://github.com/jerus-org/gen-orb-mcp/pull/129
[#130]: https://github.com/jerus-org/gen-orb-mcp/pull/130
[#131]: https://github.com/jerus-org/gen-orb-mcp/pull/131
[#133]: https://github.com/jerus-org/gen-orb-mcp/pull/133
[#134]: https://github.com/jerus-org/gen-orb-mcp/pull/134
[#135]: https://github.com/jerus-org/gen-orb-mcp/pull/135
[#136]: https://github.com/jerus-org/gen-orb-mcp/pull/136
[#138]: https://github.com/jerus-org/gen-orb-mcp/pull/138
[#139]: https://github.com/jerus-org/gen-orb-mcp/pull/139
[#141]: https://github.com/jerus-org/gen-orb-mcp/pull/141
[#140]: https://github.com/jerus-org/gen-orb-mcp/pull/140
[#142]: https://github.com/jerus-org/gen-orb-mcp/pull/142
[#143]: https://github.com/jerus-org/gen-orb-mcp/pull/143
[#144]: https://github.com/jerus-org/gen-orb-mcp/pull/144
[#145]: https://github.com/jerus-org/gen-orb-mcp/pull/145
[#146]: https://github.com/jerus-org/gen-orb-mcp/pull/146
[#147]: https://github.com/jerus-org/gen-orb-mcp/pull/147
[#148]: https://github.com/jerus-org/gen-orb-mcp/pull/148
[#149]: https://github.com/jerus-org/gen-orb-mcp/pull/149
[#150]: https://github.com/jerus-org/gen-orb-mcp/pull/150
[#151]: https://github.com/jerus-org/gen-orb-mcp/pull/151
[#152]: https://github.com/jerus-org/gen-orb-mcp/pull/152
[#153]: https://github.com/jerus-org/gen-orb-mcp/pull/153
[#154]: https://github.com/jerus-org/gen-orb-mcp/pull/154
[#155]: https://github.com/jerus-org/gen-orb-mcp/pull/155
[#156]: https://github.com/jerus-org/gen-orb-mcp/pull/156
[#157]: https://github.com/jerus-org/gen-orb-mcp/pull/157
[#165]: https://github.com/jerus-org/gen-orb-mcp/pull/165
[#158]: https://github.com/jerus-org/gen-orb-mcp/pull/158
[#159]: https://github.com/jerus-org/gen-orb-mcp/pull/159
[#160]: https://github.com/jerus-org/gen-orb-mcp/pull/160
[#161]: https://github.com/jerus-org/gen-orb-mcp/pull/161
[#162]: https://github.com/jerus-org/gen-orb-mcp/pull/162
[#164]: https://github.com/jerus-org/gen-orb-mcp/pull/164
[#167]: https://github.com/jerus-org/gen-orb-mcp/pull/167
[#168]: https://github.com/jerus-org/gen-orb-mcp/pull/168
[#170]: https://github.com/jerus-org/gen-orb-mcp/pull/170
[#171]: https://github.com/jerus-org/gen-orb-mcp/pull/171
[#172]: https://github.com/jerus-org/gen-orb-mcp/pull/172
[#174]: https://github.com/jerus-org/gen-orb-mcp/pull/174
[#175]: https://github.com/jerus-org/gen-orb-mcp/pull/175
[#176]: https://github.com/jerus-org/gen-orb-mcp/pull/176
[#177]: https://github.com/jerus-org/gen-orb-mcp/pull/177
[#178]: https://github.com/jerus-org/gen-orb-mcp/pull/178
[#179]: https://github.com/jerus-org/gen-orb-mcp/pull/179
[#180]: https://github.com/jerus-org/gen-orb-mcp/pull/180
[#182]: https://github.com/jerus-org/gen-orb-mcp/pull/182
[#181]: https://github.com/jerus-org/gen-orb-mcp/pull/181
[#183]: https://github.com/jerus-org/gen-orb-mcp/pull/183
[#184]: https://github.com/jerus-org/gen-orb-mcp/pull/184
[#186]: https://github.com/jerus-org/gen-orb-mcp/pull/186
[#187]: https://github.com/jerus-org/gen-orb-mcp/pull/187
[#188]: https://github.com/jerus-org/gen-orb-mcp/pull/188
[#189]: https://github.com/jerus-org/gen-orb-mcp/pull/189
[#191]: https://github.com/jerus-org/gen-orb-mcp/pull/191
[#193]: https://github.com/jerus-org/gen-orb-mcp/pull/193
[#196]: https://github.com/jerus-org/gen-orb-mcp/pull/196
[#190]: https://github.com/jerus-org/gen-orb-mcp/pull/190
[#192]: https://github.com/jerus-org/gen-orb-mcp/pull/192
[#194]: https://github.com/jerus-org/gen-orb-mcp/pull/194
[#195]: https://github.com/jerus-org/gen-orb-mcp/pull/195
[#197]: https://github.com/jerus-org/gen-orb-mcp/pull/197
[#198]: https://github.com/jerus-org/gen-orb-mcp/pull/198
[#199]: https://github.com/jerus-org/gen-orb-mcp/pull/199
[#206]: https://github.com/jerus-org/gen-orb-mcp/pull/206
[#202]: https://github.com/jerus-org/gen-orb-mcp/pull/202
[#203]: https://github.com/jerus-org/gen-orb-mcp/pull/203
[#204]: https://github.com/jerus-org/gen-orb-mcp/pull/204
[#205]: https://github.com/jerus-org/gen-orb-mcp/pull/205
[#201]: https://github.com/jerus-org/gen-orb-mcp/pull/201
[#207]: https://github.com/jerus-org/gen-orb-mcp/pull/207
[#200]: https://github.com/jerus-org/gen-orb-mcp/pull/200
[#208]: https://github.com/jerus-org/gen-orb-mcp/pull/208
[#209]: https://github.com/jerus-org/gen-orb-mcp/pull/209
[#212]: https://github.com/jerus-org/gen-orb-mcp/pull/212
[#213]: https://github.com/jerus-org/gen-orb-mcp/pull/213
[#214]: https://github.com/jerus-org/gen-orb-mcp/pull/214
[#215]: https://github.com/jerus-org/gen-orb-mcp/pull/215
[#211]: https://github.com/jerus-org/gen-orb-mcp/pull/211
[#216]: https://github.com/jerus-org/gen-orb-mcp/pull/216
[#217]: https://github.com/jerus-org/gen-orb-mcp/pull/217
[#210]: https://github.com/jerus-org/gen-orb-mcp/pull/210
[#218]: https://github.com/jerus-org/gen-orb-mcp/pull/218
[#219]: https://github.com/jerus-org/gen-orb-mcp/pull/219
[#220]: https://github.com/jerus-org/gen-orb-mcp/pull/220
[#221]: https://github.com/jerus-org/gen-orb-mcp/pull/221
[#222]: https://github.com/jerus-org/gen-orb-mcp/pull/222
[#223]: https://github.com/jerus-org/gen-orb-mcp/pull/223
[#224]: https://github.com/jerus-org/gen-orb-mcp/pull/224
[#226]: https://github.com/jerus-org/gen-orb-mcp/pull/226
[#225]: https://github.com/jerus-org/gen-orb-mcp/pull/225
[#227]: https://github.com/jerus-org/gen-orb-mcp/pull/227
[#228]: https://github.com/jerus-org/gen-orb-mcp/pull/228
[#229]: https://github.com/jerus-org/gen-orb-mcp/pull/229
[#230]: https://github.com/jerus-org/gen-orb-mcp/pull/230
[#232]: https://github.com/jerus-org/gen-orb-mcp/pull/232
[#231]: https://github.com/jerus-org/gen-orb-mcp/pull/231
[#233]: https://github.com/jerus-org/gen-orb-mcp/pull/233
[#234]: https://github.com/jerus-org/gen-orb-mcp/pull/234
[#236]: https://github.com/jerus-org/gen-orb-mcp/pull/236
[#235]: https://github.com/jerus-org/gen-orb-mcp/pull/235
[#237]: https://github.com/jerus-org/gen-orb-mcp/pull/237
[#239]: https://github.com/jerus-org/gen-orb-mcp/pull/239
[#238]: https://github.com/jerus-org/gen-orb-mcp/pull/238
[#243]: https://github.com/jerus-org/gen-orb-mcp/pull/243
[#242]: https://github.com/jerus-org/gen-orb-mcp/pull/242
[#240]: https://github.com/jerus-org/gen-orb-mcp/pull/240
[#241]: https://github.com/jerus-org/gen-orb-mcp/pull/241
[#245]: https://github.com/jerus-org/gen-orb-mcp/pull/245
[#246]: https://github.com/jerus-org/gen-orb-mcp/pull/246
[#247]: https://github.com/jerus-org/gen-orb-mcp/pull/247
[#248]: https://github.com/jerus-org/gen-orb-mcp/pull/248
[#244]: https://github.com/jerus-org/gen-orb-mcp/pull/244
[#249]: https://github.com/jerus-org/gen-orb-mcp/pull/249
[Unreleased]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.59...HEAD
[0.1.59]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.58...v0.1.59
[0.1.58]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.57...v0.1.58
[0.1.57]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.56...v0.1.57
[0.1.56]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.55...v0.1.56
[0.1.55]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.54...v0.1.55
[0.1.54]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.53...v0.1.54
[0.1.53]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.52...v0.1.53
[0.1.52]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.51...v0.1.52
[0.1.51]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.50...v0.1.51
[0.1.50]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.49...v0.1.50
[0.1.49]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.48...v0.1.49
[0.1.48]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.47...v0.1.48
[0.1.47]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.46...v0.1.47
[0.1.46]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.45...v0.1.46
[0.1.45]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.44...v0.1.45
[0.1.44]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.43...v0.1.44
[0.1.43]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.42...v0.1.43
[0.1.42]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.41...v0.1.42
[0.1.41]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.40...v0.1.41
[0.1.40]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.39...v0.1.40
[0.1.39]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.38...v0.1.39
[0.1.38]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.37...v0.1.38
[0.1.37]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.36...v0.1.37
[0.1.36]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.35...v0.1.36
[0.1.35]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.34...v0.1.35
[0.1.34]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.33...v0.1.34
[0.1.33]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.32...v0.1.33
[0.1.32]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.31...v0.1.32
[0.1.31]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.30...v0.1.31
[0.1.30]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.29...v0.1.30
[0.1.29]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.28...v0.1.29
[0.1.28]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.27...v0.1.28
[0.1.27]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.26...v0.1.27
[0.1.26]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.25...v0.1.26
[0.1.25]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.24...v0.1.25
[0.1.24]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.23...v0.1.24
[0.1.23]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.22...v0.1.23
[0.1.22]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.21...v0.1.22
[0.1.21]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.20...v0.1.21
[0.1.20]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.19...v0.1.20
[0.1.19]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.18...v0.1.19
[0.1.18]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.17...v0.1.18
[0.1.17]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.16...v0.1.17
[0.1.16]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.15...v0.1.16
[0.1.15]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.14...v0.1.15
[0.1.14]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.13...v0.1.14
[0.1.13]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.12...v0.1.13
[0.1.12]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.11...v0.1.12
[0.1.11]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.10...v0.1.11
[0.1.10]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.9...v0.1.10
[0.1.9]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.8...v0.1.9
[0.1.8]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.7...v0.1.8
[0.1.7]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.6...v0.1.7
[0.1.6]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.5...v0.1.6
[0.1.5]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/jerus-org/gen-orb-mcp/compare/v0.1.0-alpha.1...v0.1.0
[0.1.0-alpha.1]: https://github.com/jerus-org/gen-orb-mcp/releases/tag/v0.1.0-alpha.1
