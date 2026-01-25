# OpenRouter TUI

Rust TUI for browsing OpenRouter account credits, model catalog, and model details (pricing + throughput).

## Requirements

- Rust (stable)
- Linux terminal with ANSI support
- OpenRouter API key

## Setup

### API Key

Provide your key via environment variable or config file:

```bash
export OPENROUTER_API_KEY="your_key_here"
```

Or create a config file:

```
~/.config/openrouter-tui/config.toml
```

```toml
api_key = "your_key_here"
```

Note: The credits endpoint requires a provisioning key. If you use a standard API key, the app will still load models and details but credits may be unavailable.

## Run (Linux)

```bash
cargo run
```

## Install

```bash
cargo install --path .
```

## Controls

- `Tab`: Switch views
- `Shift+Tab`: Previous view
- `Up/Down`: Move selection (models)
- `Enter`: Open model details
- `Backspace`: Return to models
- `r`: Refresh credits/models
- `q`: Quit
