// mcp-host: Streaming-aware MCP integration for chat applications

pub mod chat_integration;
pub mod streaming_interceptor;
pub mod instrumentation;

// Re-export main types
pub use chat_integration::{
    McpChatIntegration,
    ChatIntegrationConfig,
    StreamingMode,
    StreamingResponseHandle,
    ProcessedToken,
    ExecutedTool,
    InstrumentationConfig,
};

pub use streaming_interceptor::{
    StreamingInterceptor,
    TokenClass,
    STREAMING_FRIENDLY_TOOL_PROMPT,
};

pub use instrumentation::{
    McpEvent,
    McpEventType,
};

// Common types that might be needed
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub params: serde_json::Value,
}