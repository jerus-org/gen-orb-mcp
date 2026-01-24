# gen-orb-mcp

Generate MCP (Model Context Protocol) servers from CircleCI orb definitions.

[![Crates.io](https://img.shields.io/crates/v/gen-orb-mcp.svg)](https://crates.io/crates/gen-orb-mcp)
[![Documentation](https://docs.rs/gen-orb-mcp/badge.svg)](https://docs.rs/gen-orb-mcp)
[![License](https://img.shields.io/crates/l/gen-orb-mcp.svg)](https://github.com/jerus-org/gen-orb-mcp#license)

## Status

This crate is under active development. The crate name has been reserved on crates.io.

## Overview

**gen-orb-mcp** will enable AI coding assistants to understand and work with private CircleCI orbs by generating MCP servers that expose orb commands, jobs, and executors as resources.

### Planned Features

- **Parse any CircleCI orb** - Works with standard orb YAML structure
- **Generate MCP servers** - Produces standalone MCP servers exposing orb resources
- **Binary deployment** - Compile to native Linux x86_64 binaries
- **Private registry support** - Deploy to Docker Hub or private registries
- **Offline operation** - Generated servers run entirely offline

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
