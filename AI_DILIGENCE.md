# AI Diligence Statement

## Overview

This document describes how artificial intelligence was used in the development of **gen-orb-mcp**, in accordance with transparency best practices for AI-assisted software development.

## AI Involvement

### Development Partnership

gen-orb-mcp was developed as a solo project by Jeremiah Russell with substantial assistance from **Claude Code** (Anthropic's AI coding assistant). This collaborative approach is documented in the project's [Finalized Plan](docs/FINALIZED_PLAN.md):

> **Developer:** Solo (Jeremiah Russell) with Claude Code support

### Scope of AI Contribution

Claude Code assisted with:

- **Code Generation**: Writing Rust code for the parser, code generator, and CLI components
- **Architecture Design**: Discussing and refining the MCP server generation pipeline
- **Template Development**: Creating Handlebars templates for generated MCP servers
- **CI/CD Configuration**: Developing and debugging CircleCI release workflows
- **Documentation**: Drafting and refining technical documentation
- **Problem Solving**: Diagnosing issues and proposing solutions during development

### Human Oversight

All AI-generated contributions were:

- **Reviewed** by the human developer before acceptance
- **Tested** through automated test suites and manual verification
- **Iterated** based on human judgment and project requirements
- **Committed** with explicit co-authorship attribution

## Attribution

Commits that include AI assistance are attributed using the standard Git co-authorship format:

```
Co-Authored-By: Claude <noreply@anthropic.com>
```

This attribution appears in commit messages throughout the repository's history.

## Quality Assurance

The following measures ensure code quality regardless of origin:

- **Automated Testing**: Unit tests, integration tests, and CLI tests via `trycmd`
- **Static Analysis**: `cargo clippy` with `-D warnings` enforced
- **Code Quality**: SonarQube analysis for code smells, bugs, and security hotspots
- **Formatting**: `rustfmt` enforced via CI
- **Security Auditing**: `cargo-audit` for dependency vulnerability scanning
- **Code Review**: All changes reviewed before merge via pull request workflow

## Licensing

AI-assisted contributions are provided under the same dual license as the project:

- Apache License, Version 2.0
- MIT License

AI assistance does not affect the licensing terms or user rights.

## Transparency Commitment

We believe in transparent disclosure of AI involvement in software development. This statement will be updated as AI tooling and best practices evolve.

## Questions

For questions about AI involvement in this project, please open an issue or contact the maintainer.
