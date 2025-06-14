use anyhow::Result;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct RetryStrategy {
    max_retries: u32,
    base_delay_ms: u64,
    temperature_reduction: f32,
}

impl RetryStrategy {
    pub fn new(max_retries: u32, base_delay_ms: u64, temperature_reduction: f32) -> Self {
        Self {
            max_retries,
            base_delay_ms,
            temperature_reduction,
        }
    }

    pub async fn execute_with_retry<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut(u32) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.max_retries {
            match operation(attempt).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        "Attempt {} failed: {}. Retrying...", 
                        attempt + 1, 
                        e
                    );
                    
                    last_error = Some(e);
                    
                    if attempt < self.max_retries {
                        let delay = self.calculate_delay(attempt);
                        sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }

    fn calculate_delay(&self, attempt: u32) -> u64 {
        // Exponential backoff with jitter
        let exponential_delay = self.base_delay_ms * 2u64.pow(attempt);
        let jitter = (rand::random::<f64>() * 0.3 * exponential_delay as f64) as u64;
        exponential_delay + jitter
    }

    pub fn calculate_temperature(&self, base_temperature: f32, attempt: u32) -> f32 {
        // Reduce temperature on each retry to get more deterministic results
        let reduction = self.temperature_reduction * attempt as f32;
        (base_temperature - reduction).max(0.0)
    }
}

#[derive(Debug)]
pub struct RetryContext {
    pub attempt: u32,
    pub temperature: f32,
    pub previous_errors: Vec<String>,
}

impl RetryContext {
    pub fn new(attempt: u32, temperature: f32) -> Self {
        Self {
            attempt,
            temperature,
            previous_errors: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.previous_errors.push(error);
    }

    pub fn build_retry_prompt(&self, original_prompt: &str) -> String {
        if self.previous_errors.is_empty() {
            return original_prompt.to_string();
        }

        let mut prompt = original_prompt.to_string();
        prompt.push_str("\n\nIMPORTANT: Previous attempts failed with these errors:\n");
        
        for (i, error) in self.previous_errors.iter().enumerate() {
            prompt.push_str(&format!("Attempt {}: {}\n", i + 1, error));
        }
        
        prompt.push_str("\nPlease correct these issues in your response. Ensure:\n");
        prompt.push_str("1. Tool calls use valid JSON format\n");
        prompt.push_str("2. Parameter names match the schema exactly\n");
        prompt.push_str("3. Required parameters are not missing\n");
        
        prompt
    }
}