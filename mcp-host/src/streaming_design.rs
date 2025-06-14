// Exploring streaming + tool interception design options

use anyhow::Result;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Option 1: Buffering Stream Processor
/// Delays tokens until we know they're safe to show
pub struct BufferingStreamProcessor {
    buffer: String,
    tool_pattern: regex::Regex,
    look_ahead_chars: usize, // How many chars to buffer before deciding
}

impl BufferingStreamProcessor {
    pub async fn process_stream<S>(
        mut stream: S,
    ) -> Result<ProcessedStream>
    where
        S: Stream<Item = String> + Unpin,
    {
        let mut buffer = String::new();
        let mut safe_tokens = Vec::new();
        let mut tool_calls = Vec::new();
        
        while let Some(token) = stream.next().await {
            buffer.push_str(&token);
            
            // Check if we have a complete tool call
            if let Some(tool_call) = Self::extract_complete_tool_call(&buffer) {
                // Found tool call, don't emit these tokens
                tool_calls.push(tool_call);
                buffer.clear();
            } else if Self::definitely_not_tool_call(&buffer) {
                // Safe to emit
                safe_tokens.push(buffer.clone());
                buffer.clear();
            }
            // Otherwise keep buffering
        }
        
        Ok(ProcessedStream {
            safe_tokens,
            tool_calls,
            remaining_buffer: buffer,
        })
    }
}

/// Option 2: Dual-Stream Architecture
/// Run two separate streams - one for UI, one for tool detection
pub struct DualStreamProcessor {
    /// Stream for immediate UI display
    ui_stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    /// Separate analysis for tool detection
    tool_detector: ToolDetector,
}

impl DualStreamProcessor {
    pub fn new<S>(base_stream: S) -> (Self, mpsc::Receiver<ToolCall>)
    where
        S: Stream<Item = String> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(10);
        
        // Fork the stream
        let (ui_stream, tool_stream) = Self::fork_stream(base_stream);
        
        // Start tool detection in background
        tokio::spawn(async move {
            let mut detector = ToolDetector::new(tx);
            detector.process_stream(tool_stream).await;
        });
        
        (Self {
            ui_stream: Box::pin(ui_stream),
            tool_detector: ToolDetector::new(tx),
        }, rx)
    }
}

/// Option 3: Prompt Engineering Solution
/// Force the LLM to structure responses predictably
pub struct StructuredResponsePrompt {
    /// System prompt that enforces clear separation
    pub const SYSTEM_PROMPT: &'static str = r#"
When you need to use a tool, you MUST follow this exact format:

1. First, output ONLY the marker: [TOOL_CALL]
2. On the next line, output ONLY the JSON tool call
3. On the next line, output ONLY the marker: [/TOOL_CALL]
4. Then continue with your response

Example:
User: What files are in this directory?

[TOOL_CALL]
{"tool": "list_files", "params": {"path": "."}}
[/TOOL_CALL]

I'll list the files in the current directory for you.

CRITICAL: Never output any text on the same line as tool markers or JSON.
"#;
}

/// Option 4: Two-Phase Response
/// Ask LLM to declare intent first, then execute
pub struct TwoPhaseProcessor {
    phase: ProcessingPhase,
}

pub enum ProcessingPhase {
    /// First, ask LLM "Do you need to use tools for this?"
    IntentDetection,
    /// If yes, get tool calls without narrative
    ToolExecution,
    /// Finally, get the narrative response
    NarrativeGeneration,
}

impl TwoPhaseProcessor {
    pub async fn process_request(&mut self, user_message: &str) -> Result<Response> {
        match self.phase {
            ProcessingPhase::IntentDetection => {
                let intent = self.detect_tool_intent(user_message).await?;
                if intent.needs_tools {
                    self.phase = ProcessingPhase::ToolExecution;
                    // Continue to tool execution
                } else {
                    // Skip to narrative
                    self.phase = ProcessingPhase::NarrativeGeneration;
                }
            }
            ProcessingPhase::ToolExecution => {
                let tools = self.get_tool_calls().await?;
                // Execute tools...
                self.phase = ProcessingPhase::NarrativeGeneration;
            }
            ProcessingPhase::NarrativeGeneration => {
                // Stream the narrative response
            }
        }
    }
}

