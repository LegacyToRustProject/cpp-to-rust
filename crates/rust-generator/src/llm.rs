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
    base_url: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let base_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| "https://api.anthropic.com".to_string());
        Self {
            api_key,
            model,
            base_url,
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

        let url = format!("{}/v1/messages", self.base_url);
        let response = self
            .client
            .post(&url)
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

/// Google Gemini LLM provider.
pub struct GeminiProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("GOOGLE_API_KEY")
            .map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY environment variable not set"))?;
        Ok(Self::new(api_key, "gemini-2.0-flash".to_string()))
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn generate(&self, system: &str, user: &str) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = json!({
            "system_instruction": {
                "parts": [{ "text": system }]
            },
            "contents": [
                {
                    "role": "user",
                    "parts": [{ "text": user }]
                }
            ],
            "generationConfig": {
                "maxOutputTokens": 8192,
                "temperature": 0.2
            }
        });

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            bail!("Gemini API error ({}): {}", status, response_text);
        }

        let response_json: serde_json::Value = serde_json::from_str(&response_text)?;
        let content = response_json["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["content"]["parts"].as_array())
            .and_then(|parts| parts.first())
            .and_then(|p| p["text"].as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("No text content in Gemini response: {}", response_text)
            })?;

        Ok(content.to_string())
    }

    fn name(&self) -> &str {
        "gemini"
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

    #[test]
    fn test_gemini_provider_creation() {
        let provider = GeminiProvider::new("test-key".to_string(), "gemini-2.0-flash".to_string());
        assert_eq!(provider.name(), "gemini");
        assert_eq!(provider.model, "gemini-2.0-flash");
    }
}
