# Patterns & Potential Core Improvements

This document tracks patterns that emerge from our examples and potential improvements to the core MCP server.

## Discovered Patterns

### 1. Directory Enforcement
Many tools want to enforce output to specific directories:
- Diagrams → `./diagrams/`
- Reports → `./reports/`
- Temp files → `./tmp/`

**Current approach**: Each tool validates manually
**Potential improvement**: Add `output_directory` constraint to tool definition

### 2. File Extension Validation
Tools often care about file extensions:
- PlantUML only wants `.puml`
- Mermaid only wants `.mmd`

**Current approach**: Manual validation in handlers
**Potential improvement**: Add `allowed_extensions` to path arguments

### 3. Command Timeouts
Resource-intensive tools need timeouts:
- PlantUML rendering
- Data processing
- External API calls

**Current approach**: Wrap with `/usr/bin/timeout`
**Potential improvement**: Add `timeout_seconds` to tool definition

### 4. Template/Boilerplate Support
Many tools benefit from templates:
- Diagram templates
- Code scaffolding
- Report structures

**Current approach**: Hardcode in YAML or external files
**Potential improvement**: Add `templates` section to tools.yaml

### 5. Pipeline Patterns
Common pattern: generate → validate → process → output

**Current approach**: User coordinates multiple tools
**Potential improvement**: Add simple pipeline support?

## Proposed Core Enhancements

### Enhancement 1: Directory Constraints
```yaml
tools:
  - name: save_diagram
    command: internal
    internal_handler: write_file
    constraints:
      output_directory: "./diagrams"
      create_directory: true
    args:
      - name: path
        description: Filename only (directory enforced)
        type: string
        is_path: true
```

### Enhancement 2: File Type Validation
```yaml
args:
  - name: diagram_file
    type: string
    is_path: true
    allowed_extensions: [".puml", ".plantuml"]
    allowed_patterns: ["*.puml"]  # glob patterns
```

### Enhancement 3: Built-in Timeouts
```yaml
tools:
  - name: render_diagram
    command: plantuml
    timeout_seconds: 30  # Built-in timeout
    resource_limits:
      max_memory_mb: 512
      max_cpu_percent: 80
```

### Enhancement 4: Template Support
```yaml
tools:
  - name: create_diagram
    templates:
      class: |
        @startuml
        class ${CLASS_NAME} {
          ${CONTENT}
        }
        @enduml
      sequence: |
        @startuml
        ${ACTOR} -> ${SYSTEM}: ${MESSAGE}
        @enduml
```

## Implementation Philosophy

Before adding any enhancement:

1. **Can it be done in YAML?** - Prefer configuration over code
2. **Does it increase attack surface?** - Security first
3. **Is it truly common?** - Don't add features for one use case
4. **Does it maintain auditability?** - Keep it simple and clear

## Non-Goals

Things we intentionally DON'T want to add:

1. **Dynamic tool generation** - Tools must be statically defined
2. **Scripting language** - YAML configuration only
3. **Complex pipelines** - Let the LLM coordinate
4. **Network operations** - Stdio only
5. **Plugin system** - Everything in core must be auditable

## Current Workarounds

These patterns work today without core changes:

### Directory Enforcement
```yaml
# Use shell to ensure directory
command: /bin/sh
static_flags: ["-c"]
args:
  - name: script
    cli_flag: null
    # Script: mkdir -p diagrams && echo "$content" > "diagrams/$filename"
```

### Timeouts
```yaml
command: /usr/bin/timeout
static_flags: ["30", "plantuml"]
```

### Templates
```yaml
# Use echo or cat with static files
command: /bin/cat
static_flags: ["templates/class-diagram.puml"]
```

## Conclusion

The cookbook examples help us discover patterns, but we should resist adding features unless they:
1. Significantly improve security
2. Reduce configuration complexity
3. Are needed by multiple use cases

The current implementation's simplicity is a feature, not a bug.