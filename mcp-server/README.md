# gamecode-mcp2

A minimal, auditable Model Context Protocol (MCP) server for LLM-to-system interaction.

## Overview

`gamecode-mcp2` implements the Model Context Protocol specification, enabling Large Language Models to execute tools and interact with systems in a controlled, secure manner. It prioritizes security and auditability through explicit configuration and minimal dependencies.

## Key Features

- **Security-first design**: No dynamic code execution, only explicitly configured tools
- **Minimal dependencies**: Pure JSON-RPC 2.0 over stdio
- **Explicit configuration**: All tools defined in YAML with clear permissions
- **Auditable**: Simple codebase designed for security review
- **Least-privilege alternatives**: Replace risky MCP tools with safer, more restricted versions

## Installation

```bash
cargo install gamecode-mcp2
```

## Quick Start

1. Create a `tools.yaml` file:

```yaml
tools:
  - name: read_file
    description: Read the contents of a file
    command: cat
    args:
      - name: path
        description: Path to the file to read
        required: true
        type: string
        cli_flag: null  # Positional argument
```

2. Run the server:

```bash
gamecode-mcp2
```

3. Configure your MCP client (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "gamecode": {
      "command": "/path/to/gamecode-mcp2"
    }
  }
}
```

## Tool Configuration

Tools are defined in YAML with the following structure:

```yaml
tools:
  - name: tool_name
    description: What this tool does
    command: command_to_execute  # or "internal" for built-in handlers
    args:
      - name: argument_name
        description: What this argument is for
        required: true
        type: string  # string, number, boolean, or array
        cli_flag: --flag  # optional, null for positional args
        is_path: true  # optional, enables path validation
    validation:  # optional
      validate_paths: true
      allow_absolute_paths: false
```

### Built-in Handlers

The server includes safe implementations of common operations:
- `add`, `multiply`: Basic arithmetic
- `list_files`: List directory contents
- `write_file`: Write content to files (with validation)

## Tool Loading Order

The server looks for tools in this order:
1. Command-line flag: `--tools-file`
2. Environment variable: `GAMECODE_TOOLS_FILE`
3. Local `tools.yaml` in current directory
4. Auto-detection based on project type
5. Config directory: `~/.config/gamecode-mcp/tools.yaml`

## Server-Side Value Injection

The `--inject` flag allows you to pass server-side values that are invisible to the LLM but available to your tools. This is essential for multi-tenant scenarios where the LLM must not control security-critical parameters.

### How it works

```bash
gamecode-mcp2 --inject tenant=customer123 --inject environment=production
```

When tools execute, they receive these as environment variables:
- `tenant=customer123` → `GAMECODE_TENANT=customer123`
- `environment=production` → `GAMECODE_ENVIRONMENT=production`

### Security model

```
┌─────────────┐     ┌──────────────┐     ┌─────────────────┐     ┌──────┐
│ Orchestrator│ --> │ gamecode-mcp2│ --> │ Tool Execution  │ --> │ Tool │
│   (knows    │     │  (--inject)  │     │ (env vars set) │     │      │
│   tenant)   │     │              │     │                 │     │      │
└─────────────┘     └──────────────┘     └─────────────────┘     └──────┘
                           ↑
                           │ MCP Protocol (no tenant info)
                           │
                    ┌──────────────┐
                    │     LLM      │
                    │ (cannot see  │
                    │  or modify   │
                    │   tenant)    │
                    └──────────────┘
```

### Example: Multi-tenant SaaS

```bash
# Your orchestrator spawns a new MCP server per request
gamecode-mcp2 --inject tenant=$CUSTOMER_ID --inject env=$ENVIRONMENT

# Your tool script accesses the values
#!/bin/bash
# query-data.sh
psql -h $GAMECODE_ENV.db.example.com \
     -d tenant_$GAMECODE_TENANT \
     -c "$1"
```

**Important**: This provides a separation of concerns but is not a complete security solution. Always validate tool inputs and follow defense-in-depth principles.

## Examples

See the `examples/` directory for tool configurations for various use cases:
- `core/`: Basic file and system operations
- `development/`: Language-specific development tools
- `security/`: Security-focused configurations
- `data/`: Data processing tools
- `multi-tenant-example.yaml`: Using injected values for tenant isolation

## Security Considerations

- Create least-privilege versions of risky tools by disabling other MCP servers and defining safer alternatives here
- Commands are executed directly without shell interpretation
- Optional path validation prevents directory traversal
- All operations are logged to stderr for auditing
- Single-threaded processing ensures predictable behavior
- No dynamic code evaluation - all tools must be explicitly configured

## License

MIT License - see LICENSE file for details
