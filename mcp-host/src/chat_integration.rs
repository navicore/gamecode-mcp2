/// Complete chat integration API with streaming support
use crate::streaming_interceptor::{StreamingInterceptor, TokenClass, STREAMING_FRIENDLY_TOOL_PROMPT};
use anyhow::Result;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Main integration point for chat applications
pub struct McpChatIntegration {
    mcp_client: Arc<Mutex<gamecode_mcp_client::McpClient>>,
    config: ChatIntegrationConfig,
    instrumentation: Option<InstrumentationHandle>,
}

#[derive(Debug, Clone)]
pub struct ChatIntegrationConfig {
    /// How to handle streaming responses
    pub streaming_mode: StreamingMode,
    /// Whether to enhance system prompts with tool descriptions
    pub enhance_system_prompts: bool,
    /// Maximum tool execution rounds
    pub max_tool_rounds: usize,
    /// Debug instrumentation settings
    pub instrumentation: InstrumentationConfig,
}

#[derive(Debug, Clone)]
pub enum StreamingMode {
    /// Buffer tokens until we can classify them (recommended)
    SmartBuffering {
        max_buffer_chars: usize,
    },
    /// Pass through everything, let UI handle it
    Passthrough,
    /// Replace tool calls with placeholder text
    WithPlaceholders {
        placeholder_text: String,
    },
}

#[derive(Debug, Clone)]
pub struct InstrumentationConfig {
    /// Path to instrumentation log file
    pub log_path: Option<String>,
    /// Log all token classifications
    pub log_token_classifications: bool,
    /// Log tool execution timing
    pub log_performance_metrics: bool,
}

impl McpChatIntegration {
    pub async fn new(
        mcp_client: Arc<Mutex<gamecode_mcp_client::McpClient>>,
        config: ChatIntegrationConfig,
    ) -> Result<Self> {
        let instrumentation = if let Some(path) = &config.instrumentation.log_path {
            Some(InstrumentationHandle::new(path).await?)
        } else {
            None
        };

        Ok(Self {
            mcp_client,
            config,
            instrumentation,
        })
    }

    /// Enhance a system prompt with tool information
    pub async fn enhance_system_prompt(&self, original_prompt: &str) -> Result<String> {
        if !self.config.enhance_system_prompts {
            return Ok(original_prompt.to_string());
        }

        let tools = self.mcp_client.lock().await.list_tools().await?;
        if tools.is_empty() {
            return Ok(original_prompt.to_string());
        }

        let mut enhanced = original_prompt.to_string();
        
        // Add streaming-friendly tool instructions
        enhanced.push_str("\n\n## Available Tools\n\n");
        enhanced.push_str(STREAMING_FRIENDLY_TOOL_PROMPT);
        enhanced.push_str("\n\nYour available tools are:\n");
        
        for tool in &tools {
            enhanced.push_str(&format!(
                "- `{}`: {}\n  Parameters: {}\n",
                tool.name,
                tool.description,
                serde_json::to_string(&tool.input_schema).unwrap_or_default()
            ));
        }

        self.log_event(InstrumentationEvent::SystemPromptEnhanced {
            original_length: original_prompt.len(),
            enhanced_length: enhanced.len(),
            tools_count: tools.len(),
        }).await;

        Ok(enhanced)
    }

    /// Process a streaming response from an LLM
    pub async fn process_streaming_response<S>(
        &self,
        token_stream: S,
    ) -> Result<StreamingResponseHandle>
    where
        S: Stream<Item = String> + Send + 'static,
    {
        let (processed_tx, processed_rx) = mpsc::channel(100);
        let (tool_tx, tool_rx) = mpsc::channel(10);
        let mcp_client = self.mcp_client.clone();
        let config = self.config.clone();
        let instrumentation = self.instrumentation.clone();

        // Spawn processing task
        tokio::spawn(async move {
            Self::process_stream_internal(
                token_stream,
                processed_tx,
                tool_tx,
                mcp_client,
                config,
                instrumentation,
            ).await;
        });

        Ok(StreamingResponseHandle {
            token_stream: processed_rx,
            tool_stream: tool_rx,
        })
    }

