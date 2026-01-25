# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs`: binary entry point.
- `src/app.rs`: application state, event loop, background tasks.
- `src/ui.rs`: ratatui layout/rendering.
- `src/api.rs`: OpenRouter HTTP client and request helpers.
- `src/models.rs`: API data models and parsing helpers.
- `src/config.rs`: config loading (`OPENROUTER_API_KEY` + config file).
- `src/error.rs`: shared error types.
- `openapi.json`: reference schema for the API.
- `target/`: build output (generated).

## Build, Test, and Development Commands
- `cargo run`: build and run the TUI (requires API key).
- `cargo build`: compile the project.
- `cargo test`: run tests (none in repo yet).
- `cargo fmt`: format code with rustfmt.
- `cargo clippy -- -D warnings`: lint and fail on warnings.
- `cargo install --path .`: install locally.

## Coding Style & Naming Conventions
- Rust 2024 edition; follow rustfmt defaults (4-space indent).
- Types and traits use `CamelCase`; modules/functions use `snake_case`.
- Constants use `SCREAMING_SNAKE_CASE`.
- Prefer small, focused functions and explicit error handling (`Result`).

## Testing Guidelines
- No automated tests are currently present.
- Add unit tests in-module with `#[cfg(test)] mod tests` and integration tests under `tests/` (e.g., `tests/api_test.rs`).
- Run `cargo test` before submitting changes that affect behavior.

## Commit & Pull Request Guidelines
- Git history is empty; use short, imperative subjects (e.g., "Add model cache").
- PRs should include: summary, testing performed, and any config changes.
- For UI changes, attach a terminal screenshot or GIF when practical.

## Security & Configuration
- Provide the API key via `OPENROUTER_API_KEY` or `~/.config/openrouter-tui/config.toml`:
  ```toml
  api_key = "your_key_here"
  ```
- Never commit API keys or tokens. Credits may require a provisioning key (see README).
