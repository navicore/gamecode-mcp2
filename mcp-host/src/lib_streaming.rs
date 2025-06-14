// New streaming-aware API for mcp-host
pub mod chat_integration;
pub mod streaming_interceptor;
pub mod instrumentation;

// Keep existing modules for backward compatibility
pub mod conversation;
pub mod llm;
pub mod prompts;
pub mod retry;

// Re-export main integration types
pub use chat_integration::{
    McpChatIntegration,
    ChatIntegrationConfig,
    StreamingMode,
    StreamingResponseHandle,
    ProcessedToken,
    ExecutedTool,
};

pub use streaming_interceptor::{
    StreamingInterceptor,
    TokenClass,
    STREAMING_FRIENDLY_TOOL_PROMPT,
};

// Keep existing exports for compatibility
pub use conversation::ConversationManager;
pub use llm::{LlmProvider, LlmRequest, LlmResponse, OllamaProvider};
pub use prompts::PromptTemplate;
pub use retry::RetryStrategy;

// Re-export old types but mark as deprecated
#[deprecated(note = "Use McpChatIntegration instead")]
pub use crate::old_api::McpHost;

#[deprecated(note = "Use ChatIntegrationConfig instead")]
pub use crate::old_api::McpHostConfig;

mod old_api {
    // The existing implementation would go here
    pub struct McpHost;
    pub struct McpHostConfig;
}