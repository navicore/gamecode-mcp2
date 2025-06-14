# MCP-Host Examples

This directory contains examples demonstrating different ways to integrate `mcp-host` into chat applications.

## Examples Overview

### `minimal_streaming.rs`
The simplest possible integration showing:
- Basic MCP client connection
- Smart buffering mode (default)
- Token classification output
- Tool execution results

**When to use**: Start here to understand the basic flow.

### `streaming_chat_app.rs`
Full-featured chat application example showing:
- Complete conversation loop
- Ollama integration simulation
- UI handling with typing indicators
- Tool usage indicators
- Message history management

**When to use**: Reference implementation for building a real chat app.

### `passthrough_mode.rs`
Demonstrates passthrough mode where:
- No tool interception occurs
- All tokens pass through unchanged
- Useful for debugging or when tools are handled elsewhere

**When to use**: When you want MCP connection but handle tools differently.

### `placeholder_mode.rs`
Shows placeholder mode where:
- Tool calls are replaced with user-friendly text
- Tools still execute in the background
- Clean output for end users

**When to use**: When you want to hide technical details from users.

### `instrumentation_demo.rs`
Comprehensive instrumentation example showing:
- Full debug logging to JSONL file
- Token classification metrics
- Performance timing
- Multiple streaming patterns

**When to use**: When debugging integration issues or optimizing performance.

## Running the Examples

1. First, ensure the MCP server is built:
```bash
cargo build --bin gamecode-mcp2
```

2. Create a `tools.yaml` file (or copy from examples):
```bash
cp examples/mcp_host_example_tools.yaml tools.yaml
```

3. Run any example:
```bash
cargo run --example minimal_streaming
cargo run --example streaming_chat_app
# etc.
```

## Integration Patterns

### Basic Integration Flow
1. Initialize MCP client connection
2. Configure streaming mode and options
3. Create `McpChatIntegration` instance
4. For each LLM response:
   - Pass token stream to `process_streaming_response()`
   - Handle tokens from `token_stream`
   - Handle tool executions from `tool_stream`

### Streaming Modes

**Smart Buffering** (Recommended)
- Automatically detects and extracts tool calls
- Minimal latency for narrative text
- Configurable buffer size

**Passthrough**
- No processing, all tokens pass through
- Useful when tool handling is done elsewhere

**With Placeholders**
- Replace tool calls with user-friendly text
- Tools still execute in background
- Good for polished user interfaces

### Error Handling

All examples use `Result<()>` for simplicity, but production code should:
- Handle connection failures gracefully
- Implement reconnection logic
- Log errors to instrumentation
- Provide user feedback for tool failures

## Debugging Tips

1. Enable instrumentation logging:
```rust
instrumentation: InstrumentationConfig {
    log_path: Some("debug.jsonl".to_string()),
    log_token_classifications: true,
    log_performance_metrics: true,
}
```

2. Analyze the JSONL output:
```bash
# Count event types
cat debug.jsonl | jq '.event_type' | sort | uniq -c

# Find slow operations
cat debug.jsonl | jq 'select(.duration_ms > 1000)'
```

3. Test with different token patterns to ensure robust parsing

## Production Considerations

- Use connection pooling for multiple chat sessions
- Implement proper error boundaries
- Add metrics collection
- Consider rate limiting
- Test with your specific LLM's response patterns