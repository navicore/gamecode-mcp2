use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info};

use crate::protocol::Tool;

#[derive(Debug, Deserialize)]
pub struct ToolsConfig {
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
    pub example_output: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArgDefinition {
    pub name: String,
    pub description: String,
    pub required: bool,
    #[serde(rename = "type")]
    pub arg_type: String,
    pub cli_flag: Option<String>,
    pub default: Option<String>,
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

    pub async fn load_from_file(&mut self, path: &Path) -> Result<()> {
        info!("Loading tools from: {}", path.display());
        
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read tools file")?;
            
        let config: ToolsConfig = serde_yaml::from_str(&content)
            .context("Failed to parse YAML")?;
            
        for tool in config.tools {
            info!("Loaded tool: {}", tool.name);
            self.tools.insert(tool.name.clone(), tool);
        }
        
        Ok(())
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

    pub async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        let tool = self.tools.get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", name))?;
        
        // Handle internal handlers
        if let Some(handler) = &tool.internal_handler {
            return self.execute_internal_handler(handler, &args).await;
        }
        
        // Execute external command
        if tool.command.is_empty() || tool.command == "internal" {
            return Err(anyhow::anyhow!("Tool '{}' has no command", name));
        }
        
        let mut cmd = Command::new(&tool.command);
        
        // Add static flags
        for flag in &tool.static_flags {
            cmd.arg(flag);
        }
        
        // Add arguments
        if let Some(obj) = args.as_object() {
            for arg_def in &tool.args {
                if let Some(value) = obj.get(&arg_def.name) {
                    if let Some(cli_flag) = &arg_def.cli_flag {
                        cmd.arg(cli_flag);
                        cmd.arg(value.to_string().trim_matches('"'));
                    } else {
                        // Positional argument
                        cmd.arg(value.to_string().trim_matches('"'));
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

    async fn execute_internal_handler(&self, handler: &str, args: &Value) -> Result<Value> {
        match handler {
            "add" => {
                let a = args.get("a").and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'a'"))?;
                let b = args.get("b").and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'b'"))?;
                Ok(json!({
                    "result": a + b,
                    "operation": "addition"
                }))
            }
            "multiply" => {
                let a = args.get("a").and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'a'"))?;
                let b = args.get("b").and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow::anyhow!("Missing parameter 'b'"))?;
                Ok(json!({
                    "result": a * b,
                    "operation": "multiplication"
                }))
            }
            "list_files" => {
                let path = args.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                    
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
            _ => Err(anyhow::anyhow!("Unknown internal handler: {}", handler))
        }
    }
}