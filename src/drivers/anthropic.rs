use anyhow::{Result, bail, Context};
use serde_json::json;
use rust_i18n::t;
use crate::config::Service;
use super::LLMService;

pub struct AnthropicDriver {
    // URL is hardcoded
    api_key: String,
    model: String,
    system_prompt: String,
}

impl LLMService for AnthropicDriver {
    fn new(service: &Service, model: &str, system_prompt: &str) -> Result<Self> {
         let api_key = service.api_key.as_deref().context(t!("api_key_required", service = "Anthropic"))?;
         
         Ok(Self {
             api_key: api_key.to_string(),
             model: model.to_string(),
             system_prompt: system_prompt.to_string(),
         })
    }

    fn complete(&self, prompt: &str) -> Result<(String, Option<String>)> {
        let base_url = "https://api.anthropic.com";
        let endpoint = format!("{}/v1/messages", base_url);

        let body = json!({
            "model": self.model,
            "system": self.system_prompt,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "max_tokens": 1024 
        });

        let res = ureq::post(&endpoint)
            .set("x-api-key", &self.api_key)
            .set("anthropic-version", "2023-06-01")
            .set("Content-Type", "application/json")
            .send_json(body);

        match res {
            Ok(response) => {
                let json: serde_json::Value = response.into_json().context("Failed to parse Anthropic response")?;
                
                let content = json["content"][0]["text"]
                    .as_str()
                    .map(|s| s.to_string())
                    .context("Invalid response format from Anthropic")?;
                
                 if let Some(start) = content.find("<think>") {
                     if let Some(end) = content.find("</think>") {
                          let thinking = content[start + 7..end].trim().to_string();
                          let response_part = content[end + 8..].trim().to_string();
                          return Ok((response_part, Some(thinking)));
                     }
                }

                Ok((content, None))
            },
            Err(ureq::Error::Status(code, response)) => {
                 let text = response.into_string().unwrap_or_default();
                 bail!("Anthropic API error: Status: {}, Body: {}", code, text);
            },
            Err(e) => bail!("Request failed: {}", e),
        }
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn list_models(&self) -> Result<Vec<String>> {
        let base_url = "https://api.anthropic.com";
        let endpoint = format!("{}/v1/models", base_url);

        let res = ureq::get(&endpoint)
             .set("x-api-key", &self.api_key)
             .set("anthropic-version", "2023-06-01")
             .call();

        match res {
            Ok(response) => {
                let json: serde_json::Value = response.into_json().context("Failed to parse Anthropic models response")?;
                let data = json["data"].as_array().context("Invalid response format from Anthropic (missing data array)")?;
                
                let mut ids = Vec::new();
                for d in data {
                    if let Some(id) = d["id"].as_str() {
                        ids.push(id.to_string());
                    }
                }
                Ok(ids)
            },
            Err(ureq::Error::Status(code, response)) => {
                 let text = response.into_string().unwrap_or_default();
                 bail!("Anthropic API error: Status: {}, Body: {}", code, text);
            },
            Err(e) => bail!("Request failed: {}", e),
        }
    }
}
