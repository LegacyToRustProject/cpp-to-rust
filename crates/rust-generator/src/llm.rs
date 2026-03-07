use anyhow::{bail, Result};
use async_trait::async_trait;
use serde_json::json;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, system: &str, user: &str) -> Result<String>;
    fn name(&self) -> &str;
}

pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY environment variable not set"))?;
        Ok(Self::new(api_key, "claude-sonnet-4-20250514".to_string()))
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let body = json!({
            "model": self.model,
            "max_tokens": 8192,
            "system": system,
            "messages": [
                {
                    "role": "user",
                    "content": user
                }
            ]
        });

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            bail!("Claude API error ({}): {}", status, response_text);
        }

        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        let content = response_json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .ok_or_else(|| anyhow::anyhow!("No text content in Claude response"))?;

        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "claude"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_provider_creation() {
        let provider = ClaudeProvider::new("test-key".to_string(), "test-model".to_string());
        assert_eq!(provider.name(), "claude");
        assert_eq!(provider.model, "test-model");
    }
}
