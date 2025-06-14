# MCP-Host

A streaming-aware integration layer for adding Model Context Protocol (MCP) tool support to chat applications.

## Overview

`mcp-host` enables chat applications to seamlessly integrate MCP tools while maintaining full control over their LLM interactions. It provides intelligent streaming interception to identify and execute tool calls without disrupting the user experience.

## Key Features

- **Streaming-aware**: Process tokens in real-time without blocking
- **Smart buffering**: Automatically detect and extract tool calls
- **Multiple modes**: Choose between smart buffering, passthrough, or placeholder modes
- **Tool execution**: Execute MCP tools transparently in the background
- **Instrumentation**: Built-in debugging and performance monitoring
- **Chat app friendly**: Designed to integrate with existing chat architectures

## Architecture

```mermaid
graph TB
    subgraph "Chat Application"
        USER[User Input]
        UI[UI Display]
        CHAT[Chat Logic]
    end
    
    subgraph "mcp-host"
        MCP[McpChatIntegration]
        SI[StreamingInterceptor]
        
        SI --> |Narrative| UI
        SI --> |Tool Calls| MCP
    end
    
    subgraph "External"
        LLM[LLM Provider]
        TOOLS[MCP Tools]
    end
    
    USER --> CHAT
    CHAT --> LLM
    LLM --> |Token Stream| SI
    MCP --> TOOLS
    TOOLS --> |Results| CHAT
```

## Quick Start

```rust
use gamecode_mcp_host::{
    McpChatIntegration, ChatIntegrationConfig, StreamingMode,
    ProcessedToken, InstrumentationConfig,
};

// Initialize MCP client
let mcp_client = Arc::new(Mutex::new(
    gamecode_mcp_client::McpClient::connect("path/to/mcp-server", &[]).await?
));

// Configure integration
let config = ChatIntegrationConfig {
    streaming_mode: StreamingMode::SmartBuffering { max_buffer_chars: 150 },
    enhance_system_prompts: true,
    max_tool_rounds: 3,
    instrumentation: InstrumentationConfig {
        log_path: Some("mcp_debug.log".to_string()),
        log_token_classifications: false,
        log_performance_metrics: true,
    },
};

// Create integration
let integration = McpChatIntegration::new(mcp_client, config).await?;

// Process streaming response
let mut handle = integration.process_streaming_response(token_stream).await?;

// Handle tokens
while let Some(token) = handle.token_stream.recv().await {
    match token {
        ProcessedToken::Narrative(text) => {
            // Display to user
        }
        ProcessedToken::ToolCall(_) => {
            // Hidden from user
        }
        ProcessedToken::Buffered(_) => {
            // Still being analyzed
        }
    }
}

// Handle tool executions
while let Some(tool) = handle.tool_stream.recv().await {
    println!("Tool {} executed in {}ms", tool.tool_name, tool.execution_time_ms);
}
```

## Streaming Modes

### Smart Buffering (Default)
Intelligently detects tool calls in the stream and extracts them for execution while passing narrative text through immediately.

### Passthrough
No processing - all tokens pass through unchanged. Use when you want MCP connectivity but handle tools differently.

### With Placeholders
Replace tool calls with user-friendly placeholder text while still executing tools in the background.

## State Machine Design

The streaming interceptor uses a state machine to classify tokens:

```mermaid
stateDiagram-v2
    [*] --> Narrative: Start
    Narrative --> MaybeToolStart: Tool pattern detected
    Narrative --> Narrative: Regular text
    
    MaybeToolStart --> InToolCall: Valid JSON start
    MaybeToolStart --> Narrative: False alarm
    
    InToolCall --> InToolCall: Tracking braces
    InToolCall --> AfterToolCall: Complete JSON (depth=0)
    InToolCall --> Narrative: Invalid JSON
    
    AfterToolCall --> MaybeToolStart: Another tool pattern
    AfterToolCall --> Narrative: Resume text
```

## Examples

See the [examples directory](../examples/) for complete working examples:

- `minimal_streaming.rs` - Basic integration
- `streaming_chat_app.rs` - Full chat application
- `passthrough_mode.rs` - No interception mode
- `placeholder_mode.rs` - User-friendly placeholders
- `instrumentation_demo.rs` - Debug logging

## Documentation

- [Streaming Architecture](docs/streaming-architecture.md) - Detailed explanation of the streaming system
- [Implementation Details](docs/implementation-diagram.md) - Internal type architecture and data flow

## Integration with Chat Applications

The key insight: Chat applications need to maintain control of their LLM integration while adding tool support. This crate provides that capability without requiring changes to existing chat architectures.

Your chat app continues to:
- Manage conversation history
- Control LLM parameters
- Handle authentication
- Manage context windows

`mcp-host` adds:
- Transparent tool detection
- Automatic tool execution
- Result integration
- Debug instrumentation

## Performance

- **Minimal latency**: Narrative text passes through immediately
- **Concurrent execution**: Tools run in parallel with token display
- **Bounded memory**: Buffer size limits prevent unbounded growth
- **Single-pass parsing**: O(n) complexity for token classification

## Future Enhancements

- [ ] Streaming tool results back into the response
- [ ] Tool call validation and sandboxing
- [ ] Response caching for identical tool calls
- [ ] Model-specific prompt optimization
- [ ] WebAssembly support for browser integration