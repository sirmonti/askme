use anyhow::{Result, bail, Context};
use serde_json::json;
use rust_i18n::t;
use crate::config::Service;
use super::LLMService;

pub struct GeminiDriver {
    // URL is hardcoded
    api_key: String,
    model: String,
    system_prompt: String,
}

impl LLMService for GeminiDriver {
    fn new(service: &Service, model: &str, system_prompt: &str) -> Result<Self> {
         let api_key = service.api_key.as_deref().context(t!("api_key_required", service = "Gemini"))?;
         
         Ok(Self {
             api_key: api_key.to_string(),
             model: model.to_string(),
             system_prompt: system_prompt.to_string(),
         })
    }

    fn complete(&self, prompt: &str) -> Result<(String, Option<String>)> {
        let base_url = "https://generativelanguage.googleapis.com/v1beta";
        let endpoint = format!("{}/models/{}:generateContent", base_url, self.model);

        let body = json!({
            "system_instruction": {
                "parts": [{ "text": self.system_prompt }]
            },
            "contents": [{
                "role": "user",
                "parts": [{ "text": prompt }]
            }]
        });

        let res = ureq::post(&endpoint)
            .set("x-goog-api-key", &self.api_key)
            .set("Content-Type", "application/json")
            .send_json(body);

        match res {
            Ok(response) => {
                let json: serde_json::Value = response.into_json().context("Failed to parse Gemini response")?;
                
                // candidates[0].content.parts[0].text
                let content = json["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .map(|s| s.to_string())
                    .context("Invalid response format from Gemini")?;
                
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
                 bail!("Gemini API error: Status: {}, Body: {}", code, text);
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
        let base_url = "https://generativelanguage.googleapis.com/v1beta";
        let endpoint = format!("{}/models", base_url);

        let res = ureq::get(&endpoint)
             .set("x-goog-api-key", &self.api_key)
             .call();

        match res {
            Ok(response) => {
                let json: serde_json::Value = response.into_json().context("Failed to parse Gemini models response")?;
                let models = json["models"].as_array().context("Invalid response format from Gemini (missing models array)")?;
                
                let mut names = Vec::new();
                for m in models {
                    if let Some(name) = m["name"].as_str() {
                        let clean_name = name.trim_start_matches("models/");
                        names.push(clean_name.to_string());
                    }
                }
                Ok(names)
            },
            Err(ureq::Error::Status(code, response)) => {
                 let text = response.into_string().unwrap_or_default();
                 bail!("Gemini API error: Status: {}, Body: {}", code, text);
            },
            Err(e) => bail!("Request failed: {}", e),
        }
    }
}
