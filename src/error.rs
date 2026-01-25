use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("missing OpenRouter API key")]
    MissingApiKey,
    #[error("unauthorized: check your OpenRouter API key")]
    Unauthorized,
    #[error("rate limited by OpenRouter")]
    RateLimited,
    #[error("unexpected API status: {0}")]
    ApiStatus(String),
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("config error: {0}")]
    Config(String),
}
