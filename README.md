[![Dependabot Updates](https://github.com/navicore/gamecode-mcp2/actions/workflows/dependabot/dependabot-updates/badge.svg)](https://github.com/navicore/gamecode-mcp2/actions/workflows/dependabot/dependabot-updates)
[![Rust CI](https://github.com/navicore/gamecode-mcp2/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/navicore/gamecode-mcp2/actions/workflows/rust-ci.yml)

# GameCode MCP2

A clean (and possibly naive) implementation of the Model Context Protocol (MCP)
for tool integration.

Motivation - as few dependencies as possible, as simple and auditable a
configuration as possible.

## ⚠️ Security Notice

MCP is early technology. Allowing LLMs to execute system commands is inherently risky.
This implementation prioritizes auditability over features - you can read every line
that processes LLM requests. Even so, proceed with caution. Only time will tell if
MCP's approach is sound.


## Key Features

- **Direct tool exposure** - Tools defined in `tools.yaml` are exposed directly via MCP, not through meta-tools
- **Clean protocol implementation** - Pure JSON-RPC 2.0 over stdio without external dependencies
- **Dynamic tool loading** - Configure tools via YAML without recompiling
- **Built-in and external tools** - Support for both internal handlers and external commands

## Architecture

This workspace contains two crates:

- `mcp-server` - The MCP server that loads tools from `tools.yaml` and exposes them via the protocol
- `mcp-client` - A client library for testing and integration

## Usage

### Running the Server

```bash
# Build the server
cargo build --release --bin gamecode-mcp2

# Create a tools.yaml file (see examples/tools.yaml)
cp examples/tools.yaml .

# Run the server (it communicates via stdio)
./target/release/gamecode-mcp2
```

### Using with Claude Desktop

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "gamecode": {
      "command": "/path/to/gamecode-mcp2"
    }
  }
}
```

### Using with gamecode-cli

The `mcp-client` crate can be used as a dependency in gamecode-cli for MCP integration.

## Tool Configuration

Tools are defined in `tools.yaml`:

```yaml
tools:
  - name: my_tool
    description: Description for the LLM
    command: /path/to/command  # or "internal" for built-in
    args:
      - name: param1
        description: Parameter description
        required: true
        type: string
        cli_flag: --param  # null for positional
    internal_handler: handler_name  # for internal tools
```

## Protocol

This implementation follows the MCP specification:

- `initialize` - Handshake with client
- `tools/list` - Returns all available tools
- `tools/call` - Execute a specific tool

Tools are exposed directly, not through meta-tools like "run".
