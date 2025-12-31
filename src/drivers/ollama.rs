use anyhow::{Result, bail, Context};
use serde_json::json;
use rust_i18n::t;
use crate::config::Service;
use super::LLMService;

pub struct OllamaDriver {
    url: String,
    model: String,
    system_prompt: String,
    api_key: Option<String>,
}

impl LLMService for OllamaDriver {
    fn new(service: &Service, model: &str, system_prompt: &str) -> Result<Self> {
         let url = service.url.as_deref().unwrap_or("http://localhost:11434");
         let api_key = service.api_key.as_deref();
         
         if system_prompt.is_empty() {
              bail!("{}", t!("system_prompt_required", service = "Ollama"));
         }
         
         Ok(Self {
             url: url.to_string(),
             model: model.to_string(),
             system_prompt: system_prompt.to_string(),
             api_key: api_key.map(|s| s.to_string()),
         })
    }
    fn complete(&self, prompt: &str) -> Result<(String, Option<String>)> {
        let mut messages = Vec::new();
        messages.push(json!({"role": "system", "content": self.system_prompt}));
        messages.push(json!({"role": "user", "content": prompt}));
        
        let body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false
        });

        let base_url = self.url.trim_end_matches('/');
        let endpoint = format!("{}/api/chat", base_url);

        let mut req = ureq::post(&endpoint);
        
        if let Some(key) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {}", key));
        }

        let res = req.send_json(body);

        match res {
             Ok(response) => {
                 let json: serde_json::Value = response.into_json().context("Failed to parse Ollama response")?;
                 let response_text = json["message"]["content"]
                    .as_str()
                    .map(|s| s.to_string())
                    .context("Invalid response format from Ollama")?;
                 
                 // Extract thinking if present
                 // Note: Ollama might return it in a different way depending on model or custom fields?
                 // User said: "chain of reasoning comes in the 'thinking' field of the response"
                 // This implies it's a top-level field or inside the message object?
                 // Usually for chat API it's inside message object? Or maybe for /api/generate it is separate?
                 // User said "thinking" field. Let's assume top level or message level.
                 // Let's check both for robustness.
                 let thinking = json.get("thinking")
                     .or_else(|| json["message"].get("thinking"))
                     .and_then(|t| t.as_str())
                     .map(|s| s.to_string());
                     
                 Ok((response_text, thinking))
            },
            Err(ureq::Error::Status(code, response)) => {
                 let text = response.into_string().unwrap_or_default();
                 match code {
                     404 => bail!("{}", t!("api_error_not_found")),
                     _ => bail!("Ollama API error: Status: {}, Body: {}", code, text),
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
        let endpoint = format!("{}/api/tags", base_url);

        let mut req = ureq::get(&endpoint);
        if let Some(key) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {}", key));
        }

        let res = req.call();

        match res {
            Ok(response) => {
                let json: serde_json::Value = response.into_json().context("Failed to parse Ollama tags response")?;
                let models = json["models"].as_array().context("Invalid response format from Ollama (missing models array)")?;
                
                let mut names = Vec::new();
                for m in models {
                    if let Some(name) = m["name"].as_str() {
                        names.push(name.to_string());
                    }
                }
                Ok(names)
            },
            Err(ureq::Error::Status(code, response)) => {
                 let text = response.into_string().unwrap_or_default();
                 bail!("Ollama API error: Status: {}, Body: {}", code, text);
            },
            Err(e) => bail!("Request failed: {}", e),
        }
    }
}
