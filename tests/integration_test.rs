#[cfg(test)]
mod tests {
    use mcp_client::McpClient;
    use serde_json::json;

    #[tokio::test]
    async fn test_server_integration() -> anyhow::Result<()> {
        // This test requires the server to be built
        // Run: cargo build --bin gamecode-mcp2
        
        let server_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/debug/gamecode-mcp2");
        
        // Copy example tools.yaml to current directory for test
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../examples/tools.yaml"),
            "tools.yaml"
        )?;
        
        let mut client = McpClient::connect(server_path, &[]).await?;
        
        // Initialize
        let init_result = client.initialize("test-client", "1.0.0").await?;
        assert_eq!(init_result.server_info.name, "gamecode-mcp2");
        
        // List tools
        let tools = client.list_tools().await?;
        assert!(tools.iter().any(|t| t.name == "add"));
        assert!(tools.iter().any(|t| t.name == "multiply"));
        assert!(tools.iter().any(|t| t.name == "list_files"));
        
        // Call add tool
        let result = client.call_tool("add", json!({ "a": 5, "b": 3 })).await?;
        assert_eq!(result["result"], 8);
        assert_eq!(result["operation"], "addition");
        
        // Call multiply tool
        let result = client.call_tool("multiply", json!({ "a": 6, "b": 7 })).await?;
        assert_eq!(result["result"], 42);
        
        // Clean up
        std::fs::remove_file("tools.yaml").ok();
        
        Ok(())
    }
}