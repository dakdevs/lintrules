use anyhow::Result;
use lintrules_provider::{Evaluation, Header, JevProvider, ProviderRequest, required_env};
use serde_json::{Value, json};

pub struct Vercel;

impl JevProvider for Vercel {
    fn id(&self) -> &'static str {
        "vercel"
    }

    fn default_model(&self) -> &'static str {
        "typesafe-ai/jev"
    }

    fn required_env(&self) -> &'static [&'static str] {
        &["AI_GATEWAY_API_KEY"]
    }

    fn request(&self, evaluation: Evaluation<'_>) -> Result<ProviderRequest> {
        Ok(ProviderRequest {
            url: "https://ai-gateway.vercel.sh/v4/ai/evaluation-model".to_owned(),
            token: required_env("AI_GATEWAY_API_KEY")?,
            body: json!({
                "state": evaluation.state,
                "questions": boolean_questions(evaluation.questions),
            }),
            headers: vec![
                Header {
                    name: "ai-gateway-protocol-version",
                    value: "0.0.1".to_owned(),
                },
                Header {
                    name: "ai-gateway-auth-method",
                    value: "api-key".to_owned(),
                },
                Header {
                    name: "ai-evaluation-model-specification-version",
                    value: "4".to_owned(),
                },
                Header {
                    name: "ai-model-id",
                    value: evaluation.model.to_owned(),
                },
            ],
        })
    }
}

fn boolean_questions(mut questions: Value) -> Value {
    if let Some(questions) = questions.as_object_mut() {
        for question in questions.values_mut() {
            question["type"] = Value::String("boolean".to_owned());
        }
    }
    questions
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::boolean_questions;

    #[test]
    fn converts_noul_questions_to_vercel_boolean_questions() {
        let questions = boolean_questions(json!({
            "complies": {
                "instructions": "Does this file comply?",
                "criteria": {"true": "Complies", "false": "Does not comply"},
            },
        }));

        assert_eq!(questions["complies"]["type"], "boolean");
    }
}
