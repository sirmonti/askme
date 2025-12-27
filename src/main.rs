mod config;
mod llm;
mod drivers;

use clap::{CommandFactory, FromArgMatches, Parser, Arg, ArgAction, error::ErrorKind, error::ContextKind};
use config::Config;
use std::process;
use std::io::{self, Read};

use rust_i18n::t;
use serde_json::json;

/// Main command-line arguments structure.
/// Uses `clap` to parse arguments provided by the user.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The prompt or query to send to the LLM.
    prompt: Option<String>,

    /// Override the system prompt with a specific key.
    #[arg(short = 'p', long = "prompt")]
    system_prompt: Option<String>,

    /// Service to use (e.g., openai, ollama).
    #[arg(short = 's', long)]
    service: Option<String>,

    /// Override the model defined in configuration.
    #[arg(short = 'm', long)]
    model: Option<String>,

    /// List configured services or prompts.
    /// If no value is specified, it defaults to listing services.
    #[arg(short = 'l', long, num_args(0..=1), default_missing_value = "services")]
    list: Option<String>,

    /// Show the content of a specific system prompt (sprompt).
    #[arg(long)]
    sprompt: Option<String>,

    /// Suppress the reasoning chain output (thinking) if available.
    #[arg(short = 'n', long)]
    nothink: bool,

    /// Output the full response as a JSON object.
    #[arg(short = 'j', long)]
    json: bool,

    /// Path to a custom configuration file.
    #[arg(short = 'c', long)]
    config: Option<String>,

    /// List available models for a specific service (e.g., ollama).
    #[arg(long)]
    lmodels: Option<String>,
}

// Load the internationalization (i18n) macro for translation.
rust_i18n::i18n!("locales");


/// Builds the CLI command with internationalization.
/// Responsible for assigning translated help descriptions to each argument.
fn build_cli() -> clap::Command {
    let mut cmd = Args::command();
    cmd = cmd.about(t!("cli_about"));
    
    // Pre-calculate localized strings to avoid type inference issues with t! macro
    let val_service = t!("val_service").to_string();
    let val_model = t!("val_model").to_string();
    let val_name = t!("val_name").to_string();
    let val_file = t!("val_file").to_string();

    cmd = cmd.mut_arg("prompt", |a| a.help(t!("help_prompt")));

    // Fix: We need to reference the argument by its LONG ID 'prompt' (the flag),
    // but there's a collision with the positional 'prompt'.
    // Clap distinguishes them. The positional one is "prompt", the flag is "prompt" (long) or 'p' (short).
    // However, when modifying args in clap after derive, we need to be careful.
    // Let's assume the derive macro handles the unique ID generation.
    // Usually, clap identifiers for derive are the field names.
    // Field 'prompt' => id "prompt" (positional)
    // Field 'system_prompt' (long "prompt") => id "system_prompt" (or similar).
    // Let's check how clap derive names it. It usually checks the field name. 
    // Since we named the field `system_prompt` but long `prompt`, the ID is likely `system_prompt`.
    
    // Use clone for system_prompt to avoid moving original val_name
    let vn_sys = val_name.clone();
    cmd = cmd.mut_arg("system_prompt", move |a| a.help(t!("help_sprompt_override")).value_name(vn_sys));
    
    // Use clones for reused strings
    let vs1 = val_service.clone();
    cmd = cmd.mut_arg("service", move |a| a.help(t!("help_service")).value_name(vs1));
    
    cmd = cmd.mut_arg("model", move |a| a.help(t!("help_model")).value_name(val_model));
    
    // Note: We access argument by its long ID usually
    cmd = cmd.mut_arg("list", |a| a.help(t!("help_list")).value_name("P/S"));
    
    cmd = cmd.mut_arg("sprompt", move |a| a.help(t!("help_sprompt")).value_name(val_name));
    
    cmd = cmd.mut_arg("nothink", |a| a.help(t!("help_nothink")));
    cmd = cmd.mut_arg("json", |a| a.help(t!("help_json")));
    
    cmd = cmd.mut_arg("config", move |a| a.help(t!("help_config")).value_name(val_file));
    
    cmd = cmd.mut_arg("lmodels", move |a| a.help(t!("help_lmodels")).value_name(val_service)); // Last use doesn't need clone
    
    // Disable automatic flags to handle them manually with translations
    cmd = cmd.disable_help_flag(true);
    cmd = cmd.disable_version_flag(true);

    cmd = cmd.arg(
        Arg::new("help")
            .long("help")
            .short('h')
            .action(ArgAction::Help)
            .help(t!("help_help"))
            .global(true)
    );
     cmd = cmd.arg(
        Arg::new("version")
            .long("version")
            .short('V')
            .action(ArgAction::Version)
            .help(t!("help_version"))
            .global(true)
    );
    cmd
}

