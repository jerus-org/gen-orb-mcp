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

## How Generated MCP Servers Work

### Resources vs Tools

The Model Context Protocol (MCP) supports two types of capabilities:

- **Tools**: Callable functions that perform actions (e.g., run a build, fetch data)
- **Resources**: Read-only documentation and data that AI assistants can access

**gen-orb-mcp generates MCP servers that expose orb definitions as resources**, not tools. This is intentional - the purpose is to provide AI assistants with documentation about your private orbs so they can help you write and understand CircleCI configurations.

### What Users See (and Don't See)

When you configure a generated MCP server in your AI coding assistant (like Claude Code):

| What you'll see | What you won't see |
|----------------|-------------------|
| Connect/disconnect options in `/mcp` | A list of browsable resources |
| Server status (connected/disconnected) | Resource exploration UI |
| Server name in the MCP list | Individual resource access options |

**This is expected behavior.** The AI assistant accesses resources implicitly through the MCP protocol - there's no user-facing interface to browse resources because the AI handles this automatically.

### How the AI Uses Resources

When the MCP server is connected, your AI assistant can:

1. **Read the orb overview** - Full documentation of all commands, jobs, and executors
2. **Access individual components** - Detailed parameter information for any command/job/executor
3. **Reference this knowledge** - When helping you write CircleCI configurations

The AI accesses these resources behind the scenes without requiring user interaction.

### Verifying Your MCP Server is Working

To confirm your generated MCP server is providing value:

1. **Ask about your orb**: "What commands are available in my zola-orb?"
2. **Request parameter details**: "What parameters does the `build` job accept?"
3. **Get configuration help**: "Help me configure the `deploy` job for my project"

If the AI can answer these questions with accurate, detailed information about your private orb, the MCP server is working correctly.

### Example Session

```
User: What jobs are available in my zola-orb?

AI: Your zola-orb provides the following jobs:

1. **build** - Build the Zola static site
   - Parameters: zola_version (optional), base_url (optional)

2. **deploy** - Deploy built site to hosting provider
   - Parameters: target (required), deploy_token (env_var_name)

3. **check** - Validate site configuration and content
   - Parameters: drafts (boolean, optional)
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
