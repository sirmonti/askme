# AskMe - CLI tool for interacting with LLM services

## Introduction

Conceived as a Rust development exercise using Antigravity (The Google AI IDE), **AskMe** is a Command Line Interface (CLI) tool designed to interact with Large Language Models (LLMs) providers directly from your command line, with support for OpenAI, Ollama (local models), Gemini, and Anthropic. 

Designed to be lightweight, cross-platform, and easily pipeable, **AskMe** is a powerful utility for developers and power users who want to integrate AI capabilities into their command-line workflows.

## Command Line Options and Usage Examples

The `askme` command provides several flags to control its behavior.

### Usage

```bash
askme [OPTIONS] [PROMPT]
```

### Options

| Option | Short | Description |
| :--- | :--- | :--- |
| `--help` | `-h` | Print help information. |
| `--version` | `-V` | Print version information. |
| `--service <NAME>` | `-s` | Specify the LLM service to use (e.g., `openai`, `local`)._Overrides config default._ |
| `--model <NAME>` | `-m` | Specify the model to use (e.g., `gpt-4`, `llama3`). _Overrides service default._ |
| `--prompt <NAME/TEXT>` | `-p` | Provide a custom system prompt or use a named system prompt from config. |
| `--nothink` | `-n` | Do not show the reasoning chain ("thinking") for models that support it (e.g. DeepSeek). |
| `--json` | `-j` | Output the result in raw JSON format. |
| `--extractjs` | `-E` | Extract JSON blocks from the response. Returns an object or array of objects. |
| `--list [TARGET]` | `-l` | List configured services (`services` or `s`) or system prompts (`prompts` or `p`). Default is `services`. |
| `--config <PATH>` | `-c` | Specify a custom configuration file path. |
| `--sprompt <NAME>` | | Show the full content of a specific system prompt configuration. |
| `--lmodels <SERVICE>` | | List available models for a specific service (fetches from API if supported). |

### Examples

**1. Simple Query (Default Service)**
```bash
askme "How do I reverse a string in Rust?"
```

**2. Specify Service and Model**
```bash
askme --service openai --model gpt-4 "Explain quantum entanglement like I'm 5"
```

**3. List Configured Services**
```bash
askme --list services
```

**4. List Available Models for a Service**
```bash
askme --lmodels ollama
```

**5. JSON Output**
```bash
askme --json "Generate a JSON object with user data"
```

**6. Using Configured System Prompt**
Use a predefined system prompt from your configuration file (e.g., `coder`):
```bash
askme -p coder "Write a Python script to scrape a website"
```

**7. Using Literal System Prompt**
Provide a custom system prompt directly in the command:
```bash
askme -p "You are a poetic assistant. Answer in rhymes." "What is the capital of France?"
```

AskMe is driven by a configuration file, typically named `askme.yml`. The tool searches for this file in the following order:
1.  Path specified via `--config`.
2.  Current working directory.
3.  User's configuration directory (e.g., `~/.config/askme/` on Linux, `%APPDATA%\askme\` on Windows).
4.  Global configuration directory (e.g., `/etc/askme.yml` on Linux).

Global configuration file is always loaded first. If you have both global and user configuration files, both will be loaded and merged with the user configuration overriding global one.

### Structure

The configuration file is in YAML format and consists of three main sections:
-   **Defaults**: Global default settings.
-   **System Prompts**: Reusable system prompts.
-   **Services**: Definitions for LLM providers.

#### Example `askme.yml`

```yaml
# Global defaults
default_service: localollama
default_prompt: helper

# Dictionary of system prompts
system_prompts:
  helper: "You are a helpful assistant."
  coder: "You are an expert software engineer. Provide code snippets."
  piperesponse: "When asked for a JSON response, provide only the JSON code wrapped in standard markdown code blocks"
# Service definitions
services:
  # OpenAI configuration
  openaigpt4:
    class: openai
    description: "Main OpenAI Service"
    model: gpt-4
    api_key: sk-proj-123... # Or use env var expansion if supported in future
    system_prompt: coder

  # Ollama configuration (Local)
  localollama:
    class: ollama
    description: "Local Llama 3 model"
    url: http://myollama.local:11434
    model: llama3
    system_prompt: helper

  # Google Gemini configuration
  mygemini:
    class: gemini
    description: "Google Gemini Pro"
    api_key: AIzaSy...
    model: gemini-1.5-pro

  # Anthropic Claude configuration
  myclaude:
    class: anthropic
    description: "Claude 3.5 Sonnet"
    api_key: sk-ant-...
    model: claude-3-5-sonnet-20240620
```

#### Service Classes
-   `openai`: For OpenAI-compatible APIs.
-   `ollama`: For local Ollama instances or Ollama-compatible APIs.
-   `gemini`: For Google's Gemini API (ignores `url` param).
-   `anthropic`: For Anthropic's Claude API (ignores `url` param).

## Chaining with Other Applications

One of the most powerful features of AskMe is its ability to accept input from **stdin**. You can use the hyphen `-` as the prompt argument to tell `askme` to read from the standard input.

This allows you to pipe output from other commands directly into an LLM analysis context.

### Examples

**1. Explain a Source File**
Pipe a code file into AskMe to get an explanation:
```bash
cat src/main.rs | askme -p "Explain what this Rust code does" -
```
Note that we are using the system prompt as an instruction.

**2. Analyze Git Changes**
Ask for a summary of your uncommitted changes:
```bash
git diff | askme -s openai -p "Summarize these changes for a commit message" -
```

**3. Debugging Logs**
grep for errors and ask for a diagnosis:
```bash
grep "ERROR" /var/log/syslog | tail -n 20 | askme -p "Analyze these error logs and suggest a fix" -
```

**4. Chaining JSON Output**
Use the JSON output flag to pipe structured data into other tools like `jq`:
```bash
askme --json --prompt piperesponse --extractjs "Generate a list of 5 dummy users with attributes: name, email, role. Role must be admin, editor, viewer or user. Return the output in json format" | jq '.["response"].[] | select(.role == "admin")'
```

This flexibility makes AskMe a versatile tool for automating documentation, code review, and system administration tasks.
