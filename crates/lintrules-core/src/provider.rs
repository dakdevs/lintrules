use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result};
use lintrules_provider::{Evaluation, JevProvider};
use lintrules_provider_cloudflare::Cloudflare;
use lintrules_provider_typesafe::Typesafe;
use lintrules_provider_vercel::Vercel;
use reqwest::blocking::Client;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::config::{Config, ProviderName, Thresholds};

#[derive(Debug, Clone, Serialize)]
pub struct Question {
    pub instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<Value>,
}

fn provider(name: &ProviderName) -> Arc<dyn JevProvider> {
    match name {
        ProviderName::Typesafe => Arc::new(Typesafe),
        ProviderName::Cloudflare => Arc::new(Cloudflare),
        ProviderName::Vercel => Arc::new(Vercel),
    }
}

pub struct Jev {
    client: Client,
    provider: Arc<dyn JevProvider>,
    model: String,
    cache: Option<Arc<Mutex<Cache>>>,
    thresholds: Thresholds,
}

#[derive(Debug, Clone)]
struct Cache {
    path: std::path::PathBuf,
    entries: BTreeMap<String, Value>,
}

impl Jev {
    /// Creates a provider client with optional persistent caching.
    ///
    /// # Errors
    /// Returns an error for invalid credentials, unreadable cache, or client setup failure.
    pub fn new(config: &Config, root: &Path, no_cache: bool) -> Result<Self> {
        let provider = provider(&config.provider);
        provider.validate()?;
        let model = config
            .model
            .clone()
            .unwrap_or_else(|| provider.default_model().to_owned());
        let cache = if config.cache && !no_cache {
            Some(Arc::new(Mutex::new(Cache::load(
                &root.join(".lintrules/.cache.json"),
            )?)))
        } else {
            None
        };
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(45)).build()?,
            provider,
            model,
            cache,
            thresholds: config.thresholds.clone(),
        })
    }

    /// Evaluates questions against the supplied state, using cached results when available.
    ///
    /// # Errors
    /// Returns an error for serialization, provider, response parsing, or cache failures.
    pub fn ask(
        &self,
        state: &Value,
        questions: BTreeMap<String, Question>,
    ) -> Result<BTreeMap<String, f64>> {
        let questions = serde_json::to_value(questions)?;
        let request = json!({
            "provider": self.provider.id(),
            "model": self.model,
            "state": state,
            "questions": questions,
        });
        let key = digest(&request)?;
        if let Some(cached) = self.cache.as_ref().and_then(|cache| {
            cache
                .lock()
                .ok()
                .and_then(|cache| cache.entries.get(&key).cloned())
        }) {
            return probabilities(&cached);
        }
        let response = self.provider.evaluate(
            &self.client,
            Evaluation {
                model: &self.model,
                state: request["state"].clone(),
                questions: request["questions"].clone(),
            },
        )?;
        let parsed = probabilities(&response)?;
        if !parsed.values().any(|probability| {
            *probability > self.thresholds.pass && *probability < self.thresholds.violation
        }) && let Some(cache) = &self.cache
        {
            let mut cache = cache
                .lock()
                .map_err(|_| anyhow::anyhow!("evaluation cache lock was poisoned"))?;
            cache.entries.insert(key, response);
            cache.save()?;
        }
        Ok(parsed)
    }
}

fn probabilities(response: &Value) -> Result<BTreeMap<String, f64>> {
    let answers = response
        .get("answers")
        .and_then(Value::as_object)
        .context("provider response has no answers object")?;
    let mut result = BTreeMap::new();
    for (id, answer) in answers {
        let probability = ["noul", "boolean", "probability"]
            .iter()
            .find_map(|key| answer.get(*key).and_then(Value::as_f64))
            .with_context(|| format!("answer {id} has no boolean probability"))?;
        result.insert(id.to_owned(), probability);
    }
    Ok(result)
}

impl Cache {
    fn load(path: &Path) -> Result<Self> {
        let entries = if path.exists() {
            serde_json::from_str(&fs::read_to_string(path)?).unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        Ok(Self {
            path: path.to_path_buf(),
            entries,
        })
    }

    fn save(&self) -> Result<()> {
        let text = serde_json::to_string(&self.entries)?;
        fs::write(&self.path, text)?;
        Ok(())
    }
}

fn digest(value: &Value) -> Result<String> {
    let encoded = serde_json::to_vec(value)?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

#[cfg(test)]
mod tests {
    use super::{probabilities, provider};
    use crate::config::ProviderName;
    use serde_json::json;

    #[test]
    fn probabilities_accepts_provider_formats_without_consuming_response() {
        let response = json!({"answers": {
            "a": {"noul": 0.0},
            "b": {"boolean": 0.5},
            "c": {"probability": 1.0}
        }});
        let parsed = probabilities(&response).unwrap();
        assert_eq!(
            parsed,
            [
                ("a".to_owned(), 0.0),
                ("b".to_owned(), 0.5),
                ("c".to_owned(), 1.0)
            ]
            .into()
        );
        assert_eq!(probabilities(&response).unwrap(), parsed);
    }

    #[test]
    fn probabilities_rejects_missing_or_invalid_answers() {
        for response in [
            json!({}),
            json!({"answers": []}),
            json!({"answers": {"a": {"noul": "yes"}}}),
        ] {
            assert!(probabilities(&response).is_err());
        }
    }

    #[test]
    fn provider_registry_defines_the_default_models() {
        assert_eq!(
            provider(&ProviderName::Typesafe).default_model(),
            "jev-latest"
        );
        assert_eq!(
            provider(&ProviderName::Cloudflare).default_model(),
            "typesafe/jev"
        );
        assert_eq!(
            provider(&ProviderName::Vercel).default_model(),
            "typesafe-ai/jev"
        );
    }
}
