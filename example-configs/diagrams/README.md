# Diagram Tools for MCP

Diagramming is a capability rarely provided by MCP hosts, making this a high-value addition.

## Why These Tools?

Most MCP hosts (Claude Code, Claude Desktop, etc.) focus on code and text manipulation. They typically don't include:
- PlantUML rendering
- Mermaid diagram generation  
- Graphviz/DOT processing
- Architecture diagram creation

**This is where GameCode MCP2 shines** - augmenting your MCP host with specialized capabilities.

## Available Configurations

### plantuml/
- `source-only.yaml` - Just save .puml files (safest)
- `with-rendering.yaml` - Auto-render to PNG/SVG
- `advanced.yaml` - Includes templates and batch processing

### mermaid/
- `source-only.yaml` - Save .mmd files
- `live-preview.yaml` - With local server preview
- `export.yaml` - Convert to various formats

### graphviz/
- `basic.yaml` - DOT file creation
- `auto-layout.yaml` - Automatic graph layouts

## Usage Scenarios

### Scenario 1: Architecture Documentation
Using Claude Code? It can edit code but can't create visual documentation:
```bash
# Augment Claude Code with diagramming
GAMECODE_TOOLS_FILE=examples/diagrams/plantuml/with-rendering.yaml
```

Now: *"Create a component diagram for this microservice architecture"*

### Scenario 2: Database Modeling
Transform any MCP host into a database designer:
```yaml
include:
  - examples/diagrams/plantuml/advanced.yaml
  - examples/data/sqlite/schema-tools.yaml
```

Now: *"Analyze the database schema and create an ER diagram"*

### Scenario 3: Documentation Pipeline
Combine with other tools for complete documentation:
```yaml
include:
  - examples/diagrams/mermaid/export.yaml
  - examples/documentation/markdown/processor.yaml
  - examples/integration/git/docs-commit.yaml
```

## Security Notes

Diagram rendering can be resource-intensive:
- PlantUML can consume significant CPU/memory on complex diagrams
- Always use timeouts for rendering commands
- Consider source-only configs for untrusted environments

## Implementation Guidance

These examples demonstrate patterns that could inform core improvements:

1. **Directory Isolation**: Diagrams always output to `./diagrams/`
2. **Format Validation**: Only accept known diagram formats
3. **Resource Limits**: Timeout and size constraints
4. **Graceful Degradation**: Fall back to source-only if rendering fails

## Quick Start

For most users, start with source-only:
```bash
# Safest approach - just save diagram source
GAMECODE_TOOLS_FILE=examples/diagrams/plantuml/source-only.yaml gamecode-mcp2
```

Then manually render with:
```bash
plantuml diagrams/*.puml
```

As you get comfortable, upgrade to auto-rendering configurations.