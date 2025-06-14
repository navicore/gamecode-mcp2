use anyhow::Result;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing Ollama directly...\n");

    let client = reqwest::Client::new();
    
    // Simple prompt
    let request = json!({
        "model": "qwen3:14b",
        "prompt": "Say 'Hello, I am working!'",
        "stream": false,
        "options": {
            "temperature": 0.7,
            "num_predict": 50
        }
    });

    println!("Sending request to Ollama...");
    let start = std::time::Instant::now();
    
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await;

    match response {
        Ok(resp) => {
            println!("Got response in {}s", start.elapsed().as_secs());
            let text = resp.text().await?;
            println!("Response: {}", text);
        }
        Err(e) => {
            println!("Error after {}s: {:?}", start.elapsed().as_secs(), e);
        }
    }

    Ok(())
}