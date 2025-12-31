use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::Read, path::{Path, PathBuf}};
use anyhow::{Context, Result, bail};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub default_service: String,
    pub default_prompt: String,
    pub system_prompts: HashMap<String, String>,
    pub services: HashMap<String, Service>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Service {
    pub url: Option<String>,
    pub class: String, // "openai" or "ollama"
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub system_prompt: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct PartialConfig {
    pub default_service: Option<String>,
    pub default_prompt: Option<String>,
    pub system_prompts: Option<HashMap<String, String>>,
    pub services: Option<HashMap<String, Service>>,
}

impl PartialConfig {
    fn merge(mut self, other: PartialConfig) -> Self {
        if let Some(ds) = other.default_service {
            self.default_service = Some(ds);
        }
        if let Some(dp) = other.default_prompt {
            self.default_prompt = Some(dp);
        }
        
        if let Some(other_prompts) = other.system_prompts {
             let mut current = self.system_prompts.unwrap_or_default();
             current.extend(other_prompts);
             self.system_prompts = Some(current);
        }

        if let Some(other_services) = other.services {
             let mut current = self.services.unwrap_or_default();
             current.extend(other_services);
             self.services = Some(current);
        }
        
        self
    }

    fn try_into_config(self) -> Result<Config> {
        let default_service = self.default_service.context("Missing 'default_service' in configuration")?;
        let default_prompt = self.default_prompt.context("Missing 'default_prompt' in configuration")?;
        let system_prompts = self.system_prompts.unwrap_or_default();
        let services = self.services.unwrap_or_default();

        Ok(Config {
            default_service,
            default_prompt,
            system_prompts,
            services,
        })
    }
}

impl Config {
    pub fn load(explicit_path: Option<String>) -> Result<Self> {
        let mut final_partial = PartialConfig::default();
        let mut loaded_any = false;

        // 1. Load Global Config
        if let Some(global_path) = Self::get_global_config_path() {
            if global_path.exists() {
                 if let Ok(partial) = Self::load_partial(&global_path) {
                     final_partial = final_partial.merge(partial);
                     loaded_any = true;
                     #[cfg(debug_assertions)]
                     eprintln!("Loaded global config: {:?}", global_path);
                 }
            }
        }

        // 2. Determine Local Config Path
        let local_path_buf;
        let local_path = if let Some(path) = explicit_path {
            local_path_buf = PathBuf::from(path);
            Some(local_path_buf.as_path())
        } else {
            // Try current directory
            let cwd_config = Path::new("askme.yml");
            if cwd_config.exists() {
                 local_path_buf = cwd_config.to_path_buf();
                 Some(local_path_buf.as_path())
            } else {
                 // Try ~/.config/askme.yml
                 if let Some(config_dir) = dirs::config_dir() {
                     local_path_buf = config_dir.join("askme.yml");
                     if local_path_buf.exists() {
                         Some(local_path_buf.as_path())
                     } else {
                         None
                     }
                 } else {
                     None
                 }
            }
        };

        #[cfg(debug_assertions)]
        eprintln!("Loaded local config: {:?}", local_path);

        if let Some(path) = local_path {
             let partial = Self::load_partial(path).context(format!("Failed to load config at {:?}", path))?;
             final_partial = final_partial.merge(partial);
        } else if !loaded_any {
             // If no explicit path gave and we didn't find any default config files
             // And we also didn't load global.
             // Wait, user requirement: "Si no existe ningún fichero de configuración, ni local ni global, el programa lanzará un mensaje de error."
             bail!("No configuration file found. Checked ./askme.yml, ~/.config/askme.yml, and global locations");
        }

        final_partial.try_into_config()
    }

    #[inline]
    fn get_global_config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            // %ProgramData%\askme\askme.yml
            std::env::var("ProgramData").ok().map(|pd| PathBuf::from(pd).join("askme").join("askme.yml"))
        }

        #[cfg(target_os = "macos")]
        {
            // /Library/Application Support/askme/askme.yml
             Some(PathBuf::from("/Library/Application Support/askme/askme.yml"))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            // /etc/askme.yml
            Some(PathBuf::from("/etc/askme.yml"))
        }
    }

    fn load_partial(path: &Path) -> Result<PartialConfig> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let partial: PartialConfig = serde_yaml::from_str(&contents)?;
        Ok(partial)
    }
}
