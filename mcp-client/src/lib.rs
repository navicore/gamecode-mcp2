use anyhow::{Context, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, info};

pub mod protocol;
pub mod transport;

use protocol::*;

pub struct McpClient {
    transport: transport::StdioTransport,
    request_id: u64,
}

impl McpClient {
    pub fn new(mut process: Child) -> Result<Self> {
        let stdin = process.stdin.take()
            .context("Failed to get stdin from process")?;
        let stdout = process.stdout.take()
            .context("Failed to get stdout from process")?;
        
        Ok(Self {
            transport: transport::StdioTransport::new(stdin, stdout, process),
            request_id: 0,
        })
    }

    pub async fn connect(command: &str, args: &[String]) -> Result<Self> {
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());
        
        let process = cmd.spawn()
            .context("Failed to spawn MCP server process")?;
            
        Self::new(process)
    }

    pub async fn initialize(&mut self, client_name: &str, client_version: &str) -> Result<InitializeResult> {
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities {
                tools: Some(ToolsCapability {}),
            },
            client_info: ClientInfo {
                name: client_name.to_string(),
                version: client_version.to_string(),
            },
        };

        let response = self.request("initialize", Some(serde_json::to_value(params)?)).await?;
        
        // Send initialized notification
        self.notify("notifications/initialized", None).await?;
        
        serde_json::from_value(response)
            .context("Failed to parse initialize response")
    }

    pub async fn list_tools(&mut self) -> Result<Vec<Tool>> {
        let response = self.request("tools/list", None).await?;
        let result: ListToolsResult = serde_json::from_value(response)
            .context("Failed to parse tools list")?;
        Ok(result.tools)
    }

    pub async fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> {
        let params = CallToolParams {
            name: name.to_string(),
            arguments,
        };

        let response = self.request("tools/call", Some(serde_json::to_value(params)?)).await?;
        let result: CallToolResult = serde_json::from_value(response)
            .context("Failed to parse tool call result")?;
        
        if result.is_error.unwrap_or(false) {
            if let Some(ContentBlock::Text { text }) = result.content.first() {
                anyhow::bail!("Tool error: {}", text);
            }
        }
        
        // Extract the text content
        if let Some(ContentBlock::Text { text }) = result.content.first() {
            serde_json::from_str(text)
                .context("Failed to parse tool result")
        } else {
            Ok(Value::Null)
        }
    }

    async fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        self.request_id += 1;
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: json!(self.request_id),
            method: method.to_string(),
            params,
        };

        self.transport.send_request(&request).await
    }

    async fn notify(&mut self, method: &str, params: Option<Value>) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };

        self.transport.send_notification(&notification).await
    }
}