# Plugin Development Guide

This guide covers the Cortex plugin system, including how to create, build, and manage plugins that extend Cortex functionality.

## Overview

Cortex plugins are WebAssembly (WASM) modules that extend the functionality of Cortex. They enable custom commands, hooks, tools, themes, and more.

**Key characteristics:**
- Plugins are compiled WASM modules for portability and security
- Each plugin is stored in its own directory under `~/.cortex/plugins/<plugin-name>/`
- Plugins declare capabilities and permissions in a manifest file
- The sandboxed WASM environment ensures safe execution

## Plugin Directory Structure

```
~/.cortex/plugins/
└── my-plugin/
    ├── plugin.toml    # Manifest file (required)
    └── plugin.wasm    # Compiled WASM module
```

## Plugin Manifest

Every plugin requires a `plugin.toml` manifest file that describes the plugin and its requirements.

### Complete Manifest Reference

```toml
[plugin]
id = "my-plugin"
name = "My Plugin"
version = "1.0.0"
description = "A plugin description"
authors = ["Author <email@example.com>"]
homepage = "https://github.com/example/my-plugin"
license = "MIT"
min_cortex_version = "0.1.0"
keywords = ["keyword1", "keyword2"]

capabilities = ["commands", "hooks", "tools", "config", "filesystem", "shell", "network"]

permissions = [
    { read_file = { paths = ["**/*.rs"] } },
    { write_file = { paths = ["output/**"] } },
    { execute = { commands = ["ls", "cat"] } },
    { network = { domains = ["api.example.com"] } },
    { environment = { vars = ["API_KEY"] } },
]

[[commands]]
name = "mycommand"
aliases = ["mc"]
description = "My command"
usage = "/mycommand [args]"
category = "utilities"

[[commands.args]]
name = "arg1"
description = "Argument description"
required = false
default = "default"
arg_type = "string"

[[hooks]]
hook_type = "tool_execute_before"
priority = 50
pattern = "*.rs"

[config]
api_key = { description = "API key", type = "string", required = true }
max_items = { description = "Max items", type = "number", default = 10 }

[wasm]
memory_pages = 256
timeout_ms = 30000
wasi_enabled = true
```

### Manifest Fields

#### Plugin Metadata

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique identifier for the plugin |
| `name` | string | Yes | Human-readable name |
| `version` | string | Yes | Semantic version (e.g., "1.0.0") |
| `description` | string | Yes | Brief description of the plugin |
| `authors` | array | No | List of authors |
| `homepage` | string | No | Project homepage URL |
| `license` | string | No | License identifier (e.g., "MIT") |
| `min_cortex_version` | string | No | Minimum Cortex version required |
| `keywords` | array | No | Keywords for discovery |

#### WASM Configuration

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `memory_pages` | number | 256 | WASM memory pages (64KB each) |
| `timeout_ms` | number | 30000 | Execution timeout in milliseconds |
| `wasi_enabled` | boolean | true | Enable WASI interface |

## Capabilities

Capabilities define what features a plugin can provide. Declare them in the `capabilities` array.

| Capability | Description |
|------------|-------------|
| `commands` | Provide custom slash commands |
| `hooks` | Register hooks for various events |
| `events` | Handle and emit events |
| `tools` | Provide MCP-style tools |
| `formatters` | Provide custom formatters |
| `themes` | Provide custom themes |
| `config` | Access configuration system |
| `filesystem` | Access file system (requires permissions) |
| `shell` | Execute shell commands (requires permissions) |
| `network` | Make network requests (requires permissions) |

## Permissions

Permissions define what resources a plugin can access. Each permission type restricts access to specific operations.

### Permission Types

#### File Read Permission

```toml
{ read_file = { paths = ["**/*.rs", "src/**"] } }
```

Allows reading files matching the specified glob patterns.

#### File Write Permission

```toml
{ write_file = { paths = ["output/**", "generated/**"] } }
```

Allows writing to files matching the specified glob patterns.

#### Execute Permission

```toml
{ execute = { commands = ["ls", "cat", "grep"] } }
```

Allows executing the specified shell commands.

#### Network Permission

```toml
{ network = { domains = ["api.example.com", "*.github.com"] } }
```

Allows network requests to the specified domains.

#### Environment Permission