    async fn process_stream_internal<S>(
        token_stream: S,
        processed_tx: mpsc::Sender<ProcessedToken>,
        tool_tx: mpsc::Sender<ExecutedTool>,
        mcp_client: Arc<Mutex<gamecode_mcp_client::McpClient>>,
        config: ChatIntegrationConfig,
        instrumentation: Option<InstrumentationHandle>,
    ) where
        S: Stream<Item = String> + Send,
    {
        match config.streaming_mode {
            StreamingMode::SmartBuffering { max_buffer_chars } => {
                let interceptor = StreamingInterceptor::new();
                let (mut token_rx, mut tool_rx) = interceptor.process_stream(token_stream).await;

                // Token processing task
                let processed_tx_clone = processed_tx.clone();
                let token_task = tokio::spawn(async move {
                    while let Some(token_class) = token_rx.recv().await {
                        let processed = match token_class {
                            TokenClass::Narrative(text) => ProcessedToken::Narrative(text),
                            TokenClass::ToolCall(json) => ProcessedToken::ToolCall(json),
                            TokenClass::Pending(text) => ProcessedToken::Buffered(text),
                        };
                        if processed_tx_clone.send(processed).await.is_err() {
                            break;
                        }
                    }
                });

                // Tool execution task
                let tool_task = tokio::spawn(async move {
                    while let Some(tool_call) = tool_rx.recv().await {
                        // Execute tool
                        let start = std::time::Instant::now();
                        let result = mcp_client
                            .lock()
                            .await
                            .call_tool(&tool_call.tool, tool_call.params.clone())
                            .await;
                        
                        let executed = ExecutedTool {
                            tool_name: tool_call.tool,
                            parameters: tool_call.params,
                            result: result.unwrap_or_else(|e| {
                                serde_json::json!({ "error": e.to_string() })
                            }),
                            execution_time_ms: start.elapsed().as_millis() as u64,
                        };

                        if let Some(ref inst) = instrumentation {
                            inst.log_event(InstrumentationEvent::ToolExecuted {
                                tool_name: executed.tool_name.clone(),
                                execution_time_ms: executed.execution_time_ms,
                                success: !executed.result.get("error").is_some(),
                            }).await;
                        }

                        if tool_tx.send(executed).await.is_err() {
                            break;
                        }
                    }
                });

                // Wait for both tasks
                let _ = tokio::join!(token_task, tool_task);
            }
            
            StreamingMode::Passthrough => {
                // Simple passthrough - no interception
                let mut stream = Box::pin(token_stream);
                while let Some(token) = stream.next().await {
                    let _ = processed_tx.send(ProcessedToken::Narrative(token)).await;
                }
            }
            
            StreamingMode::WithPlaceholders { ref placeholder_text } => {
                // Replace tool calls with placeholders
                let interceptor = StreamingInterceptor::new();
                let (mut token_rx, mut tool_rx) = interceptor.process_stream(token_stream).await;

                while let Some(token_class) = token_rx.recv().await {
                    let processed = match token_class {
                        TokenClass::Narrative(text) => ProcessedToken::Narrative(text),
                        TokenClass::ToolCall(_) => {
                            ProcessedToken::Narrative(placeholder_text.clone())
                        }
                        TokenClass::Pending(text) => ProcessedToken::Buffered(text),
                    };
                    if processed_tx.send(processed).await.is_err() {
                        break;
                    }
                }

                // Still execute tools in background
                while let Some(tool_call) = tool_rx.recv().await {
                    let result = mcp_client
                        .lock()
                        .await
                        .call_tool(&tool_call.tool, tool_call.params.clone())
                        .await;
                    
                    let executed = ExecutedTool {
                        tool_name: tool_call.tool.clone(),
                        parameters: tool_call.params,
                        result: result.unwrap_or_else(|e| {
                            serde_json::json!({ "error": e.to_string() })
                        }),
                        execution_time_ms: 0,
                    };
                    
                    let _ = tool_tx.send(executed).await;
                }
            }
        }
    }

    async fn log_event(&self, event: InstrumentationEvent) {
        if let Some(ref inst) = self.instrumentation {
            inst.log_event(event).await;
        }
    }
}

/// Handle for an active streaming response
pub struct StreamingResponseHandle {
    /// Stream of processed tokens
    pub token_stream: mpsc::Receiver<ProcessedToken>,
    /// Stream of executed tools
    pub tool_stream: mpsc::Receiver<ExecutedTool>,
}

#[derive(Debug, Clone)]
pub enum ProcessedToken {
    /// Safe narrative text to display
    Narrative(String),
    /// Hidden tool call
    ToolCall(String),
    /// Buffered text (not yet classified)
    Buffered(String),
}

#[derive(Debug, Clone)]
pub struct ExecutedTool {
    pub tool_name: String,
    pub parameters: Value,
    pub result: Value,
    pub execution_time_ms: u64,
}

/// Instrumentation handle
#[derive(Clone)]
struct InstrumentationHandle {
    tx: mpsc::UnboundedSender<InstrumentationEvent>,
}

impl InstrumentationHandle {
    async fn new(log_path: &str) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        
        // Spawn logger task
        let path = log_path.to_string();
        tokio::spawn(async move {
            if let Err(e) = Self::run_logger(rx, path).await {
                eprintln!("Instrumentation logger error: {}", e);
            }
        });

        Ok(Self { tx })
    }

    async fn log_event(&self, event: InstrumentationEvent) {
        let _ = self.tx.send(event);
    }

    async fn run_logger(
        mut rx: mpsc::UnboundedReceiver<InstrumentationEvent>,
        path: String,
    ) -> Result<()> {
        use tokio::fs::OpenOptions;
        use tokio::io::AsyncWriteExt;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        while let Some(event) = rx.recv().await {
            let json = serde_json::to_string(&event)?;
            file.write_all(json.as_bytes()).await?;
            file.write_all(b"\n").await?;
            file.flush().await?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "event_type")]
enum InstrumentationEvent {
    SystemPromptEnhanced {
        original_length: usize,
        enhanced_length: usize,
        tools_count: usize,
    },
    ToolExecuted {
        tool_name: String,
        execution_time_ms: u64,
        success: bool,
    },
    TokenClassified {
        classification: String,
        token_length: usize,
    },
}

impl Default for ChatIntegrationConfig {
    fn default() -> Self {
        Self {
            streaming_mode: StreamingMode::SmartBuffering {
                max_buffer_chars: 200,
            },
            enhance_system_prompts: true,
            max_tool_rounds: 3,
            instrumentation: InstrumentationConfig {
                log_path: None,
                log_token_classifications: false,
                log_performance_metrics: true,
            },
        }
    }
}