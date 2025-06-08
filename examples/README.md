# GameCode MCP2 Examples & Cookbook

This directory contains ready-to-use tool configurations organized by use case.

## Important: MCP Host Considerations

**Your MCP host (Claude Desktop, Claude Code, etc.) likely already provides many tools!**

- **Claude Code**: Already has excellent file editing, code search, git, and language tools
- **Claude Desktop**: May have its own file and system tools
- **Other MCP hosts**: Check what's already available before adding redundant tools

These examples are for:
1. **Complementing** your MCP host with missing capabilities
2. **Replacing** host tools when you need more control
3. **Transforming** your MCP host into different types of agents

## The Power of Configuration

With GameCode MCP2's approach, you can transform any MCP host:

```yaml
# Transform Claude Code into a documentation specialist
include:
  - examples/diagrams/plantuml/advanced.yaml
  - examples/documentation/technical-writing.yaml
  
# Transform Claude Desktop into a data analyst
include:
  - examples/data/sqlite/analysis.yaml
  - examples/data/visualization.yaml

# Transform any host into a security auditor
include:
  - examples/security/audit-tools.yaml
  - examples/security/scanning.yaml
```

## Structure

```
examples/
├── README.md                 # This file
├── core/                     # Minimal tools when host tools are insufficient
│   ├── minimal.yaml         # Bare minimum file operations
│   ├── readonly.yaml        # Safe read-only tools
│   └── README.md
├── development/             # Language-specific (when host tools need augmentation)
│   ├── rust/
│   ├── python/
│   ├── javascript/
│   └── README.md
├── diagrams/                # Diagramming (rarely provided by hosts)
│   ├── plantuml/
│   ├── mermaid/
│   ├── graphviz/
│   └── README.md
├── documentation/           # Documentation generation
│   ├── markdown/
│   ├── asciidoc/
│   └── README.md
├── data/                    # Data processing (unique capabilities)
│   ├── json/
│   ├── csv/
│   ├── sqlite/
│   └── README.md
├── security/                # Security-focused configs
│   ├── paranoid.yaml       # Maximum restrictions
│   ├── audit.yaml          # Logging everything
│   └── README.md
├── integration/            # External services your host doesn't support
│   ├── cloud/             # AWS, GCP, Azure CLI tools
│   ├── databases/         # PostgreSQL, MongoDB, etc.
│   └── README.md
└── specialized/            # Domain-specific transformations
    ├── devops/            # Transform into DevOps engineer
    ├── data-science/      # Transform into data scientist
    ├── technical-writer/  # Transform into documentation expert
    └── README.md
```

## Usage Patterns

### 1. Augment Existing Host
When Claude Code is missing a specific tool:
```yaml
# Just add what's missing
include:
  - examples/diagrams/plantuml/basic.yaml
```

### 2. Replace Host Tools
When you need more control than host provides:
```yaml
# Use our file tools instead of host's
include:
  - examples/security/paranoid.yaml  # Replaces all file operations
```

### 3. Transform Host Completely
Turn any MCP host into a specialized agent:
```yaml
# Transform into a database administrator
include:
  - examples/specialized/database-admin/full-stack.yaml
```

## Philosophy

1. **Non-redundant**: Don't duplicate what your MCP host already does well
2. **Composable**: Mix and match to create your perfect toolset
3. **Transformative**: Turn any MCP host into any type of agent
4. **Secure**: Examples demonstrate security best practices
5. **Host-agnostic**: Works with any MCP-compliant host

## Per-Category Guidelines

### core/
Only use when your MCP host's file operations are insufficient or you need tighter control.

### development/
Most MCP coding hosts already have these. Use ours for:
- Specialized build tools
- Custom linting/formatting
- Language-specific tools the host lacks

### diagrams/
Few MCP hosts provide diagramming - this is a key differentiator.

### data/
Transform any MCP host into a data analysis powerhouse.

### security/
For when you need auditable, restricted operations beyond what hosts provide.

### specialized/
Complete transformations - the real power of the GameCode MCP2 approach.

## Contributing Examples

When adding examples:
1. **Check redundancy**: Don't duplicate common MCP host features
2. **Document use case**: When would someone need this over host tools?
3. **Show transformation**: How does this change what the LLM can do?
4. **Security first**: Always use validation where appropriate
5. **Test with hosts**: Verify it works with Claude Desktop/Code/etc.