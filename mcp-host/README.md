# MCP Host

A Rust crate that bridges Language Learning Models (LLMs) with Model Context Protocol (MCP) servers, enabling LLMs to use tools through a robust retry mechanism that works across different models.

## Overview

MCP Host solves the challenge of inconsistent tool/function calling support across different LLM models by implementing a prompt-based approach with intelligent retry logic. Instead of relying on model-specific tool APIs, it uses structured prompts and validates responses, retrying with corrective feedback when necessary.

## Features

- **Universal Tool Support**: Works with any LLM model, not just those with native tool support
- **Intelligent Retry Logic**: Automatically retries failed tool calls with corrective prompts
- **Multi-Provider Support**: Currently supports Ollama, easily extensible to other providers
- **Conversation Management**: Maintains conversation history with automatic token management
- **Safety Constraints**: Built-in rate limiting and tool filtering
- **Schema Validation**: Validates tool calls against MCP tool schemas

## Architecture

```
┌─────────────┐     ┌──────────┐     ┌────────────┐
│   Your App  │────▶│ MCP Host │────▶│ MCP Server │
└─────────────┘     └─────┬────┘     └────────────┘
                          │
                          ▼
                    ┌──────────┐
                    │   LLM    │
                    │ (Ollama) │
                    └──────────┘
```

## How It Works

1. **Prompt Engineering**: Formats tool schemas into clear instructions for the LLM
2. **Response Parsing**: Extracts JSON tool calls from LLM responses
3. **Validation**: Checks tool calls against schemas
4. **Retry on Failure**: If validation fails, adds error context and retries with reduced temperature
5. **Tool Execution**: Valid tool calls are executed via MCP client
6. **Result Integration**: Tool results are added back to the conversation

## Usage

```rust
use mcp_host::{McpHost, McpHostConfig, OllamaProvider};
use std::sync::Arc;

// Initialize MCP client
let mcp_client = mcp_client::McpClient::connect(
    "path/to/mcp-server",
    &["--tools", "tools.yaml"],
).await?;
let mcp_client = Arc::new(mcp_client);

// Create LLM provider
let ollama = OllamaProvider::new("llama3.1:8b".to_string());

// Build MCP host
let mut host = McpHost::builder()
    .with_llm_provider(Box::new(ollama))
    .with_mcp_client(mcp_client)
    .with_config(McpHostConfig::default())
    .build()?;

// Process messages
let response = host.process_message("List files in the current directory").await?;
```

## Retry Mechanism

The retry mechanism is the key innovation that enables reliable tool usage across different models:

1. **Initial Attempt**: Send prompt with tool schemas and usage instructions
2. **Validation**: Check if response contains valid JSON tool calls
3. **Retry on Failure**: 
   - Add previous errors to the prompt
   - Reduce temperature for more deterministic output
   - Provide specific correction instructions
4. **Backoff**: Exponential backoff with jitter between retries

Example retry prompt addition:
```
IMPORTANT: Previous attempts failed with these errors:
Attempt 1: Invalid JSON format - expected {"tool": "name", "params": {...}}
Attempt 2: Missing required parameter 'path'

Please correct these issues in your response. Ensure:
1. Tool calls use valid JSON format
2. Parameter names match the schema exactly
3. Required parameters are not missing
```

## Model-Specific Optimizations

While the system works with any model, it includes optimizations for specific models:

- **llama3.1**: Uses native tool support when available
- **mistral**: Optimized prompts for better JSON extraction
- **qwen2.5-coder**: Code-focused prompts for better tool usage

## Safety Features

- **Rate Limiting**: Configurable requests per minute
- **Token Limits**: Maximum tokens per request
- **Tool Filtering**: Block dangerous tool patterns
- **Tool Call Limits**: Maximum tools per request

## Configuration

```rust
let config = McpHostConfig {
    max_retries: 3,
    retry_delay_ms: 1000,
    temperature_reduction: 0.1,
    safety_constraints: SafetyConfig {
        max_tokens_per_request: 4096,
        max_tools_per_request: 5,
        rate_limit_per_minute: 30,
        blocked_tool_patterns: vec!["rm".to_string()],
    },
};
```

## Extending

### Adding New LLM Providers

Implement the `LlmProvider` trait:

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse>;
    fn name(&self) -> &str;
    fn supports_tools(&self) -> bool { false }
}
```

### Custom Prompt Templates

Create model-specific prompts by extending `PromptTemplate`:

```rust
let template = PromptTemplate::new("custom-model");
// Customize tool formatting and parsing
```

## Future Enhancements

- Support for more LLM providers (OpenAI, Anthropic, etc.)
- Streaming responses
- Tool call caching
- Metrics and observability
- Fine-tuning data collection from retry patterns