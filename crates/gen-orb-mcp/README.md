# gen-orb-mcp

Generate MCP (Model Context Protocol) servers from CircleCI orb definitions.

[![Crates.io](https://img.shields.io/crates/v/gen-orb-mcp.svg)](https://crates.io/crates/gen-orb-mcp)
[![Documentation](https://docs.rs/gen-orb-mcp/badge.svg)](https://docs.rs/gen-orb-mcp)
[![License](https://img.shields.io/crates/l/gen-orb-mcp.svg)](https://github.com/jerus-org/gen-orb-mcp#license)

## Overview

**gen-orb-mcp** enables AI coding assistants to understand and work with private CircleCI orbs by generating MCP servers that expose orb commands, jobs, and executors as resources.

## Installation

```bash
cargo install gen-orb-mcp
```

Or add to your `Cargo.toml`:

```toml
gen-orb-mcp = "0.1.0"
```

## Usage

### Generate an MCP server from an orb

```bash
gen-orb-mcp generate --orb-path ./src/@orb.yml --output ./dist/
```

### Validate an orb definition

```bash
gen-orb-mcp validate --orb-path ./src/@orb.yml
```

### Options

```
gen-orb-mcp generate [OPTIONS] --orb-path <ORB_PATH>

Options:
  -o, --orb-path <ORB_PATH>  Path to the orb YAML file (e.g., src/@orb.yml)
  -o, --output <OUTPUT>      Output directory for generated server [default: ./dist]
  -f, --format <FORMAT>      Output format [default: binary] [possible values: binary, source]
  -h, --help                 Print help
```

## Features

- **Parse any CircleCI orb** - Works with standard orb YAML structure
- **Generate MCP servers** - Produces standalone MCP servers exposing orb resources
- **Binary deployment** - Compile to native Linux x86_64 binaries
- **Private registry support** - Deploy to Docker Hub or private registries
- **Offline operation** - Generated servers run entirely offline

## Roadmap

### Phase 1: MVP (Current)
- [x] Parse CircleCI orb YAML definitions
- [x] Generate single-version MCP servers
- [x] Binary output for Linux x86_64
- [ ] Private registry deployment support

### Phase 2: Enhanced
- [ ] Multi-version support (last 5 versions)
- [ ] Delta encoding for version differences
- [ ] Migration tooling between orb versions
- [ ] Container deployment format
- [ ] Skill file generation

### Phase 3: Smart Router
- [ ] Context-aware version routing
- [ ] Automatic version selection based on project context

## Contributing

See [Contributing Guide](https://github.com/jerus-org/gen-orb-mcp/blob/main/CONTRIBUTING.md) for guidelines.

## Code of Conduct

See [Code of Conduct](https://github.com/jerus-org/gen-orb-mcp/blob/main/CODE_OF_CONDUCT.md).

## Security

See [Security Policy](https://github.com/jerus-org/gen-orb-mcp/blob/main/SECURITY.md) for reporting vulnerabilities.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