/// Main application function.
fn main() {
    // 1. Detect and configure the language (locale)
    let system_locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
    let lang_code = system_locale.split(|c| c == '-' || c == '_').next().unwrap_or("en");
    
    // Check if the language is supported, otherwise fallback to English
    let supported = ["en", "es", "fr", "it", "de", "zh"];
    if supported.contains(&lang_code) {
        rust_i18n::set_locale(lang_code);
    } else {
        rust_i18n::set_locale("en");
    }

    // 2. Build the command and inject translations
    let cmd = build_cli();

    // 3. Process command-line arguments
    let matches = match cmd.try_get_matches() {
        Ok(m) => m,
        Err(e) => {
            let kind = e.kind();
            
            // Helper to get context value as string
            let get_context = |k: ContextKind| -> String {
                e.context().find_map(|(kind, value)| {
                    if kind == k {
                        Some(value.to_string())
                    } else {
                        None
                    }
                }).unwrap_or_default()
            };

            // Custom error handling to show localized messages
            match kind {
                ErrorKind::UnknownArgument => {
                    let arg = get_context(ContextKind::InvalidArg);
                    eprintln!("{}", t!("err_unknown_arg", arg = arg));
                },
                ErrorKind::MissingRequiredArgument => {
                    let arg = get_context(ContextKind::InvalidArg);
                    eprintln!("{}", t!("err_missing_arg", arg = arg));
                },
                ErrorKind::InvalidValue => {
                    let arg = get_context(ContextKind::InvalidArg);
                    let val = get_context(ContextKind::InvalidValue);
                    eprintln!("{}", t!("err_invalid_value", arg = arg, value = val));
                },
                ErrorKind::ArgumentConflict => {
                   let arg = get_context(ContextKind::InvalidArg);
                   let other = get_context(ContextKind::PriorArg);
                   eprintln!("{}", t!("err_argument_conflict", arg = arg, other = other));
                }
                ErrorKind::DisplayHelp => {
                    // Standard help, usually printed to stdout
                     let _ = build_cli().print_help();
                }
                ErrorKind::DisplayVersion => {
                     println!("{}", e);
                }
                _ => {
                    eprintln!("Error: {}", e);
                }
            }
            process::exit(1);
        }
    };
    
    // Check for manual help/version args if present
    if matches.contains_id("help") {
         let _ = build_cli().print_help();
         process::exit(0);
    }
    if matches.contains_id("version") {
         // Print version
         println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
         process::exit(0);
    }

    let args = Args::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // 4. Load configuration
    let config = match Config::load(args.config.clone()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", t!("error_generic", error = format!("Configuration invalid: {}", e)));
            process::exit(1);
        }
    };

    // 5. Handle subcommands and special options

    // Option --sprompt: Show content of a system prompt
    if let Some(prompt_key) = args.sprompt {
        match config.system_prompts.get(&prompt_key) {
            Some(prompt) => println!("{}", prompt),
            None => {
                eprintln!("{}", t!("sys_prompt_not_found", key = prompt_key));
                process::exit(1);
            }
        }
        return;
    }

    // Option --list: List services or prompts
    if let Some(list_type) = args.list {
        match list_type.to_lowercase().as_str() {
            "services" | "s" => print_services(&config, args.json),
            "prompts" | "p" => print_prompts(&config, args.json),
            _ => {
                eprintln!("{}", t!("invalid_list_type", list_type = list_type));
                process::exit(1);
            }
        }
        return;
    }

    // Option --lmodels: List models available remotely
    if let Some(service_name) = args.lmodels {
        match llm::Client::list_models(&config, &service_name) {
            Ok(models) => {
                if args.json {
                     let json_output = json!(models);
                     println!("{}", json_output.to_string());
                } else {
                     println!("Models for service '{}':", service_name);
                     for model in models {
                         println!(" - {}", model);
                     }
                }
            },
            Err(e) => {
                eprintln!("Error listing models for service '{}': {}", service_name, e);
                process::exit(1);
            }
        }
        return;
    }


    let mut prompt = args.prompt.unwrap_or_default();

    // Handle standard input (stdin) if prompt is "-"
    if prompt == "-" {
        let mut buffer = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buffer) {
             eprintln!("Error reading from stdin: {}", e);
             process::exit(1);
        }
        prompt = buffer.trim().to_string();
    }

    // If no prompt, show help and general info
    if prompt.is_empty() {
        println!("{}", t!("no_prompt"));
        println!("{}", t!("usage"));
        println!("{}\n", t!("help_info"));

        let default_service_name = &config.default_service;
        let default_model = config.services.iter()
            .find(|s| s.name == *default_service_name)
            .and_then(|s| s.model.as_deref())
            .unwrap_or("None");

        println!("{}", t!("default_service", service = default_service_name, model = default_model));
        
        println!("\n{}", t!("available_services"));
        print_services(&config, false);
        
        process::exit(0);
    }

    // 6. Execute LLM request

    // Determine service
    let service_name = args.service.as_deref().unwrap_or(&config.default_service);
    let service = config.services.iter().find(|s| &s.name == service_name).unwrap_or_else(|| {
        eprintln!("Service '{}' not found.", service_name);
        process::exit(1);
    });

    // Make the request
    // Note: Use reference to config to allow zero-copy in some cases
    match llm::Client::make_request(
        &config, 
        service, 
        &prompt, 
        args.model.as_deref(),
        args.system_prompt.as_deref()
    ) {
        Ok(llm_response) => {
            if args.json {
                // Structured JSON output
                let json_output = json!({
                    "service": llm_response.service,
                    "model": llm_response.model,
                    "system_prompt": llm_response.system_prompt,
                    "prompt": prompt,
                    "response": llm_response.response,
                    "think": llm_response.thinking
                });
                println!("{}", json_output.to_string());
            } else {
                // Normal text output
                let nothink_flag = args.nothink;
                
                if !nothink_flag {
                    if let Some(think_str) = llm_response.thinking {
                        println!("<think>{}</think>", think_str);
                    }
                }
                println!("{}", llm_response.response);
            }
        },
        Err(e) => {
            eprintln!("{}", t!("error_processing_request", error = e));
            process::exit(1);
        }
    }
}

