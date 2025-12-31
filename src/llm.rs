use crate::config::Config;
use crate::drivers::{LLMService, openai::OpenAIDriver, ollama::OllamaDriver};
use anyhow::{Result, bail, Context};
use rust_i18n::t;

pub struct Client<'a> {
    #[allow(dead_code)]
    service_name: String,
    driver: Box<dyn LLMService + 'a>,
}

impl<'a> Client<'a> {
    pub fn new(service_name: Option<&str>, config: &'a Config, model_override: Option<&'a String>, sys_prompt_override: Option<&'a str>) -> Result<Self> {
         // Determine service name
         let service_name = service_name
            .unwrap_or(&config.default_service);

        // Get service config
        // Get service config
        let service_config = config.services.get(service_name)
            .context(t!("service_not_found", name = service_name))?;

        // Resolve Model
        let model = model_override.map(|s| s.as_str()).or(service_config.model.as_deref());
        
        // Resolve System Prompt
        let system_prompt_text = if let Some(sys_override) = sys_prompt_override {
             if let Some(text) = config.system_prompts.get(sys_override) {
                 Some(text.as_str())
             } else {
                 Some(sys_override)
             }
        } else {
            // Determine reference: use service's system_prompt or config's default_prompt
            let sys_ref = service_config.system_prompt.as_ref().unwrap_or(&config.default_prompt);
            
            // Check if sys_ref is a key in system_prompts
             if let Some(text) = config.system_prompts.get(sys_ref) {
                 Some(text.as_str())
             } else {
                 // Fallback: If not found in map, treat as raw text (backward compatibility)
                 Some(sys_ref.as_str())
             }
        };

        // Instantiate driver
        let driver: Box<dyn LLMService + 'a> = match service_config.class.as_str() {
            "openai" => {
                 let model = model.context(t!("model_required", service = "OpenAI"))?;
                 let sys_prompt = system_prompt_text.context(t!("system_prompt_required", service = "OpenAI"))?;
                 
                 Box::new(OpenAIDriver::new(service_config, model, sys_prompt)?)
            },
            "ollama" => {
                 let model = model.context(t!("model_required", service = "Ollama"))?;
                 let sys_prompt = system_prompt_text.context(t!("system_prompt_required", service = "Ollama"))?;
                 
                 Box::new(OllamaDriver::new(service_config, model, sys_prompt)?)
            },
            "gemini" => {
                 let model = model.context(t!("model_required", service = "Gemini"))?;
                 let sys_prompt = system_prompt_text.context(t!("system_prompt_required", service = "Gemini"))?;
                 
                 Box::new(crate::drivers::gemini::GeminiDriver::new(service_config, model, sys_prompt)?)
            },
            "anthropic" => {
                 let model = model.context(t!("model_required", service = "Anthropic"))?;
                 let sys_prompt = system_prompt_text.context(t!("system_prompt_required", service = "Anthropic"))?;
                 
                 Box::new(crate::drivers::anthropic::AnthropicDriver::new(service_config, model, sys_prompt)?)
            },
            _ => bail!("{}", t!("unknown_service_class_detailed", class = service_config.class, valid = "openai, ollama, gemini, anthropic")),
        };

        Ok(Self {
            service_name: service_name.to_string(),
            driver,
        })
    }
    pub fn complete(&self, prompt: &str) -> Result<(String, Option<String>)> {
        self.driver.complete(prompt)
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    pub fn model(&self) -> &str {
        self.driver.model()
    }

    pub fn system_prompt(&self) -> &str {
        self.driver.system_prompt()
    }

    pub fn list_models(&self) -> Result<Vec<String>> {
        self.driver.list_models()
    }
}
