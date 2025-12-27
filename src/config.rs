use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::borrow::Cow;

/// Main configuration structure.
/// Contains all configured services and prompts after processing and validation.
#[derive(Debug)]
pub struct Config {
    /// List of configured services (OpenAI, Ollama, etc.).
    pub services: Vec<ServiceConfig>,
    /// Name of the default service.
    pub default_service: Cow<'static, str>,
    /// Key of the default system prompt.
    pub default_prompt: Cow<'static, str>,
    /// Map of available system prompts (key -> content).
    pub system_prompts: HashMap<String, String>,
}

/// Individual configuration for an LLM service.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServiceConfig {
    /// Unique identifier for the service.
    pub name: Cow<'static, str>,
    /// Optional description of the service.
    pub description: Option<Cow<'static, str>>,
    /// Base URL of the service API.
    pub url: Option<Cow<'static, str>>,
    /// Service type (mapped from "class" field in YAML).
    #[serde(rename = "class")]
    pub service_type: ServiceType,
    /// Model name to use by default for this service.
    pub model: Option<Cow<'static, str>>,
    /// API key (if required).
    pub api_key: Option<Cow<'static, str>>,
    /// System prompt key to use by default for this service.
    pub system_prompt: Option<Cow<'static, str>>,
}

/// Enumeration of supported service types.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Openai,
    Ollama,
}

/// Intermediate structure for loading from YAML.
/// Allows optional fields to support partial loading and merging.
#[derive(Debug, Deserialize, Default)]
pub struct RawConfig {
    pub services: Option<Vec<ServiceConfig>>,
    pub default_service: Option<Cow<'static, str>>,
    pub default_prompt: Option<Cow<'static, str>>,
    pub system_prompts: Option<HashMap<String, String>>,
}

/// Determines the path of the global configuration file based on the OS.
#[inline]
fn get_global_config_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(program_data) = std::env::var("ProgramData") {
            let mut path = PathBuf::from(program_data);
            path.push("askme");
            path.push("askme.yml");
            return path;
        }
        PathBuf::from("C:\\ProgramData\\askme\\askme.yml")
    }

    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Library/Application Support/askme/askme.yml")
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        PathBuf::from("/etc/askme.yml")
    }
}

impl Config {
    /// Loads the complete configuration.
    /// 1. Attempts to load global configuration.
    /// 2. Determines and loads local configuration (or the one explicitly specified).
    /// 3. Merges both configurations (local config takes precedence).
    pub fn load(explicit_path: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Load Global Config (Optional)
        let global_path = get_global_config_path();
        if cfg!(debug_assertions) {
            eprintln!("Global path: {:?}", global_path);
        }
        let mut final_config = RawConfig::load_from_file(&global_path).unwrap_or_default();

        // 2. Determine Local Config Path
        let local_path = if let Some(p) = explicit_path {
             // Explicit path must exist
             PathBuf::from(p)
        } else if Path::new("askme.yml").exists() {
             PathBuf::from("askme.yml")
        } else {
             // Search in user's config directory (e.g. ~/.config/askme.yml)
             dirs::config_dir().map(|mut p| {
                p.push("askme.yml");
                p
             }).unwrap_or_else(|| PathBuf::from("askme.yml"))
        };

        if cfg!(debug_assertions) {
            eprintln!("Local path: {:?}", local_path);
        }

        // 3. Load Local Config (Optional/Override)
        if local_path.exists() {
             match RawConfig::load_from_file(&local_path) {
                Ok(local_config) => final_config.merge(local_config),
                Err(e) => {
                    return Err(format!("Error loading local config {:?}: {}", local_path, e).into());
                }
             }
        }

        // 4. Validate and Convert to final struct
        final_config.try_into_config().map_err(|e| e.into())
    }
}

impl RawConfig {
    /// Loads a partial configuration from a YAML file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: RawConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }

    /// Merges this configuration with another ('other' takes precedence).
    pub fn merge(&mut self, other: RawConfig) {
        if let Some(ds) = other.default_service {
            self.default_service = Some(ds);
        }
        if let Some(dp) = other.default_prompt {
            self.default_prompt = Some(dp);
        }
        
        if let Some(other_prompts) = other.system_prompts {
             let prompts = self.system_prompts.get_or_insert_with(HashMap::new);
             prompts.extend(other_prompts);
        }

        if let Some(other_services) = other.services {
             // Merge services by name (replace if exists)
             let current_services = self.services.take().unwrap_or_default();
             let mut service_map: HashMap<Cow<'static, str>, ServiceConfig> = HashMap::new();
             
             for s in current_services {
                 service_map.insert(s.name.clone(), s);
             }
             for s in other_services {
                 service_map.insert(s.name.clone(), s);
             }
             
             self.services = Some(service_map.into_values().collect());
        }
    }

    /// Attempts to convert the partial configuration (RawConfig) into final Config.
    /// Fails if mandatory fields like default_service or default_prompt are missing.
    pub fn try_into_config(self) -> Result<Config, String> {
        Ok(Config {
            services: self.services.unwrap_or_default(),
            default_service: self.default_service.ok_or("Missing default_service")?,
            default_prompt: self.default_prompt.ok_or("Missing default_prompt")?,
            // System prompts are merged/collected
            system_prompts: self.system_prompts.unwrap_or_default(),
        })
    }
}
