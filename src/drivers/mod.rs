use crate::config::Service;
use anyhow::Result;

pub trait LLMService {
    fn new(service: &Service, model: &str, system_prompt: &str) -> Result<Self> where Self: Sized;
    fn complete(&self, prompt: &str) -> Result<(String, Option<String>)>;
    fn model(&self) -> &str;
    fn system_prompt(&self) -> &str;
    fn list_models(&self) -> Result<Vec<String>>;
}

pub mod openai;
pub mod ollama;
pub mod gemini;
pub mod anthropic;
