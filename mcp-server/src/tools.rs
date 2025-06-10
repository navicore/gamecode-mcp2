// Tool execution is the critical security boundary.
// Every tool must be explicitly configured - no implicit capabilities.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info};

use crate::protocol::Tool;
use crate::validation;

// Tools config - what tools exist is controlled by YAML, not code
#[derive(Debug, Deserialize)]
pub struct ToolsConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub args: Vec<ArgDefinition>,
    #[serde(default)]
    pub static_flags: Vec<String>,
    pub internal_handler: Option<String>,
    #[allow(dead_code)]
    pub example_output: Option<Value>,
    #[serde(default)]
    pub validation: ValidationConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ValidationConfig {
    #[serde(default)]
    pub validate_paths: bool,
    #[serde(default)]
    pub allow_absolute_paths: bool,
    #[serde(default)]  
    pub validate_args: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArgDefinition {
    pub name: String,
    pub description: String,
    pub required: bool,
    #[serde(rename = "type")]
    pub arg_type: String,
    pub cli_flag: Option<String>,
    #[allow(dead_code)]
    pub default: Option<String>,
    #[serde(default)]
    pub is_path: bool,  // Mark arguments that are file paths
}

pub struct ToolManager {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolManager {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    #[cfg(test)]
    pub fn new_for_testing() -> Self {
        // For tests, just create empty manager that tests can load explicitly
        Self::new()
    }

    // Explicit tool loading - admin controls what tools are available
    pub async fn load_from_file(&mut self, path: &Path) -> Result<()> {
        info!("Loading tools from: {}", path.display());

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read tools file")?;

        // YAML parsing is the only text processing we can't avoid
        let config: ToolsConfig = serde_yaml::from_str(&content).context("Failed to parse YAML")?;

        // Process includes first
        for include in &config.include {
            let include_path = self.resolve_include_path(path, include)?;
            info!("Including tools from: {}", include_path.display());

            // Recursively load included files
            Box::pin(self.load_from_file(&include_path)).await?;
        }

        // Then load tools from this file
        for tool in config.tools {
            info!("Loaded tool: {}", tool.name);
            self.tools.insert(tool.name.clone(), tool);
        }

        Ok(())
    }

    fn resolve_include_path(&self, base_path: &Path, include: &str) -> Result<PathBuf> {
        let base_dir = base_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine parent directory"))?;

        // Support both relative and absolute paths
        let include_path = if include.starts_with('/') {
            PathBuf::from(include)
        } else {
            match include.starts_with("~/") {
                true => {
                    if let Some(home) = directories::UserDirs::new() {
                        home.home_dir().join(&include[2..])
                    } else {
                        return Err(anyhow::anyhow!("Cannot resolve home directory"));
                    }
                }
                false => {
                    // Relative path
                    base_dir.join(include)
                }
            }
        };

        if !include_path.exists() {
            return Err(anyhow::anyhow!(
                "Include file not found: {}",
                include_path.display()
            ));
        }

        Ok(include_path)
    }

    pub async fn load_from_default_locations(&mut self) -> Result<()> {
        // Check for tools.yaml in various locations
        let paths = vec![
            PathBuf::from("./tools.yaml"),
            PathBuf::from("~/.config/gamecode-mcp/tools.yaml"),
        ];

        if let Ok(tools_file) = std::env::var("GAMECODE_TOOLS_FILE") {
            return self.load_from_file(Path::new(&tools_file)).await;
        }

        for path in paths {
            let expanded = if path.starts_with("~") {
                if let Some(home) = directories::UserDirs::new() {
                    home.home_dir().join(path.strip_prefix("~").unwrap())
                } else {
                    continue;
                }
            } else {
                path
            };

            if expanded.exists() {
                return self.load_from_file(&expanded).await;
            }
        }

        Err(anyhow::anyhow!("No tools.yaml file found"))
    }

    pub async fn load_mode(&mut self, mode: &str) -> Result<()> {
        // Clear existing tools when switching modes
        self.tools.clear();

        // Load the mode-specific configuration
        let mode_file = format!("tools/profiles/{}.yaml", mode);
        let mode_path = PathBuf::from(&mode_file);

        if mode_path.exists() {
            self.load_from_file(&mode_path).await
        } else {
            // Try in config directory
            if let Some(home) = directories::UserDirs::new() {
                let config_path = home
                    .home_dir()
                    .join(".config/gamecode-mcp")
                    .join(&mode_file);
                if config_path.exists() {
                    return self.load_from_file(&config_path).await;
                }
            }

            Err(anyhow::anyhow!("Mode configuration '{}' not found", mode))
        }
    }

    pub async fn load_with_precedence(&mut self, cli_override: Option<String>) -> Result<()> {
        // Clear precedence order:
        // 1. Command-line flag (--tools-file)
        if let Some(tools_file) = cli_override {
            info!("Loading tools from command-line override: {}", tools_file);
            return self.load_from_file(Path::new(&tools_file)).await;
        }
        
        // 2. Environment variable
        if let Ok(tools_file) = std::env::var("GAMECODE_TOOLS_FILE") {
            info!("Loading tools from GAMECODE_TOOLS_FILE: {}", tools_file);
            return self.load_from_file(Path::new(&tools_file)).await;
        }
        
        // 3. Local tools.yaml in current directory
        let local_tools = PathBuf::from("./tools.yaml");
        if local_tools.exists() {
            info!("Loading tools from local tools.yaml");
            return self.load_from_file(&local_tools).await;
        }
        
        // 4. Auto-detection (only if no local tools.yaml)
        if let Ok(mode) = self.detect_project_type() {
            info!("Auto-detected {} project", mode);
            if let Ok(_) = self.load_auto_detected_tools(&mode).await {
                return Ok(());
            }
        }
        
        // 5. Config directory fallback
        if let Some(home) = directories::UserDirs::new() {
            let config_tools = home.home_dir()
                .join(".config/gamecode-mcp/tools.yaml");
            if config_tools.exists() {
                info!("Loading tools from config directory");
                return self.load_from_file(&config_tools).await;
            }
        }
        
        Err(anyhow::anyhow!("No tools configuration found. Create tools.yaml or use --tools-file"))
    }
    
    fn detect_project_type(&self) -> Result<String> {
        let detections = vec![
            ("Cargo.toml", "rust"),
            ("package.json", "javascript"),
            ("requirements.txt", "python"),
            ("go.mod", "go"),
            ("pom.xml", "java"),
            ("build.gradle", "java"),
            ("Gemfile", "ruby"),
        ];
        
        for (file, mode) in detections {
            if PathBuf::from(file).exists() {
                return Ok(mode.to_string());
            }
        }
        
        Err(anyhow::anyhow!("No project type detected"))
    }
    
    async fn load_auto_detected_tools(&mut self, mode: &str) -> Result<()> {
        // Try to load language-specific tools
        let lang_file = format!("tools/languages/{}.yaml", mode);
        if PathBuf::from(&lang_file).exists() {
            self.load_from_file(Path::new(&lang_file)).await?;
        }
        
        // Always load core tools as well
        if PathBuf::from("tools/core.yaml").exists() {
            self.load_from_file(Path::new("tools/core.yaml")).await?;
        }
        
        // Load git tools if .git exists
        if PathBuf::from(".git").exists() && PathBuf::from("tools/git.yaml").exists() {
            self.load_from_file(Path::new("tools/git.yaml")).await?;
        }
        
        Ok(())
    }

    // Convert to MCP schema - LLM sees exactly this, nothing hidden
    pub fn get_mcp_tools(&self) -> Vec<Tool> {
        self.tools
            .values()
            .map(|def| {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                // Build JSON schema from arg definitions
                for arg in &def.args {
                    let arg_schema = match arg.arg_type.as_str() {
                        "string" => json!({
                            "type": "string",
                            "description": arg.description
                        }),
                        "number" => json!({
                            "type": "number",
                            "description": arg.description
                        }),
                        "boolean" => json!({
                            "type": "boolean",
                            "description": arg.description
                        }),
                        "array" => json!({
                            "type": "array",
                            "description": arg.description
                        }),
                        _ => json!({
                            "type": "string",
                            "description": arg.description
                        }),
                    };

                    properties.insert(arg.name.clone(), arg_schema);

                    if arg.required {
                        required.push(json!(arg.name));
                    }
                }

                let schema = json!({
                    "type": "object",
                    "properties": properties,
                    "required": required
                });

                Tool {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    input_schema: schema,
                }
            })
            .collect()
    }

    // Tool execution - the critical security boundary
    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", name))?;

        // Internal handlers are hardcoded - no dynamic code execution
        if let Some(handler) = &tool.internal_handler {
            return self.execute_internal_handler(handler, &args).await;
        }

        // External commands - only what's explicitly configured
        if tool.command.is_empty() || tool.command == "internal" {
            return Err(anyhow::anyhow!("Tool '{}' has no command", name));
        }

        let mut cmd = Command::new(&tool.command);

        // Add static flags
        for flag in &tool.static_flags {
            cmd.arg(flag);
        }

        // Argument construction - no shell interpretation, direct args only
        if let Some(obj) = args.as_object() {
            for arg_def in &tool.args {
                if let Some(value) = obj.get(&arg_def.name) {
                    // Optional validation
                    if tool.validation.validate_args {
                        validation::validate_typed_value(value, &arg_def.arg_type)?;
                    }
                    
                    // Path validation if marked as path
                    if arg_def.is_path && tool.validation.validate_paths {
                        if let Some(path_str) = value.as_str() {
                            validation::validate_path(path_str, tool.validation.allow_absolute_paths)?;
                        }
                    }
                    
                    let arg_value = value.to_string().trim_matches('"').to_string();
                    
                    if let Some(cli_flag) = &arg_def.cli_flag {
                        cmd.arg(cli_flag);
                        cmd.arg(&arg_value);
                    } else {
                        // Positional argument
                        cmd.arg(&arg_value);
                    }
                }
            }
        }

        debug!("Executing command: {:?}", cmd);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute command")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);

            // Try to parse as JSON first
            if let Ok(json_value) = serde_json::from_str::<Value>(&stdout) {
                Ok(json_value)
            } else {
                Ok(json!({
                    "output": stdout.trim(),
                    "status": "success"
                }))
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Command failed: {}", stderr))
        }
    }

    // Internal handlers - hardcoded, no dynamic evaluation
    async fn execute_internal_handler(&self, handler: &str, args: &Value) -> Result<Value> {
        match handler {
            "add" => {
                let a = args
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'a'"))?;
                let b = args
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'b'"))?;
                Ok(json!({
                    "result": a + b,
                    "operation": "addition"
                }))
            }
            "multiply" => {
                let a = args
                    .get("a")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'a'"))?;
                let b = args
                    .get("b")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'b'"))?;
                Ok(json!({
                    "result": a * b,
                    "operation": "multiplication"
                }))
            }
            "list_files" => {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

                let mut files = Vec::new();
                let mut entries = tokio::fs::read_dir(path).await?;

                while let Some(entry) = entries.next_entry().await? {
                    let metadata = entry.metadata().await?;
                    files.push(json!({
                        "name": entry.file_name().to_string_lossy(),
                        "is_dir": metadata.is_dir(),
                        "size": metadata.len()
                    }));
                }

                Ok(json!({
                    "path": path,
                    "files": files
                }))
            }
            "write_file" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'path'"))?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'content'"))?;

                tokio::fs::write(path, content).await?;

                Ok(json!({
                    "status": "success",
                    "path": path,
                    "bytes_written": content.len()
                }))
            }
            "create_graphviz_diagram" => {
                let filename = args
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'filename'"))?;
                let format = args
                    .get("format")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'format'"))?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'content'"))?;

                // Save DOT source file
                let dot_file = format!("{}.dot", filename);
                tokio::fs::write(&dot_file, content).await?;

                // Generate diagram using GraphViz
                let output_file = format!("{}.{}", filename, format);
                let output = tokio::process::Command::new("dot")
                    .arg(format!("-T{}", format))
                    .arg(&dot_file)
                    .arg("-o")
                    .arg(&output_file)
                    .output()
                    .await?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("GraphViz error: {}", stderr));
                }

                Ok(json!({
                    "status": "success",
                    "source_file": dot_file,
                    "output_file": output_file,
                    "format": format
                }))
            }
            "create_plantuml_diagram" => {
                let filename = args
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'filename'"))?;
                let format = args
                    .get("format")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'format'"))?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'content'"))?;

                // Save PlantUML source file
                let puml_file = format!("{}.puml", filename);
                tokio::fs::write(&puml_file, content).await?;

                // Generate diagram using PlantUML
                let output = tokio::process::Command::new("plantuml")
                    .arg(format!("-t{}", format))
                    .arg(&puml_file)
                    .output()
                    .await?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow::anyhow!("PlantUML error: {}", stderr));
                }

                // PlantUML generates output with same base name
                let output_file = format!("{}.{}", filename, format);

                Ok(json!({
                    "status": "success",
                    "source_file": puml_file,
                    "output_file": output_file,
                    "format": format
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown internal handler: {}", handler)),
        }
    }
}
