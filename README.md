# AskMe - CLI Tool for Querying Large Language Models

Created as an experiment to test AI-assisted development, **AskMe** is a simple but powerful command-line interface (CLI) tool written in Rust for interacting with Large Language Models (LLMs). It serves as a unified bridge to various AI providers, supporting both cloud-based services like OpenAI and local inference engines like Ollama.

AskMe allows you to integrate AI capabilities directly into your terminal workflow, supporting custom system prompts, service selection, and seamless piping with other tools.

## Command Line Options and Usage

The basic usage syntax is:

```bash
askme [OPTIONS] [PROMPT]
```

### Arguments

| Short | Long | Description |
| :--- | :--- | :--- |
| | `[PROMPT]` | The query or instruction to send to the LLM. Use `-` to read from stdin. |
| `-p` | `--prompt <PREFIX\|TEXT>` | Specifies the **system prompt**. You can use a predefined key from your config file (e.g., `coder`) or pass a raw string directly. |
| `-s` | `--service <NAME>` | Selects the LLM service to use (e.g., `openai`, `ollama-local`). Defaults to the configured default. |
| `-m` | `--model <MODEL>` | Overrides the default model for the selected service. |
| `-l` | `--list [TYPE]` | Lists configured `services` (default) or available `prompts`. |
| | `--sprompt <KEY>` | Displays the full content of a stored system prompt. |
| `-n` | `--nothink` | Suppresses the "Chain of Thought" (thinking process) output if the model provides it. |
| `-j` | `--json` | Outputs the full response in JSON format, ideal for programmatic processing. |
| `-c` | `--config <FILE>` | Uses a specific configuration file instead of the default locations. |
| | `--lmodels <SERVICE>` | Lists remote models available on the specified service (e.g., querying Ollama API). |
| `-h` | `--help` | Prints the help message. |
| `-V` | `--version` | Prints the version information. |

### Usage Examples

**Basic query using the default service:**
```bash
askme "Explain the concept of ownership in Rust"
```

**Using a specific service and model:**
```bash
askme -s openai -m gpt-4 "Write a haiku about coding"
```

**Using a predefined system prompt:**
```bash
# Assuming 'summarizer' is defined in your config
askme -p summarizer "Long text content..."
```

**Using an ad-hoc system prompt:**
```bash
askme -p "You are a friendly pirate" "Hello there!"
```

**Outputting JSON for scripts:**
```bash
askme -j "Generate a random JSON object"
```

## Configuration Files

AskMe uses a YAML configuration file (`askme.yml`) to manage services and prompts. It searches for this file in the following order:
1. The path specified by `--config`.
2. The current working directory.
3. The user's configuration directory (e.g., `~/.config/askme/askme.yml` on Linux).
4. System-wide configuration paths.

### Configuration Structure

The file consists of four main sections:
- `default_service`: The name of the service to use when none is specified.
- `default_prompt`: The default system prompt key.
- `services`: A list of LLM providers.
- `system_prompts`: A dictionary of reusable system prompts.

### Example `askme.yml`

```yaml
default_service: local
default_prompt: default

services:
  - name: openai
    class: openai
    api_key: sk-proj-12345...
    model: gpt-3.5-turbo
    description: "Official OpenAI API"

  - name: local
    class: ollama
    url: http://localhost:11434
    model: llama3
    description: "Local Ollama instance"

system_prompts:
  default: "You are a helpful assistant."
  coder: "You are an expert software engineer. Provide concise, correct code snippets."
  reviewer: "Analyze the following code diff for bugs and security issues."
```

## Chaining with Other Applications

One of AskMe's most powerful features is its ability to integrate into Unix pipelines using standard input (`stdin`). You can pipe the output of other commands into AskMe by using `-` as the prompt argument.

### Examples

**Summarize a file content:**
```bash
cat README.md | askme -p summarizer -
```

**Analyze a Git diff:**
```bash
git diff | askme -p reviewer -
```

**Get an explanation of a log error:**
```bash
tail -n 20 error.log | askme "Explain what caused this error based on these logs" -
```

**Chain multiple AI steps:**
```bash
# Generate code, then immediately ask for an explanation of the generated code
askme "Write a python script to scrape a web page" | askme "Explain how this code works" -
```
