use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tracing::debug;

use crate::protocol::*;

pub struct StdioTransport {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _process: Child,
}

impl StdioTransport {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout, process: Child) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
            _process: process,
        }
    }

    pub async fn send_request(&mut self, request: &JsonRpcRequest) -> Result<serde_json::Value> {
        // Send request
        let request_str = serde_json::to_string(request)?;
        debug!("Sending request: {}", request_str);
        
        self.stdin.write_all(request_str.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        
        // Read response
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line).await?;
        
        debug!("Received response: {}", response_line);
        
        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .context("Failed to parse JSON-RPC response")?;
        
        if let Some(error) = response.error {
            anyhow::bail!("RPC error {}: {}", error.code, error.message);
        }
        
        response.result.context("No result in response")
    }

    pub async fn send_notification(&mut self, notification: &JsonRpcNotification) -> Result<()> {
        let notification_str = serde_json::to_string(notification)?;
        debug!("Sending notification: {}", notification_str);
        
        self.stdin.write_all(notification_str.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        
        Ok(())
    }
}