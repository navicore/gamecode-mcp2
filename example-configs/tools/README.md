# Modular Tools Configuration

This directory demonstrates the modular tools configuration system for gamecode-mcp2.

## Directory Structure

```
tools/
├── core.yaml           # Core tools (file operations, search)
├── git.yaml            # Git version control tools
├── languages/          # Language-specific tools
│   ├── rust.yaml      # Rust development tools
│   ├── python.yaml    # Python development tools
│   └── ...
└── profiles/          # Complete tool profiles
    ├── rust-dev.yaml  # Full Rust development environment
    ├── python-dev.yaml # Full Python development environment
    └── ...
```

## Usage

### 1. Auto-detection (Default)

The server automatically detects your project type and loads appropriate tools:

```bash
# In a Rust project (has Cargo.toml)
gamecode-mcp2  # Automatically loads rust.yaml + core.yaml + git.yaml
```

### 2. Mode Override

Force a specific mode using the environment variable:

```bash
GAMECODE_MODE=python gamecode-mcp2
```

### 3. Custom Configuration

Create a tools.yaml in your project root:

```yaml
# tools.yaml
include:
  - tools/core.yaml
  - tools/git.yaml
  - tools/languages/rust.yaml
  - my-custom-tools.yaml

tools:
  - name: my_custom_tool
    description: Project-specific tool
    command: ./scripts/my-tool.sh
    args:
      - name: input
        description: Input file
        required: true
        type: string
```

## Include System

The `include` directive allows you to compose tool sets:

- Paths are relative to the including file
- Supports `~` for home directory
- Recursive includes are supported
- Tools are loaded in order (later definitions override earlier ones)

## Environment Variables

- `GAMECODE_MODE`: Force a specific mode/profile
- `GAMECODE_TOOLS_FILE`: Override the default tools.yaml location

## Creating Custom Profiles

1. Create a new file in `tools/profiles/`
2. Include the base tools you need
3. Add profile-specific tools

Example:

```yaml
# tools/profiles/web-dev.yaml
include:
  - ../core.yaml
  - ../git.yaml
  - ../languages/javascript.yaml
  - ../languages/typescript.yaml
  - ../infra/docker.yaml

tools:
  - name: npm_dev
    description: Start development server
    command: npm
    static_flags: ["run", "dev"]
```