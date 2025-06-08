#!/bin/bash
# Setup script for GameCode MCP2 examples directory structure

echo "Setting up GameCode MCP2 examples directory structure..."

# Create main category directories
mkdir -p core
mkdir -p development/{rust,python,javascript,go,java}
mkdir -p diagrams/{plantuml,mermaid,graphviz}
mkdir -p documentation/{markdown,asciidoc,api}
mkdir -p data/{json,csv,sqlite,analysis}
mkdir -p security/{scanning,audit}
mkdir -p integration/{git,docker,cloud,databases}
mkdir -p specialized/{technical-writer,data-scientist,devops-engineer,security-auditor}

# Create README files for each category
cat > core/README.md << 'EOF'
# Core Tools

Basic file operations and utilities that complement or replace MCP host tools.

Use these when:
- Your MCP host lacks specific file operations
- You need more control over file access
- You want consistent behavior across different hosts
EOF

cat > development/README.md << 'EOF'
# Development Tools

Language-specific development tools.

Most MCP coding hosts already provide these. Use ours when:
- You need specialized build tools
- Your host lacks certain language support  
- You want consistent tooling across hosts
EOF

cat > security/README.md << 'EOF'
# Security Tools

Security-focused configurations for different threat models.

Includes:
- Paranoid mode (maximum restrictions)
- Audit mode (log everything)
- Scanning tools (security analysis)
EOF

# Move existing examples to appropriate locations
if [ -f "tools.yaml" ]; then
    mv tools.yaml core/basic.yaml
fi

if [ -f "paranoid.yaml" ]; then
    mv paranoid.yaml security/paranoid.yaml
fi

echo "Directory structure created!"
echo ""
echo "Next steps:"
echo "1. Move existing YAML files to appropriate directories"
echo "2. Create README.md in each subdirectory"
echo "3. Add examples for each category"
echo ""
echo "Example structure created:"
find . -type d -name ".*" -prune -o -type d -print | sort