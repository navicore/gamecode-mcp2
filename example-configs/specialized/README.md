# Specialized Agent Configurations

Transform any MCP host into a specialized agent by loading complete toolsets.

## The Transformation Concept

Your MCP host (Claude Code, Claude Desktop, etc.) has general capabilities. These configurations transform it into specialized roles:

```
Claude Code + devops.yaml = DevOps Engineer
Claude Desktop + data-scientist.yaml = Data Analyst  
Any MCP Host + technical-writer.yaml = Documentation Expert
```

## Available Transformations

### technical-writer/
Transform into a documentation specialist:
- Generate multiple diagram types
- Process markdown/asciidoc
- Create API documentation
- Build static sites

### data-scientist/
Transform into a data analysis expert:
- Statistical analysis tools
- Data visualization
- Jupyter notebook integration
- ML model evaluation

### security-auditor/
Transform into a security specialist:
- Code scanning tools
- Dependency checking
- Log analysis
- Report generation

### devops-engineer/
Transform into infrastructure automation:
- Container management
- CI/CD pipeline tools
- Cloud CLI wrappers
- Monitoring setup

## How Transformations Work

Each transformation is a complete `tools.yaml` that includes:

1. **Core tools** specific to the domain
2. **Workflows** via tool combinations  
3. **Safety constraints** appropriate to the role
4. **Documentation** of capabilities

Example structure:
```yaml
# specialized/data-scientist/full.yaml
include:
  - ../../data/csv/advanced.yaml
  - ../../data/sqlite/analysis.yaml
  - ../../diagrams/matplotlib/charts.yaml
  
tools:
  - name: analyze_dataset
    description: Run statistical analysis on CSV data
    # ... domain-specific tool
```

## Using Transformations

### Complete Transformation
Replace all host tools with specialized set:
```bash
GAMECODE_TOOLS_FILE=examples/specialized/devops-engineer/full.yaml gamecode-mcp2
```

### Additive Transformation
Add specialization to existing host tools:
```yaml
# my-enhanced-claude.yaml
include:
  - examples/specialized/technical-writer/core.yaml
  
# Host tools remain available
```

## Creating New Transformations

To create a new specialized agent:

1. **Identify domain tools** - What CLI tools do experts use?
2. **Design workflows** - How do tools combine?
3. **Set constraints** - What's safe for this domain?
4. **Document capabilities** - What can the LLM now do?

### Example: Creating a "Database Administrator" transformation

```yaml
# specialized/database-admin/full.yaml
include:
  - ../../data/sqlite/admin.yaml
  - ../../data/postgresql/client.yaml
  - ../../diagrams/erd-tools.yaml
  - ../../security/audit.yaml

tools:
  # Schema analysis
  - name: analyze_schema
    description: Analyze database schema for issues
    command: /usr/local/bin/schema-analyzer
    # ...
    
  # Performance tuning  
  - name: explain_query
    description: Run EXPLAIN ANALYZE on SQL
    command: /usr/local/bin/psql
    # ...
    
  # Backup management
  - name: backup_database  
    description: Create database backup
    command: /usr/local/bin/pg_dump
    validation:
      validate_paths: true
    # ...
```

## Security Considerations by Role

Different roles need different security postures:

### High-Risk Roles
- **DevOps Engineer**: Can modify infrastructure
- **System Administrator**: Has elevated permissions

Use paranoid validation and audit everything.

### Medium-Risk Roles  
- **Data Scientist**: Processes potentially sensitive data
- **Database Admin**: Accesses production data

Use path validation and data sanitization.

### Low-Risk Roles
- **Technical Writer**: Mostly generates text
- **Report Generator**: Read-only operations

Standard validation is sufficient.

## The Power of Composition

The real power comes from mixing transformations:

```yaml
# Become a DevSecOps Engineer
include:
  - examples/specialized/devops-engineer/core.yaml
  - examples/specialized/security-auditor/scanning.yaml

# Become a Full-Stack Developer with DevOps
include:
  - examples/development/javascript/full.yaml
  - examples/specialized/devops-engineer/deployment.yaml
```

## Future Possibilities

This pattern enables:
- **Marketplace of transformations** - Share specialized configurations
- **Role-based access** - Different tools for different users
- **Compliance modes** - HIPAA-compliant, SOC2-compliant configs
- **Industry-specific** - Finance, healthcare, government

The key insight: **The same LLM becomes a different expert based solely on available tools.**