pub mod openai;
pub mod ollama;


use std::error::Error;

/// Shared configuration to initialize any LLM driver.
/// Contains the minimum necessary parameters to establish a connection.
pub struct DriverConfig<'a> {
    /// URL of the API endpoint.
    pub url: Option<&'a str>,
    /// Model to use.
    pub model: &'a str,
    /// API key for authentication.
    pub api_key: Option<&'a str>,
    /// Initial system prompt.
    pub system_prompt: Option<&'a str>,
}

/// Common trait that all LLM services must implement.
/// Defines the standard interface to interact with different providers.
pub trait LLMService<'a> {
    /// Creates a new instance of the service with the given configuration.
    fn new(config: DriverConfig<'a>) -> Result<Self, Box<dyn Error>> where Self: Sized;
    
    /// Performs a chat completion request.
    /// Returns a tuple: (response content, optional "thinking" content).
    fn complete(&self, prompt: &str) -> Result<(String, Option<String>), Box<dyn Error>>;
    
    /// Lists available models in the service.
    fn list_models(&self) -> Result<Vec<String>, Box<dyn Error>>;
}
