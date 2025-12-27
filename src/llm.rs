use std::error::Error;
use crate::config::{Config, ServiceConfig};
use crate::drivers::{LLMService, DriverConfig};
use crate::drivers::openai::OpenAI;
use crate::drivers::ollama::Ollama;

/// Structure representing a normalized response from an LLM.
pub struct LLMResponse {
    /// The main text of the response.
    pub response: String,
    /// The content of the chain of thought (if available and supported).
    pub thinking: Option<String>,
    /// The content of the system prompt used.
    pub system_prompt: String,
    /// The specific model that generated the response.
    pub model: String,
    /// The name of the service used.
    pub service: String,
}

/// Main client for handling requests to LLs.
/// Acts as a facade for different drivers.
pub struct Client;

impl Client {
    /// Makes a request to a configured LLM service.
    /// 
    /// # Arguments
    /// * `config` - The global configuration.
    /// * `service_config` - The specific configuration of the service to use.
    /// * `prompt` - The user's prompt.
    /// * `model_override` - Option to override the default model of the service.
    pub fn make_request(
        config: &Config,
        service_config: &ServiceConfig,
        prompt: &str,
        model_override: Option<&str>,
        system_prompt_override: Option<&str>,
    ) -> Result<LLMResponse, Box<dyn Error>> {
        // Determine model: override > service config > error
        let model = model_override.or(service_config.model.as_deref())
            .ok_or("Model is required for this service")?;
        
        // Resolve system prompt
        // Logic: 
        // 1. system_prompt_override (key) -> config.system_prompts[key]
        // 2. service.system_prompt (key) -> config.system_prompts[key]
        // 3. config.default_prompt (key) -> config.system_prompts[key]
        // We need the resolved content to pass it to the driver.
        
        let prompt_key = system_prompt_override
            .or(service_config.system_prompt.as_deref())
            .unwrap_or(&config.default_prompt);
            
        let system_prompt_content = config.system_prompts.get(prompt_key)
            .map(|s| s.as_str())
            .unwrap_or(prompt_key);

        // Configuration to initialize the driver
        let driver_config = DriverConfig {
            url: service_config.url.as_deref(),
            model,
            api_key: service_config.api_key.as_deref(),
            system_prompt: Some(system_prompt_content),
        };

        // Instantiate the corresponding driver based on service type
        let driver: Box<dyn LLMService> = match service_config.service_type {
            crate::config::ServiceType::Openai => Box::new(OpenAI::new(driver_config)?),
            crate::config::ServiceType::Ollama => Box::new(Ollama::new(driver_config)?),
        };

        // Execute the request
        let (response, thinking) = driver.complete(prompt)?;
        
        Ok(LLMResponse {
            response,
            thinking,
            system_prompt: system_prompt_content.to_string(),
            model: model.to_string(),
            service: service_config.name.to_string(),
        })
    }

    /// Lists models available on a remote service.
    /// Useful for exploring what models an Ollama server or OpenAI compatible endpoint offers.
    pub fn list_models(config: &Config, service_name: &str) -> Result<Vec<String>, Box<dyn Error>> {
         let service_config = config.services.iter()
            .find(|s| s.name == service_name)
            .ok_or_else(|| format!("Service '{}' not found", service_name))?;

        // We need a dummy configuration to initialize the driver.
        // Listing models doesn't require all fields, but new() does.
        
        let driver_config = DriverConfig {
            url: service_config.url.as_deref(),
            model: "placeholder",
            api_key: service_config.api_key.as_deref(),
            system_prompt: Some("placeholder"),
        };

        let driver: Box<dyn LLMService> = match service_config.service_type {
            crate::config::ServiceType::Openai => Box::new(OpenAI::new(driver_config)?),
            crate::config::ServiceType::Ollama => Box::new(Ollama::new(driver_config)?),
        };
        
        driver.list_models()
    }
}