/// Helper to print the list of configured prompts.
fn print_prompts(config: &Config, json_output: bool) {
    if json_output {
        let prompts_list: Vec<serde_json::Value> = config.system_prompts.iter().map(|(k, v)| {
            json!({
                "name": k,
                "prompt": v
            })
        }).collect();

        let output = json!({
            "default": config.default_prompt,
            "prompts": prompts_list
        });
        println!("{}", output.to_string());
        return;
    }

    println!("{}", t!("configured_prompts"));
    for (key, value) in &config.system_prompts {
        let mark = if key == &config.default_prompt { "*" } else { " " };
        // Truncate long prompts for display (max 60 chars)
        let display_val = if value.len() > 60 {
            format!("{}...", &value[..57])
        } else {
            value.clone()
        };
        println!(" {} {}: \"{}\"", mark, key, display_val);
    }
}

/// Helper to print the list of configured services.
fn print_services(config: &Config, json_output: bool) {
    if json_output {
        let services_list: Vec<serde_json::Value> = config.services.iter().map(|s| {
            json!({
                "name": s.name,
                "type": s.service_type,
                "model": s.model.as_deref().unwrap_or("None"),
                "descr": s.description.as_deref().unwrap_or("")
            })
        }).collect();

        let output = json!({
            "default": config.default_service,
            "services": services_list
        });
        println!("{}", output.to_string());
        return;
    }

    println!("{}", t!("configured_services"));
    for service in &config.services {
        let default_mark = if config.default_service == service.name { "*" } else { " " };
        let desc = service.description.as_deref().unwrap_or("");
        let desc_str = if !desc.is_empty() { format!(" - {}", desc) } else { "".to_string() };
        
        println!(" {} {} (Type: {:?}, Model: {}){}", 
            default_mark, 
            service.name, 
            service.service_type, 
            service.model.as_deref().unwrap_or("None"),
            desc_str
        );
    }
}
