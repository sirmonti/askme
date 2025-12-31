use anyhow::{Result, bail, Context};
use serde_json::json;
use rust_i18n::t;
use crate::config::Service;
use super::LLMService;

pub struct OpenAIDriver {
    url: String,
    api_key: String,
    model: String,
    system_prompt: String,
}

impl LLMService for OpenAIDriver {
    fn new(service: &Service, model: &str, system_prompt: &str) -> Result<Self> {
         let url = service.url.as_deref().unwrap_or("https://api.openai.com");
         let api_key = service.api_key.as_deref().context(t!("api_key_required", service = "OpenAI"))?;
         
         if system_prompt.is_empty() {
              bail!("{}", t!("system_prompt_required", service = "OpenAI"));
         }
         
         Ok(Self {
             url: url.to_string(),
             api_key: api_key.to_string(),
             model: model.to_string(),
             system_prompt: system_prompt.to_string(),
         })
    }
    fn complete(&self, prompt: &str) -> Result<(String, Option<String>)> {
        let mut messages = Vec::new();
        messages.push(json!({"role": "system", "content": self.system_prompt}));
        messages.push(json!({"role": "user", "content": prompt}));

        let body = json!({
            "model": self.model,
            "messages": messages
        });

        // Ensure URL doesn't end with slash before appending
        let base_url = self.url.trim_end_matches('/');
        let endpoint = format!("{}/v1/chat/completions", base_url);

        let res = ureq::post(&endpoint)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(body);

        match res {
            Ok(response) => {
                 let json: serde_json::Value = response.into_json().context("Failed to parse OpenAI response")?;
                 let content = json["choices"][0]["message"]["content"]
                    .as_str()
                    .map(|s| s.to_string())
                    .context("Invalid response format from OpenAI")?;

                // Extract reasoning from <think> tags
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
                 match code {
                     401 => bail!("{}", t!("api_error_unauthorized")),
                     404 => bail!("{}", t!("api_error_not_found")),
                     _ => bail!("OpenAI API error: Status: {}, Body: {}", code, text),
                 }
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
        let base_url = self.url.trim_end_matches('/');
        let endpoint = format!("{}/v1/models", base_url);

        let res = ureq::get(&endpoint)
             .set("Authorization", &format!("Bearer {}", self.api_key))
             .call();

        match res {
            Ok(response) => {
                let json: serde_json::Value = response.into_json().context("Failed to parse OpenAI models response")?;
                let data = json["data"].as_array().context("Invalid response format from OpenAI (missing data array)")?;
                
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
                 bail!("OpenAI API error: Status: {}, Body: {}", code, text);
            },
            Err(e) => bail!("Request failed: {}", e),
        }
    }
}