```toml
{ environment = { vars = ["API_KEY", "DEBUG"] } }
```

Allows reading the specified environment variables.

## Commands

Plugins can provide custom slash commands that users invoke interactively.

### Command Definition

```toml
[[commands]]
name = "mycommand"
aliases = ["mc", "mycmd"]
description = "Execute my custom command"
usage = "/mycommand <action> [options]"
category = "utilities"

[[commands.args]]
name = "action"
description = "The action to perform"
required = true
arg_type = "string"

[[commands.args]]
name = "verbose"
description = "Enable verbose output"
required = false
default = "false"
arg_type = "boolean"
```

### Argument Types

| Type | Description |
|------|-------------|
| `string` | Text value |
| `number` | Numeric value |
| `boolean` | True/false value |
| `path` | File system path |

## Hooks

Hooks allow plugins to intercept and respond to various events in Cortex.

### Available Hook Types

#### Tool Hooks

| Hook Type | Description |
|-----------|-------------|
| `tool_execute_before` | Called before a tool executes |
| `tool_execute_after` | Called after a tool executes |

#### Chat Hooks

| Hook Type | Description |
|-----------|-------------|
| `chat_message` | Called when a chat message is sent |

#### AI Hooks

| Hook Type | Description |
|-----------|-------------|
| `prompt_inject` | Inject content into prompts |
| `ai_response_before` | Called before AI generates response |
| `ai_response_stream` | Called during streaming response |
| `ai_response_after` | Called after AI response completes |

#### Session Hooks

| Hook Type | Description |
|-----------|-------------|
| `session_start` | Called when a session starts |
| `session_end` | Called when a session ends |

#### File Hooks

| Hook Type | Description |
|-----------|-------------|
| `file_operation_before` | Called before file operations |
| `file_operation_after` | Called after file operations |
| `file_edited` | Called when a file is edited |

#### Command Hooks

| Hook Type | Description |
|-----------|-------------|
| `command_execute_before` | Called before command execution |
| `command_execute_after` | Called after command execution |

#### Input Hooks

| Hook Type | Description |
|-----------|-------------|
| `input_intercept` | Intercept user input |

#### Error Hooks

| Hook Type | Description |
|-----------|-------------|
| `error_handle` | Handle errors |

#### Config Hooks

| Hook Type | Description |
|-----------|-------------|
| `config_changed` | Called when configuration changes |
| `model_changed` | Called when the model changes |

#### Workspace Hooks

| Hook Type | Description |
|-----------|-------------|
| `workspace_changed` | Called when workspace changes |

#### UI Hooks

| Hook Type | Description |
|-----------|-------------|
| `ui_render` | Called during UI rendering |
| `ui_widget_register` | Register custom UI widgets |
| `ui_key_binding` | Register custom key bindings |
| `ui_theme_override` | Override UI theme elements |

### Hook Definition

```toml
[[hooks]]
hook_type = "tool_execute_before"
priority = 50
pattern = "*.rs"

[[hooks]]
hook_type = "ai_response_after"
priority = 100
```

The `priority` field (0-100) determines execution order when multiple hooks are registered. Higher priority hooks execute first.

## Plugin Configuration

Plugins can define configuration options that users can customize.

```toml
[config]
api_key = { description = "API key for authentication", type = "string", required = true }
max_items = { description = "Maximum items to process", type = "number", default = 10 }
debug_mode = { description = "Enable debug logging", type = "boolean", default = false }
output_dir = { description = "Output directory path", type = "string", default = "./output" }
```

### Configuration Field Properties

| Property | Type | Description |
|----------|------|-------------|
| `description` | string | Help text for the option |
| `type` | string | Data type: "string", "number", "boolean" |
| `required` | boolean | Whether the option is required |
| `default` | any | Default value if not specified |

## Building Plugins

### Prerequisites

1. Install Rust: https://rustup.rs/
2. Add the WASI target:

```bash
rustup target add wasm32-wasi
```

### Project Setup

Create a new Rust library project:

```bash
cargo new --lib my-plugin
cd my-plugin
```

Update `Cargo.toml`:

```toml
[package]
name = "my-plugin"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# Add any WASM-compatible dependencies
```

### Building

