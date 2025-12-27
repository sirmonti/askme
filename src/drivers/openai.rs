use super::{LLMService, DriverConfig};
use serde_json::{json, Value};
use std::error::Error;

/// Driver for OpenAI-compatible services.
pub struct OpenAI<'a> {
    url: String, // Constructed dynamically, so String
    model: &'a str,
    api_key: Option<&'a str>,
    system_prompt: &'a str,
}

impl<'a> LLMService<'a> for OpenAI<'a> {
    fn new(config: DriverConfig<'a>) -> Result<Self, Box<dyn Error>> {
        let base_url = config.url.unwrap_or("https://api.openai.com").trim_end_matches('/');
        let url = format!("{}/v1/chat/completions", base_url);
        
        let system_prompt = config.system_prompt.ok_or("System prompt is required for OpenAI driver")?;

        Ok(OpenAI {
            url,
            model: config.model,
            api_key: config.api_key,
            system_prompt,
        })
    }

    fn complete(&self, prompt: &str) -> Result<(String, Option<String>), Box<dyn Error>> {
        // Construct the request body
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

        let full_content = json_resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| format!("Unexpected OpenAI response format: {}", text))?
            .to_string();

        // Extract <think> content if present
        // Some models output the reasoning chain within <think> tags.
        let start_tag = "<think>";
        let end_tag = "</think>";

        if let (Some(start_idx), Some(end_idx)) = (full_content.find(start_tag), full_content.find(end_tag)) {
            if start_idx < end_idx {
                let thinking = full_content[start_idx + start_tag.len()..end_idx].to_string();

                // Clean content by removing the thinking block
                let clean_content = format!("{}{}", 
                    &full_content[..start_idx], 
                    &full_content[end_idx + end_tag.len()..]
                ).trim().to_string();
                
                return Ok((clean_content, Some(thinking)));
            }
        }

        Ok((full_content, None))
    }
    
    fn list_models(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // OpenAI list: GET /v1/models
        // Format: { "data": [ { "id": "model-id", ... }, ... ] }

        let models_url = self.url.replace("/chat/completions", "/models"); // /v1/chat/completions -> /v1/models
        
        let mut request = ureq::get(&models_url);
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", &format!("Bearer {}", key));
        }

        let response = request.call().map_err(Box::new)?;
        if response.status().as_u16() >= 400 {
             return Err(format!("Failed to list models: {}", response.status()).into());
        }

        let text = response.into_body().read_to_string()?;
        let json_resp: Value = serde_json::from_str(&text)?;
        
        let models = json_resp["data"].as_array().ok_or("Invalid response format: 'data' array missing")?;
        
        let ids = models.iter()
            .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
            .collect();
            
        Ok(ids)
    }
}
