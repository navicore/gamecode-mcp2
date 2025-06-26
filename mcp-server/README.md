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

For scenarios where you need to provide values that the LLM should not control (e.g., multi-tenant environments), use the `--inject` flag:

```bash
gamecode-mcp2 --inject tenant=customer123 --inject environment=production
```

Injected values are:
- Set as environment variables with `GAMECODE_` prefix (e.g., `GAMECODE_TENANT`)
- Available to all executed tools but not visible to the LLM
- Useful for enforcing security boundaries in multi-tenant deployments

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
