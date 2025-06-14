/// Practical streaming interceptor for MCP tool calls
use anyhow::Result;
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

/// Token classifier for streaming responses
#[derive(Debug, Clone, PartialEq)]
pub enum TokenClass {
    /// Normal narrative text to show user
    Narrative(String),
    /// Part of a tool call - hide from user
    ToolCall(String),
    /// Ambiguous - need more context
    Pending(String),
}

/// Streaming interceptor that can detect and extract tool calls
pub struct StreamingInterceptor {
    /// Current buffer of pending tokens
    buffer: String,
    /// State machine for parsing
    state: ParserState,
    /// Pattern to detect tool call start
    tool_start_pattern: regex::Regex,
    /// Maximum buffer size before forcing a decision
    max_buffer_size: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum ParserState {
    /// Normal narrative text
    Narrative,
    /// Possibly entering a tool call
    MaybeToolStart,
    /// Inside a JSON tool call
    InToolCall { brace_depth: i32 },
    /// Tool call complete, looking for more
    AfterToolCall,
}

impl StreamingInterceptor {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            state: ParserState::Narrative,
            // Look for common patterns that indicate tool usage
            tool_start_pattern: regex::Regex::new(
                r#"(?x)
                (\{["'\s]*tool["'\s]*:|  # JSON start with "tool":
                \[TOOL|                   # Explicit marker
                <tool>|                   # XML-style marker
                ^\s*\{)                   # JSON at line start
                "#
            ).unwrap(),
            max_buffer_size: 200, // Characters to buffer before forcing output
        }
    }

    /// Process a stream of tokens, classifying each
    pub async fn process_stream<S>(
        mut self,
        stream: S,
    ) -> (mpsc::Receiver<TokenClass>, mpsc::Receiver<ToolCall>) 
    where
        S: Stream<Item = String> + Send + 'static + Unpin,
    {
        let (token_tx, token_rx) = mpsc::channel(100);
        let (tool_tx, tool_rx) = mpsc::channel(10);

        tokio::spawn(async move {
            let mut stream = Box::pin(stream);
            
            while let Some(token) = stream.next().await {
                self.process_token(token, &token_tx, &tool_tx).await;
            }
            
            // Flush any remaining buffer
            self.flush_buffer(&token_tx).await;
        });

        (token_rx, tool_rx)
    }

    async fn process_token(
        &mut self,
        token: String,
        token_tx: &mpsc::Sender<TokenClass>,
        tool_tx: &mpsc::Sender<ToolCall>,
    ) {
        self.buffer.push_str(&token);

        match &self.state {
            ParserState::Narrative => {
                // Check if this might be starting a tool call
                if self.tool_start_pattern.is_match(&self.buffer) {
                    self.state = ParserState::MaybeToolStart;
                } else if self.buffer.len() > self.max_buffer_size {
                    // Buffer too large, must be narrative
                    self.emit_narrative(token_tx).await;
                } else if self.buffer.ends_with('\n') || self.buffer.ends_with(". ") {
                    // Natural boundary, safe to emit
                    self.emit_narrative(token_tx).await;
                }
            }
            
            ParserState::MaybeToolStart => {
                // Try to parse as JSON
                if let Some(depth) = self.calculate_brace_depth() {
                    if depth > 0 {
                        self.state = ParserState::InToolCall { brace_depth: depth };
                    } else {
                        // False alarm, emit as narrative
                        self.emit_narrative(token_tx).await;
                        self.state = ParserState::Narrative;
                    }
                } else if self.buffer.len() > 50 {
                    // Too long without valid JSON, probably narrative
                    self.emit_narrative(token_tx).await;
                    self.state = ParserState::Narrative;
                }
            }
            
            ParserState::InToolCall { brace_depth } => {
                // Track brace depth
                let new_depth = self.calculate_brace_depth().unwrap_or(*brace_depth);
                
                if new_depth == 0 {
                    // Complete JSON object
                    if let Some(tool_call) = self.try_parse_tool_call() {
                        let _ = tool_tx.send(tool_call).await;
                        let _ = token_tx.send(TokenClass::ToolCall(self.buffer.clone())).await;
                        self.buffer.clear();
                        self.state = ParserState::AfterToolCall;
                    } else {
                        // Invalid JSON, treat as narrative
                        self.emit_narrative(token_tx).await;
                        self.state = ParserState::Narrative;
                    }
                } else {
                    self.state = ParserState::InToolCall { brace_depth: new_depth };
                }
            }
            
            ParserState::AfterToolCall => {
                // Look for another tool call or resume narrative
                if self.buffer.trim().is_empty() {
                    // Skip whitespace
                } else if self.tool_start_pattern.is_match(&self.buffer) {
                    self.state = ParserState::MaybeToolStart;
                } else {
                    self.state = ParserState::Narrative;
                }
            }
        }
    }

    async fn emit_narrative(&mut self, tx: &mpsc::Sender<TokenClass>) {
        if !self.buffer.is_empty() {
            let _ = tx.send(TokenClass::Narrative(self.buffer.clone())).await;
            self.buffer.clear();
        }
    }

    async fn flush_buffer(&mut self, tx: &mpsc::Sender<TokenClass>) {
        if !self.buffer.is_empty() {
            let class = match self.state {
                ParserState::InToolCall { .. } => TokenClass::ToolCall(self.buffer.clone()),
                _ => TokenClass::Narrative(self.buffer.clone()),
            };
            let _ = tx.send(class).await;
            self.buffer.clear();
        }
    }

    fn calculate_brace_depth(&self) -> Option<i32> {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape = false;
        
        for ch in self.buffer.chars() {
            if escape {
                escape = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => depth -= 1,
                _ => {}
            }
            
            if depth < 0 {
                return None; // Invalid JSON
            }
        }
        
        Some(depth)
    }

    fn try_parse_tool_call(&self) -> Option<ToolCall> {
        // Try to parse the buffer as a tool call
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.buffer) {
            if let Some(tool) = value.get("tool").and_then(|v| v.as_str()) {
                if let Some(params) = value.get("params") {
                    return Some(ToolCall {
                        tool: tool.to_string(),
                        params: params.clone(),
                    });
                }
            }
        }
        None
    }
}

/// Enhanced prompt instructions for cleaner streaming
pub const STREAMING_FRIENDLY_TOOL_PROMPT: &str = r#"
When you need to use a tool:
1. Output the tool call as a complete JSON object on its own line
2. Do not include any explanation before or mixed with the JSON
3. After the tool executes, you can explain what you did

Good example:
{"tool": "list_files", "params": {"path": "."}}

Bad example:
Let me list the files {"tool": "list_files", "params": {"path": "."}} for you.

This ensures clean streaming to the user.
"#;

// Example usage showing how a chat app would integrate
pub async fn example_streaming_chat() -> Result<()> {
    use futures::stream;
    
    // Simulate LLM streaming response
    let tokens = vec![
        "I'll ".to_string(),
        "help ".to_string(),
        "you ".to_string(),
        "list ".to_string(),
        "the ".to_string(),
        "files.\n\n".to_string(),
        "{\"tool".to_string(),
        "\": \"list_files\", ".to_string(),
        "\"params\": ".to_string(),
        "{\"path\": \".\"".to_string(),
        "}}".to_string(),
        "\n\nLet me ".to_string(),
        "check what's ".to_string(),
        "there.".to_string(),
    ];
    
    let stream = stream::iter(tokens);
    let interceptor = StreamingInterceptor::new();
    let (mut token_rx, mut tool_rx) = interceptor.process_stream(stream).await;
    
    // UI handler
    tokio::spawn(async move {
        while let Some(token_class) = token_rx.recv().await {
            match token_class {
                TokenClass::Narrative(text) => {
                    print!("{}", text); // Show to user immediately
                }
                TokenClass::ToolCall(_) => {
                    // Hidden from user
                }
                TokenClass::Pending(_) => {
                    // Buffered, not shown yet
                }
            }
        }
    });
    
    // Tool handler
    while let Some(tool_call) = tool_rx.recv().await {
        println!("\n[Executing tool: {}]", tool_call.tool);
        // Execute tool and handle results
    }
    
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub params: serde_json::Value,
}