# Implementation Details

## Rust Type Architecture

```mermaid
classDiagram
    class StreamingInterceptor {
        -buffer: String
        -state: ParserState
        -tool_start_pattern: Regex
        -max_buffer_size: usize
        +new() Self
        +process_stream(Stream) (Receiver~TokenClass~, Receiver~ToolCall~)
        -process_token(String, Sender, Sender)
        -emit_narrative(Sender)
        -flush_buffer(Sender)
        -calculate_brace_depth() Option~i32~
        -try_parse_tool_call() Option~ToolCall~
    }
    
    class ParserState {
        <<enumeration>>
        Narrative
        MaybeToolStart
        InToolCall(brace_depth: i32)
        AfterToolCall
    }
    
    class TokenClass {
        <<enumeration>>
        Narrative(String)
        ToolCall(String)
        Pending(String)
    }
    
    class ToolCall {
        +tool: String
        +params: Value
    }
    
    class ProcessedToken {
        <<enumeration>>
        Narrative(String)
        ToolCall(String)
        Buffered(String)
    }
    
    class McpChatIntegration {
        -mcp_client: Arc~Mutex~McpClient~~
        -config: ChatIntegrationConfig
        -instrumentation: Option~InstrumentationHandle~
        +new(client, config) Result~Self~
        +enhance_system_prompt(String) Result~String~
        +process_streaming_response(Stream) Result~StreamingResponseHandle~
    }
    
    class ChatIntegrationConfig {
        +streaming_mode: StreamingMode
        +enhance_system_prompts: bool
        +max_tool_rounds: usize
        +instrumentation: InstrumentationConfig
    }
    
    class StreamingMode {
        <<enumeration>>
        SmartBuffering(max_buffer_chars: usize)
        Passthrough
        WithPlaceholders(placeholder_text: String)
    }
    
    StreamingInterceptor --> ParserState: uses
    StreamingInterceptor --> TokenClass: produces
    StreamingInterceptor --> ToolCall: extracts
    McpChatIntegration --> StreamingInterceptor: creates
    McpChatIntegration --> ProcessedToken: converts from TokenClass
    McpChatIntegration --> ChatIntegrationConfig: configured by
    ChatIntegrationConfig --> StreamingMode: contains
```

## Data Flow Through Types

```mermaid
flowchart TB
    subgraph "Input Layer"
        TS[Token Stream<br/>Stream~String~]
    end
    
    subgraph "Parsing Layer"
        SI[StreamingInterceptor]
        PS{ParserState}
        BUF[Token Buffer<br/>String]
        
        SI --> PS
        SI --> BUF
        PS --> |State Machine| PS
    end
    
    subgraph "Classification Layer"
        TC1[TokenClass::Narrative]
        TC2[TokenClass::ToolCall]
        TC3[TokenClass::Pending]
        
        TOOL[ToolCall Struct<br/>tool: String<br/>params: Value]
    end
    
    subgraph "Integration Layer"
        MCP[McpChatIntegration]
        PT1[ProcessedToken::Narrative]
        PT2[ProcessedToken::ToolCall]
        PT3[ProcessedToken::Buffered]
        
        ET[ExecutedTool<br/>+result: Value<br/>+execution_time_ms]
    end
    
    subgraph "Output Channels"
        TCH[Token Channel<br/>Receiver~ProcessedToken~]
        ECH[Tool Channel<br/>Receiver~ExecutedTool~]
    end
    
    TS --> SI
    SI --> TC1
    SI --> TC2
    SI --> TC3
    TC2 --> TOOL
    
    TC1 --> MCP
    TC2 --> MCP
    TC3 --> MCP
    TOOL --> MCP
    
    MCP --> PT1
    MCP --> PT2
    MCP --> PT3
    MCP --> ET
    
    PT1 --> TCH
    PT2 --> TCH
    PT3 --> TCH
    ET --> ECH
    
    style TC2 fill:#f9f,stroke:#333,stroke-width:2px
    style TOOL fill:#f9f,stroke:#333,stroke-width:2px
    style ET fill:#9ff,stroke:#333,stroke-width:2px
```

