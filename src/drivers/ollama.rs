use super::{LLMService, DriverConfig};
use serde_json::{json, Value};
use std::error::Error;

/// Driver for Ollama services.
pub struct Ollama<'a> {
    url: String,
    model: &'a str,
    system_prompt: &'a str,
    api_key: Option<&'a str>,
}

impl<'a> LLMService<'a> for Ollama<'a> {
    fn new(config: DriverConfig<'a>) -> Result<Self, Box<dyn Error>> {
        // Default to localhost if url not provided
        let base_url = config.url.unwrap_or("http://localhost:11434").trim_end_matches('/');
        // We use the /api/chat endpoint for completions
        let url = format!("{}/api/chat", base_url);
        
        let system_prompt = config.system_prompt.ok_or("System prompt is required for Ollama driver")?;

        Ok(Ollama {
            url,
            model: config.model,
            system_prompt,
            api_key: config.api_key,
        })
    }

    fn complete(&self, prompt: &str) -> Result<(String, Option<String>), Box<dyn Error>> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": self.system_prompt},
                {"role": "user", "content": prompt}
            ],
            "stream": false
        });

        let mut request = ureq::post(&self.url);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", &format!("Bearer {}", key));
        }

        let response = request.send_json(&body).map_err(Box::new)?;

        if response.status().as_u16() >= 400 {
            let status = response.status();
            let text = response.into_body().read_to_string()?;
            return Err(format!("API request failed with status {}: {}", status, text).into());
        }

        let text = response.into_body().read_to_string()?;
        let json_resp: Value = serde_json::from_str(&text)?;

        // Extract reasoning content (thinking)
        // Some models/versions of Ollama put it in 'message.thinking' or top-level 'thinking'
        let thinking = if let Some(t) = json_resp["message"]["thinking"].as_str() {
             Some(t.to_string())
        } else if let Some(t) = json_resp["thinking"].as_str() {
             Some(t.to_string())
        } else {
             None
        };

        // Extract main content
        if let Some(content) = json_resp["message"]["content"].as_str() {
            Ok((content.to_string(), thinking))
        } else if let Some(content) = json_resp["response"].as_str() {
            // Older Ollama versions might use 'response'
            Ok((content.to_string(), thinking))
        } else {
            Err(format!("Unexpected Ollama response format: {}", text).into())
        }
    }
    
    fn list_models(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // Ollama list: GET /api/tags
        // Format: { "models": [ { "name": "llama2:latest", ... }, ... ] }
        
        // Base URL is currently stored with /api/chat appended.
        // We need to change /api/chat to /api/tags
        
        let tags_url = self.url.replace("/api/chat", "/api/tags");
        let response = ureq::get(&tags_url).call().map_err(Box::new)?;

        if response.status().as_u16() >= 400 {
             return Err(format!("Failed to list models: {}", response.status()).into());
        }

        let text = response.into_body().read_to_string()?;
        let json_resp: Value = serde_json::from_str(&text)?;
        
        // Extract model names
        let models = json_resp["models"].as_array().ok_or("Invalid response format: 'models' array missing")?;
        
        let names = models.iter()
            .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
            .collect();
            
        Ok(names)
    }
}
