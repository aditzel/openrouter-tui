use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Credits {
    pub total_credits: f64,
    pub total_usage: f64,
}

impl Credits {
    pub fn remaining(&self) -> f64 {
        self.total_credits - self.total_usage
    }
}

#[derive(Clone, Debug)]
pub struct ModelPricing {
    pub prompt_per_million: Option<f64>,
    pub completion_per_million: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct ModelSummary {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: Option<u32>,
    pub pricing: ModelPricing,
}

#[derive(Clone, Debug)]
pub struct Throughput {
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Latency {
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct ModelDetails {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_name: Option<String>,
    pub description: Option<String>,
    pub context_length: Option<u32>,
    pub pricing: ModelPricing,
    pub throughput: Throughput,
    pub latency: Latency,
}

#[derive(Deserialize)]
pub struct CreditsResponse {
    pub data: CreditsData,
}

#[derive(Deserialize)]
pub struct CreditsData {
    pub total_credits: f64,
    pub total_usage: f64,
}

#[derive(Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: Option<String>,
    pub context_length: Option<u32>,
    pub pricing: Option<PricingRaw>,
}

#[derive(Deserialize)]
pub struct ModelEndpointsResponse {
    pub data: ModelEndpointsData,
}

#[derive(Deserialize)]
pub struct ModelEndpointsData {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub context_length: Option<u32>,
    pub endpoints: Vec<EndpointEntry>,
}

#[derive(Clone, Deserialize)]
pub struct EndpointEntry {
    pub provider_name: Option<String>,
    pub pricing: Option<PricingRaw>,
    pub throughput_last_30m: Option<ThroughputRaw>,
    pub latency_last_30m: Option<LatencyRaw>,
    pub context_length: Option<u32>,
}

#[derive(Clone, Deserialize)]
pub struct PricingRaw {
    pub prompt: Option<String>,
    pub completion: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct ThroughputRaw {
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
}

#[derive(Clone, Deserialize)]
pub struct LatencyRaw {
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
}

pub fn parse_pricing(pricing: Option<&PricingRaw>) -> ModelPricing {
    let prompt = pricing
        .and_then(|value| value.prompt.as_deref())
        .and_then(parse_price_per_million);
    let completion = pricing
        .and_then(|value| value.completion.as_deref())
        .and_then(parse_price_per_million);

    ModelPricing {
        prompt_per_million: prompt,
        completion_per_million: completion,
    }
}

/// Parse per-token price string and convert to per-million tokens
fn parse_price_per_million(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .map(|per_token| per_token * 1_000_000.0)
}

pub fn split_provider(model_id: &str) -> (String, String) {
    if let Some((provider, rest)) = model_id.split_once('/') {
        (provider.to_string(), rest.to_string())
    } else {
        ("unknown".to_string(), model_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pricing_converts_to_per_million() {
        let raw = PricingRaw {
            prompt: Some("0.0000003".to_string()),
            completion: Some("0.0000005".to_string()),
        };

        let pricing = parse_pricing(Some(&raw));

        // 0.0000003 per token * 1_000_000 = $0.30 per million
        assert!((pricing.prompt_per_million.unwrap() - 0.30).abs() < 0.0001);
        // 0.0000005 per token * 1_000_000 = $0.50 per million
        assert!((pricing.completion_per_million.unwrap() - 0.50).abs() < 0.0001);
    }

    #[test]
    fn test_parse_pricing_handles_none() {
        let pricing = parse_pricing(None);
        assert!(pricing.prompt_per_million.is_none());
        assert!(pricing.completion_per_million.is_none());
    }

    #[test]
    fn test_parse_pricing_handles_zero() {
        let raw = PricingRaw {
            prompt: Some("0".to_string()),
            completion: Some("0".to_string()),
        };

        let pricing = parse_pricing(Some(&raw));

        assert_eq!(pricing.prompt_per_million.unwrap(), 0.0);
        assert_eq!(pricing.completion_per_million.unwrap(), 0.0);
    }
}
