use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::error::AppError;
use crate::models::{
    Credits, CreditsResponse, Latency, ModelDetails, ModelEndpointsResponse, ModelSummary,
    ModelsResponse, Throughput, parse_pricing, split_provider,
};

#[derive(Clone)]
pub struct OpenRouterClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl OpenRouterClient {
    pub fn new(api_key: String) -> Result<Self, AppError> {
        let http = Client::builder()
            .user_agent("openrouter-tui/0.1.0")
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(AppError::Request)?;

        Ok(Self {
            base_url: "https://openrouter.ai/api/v1".to_string(),
            api_key,
            http,
        })
    }

    pub fn fetch_credits(&self) -> Result<Credits, AppError> {
        let response = self.request("credits").send()?;
        let response = handle_status(response)?;
        let payload: CreditsResponse = response.json()?;

        Ok(Credits {
            total_credits: payload.data.total_credits,
            total_usage: payload.data.total_usage,
        })
    }

    pub fn fetch_models(&self) -> Result<Vec<ModelSummary>, AppError> {
        let response = self.request("models").send()?;
        let response = handle_status(response)?;
        let payload: ModelsResponse = response.json()?;

        let models = payload
            .data
            .into_iter()
            .map(|model| {
                let (provider, name) = split_provider(&model.id);
                ModelSummary {
                    id: model.id,
                    name: model.name.unwrap_or(name),
                    provider,
                    context_length: model.context_length,
                    pricing: parse_pricing(model.pricing.as_ref()),
                }
            })
            .collect();

        Ok(models)
    }

    pub fn fetch_model_details(&self, model_id: &str) -> Result<ModelDetails, AppError> {
        let (author, slug) = split_provider(model_id);
        let path = format!("models/{author}/{slug}/endpoints");
        let response = self.request(&path).send()?;
        let response = handle_status(response)?;
        let payload: ModelEndpointsResponse = response.json()?;

        let endpoint = select_best_endpoint(&payload.data.endpoints);
        let pricing = endpoint.as_ref().and_then(|value| value.pricing.as_ref());
        let throughput = endpoint
            .as_ref()
            .and_then(|value| value.throughput_last_30m.as_ref());
        let latency = endpoint
            .as_ref()
            .and_then(|value| value.latency_last_30m.as_ref());

        Ok(ModelDetails {
            id: payload.data.id,
            name: payload.data.name.unwrap_or_else(|| slug.clone()),
            provider: author,
            provider_name: endpoint
                .as_ref()
                .and_then(|value| value.provider_name.clone()),
            description: payload.data.description,
            context_length: endpoint
                .as_ref()
                .and_then(|value| value.context_length)
                .or(payload.data.context_length),
            pricing: parse_pricing(pricing),
            throughput: Throughput {
                p50: throughput.and_then(|value| value.p50),
                p90: throughput.and_then(|value| value.p90),
                p99: throughput.and_then(|value| value.p99),
            },
            latency: Latency {
                p50: latency.and_then(|value| value.p50),
                p90: latency.and_then(|value| value.p90),
                p99: latency.and_then(|value| value.p99),
            },
        })
    }

    fn request(&self, path: &str) -> reqwest::blocking::RequestBuilder {
        let mut headers = HeaderMap::new();
        let token = format!("Bearer {}", self.api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&token).unwrap_or(HeaderValue::from_static("")),
        );

        self.http
            .get(format!("{}/{}", self.base_url, path))
            .headers(headers)
    }
}

fn handle_status(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response, AppError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let error = match status.as_u16() {
        401 | 403 => AppError::Unauthorized,
        429 => AppError::RateLimited,
        _ => AppError::ApiStatus(status.to_string()),
    };

    Err(error)
}

fn select_best_endpoint(
    endpoints: &[crate::models::EndpointEntry],
) -> Option<crate::models::EndpointEntry> {
    endpoints.iter().cloned().max_by(|left, right| {
        let left_score = left
            .throughput_last_30m
            .as_ref()
            .and_then(|value| value.p50)
            .unwrap_or(0.0);
        let right_score = right
            .throughput_last_30m
            .as_ref()
            .and_then(|value| value.p50)
            .unwrap_or(0.0);
        left_score
            .partial_cmp(&right_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}
