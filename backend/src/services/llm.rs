//! LLM service for AI-powered analysis

use tracing::{info, warn};

pub struct LlmService {
    groq_api_key: String,
    client: reqwest::Client,
}

impl LlmService {
    pub fn new(groq_api_key: String) -> Self {
        Self {
            groq_api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub async fn generate(&self, prompt: &str) -> anyhow::Result<String> {
        if self.groq_api_key.is_empty() {
            return Err(anyhow::anyhow!("Groq API key not configured"));
        }

        let response = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.groq_api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": "llama-3.3-70b-versatile",
                "messages": [{"role": "user", "content": prompt}],
                "max_tokens": 500,
                "temperature": 0.3
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow::anyhow!("Groq API error: {}", error));
        }

        let result: serde_json::Value = response.json().await?;
        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;

        Ok(content.to_string())
    }
}