/// Option 5: Hybrid Approach
/// Combine multiple strategies based on context
pub struct HybridStreamProcessor {
    config: HybridConfig,
}

pub struct HybridConfig {
    /// Use markers for clear tool boundaries
    pub use_tool_markers: bool,
    /// Buffer size before streaming (in chars)
    pub buffer_threshold: usize,
    /// Regex patterns that definitely indicate tool usage
    pub tool_indicators: Vec<regex::Regex>,
    /// Patterns that definitely indicate narrative
    pub narrative_indicators: Vec<regex::Regex>,
}

impl HybridStreamProcessor {
    pub async fn process_stream<S>(
        &self,
        stream: S,
    ) -> Result<HybridStreamOutput>
    where
        S: Stream<Item = String> + Unpin,
    {
        let mut buffer = String::new();
        let mut emitted_chars = 0;
        let mut in_tool_section = false;
        
        // Smart buffering logic:
        // 1. If we see a tool marker, enter tool mode
        // 2. Buffer enough to detect JSON start
        // 3. Emit narrative tokens immediately if confident
        // 4. Use heuristics to minimize buffering delay
        
        // Implementation details...
    }
}

/// Option 6: Callback-Based Streaming
/// Let the chat app decide what to show
pub trait StreamHandler: Send + Sync {
    /// Called for each token - return true to show to user
    async fn should_display_token(&mut self, token: &str, context: &StreamContext) -> bool;
    
    /// Called when a complete tool call is detected
    async fn handle_tool_call(&mut self, tool_call: ToolCall) -> Result<()>;
    
    /// Called when tool results are ready
    async fn handle_tool_results(&mut self, results: Vec<ToolResult>) -> Result<()>;
}

pub struct CallbackStreamProcessor<H: StreamHandler> {
    handler: H,
}

impl<H: StreamHandler> CallbackStreamProcessor<H> {
    pub async fn process_stream<S>(
        mut self,
        stream: S,
    ) -> Result<()>
    where
        S: Stream<Item = String> + Unpin,
    {
        let mut context = StreamContext::new();
        
        while let Some(token) = stream.next().await {
            context.add_token(&token);
            
            // Let the app decide
            if self.handler.should_display_token(&token, &context).await {
                // App will handle display
            }
            
            // Check for complete tool calls
            if let Some(tool_call) = context.extract_tool_call() {
                self.handler.handle_tool_call(tool_call).await?;
            }
        }
        
        Ok(())
    }
}

// Example: How a chat app might implement streaming with tool support
pub async fn example_chat_with_streaming() -> Result<()> {
    // Option A: Use structured prompt to get predictable output
    let system_prompt = format!("{}\n\n{}", 
        "You are a helpful assistant.",
        StructuredResponsePrompt::SYSTEM_PROMPT
    );
    
    // Option B: Use smart buffering
    let processor = BufferingStreamProcessor::new();
    
    // Option C: Let the app control via callbacks
    struct MyStreamHandler {
        ui_tx: mpsc::Sender<String>,
    }
    
    impl StreamHandler for MyStreamHandler {
        async fn should_display_token(&mut self, token: &str, context: &StreamContext) -> bool {
            // Don't show tokens that look like JSON
            if context.in_json_block() {
                false
            } else {
                self.ui_tx.send(token.to_string()).await.ok();
                true
            }
        }
        
        async fn handle_tool_call(&mut self, tool_call: ToolCall) -> Result<()> {
            // Execute tool in background
            Ok(())
        }
        
        async fn handle_tool_results(&mut self, results: Vec<ToolResult>) -> Result<()> {
            // Maybe show a UI indicator that tools were used
            Ok(())
        }
    }
    
    Ok(())
}