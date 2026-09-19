use anyhow::Result;
use lintrules_provider::{Evaluation, JevProvider, ProviderRequest, required_env};
use serde_json::{Value, json};

pub struct Cloudflare;

impl JevProvider for Cloudflare {
    fn id(&self) -> &'static str {
        "cloudflare"
    }

    fn default_model(&self) -> &'static str {
        "typesafe/jev"
    }

    fn required_env(&self) -> &'static [&'static str] {
        &["CLOUDFLARE_ACCOUNT_ID", "CLOUDFLARE_API_TOKEN"]
    }

    fn request(&self, evaluation: Evaluation<'_>) -> Result<ProviderRequest> {
        let account = required_env("CLOUDFLARE_ACCOUNT_ID")?;
        Ok(ProviderRequest {
            url: format!("https://api.cloudflare.com/client/v4/accounts/{account}/ai/run"),
            token: required_env("CLOUDFLARE_API_TOKEN")?,
            body: json!({
                "model": evaluation.model,
                "input": {
                    "state": evaluation.state,
                    "questions": evaluation.questions,
                },
            }),
            headers: Vec::new(),
        })
    }

    fn normalize_response(&self, response: Value) -> Result<Value> {
        Ok(response.get("result").cloned().unwrap_or(response))
    }
}
