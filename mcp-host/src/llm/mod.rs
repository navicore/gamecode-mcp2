pub mod ollama;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use ollama::OllamaProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub prompt: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub stop_sequences: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub text: String,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse>;
    
    fn name(&self) -> &str;
    
    fn supports_tools(&self) -> bool {
        false
    }
}