## State Transition Implementation

```rust
// Simplified view of state transitions
match &self.state {
    ParserState::Narrative => {
        if self.tool_start_pattern.is_match(&self.buffer) {
            self.state = ParserState::MaybeToolStart;
        } else if self.buffer.len() > self.max_buffer_size {
            self.emit_narrative(token_tx).await;
        }
    }
    
    ParserState::MaybeToolStart => {
        if let Some(depth) = self.calculate_brace_depth() {
            if depth > 0 {
                self.state = ParserState::InToolCall { brace_depth: depth };
            } else {
                self.emit_narrative(token_tx).await;
                self.state = ParserState::Narrative;
            }
        }
    }
    
    ParserState::InToolCall { brace_depth } => {
        let new_depth = self.calculate_brace_depth().unwrap_or(*brace_depth);
        if new_depth == 0 {
            if let Some(tool_call) = self.try_parse_tool_call() {
                tool_tx.send(tool_call).await;
                token_tx.send(TokenClass::ToolCall(self.buffer.clone())).await;
                self.buffer.clear();
                self.state = ParserState::AfterToolCall;
            } else {
                self.emit_narrative(token_tx).await;
                self.state = ParserState::Narrative;
            }
        }
    }
    
    ParserState::AfterToolCall => {
        if self.tool_start_pattern.is_match(&self.buffer) {
            self.state = ParserState::MaybeToolStart;
        } else if !self.buffer.trim().is_empty() {
            self.state = ParserState::Narrative;
        }
    }
}
```

## Channel Architecture

```mermaid
graph LR
    subgraph "Async Tasks"
        T1[Token Stream<br/>Processor]
        T2[Token<br/>Classifier]
        T3[Tool<br/>Executor]
    end
    
    subgraph "Channels"
        C1((token_rx/tx))
        C2((tool_rx/tx))
        C3((processed_rx/tx))
        C4((executed_rx/tx))
    end
    
    subgraph "Consumer"
        UI[UI Handler]
        LOG[Logger]
    end
    
    T1 -->|TokenClass| C1
    T1 -->|ToolCall| C2
    
    C1 --> T2
    C2 --> T3
    
    T2 -->|ProcessedToken| C3
    T3 -->|ExecutedTool| C4
    
    C3 --> UI
    C4 --> UI
    C4 --> LOG
    
    style C1 fill:#ffd,stroke:#333,stroke-width:2px
    style C2 fill:#ffd,stroke:#333,stroke-width:2px
    style C3 fill:#dfd,stroke:#333,stroke-width:2px
    style C4 fill:#dfd,stroke:#333,stroke-width:2px
```

## Memory Safety & Performance

The design ensures:

1. **No unbounded growth**: Buffer size is capped
2. **Single ownership**: Each token is processed exactly once
3. **Concurrent execution**: Tool calls don't block token display
4. **Zero-copy where possible**: References used until final emission
5. **Graceful degradation**: Errors convert to narrative output

## Example State Trace

For the input: `"I'll check the files.\n\n{\"tool\": \"ls\", \"params\": {}}\n\nHere they are:"`

```
Token: "I'll check the files.\n\n"
State: Narrative → Narrative
Action: Emit "I'll check the files.\n\n"
Buffer: ""

Token: "{\"tool\": \"ls\", "
State: Narrative → MaybeToolStart → InToolCall(1)
Action: Buffer
Buffer: "{\"tool\": \"ls\", "

Token: "\"params\": {}}"
State: InToolCall(1) → InToolCall(0)
Action: Parse tool, emit ToolCall, send to executor
Buffer: ""

Token: "\n\nHere they are:"
State: AfterToolCall → Narrative
Action: Emit "\n\nHere they are:"
Buffer: ""
```