```bash
# Build for WASI target
cargo build --target wasm32-wasi --release

# The WASM file will be at:
# target/wasm32-wasi/release/my_plugin.wasm
```

### Installing Your Plugin

```bash
# Create the plugin directory
mkdir -p ~/.cortex/plugins/my-plugin/

# Copy the WASM module
cp target/wasm32-wasi/release/my_plugin.wasm ~/.cortex/plugins/my-plugin/plugin.wasm

# Copy the manifest
cp plugin.toml ~/.cortex/plugins/my-plugin/
```

## Plugin CLI Commands

Cortex provides commands for managing plugins.

### List Plugins

```bash
# List all installed plugins
cortex plugin list

# Short form
cortex plugin ls

# Output as JSON
cortex plugin list --json

# List only enabled plugins
cortex plugin list --enabled

# List only disabled plugins
cortex plugin list --disabled
```

### Install Plugin

```bash
# Install from registry
cortex plugin install my-plugin

# Short form
cortex plugin add my-plugin

# Install specific version
cortex plugin install my-plugin -v 1.2.0
cortex plugin install my-plugin --version 1.2.0

# Force reinstall
cortex plugin install my-plugin -f
cortex plugin install my-plugin --force
```

### Remove Plugin

```bash
# Remove a plugin
cortex plugin remove my-plugin

# Short forms
cortex plugin rm my-plugin
cortex plugin uninstall my-plugin

# Skip confirmation
cortex plugin remove my-plugin -y
cortex plugin remove my-plugin --yes
```

### Enable/Disable Plugin

```bash
# Enable a plugin
cortex plugin enable my-plugin

# Disable a plugin
cortex plugin disable my-plugin
```

### Show Plugin Details

```bash
# Show plugin information
cortex plugin show my-plugin

# Short form
cortex plugin info my-plugin

# Output as JSON
cortex plugin show my-plugin --json
```

## Best Practices

### Security

1. **Request minimal permissions** - Only ask for permissions your plugin actually needs
2. **Validate all inputs** - Never trust user or external input
3. **Handle errors gracefully** - Provide meaningful error messages
4. **Avoid storing sensitive data** - Don't hardcode secrets

### Performance

1. **Keep WASM size small** - Minimize dependencies
2. **Set appropriate timeouts** - Don't set `timeout_ms` too high
3. **Limit memory usage** - Use reasonable `memory_pages` values
4. **Cache when appropriate** - Avoid redundant operations

### Compatibility

1. **Specify `min_cortex_version`** - Ensure compatibility
2. **Test across versions** - Verify behavior with updates
3. **Use semantic versioning** - Follow semver for your plugin version
4. **Document breaking changes** - Update users when APIs change

## Example: Minimal Plugin

Here's a complete minimal plugin example:

### plugin.toml

```toml
[plugin]
id = "hello-world"
name = "Hello World"
version = "1.0.0"
description = "A simple hello world plugin"
authors = ["Developer <dev@example.com>"]
license = "MIT"

capabilities = ["commands"]

[[commands]]
name = "hello"
aliases = ["hi"]
description = "Say hello"
usage = "/hello [name]"
category = "utilities"

[[commands.args]]
name = "name"
description = "Name to greet"
required = false
default = "World"
arg_type = "string"
```

### Build and Install

```bash
# Build the plugin
cargo build --target wasm32-wasi --release

# Install
mkdir -p ~/.cortex/plugins/hello-world/
cp target/wasm32-wasi/release/hello_world.wasm ~/.cortex/plugins/hello-world/plugin.wasm
cp plugin.toml ~/.cortex/plugins/hello-world/

# Verify installation
cortex plugin list
cortex plugin show hello-world
```

## Troubleshooting

### Plugin Not Loading

1. Check that `plugin.toml` exists and is valid TOML
2. Verify `plugin.wasm` is present and built correctly
3. Check Cortex logs for error messages
4. Ensure the plugin ID matches the directory name

### Permission Errors

1. Verify required permissions are declared in the manifest
2. Check that permission patterns match the files/commands you need
3. Review Cortex security settings

### WASM Execution Errors

1. Ensure the WASM is built for `wasm32-wasi` target
2. Check that `wasi_enabled = true` if using WASI features
3. Increase `memory_pages` if running out of memory
4. Increase `timeout_ms` for long-running operations
