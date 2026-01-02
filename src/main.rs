mod config;
mod llm;
mod drivers;

use clap::{Parser, CommandFactory, FromArgMatches};
use config::Config;
use anyhow::{Result, Context};
use std::process;
use std::io::Read;
use regex::Regex;
#[macro_use] extern crate rust_i18n;

i18n!("locales");

fn set_system_locale() {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "en".to_string());
    let lang_code = locale.split(|c| c == '-' || c == '_').next().unwrap_or("en");
    #[cfg(debug_assertions)]
    eprintln!("System locale: {}\nLang code: {}", locale, lang_code);

    let supported = ["en", "es", "fr", "it", "de", "zh"];
    if supported.contains(&lang_code) {
        rust_i18n::set_locale(lang_code);
    } else {
        rust_i18n::set_locale("en");
    }
}

#[derive(Parser, Debug)]
#[command(
    version, 
    about, 
    long_about = None,
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Args {
    /// The user input/prompt to send to the LLM
    #[arg(index = 1, value_name = "PROMPT")]
    input: Option<String>,

    /// Service to use
    #[arg(short = 's', long)]
    service: Option<String>,

    /// Model to use
    #[arg(short = 'm', long)]
    model: Option<String>,

    /// System prompt (key in config or literal text)
    #[arg(short = 'p', long = "prompt")]
    prompt_arg: Option<String>,

    /// Show full content of a specific system prompt
    #[arg(long)]
    sprompt: Option<String>,

    /// List configured services or system prompts
    #[arg(short = 'l', long, num_args(0..=1), default_missing_value = "services")]
    list: Option<String>,

    /// Print help
    #[arg(short, long, action = clap::ArgAction::Help)]
    help: Option<bool>,

    /// Print version
    #[arg(short = 'V', long, action = clap::ArgAction::Version)]
    version: Option<bool>,

    /// Do not show reasoning chain
    #[arg(short = 'n', long)]
    nothink: bool,

    /// Output raw JSON
    #[arg(short = 'j', long)]
    json: bool,

    /// Config file path
    #[arg(short = 'c', long)]
    config: Option<String>,

    /// List available models for a service
    #[arg(long)]
    lmodels: Option<String>,

    /// Extract JSON blocks from response
    #[arg(short = 'E', long)]
    extractjs: bool,
}

