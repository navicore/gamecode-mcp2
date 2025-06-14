# Streaming Architecture

This document explains how the streaming interceptor identifies and extracts tool calls from LLM token streams while maintaining a smooth user experience.

## Parser State Machine

The streaming interceptor uses a state machine to classify tokens as they arrive:

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
    AfterToolCall --> AfterToolCall: Whitespace
```

## Token Flow Sequence

Here's how tokens flow through the system:

```mermaid
sequenceDiagram
    participant LLM as LLM Stream
    participant SI as StreamingInterceptor
    participant Buffer as Token Buffer
    participant TC as Token Classifier
    participant UI as UI Channel
    participant TE as Tool Executor
    
    LLM->>SI: Stream tokens
    
    loop For each token
        SI->>Buffer: Append token
        SI->>SI: Check parser state
        
        alt Narrative State
            SI->>TC: Check for tool pattern
            alt Pattern found
                SI->>SI: State = MaybeToolStart
            else No pattern & buffer full
                SI->>UI: Emit Narrative(buffer)
                SI->>Buffer: Clear buffer
            end
        else MaybeToolStart State
            SI->>TC: Try parse JSON
            alt Valid JSON start
                SI->>SI: State = InToolCall
            else Invalid or timeout
                SI->>UI: Emit Narrative(buffer)
                SI->>SI: State = Narrative
            end
        else InToolCall State
            SI->>TC: Calculate brace depth
            alt Complete JSON (depth=0)
                SI->>TC: Parse tool call
                SI->>UI: Emit ToolCall(json)
                SI->>TE: Send tool for execution
                SI->>Buffer: Clear buffer
                SI->>SI: State = AfterToolCall
            else Still in JSON
                SI->>SI: Update brace depth
            end
        else AfterToolCall State
            alt New tool pattern
                SI->>SI: State = MaybeToolStart
            else Regular text
                SI->>SI: State = Narrative
            end
        end
    end
    
    SI->>UI: Flush remaining buffer
```

## Integration Flow

This diagram shows how the mcp-host integrates with a chat application:

```mermaid
sequenceDiagram
    participant User
    participant ChatApp as Chat Application
    participant LLM as LLM Provider
    participant MCP as McpChatIntegration
    participant SI as StreamingInterceptor
    participant MC as MCP Client
    participant Tool as Tool Process
    
    User->>ChatApp: Send message
    ChatApp->>ChatApp: Add to conversation
    
    ChatApp->>MCP: enhance_system_prompt()
    MCP->>MC: list_tools()
    MC-->>MCP: Available tools
    MCP-->>ChatApp: Enhanced prompt
    
    ChatApp->>LLM: Request streaming response
    LLM-->>ChatApp: Token stream
    
    ChatApp->>MCP: process_streaming_response(stream)
    MCP->>SI: Create interceptor
    MCP-->>ChatApp: ResponseHandle
    
    activate SI
    par Token Processing
        loop Each token
            LLM->>SI: Token
            SI->>SI: Classify token
            alt Narrative
                SI-->>ChatApp: ProcessedToken::Narrative
                ChatApp->>User: Display text
            else Tool Call
                SI-->>ChatApp: ProcessedToken::ToolCall
                note over ChatApp: Hidden from user
            end
        end
    and Tool Execution
        loop Each tool detected
            SI->>MC: call_tool(name, params)
            MC->>Tool: Execute tool
            Tool-->>MC: Result
            MC-->>SI: Tool result
            SI-->>ChatApp: ExecutedTool
            ChatApp->>User: Show indicator (optional)
        end
    end
    deactivate SI
```

## Token Classification Examples

### Example 1: Clean Tool Call

```
Input tokens: ["Let me check.\n\n", "{\"tool\": \"ls\",", " \"params\": {}", "}"]

State transitions:
1. "Let me check.\n\n" → Narrative (ends with \n\n) → Emit
2. "{\"tool\": \"ls\"," → MaybeToolStart → InToolCall (depth=1)
3. " \"params\": {}" → InToolCall (depth=2)
4. "}" → InToolCall (depth=0) → Parse & emit tool → AfterToolCall
```

### Example 2: Mixed Content

```
Input tokens: ["I'll use ", "{\"tool\":", " \"calc\", ", "\"params\":", " {\"expr\":", " \"2+2\"}}", " to help."]

State transitions:
1. "I'll use " → Narrative → Buffer
2. "{\"tool\":" → MaybeToolStart → InToolCall
3-6. Build complete JSON in buffer
7. " to help." → AfterToolCall → Narrative → Emit
```

### Example 3: False Positive

```
Input tokens: ["The JSON format ", "{ key: value } ", "is common."]

State transitions:
1. "The JSON format " → Narrative → Buffer
2. "{ key: value } " → MaybeToolStart → Invalid JSON → Narrative
3. "is common." → Narrative → Emit all as narrative
```

## Performance Considerations

1. **Buffer Management**: 
   - Default max buffer: 200 chars
   - Prevents memory growth from malformed JSON
   - Natural boundaries (newlines, sentences) trigger early emission

2. **Parallel Processing**:
   - Token classification and tool execution run concurrently
   - UI remains responsive during tool execution
   - Tool results don't block token display

3. **State Machine Efficiency**:
   - O(1) state transitions
   - Single pass through tokens
   - Minimal regex usage (only for initial pattern detection)

## Configuration Options

### Smart Buffering (Recommended)
```rust
StreamingMode::SmartBuffering { 
    max_buffer_chars: 150  // Tune based on your LLM's style
}
```
- Automatically detects and hides tool calls
- Minimal latency for narrative text
- Configurable buffer size

### Passthrough Mode
```rust
StreamingMode::Passthrough
```
- No processing overhead
- All tokens pass through immediately
- Useful when tools are handled elsewhere

### Placeholder Mode
```rust
StreamingMode::WithPlaceholders { 
    placeholder_text: "[Working...]".to_string() 
}
```
- Replace tool JSON with user-friendly text
- Tools still execute in background
- Good for polished UIs

## Error Handling

The interceptor handles various edge cases:

1. **Malformed JSON**: Treated as narrative text
2. **Incomplete streams**: Buffer flushed on stream end
3. **Network interruptions**: Partial tool calls ignored
4. **Invalid tool names**: Passed through as narrative

## Testing Strategies

1. **Unit tests**: Test state transitions with known patterns
2. **Fuzz testing**: Random token sequences shouldn't crash
3. **Integration tests**: Real LLM responses with various models
4. **Performance tests**: Measure latency and throughput