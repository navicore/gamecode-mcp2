# Best Practices for Diagramming with Claude via MCP

## Overview

Claude can generate excellent PlantUML diagrams, but rendering them requires external tools. Here are recommended approaches, from safest to most convenient.

## Approach 1: Source-Only (Safest)

Claude writes PlantUML source files, you render them manually:

```yaml
- name: save_diagram_source
  description: Save PlantUML source to file
  command: internal
  internal_handler: write_file
  validation:
    validate_paths: true
    allow_absolute_paths: false
  args:
    - name: path
      description: Output path (e.g., architecture.puml)
      required: true
      type: string
      is_path: true
    - name: content
      description: PlantUML diagram source
      required: true
      type: string
```

**Workflow:**
1. Ask Claude to create a diagram
2. Claude generates PlantUML and saves to `diagrams/mydiagram.puml`
3. You run `plantuml diagrams/mydiagram.puml` manually
4. View the generated PNG/SVG

**Pros:** Very safe, full control
**Cons:** Manual rendering step

## Approach 2: Integrated Rendering (Convenient)

Claude renders diagrams directly:

```yaml
- name: render_diagram
  description: Render PlantUML to PNG and open it
  command: /usr/local/bin/plantuml
  static_flags:
    - "-pipe"
    - "-tpng"
  validation:
    validate_args: true
  args:
    - name: output_file
      description: Output filename
      required: true
      type: string
      cli_flag: "-o"
```

**Workflow:**
1. Ask Claude to create and render a diagram
2. Claude generates PlantUML and calls the render tool
3. Diagram appears in `diagrams/` directory
4. Optional: tool opens the image automatically

## Approach 3: Web-Based Preview (Interactive)

Create a local web server for diagram preview:

```yaml
- name: diagram_server
  description: Start local diagram preview server
  command: /usr/bin/python3
  static_flags:
    - "-m"
    - "http.server"
    - "8888"
    - "--directory"
    - "./diagrams"
```

Then browse to http://localhost:8888 to see all diagrams.

## Security Considerations

1. **Output Directory**: Always restrict output to a specific directory
   ```yaml
   validation:
     validate_paths: true
     allow_absolute_paths: false
   ```

2. **File Types**: Only allow specific extensions (.puml, .png, .svg)

3. **Size Limits**: PlantUML can consume resources with complex diagrams

4. **No Shell Injection**: Never pass diagram content through shell

## Implementation Example

Here's a safe internal handler for diagram creation:

```rust
"create_diagram" => {
    let filename = args.get("filename")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing filename"))?;
    
    let plantuml = args.get("plantuml")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing plantuml content"))?;
    
    let format = args.get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("png");
    
    // Validate filename (no path traversal)
    if filename.contains("..") || filename.contains("/") {
        return Err(anyhow!("Invalid filename"));
    }
    
    // Ensure diagrams directory exists
    tokio::fs::create_dir_all("./diagrams").await?;
    
    // Write PlantUML source
    let source_path = format!("./diagrams/{}.puml", filename);
    tokio::fs::write(&source_path, plantuml).await?;
    
    // Render using plantuml command
    let output_path = format!("./diagrams/{}.{}", filename, format);
    let output = Command::new("plantuml")
        .arg("-t").arg(format)
        .arg("-o").arg("./diagrams")
        .arg(&source_path)
        .output()
        .await?;
    
    if !output.status.success() {
        return Err(anyhow!("PlantUML rendering failed"));
    }
    
    Ok(json!({
        "source": source_path,
        "output": output_path,
        "format": format
    }))
}
```

## Recommended Setup

1. **Install PlantUML**: 
   ```bash
   brew install plantuml  # macOS
   apt-get install plantuml  # Ubuntu
   ```

2. **Create diagram directory**:
   ```bash
   mkdir -p ./diagrams
   ```

3. **Configure tools**: Use `diagrams.yaml` example

4. **Set permissions**: Ensure diagram directory is writable

## Example Claude Prompts

**Generate and save source:**
> "Create a PlantUML class diagram for a simple MVC architecture and save it as mvc-pattern.puml"

**Generate and render:**
> "Create a sequence diagram showing OAuth2 flow and render it as oauth-flow.png"

**Batch generation:**
> "Generate PlantUML diagrams for: 1) system architecture, 2) database schema, 3) deployment topology"

## Tips

1. **Naming Convention**: Use descriptive names like `auth-sequence-diagram.png`
2. **Version Control**: Commit `.puml` sources, optionally ignore rendered images
3. **Diagram Types**: Claude excels at:
   - Class diagrams
   - Sequence diagrams
   - Component diagrams
   - State machines
   - Entity relationships

4. **Iterative Refinement**: 
   > "Add error handling to the sequence diagram"
   > "Show the database connection in the component diagram"

## Alternative: Mermaid

For web-based workflows, consider Mermaid instead of PlantUML:

```yaml
- name: save_mermaid
  description: Save Mermaid diagram source
  command: internal
  internal_handler: write_file
  args:
    - name: path
      description: Output path (e.g., diagram.mmd)
      required: true
      type: string
    - name: content
      description: Mermaid diagram source
      required: true  
      type: string
```

Then use Mermaid Live Editor or integrate with your documentation.