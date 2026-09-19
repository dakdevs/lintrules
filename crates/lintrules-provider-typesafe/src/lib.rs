use anyhow::Result;
use lintrules_provider::{Evaluation, JevProvider, ProviderRequest, required_env};
use serde_json::json;

pub struct Typesafe;

impl JevProvider for Typesafe {
    fn id(&self) -> &'static str {
        "typesafe"
    }

    fn default_model(&self) -> &'static str {
        "jev-latest"
    }

    fn required_env(&self) -> &'static [&'static str] {
        &["TYPESAFE_API_KEY"]
    }

    fn request(&self, evaluation: Evaluation<'_>) -> Result<ProviderRequest> {
        Ok(ProviderRequest {
            url: "https://api.typesafe.ai/v1/systemone".to_owned(),
            token: required_env("TYPESAFE_API_KEY")?,
            body: json!({
                "model": evaluation.model,
                "state": evaluation.state,
                "questions": evaluation.questions,
            }),
            headers: Vec::new(),
        })
    }
}