fn main() -> Result<()> {
    set_system_locale();
    
    // Build command with translated help messages
    let mut command = Args::command();
    command = command.about(t!("cli_description").to_string());
    
    // Override argument help messages
    // Note: Mutating args by ID which matches field names usually
    let args_help = [
        ("input", "help_prompt"),
        ("service", "help_service"),
        ("model", "help_model"),
        ("prompt_arg", "help_system_prompt"),
        ("sprompt", "help_sprompt"),
        ("list", "help_list"),
        ("help", "help_help"),
        ("version", "help_version"),
        ("nothink", "help_nothink"),
        ("json", "help_json"),
        ("config", "help_config"),
        ("lmodels", "help_lmodels"),
        ("extractjs", "help_extractjs"),
    ];

    for (arg_id, help_key) in args_help {
         let help_msg = t!(help_key).to_string();
         command = command.mut_arg(arg_id, |a| a.help(help_msg));
    }

    let matches = command.get_matches();
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    let config = Config::load(args.config.clone()).unwrap_or_else(|err| {
        eprintln!("{}", t!("error_loading_config", error = err));
        process::exit(1);
    });

    if config.services.is_empty() {
        eprintln!("{}", t!("no_services_defined"));
        process::exit(1);
    }

    if let Some(list_target) = args.list {
        match list_target.to_lowercase().as_str() {
            "services" | "s" => {
                if args.json {
                     let mut service_list = Vec::new();
                     for (name, service) in &config.services {
                         service_list.push(serde_json::json!({
                             "name": name,
                             "type": service.class,
                             "model": service.model.as_deref().unwrap_or("None"),
                             "descr": service.description.as_deref().unwrap_or("")
                         }));
                     }
                     let output = serde_json::json!({
                         "default": config.default_service,
                         "services": service_list
                     });
                     println!("{}", output.to_string());
                } else {
                    println!("{}", t!("configured_services"));
                    for (name, service) in &config.services {
                        let prefix = if name == &config.default_service { "*" } else { "-" };
                        let desc = service.description.clone().unwrap_or_else(|| t!("no_description").to_string());
                        let model = service.model.as_deref().unwrap_or("None");
                        
                        let valid_classes = ["openai", "ollama", "gemini", "anthropic"];
                        let class_display = if valid_classes.contains(&service.class.as_str()) {
                            service.class.clone()
                        } else {
                            t!("invalid_class_display").to_string()
                        };

                        println!("{} {} (Class: {}, Model: {}) - {}", prefix, name, class_display, model, desc);
                    }
                }
            },
            "prompts" | "p" => {
                if args.json {
                     let mut prompt_list = Vec::new();
                     for (name, prompt) in &config.system_prompts {
                         prompt_list.push(serde_json::json!({
                             "name": name,
                             "prompt": prompt
                         }));
                     }
                     let output = serde_json::json!({
                         "default": config.default_prompt,
                         "prompts": prompt_list
                     });
                     println!("{}", output.to_string());
                } else {
                    println!("{}", t!("configured_prompts"));
                    for (name, prompt) in &config.system_prompts {
                        let prefix = if name == &config.default_prompt { "*" } else { "-" };
                        // Get first line and truncate
                        let first_line = prompt.lines().next().unwrap_or("");
                        let display_prompt = if first_line.len() > 50 {
                            format!("{}...", &first_line[..47])
                        } else {
                            first_line.to_string()
                        };
                        println!("{} {} : \"{}\"", prefix, name, display_prompt);
                    }
                }
            },
            _ => {
                eprintln!("{}", t!("invalid_list_target", target = list_target));
                process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(sprompt_name) = args.sprompt {
        if let Some(prompt_content) = config.system_prompts.get(&sprompt_name) {
            println!("{}", prompt_content);
        } else {
            eprintln!("{}", t!("prompt_not_found", name = sprompt_name));
            process::exit(1);
        }
        return Ok(());
    }

    if let Some(service_name) = args.lmodels {
        // Instantiate Client just to get the driver
        // We don't strictly need model or system prompt for listing models, but constructor might require them.
        // We can pass None for overrides.
        // But constructor requires resolving a model from config if not overridden.
        // If config has no model, constructor might fail if we don't pass one?
        // Let's check constructor. It tries to resolve model. If service config has no model, it errors for OpenAI/Ollama.
        // We might need to handle this.
        // However, list_models shouldn't require a model to be selected.
        // Our current architecture ties Driver instantiation to having a valid model configuration.
        // For now, let's try to instantiate. If it fails due to missing model config, that's an existing constraint.
        // Ideally we'd separate "Connecting" from "Configuring a Chat Session".
        // But for this task, let's reuse Client::new.
        
        // Use a dummy model if needed? Or rely on config.
        // If config is missing model, user must provide one via CLI, but here we don't have -m for lmodels?
        // Actually Args has model option.
        
        let client = llm::Client::new(
             Some(&service_name),
             &config,
             args.model.as_ref(), // Pass model if user provided it (might help initialization)
             None // No system prompt needed
        ).context(t!("failed_init_client_for_listing"))?;

        let models = client.list_models().context(t!("failed_list_models"))?;

        if args.json {
             let json_output = serde_json::to_string_pretty(&models).context("Failed to serialize models list")?;
             println!("{}", json_output);
        } else {
             println!("{}", t!("available_models_for", service = service_name));
             for model in models {
                 println!("- {}", model);
             }
        }
        return Ok(());
    }

    let mut input_text = args.input;
    if let Some(p) = &input_text {
        if p == "-" {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer).context(t!("failed_read_stdin"))?;
            input_text = Some(buffer);
        }
    }

    if let Some(final_input) = input_text {
        
        // Instantiate Client
        // Client::new handles checking if prompt_arg is a key in config or literal
        let client = llm::Client::new(
            args.service.as_deref(),
            &config,
            args.model.as_ref(),
            args.prompt_arg.as_deref()
        ).context(t!("failed_init_client"))?;

        // Execute query
        let (response, thinking) = client.complete(&final_input)?;
        
        let extracted_json = if args.extractjs {
            extract_json_blocks(&response)
        } else {
            None
        };

        if args.json {
             let response_val = if args.extractjs {
                 extracted_json.unwrap_or(serde_json::Value::Null)
             } else {
                 serde_json::Value::String(response.clone())
             };

             let output = serde_json::json!({
                 "service": client.service_name(),
                 "model": client.model(),
                 "system_prompt": client.system_prompt(),
                 "prompt": final_input,
                 "response": response_val,
                 "think": thinking
             });
             println!("{}", output.to_string());
        } else {
            if args.extractjs {
                if let Some(json_data) = extracted_json {
                    // Print the JSON data directly (pretty printed)
                    println!("{}", serde_json::to_string_pretty(&json_data).unwrap_or_else(|_| json_data.to_string()));
                } else {
                    // If no JSON found, print error or nothing?
                    // "Si la respuesta contiene varios bloques JSON, devolverá un array con todos ellos."
                    // If none found, returning nothing seems appropriate or maybe raw text?
                     // Requirement: "Devolverá únicamente los datos JSON". So if none, print nothing or stderr error.
                     // Let's print nothing to stdout, maybe warning to stderr
                     eprintln!("{}", t!("no_json_blocks_found"));
                }
            } else {
                if !args.nothink {
                     if let Some(thought) = thinking {
                         println!("<think>\n{}\n</think>", thought);
                     }
                }
                println!("{}", response);
            }
        }

    } else {
        println!("{}", t!("cli_description"));
        println!("{}", t!("usage_info"));
        println!();
        println!("{}", t!("available_services"));
        for (name, service) in &config.services {
             let prefix = if name == &config.default_service { "*" } else { "-" };
             let desc = service.description.clone().unwrap_or_else(|| t!("no_description").to_string());
             let model = service.model.as_deref().unwrap_or("None");
             println!("{} {} (Class: {}, Model: {}) - {}", prefix, name, service.class, model, desc);
        }
        println!();
        
        let def_service_name = &config.default_service;
        if let Some(svc) = config.services.get(def_service_name) {
             println!("{}", t!("default_service", service = def_service_name, model = svc.model.as_deref().unwrap_or("None")));
        } else {
             println!("{}", t!("default_service_not_found", service = def_service_name));
        }
        
        println!("{}", t!("default_prompt", prompt = config.default_prompt));
    }

    Ok(())
}

fn extract_json_blocks(response: &str) -> Option<serde_json::Value> {
    // Regex to find ```json ... ``` blocks
    // Dot matches newline needs to be enabled for content
    let re_json = Regex::new(r"```json\s*([\s\S]*?)\s*```").unwrap();
    
    let mut blocks = Vec::new();
    for cap in re_json.captures_iter(response) {
        if let Some(content) = cap.get(1) {
            let json_str = content.as_str();
            // Try to parse as JSON
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                blocks.push(val);
            }
        }
    }

    // If no specific json blocks found, try to find generic blocks containing valid JSON
    if blocks.is_empty() {
        let re_generic = Regex::new(r"```\s*([\s\S]*?)\s*```").unwrap();
        for cap in re_generic.captures_iter(response) {
            if let Some(content) = cap.get(1) {
                let json_str = content.as_str();
                // Try to parse as JSON
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                    blocks.push(val);
                }
            }
        }
    }

    if blocks.is_empty() {
        None
    } else if blocks.len() == 1 {
        Some(blocks[0].clone())
    } else {
        Some(serde_json::Value::Array(blocks))
    }
}

