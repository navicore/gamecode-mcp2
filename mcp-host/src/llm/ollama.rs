use super::{LlmProvider, LlmRequest, LlmResponse, TokenUsage};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
    timeout: Duration,
}

impl OllamaProvider {
    pub fn new(model: String) -> Self {
        Self::with_config(model, "http://localhost:11434", Duration::from_secs(120))
    }

    pub fn with_config(model: String, base_url: &str, timeout: Duration) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            model,
            timeout,
        }
    }
}

#[derive(Serialize)]
struct OllamaGenerateRequest {
    model: String,
    prompt: String,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    stop: Vec<String>,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    done: bool,
    #[serde(default)]
    done_reason: Option<String>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
        let ollama_request = OllamaGenerateRequest {
            model: self.model.clone(),
            prompt: request.prompt,
            temperature: request.temperature,
            stream: false,
            num_predict: request.max_tokens,
            stop: request.stop_sequences,
        };

        let response = self.client
            .post(format!("{}/api/generate", self.base_url))
            .json(&ollama_request)
            .timeout(self.timeout)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Ollama API error: {}", error_text));
        }

        let ollama_response: OllamaGenerateResponse = response.json().await?;

        Ok(LlmResponse {
            text: ollama_response.response,
            finish_reason: ollama_response.done_reason,
            usage: match (ollama_response.prompt_eval_count, ollama_response.eval_count) {
                (Some(prompt_tokens), Some(completion_tokens)) => {
                    Some(TokenUsage {
                        prompt_tokens,
                        completion_tokens,
                        total_tokens: prompt_tokens + completion_tokens,
                    })
                }
                _ => None,
            },
        })
    }

    fn name(&self) -> &str {
        "ollama"
    }

    fn supports_tools(&self) -> bool {
        // Some Ollama models like llama3.1 support tools natively
        matches!(self.model.as_str(), 
            "llama3.1:70b" | "llama3.1:8b" | "llama3.1:latest" |
            "mistral:7b-instruct" | "qwen2.5-coder"
        )
    }
}