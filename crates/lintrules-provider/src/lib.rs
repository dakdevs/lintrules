use std::env;

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde_json::Value;

pub struct Evaluation<'a> {
    pub model: &'a str,
    pub state: Value,
    pub questions: Value,
}

pub struct ProviderRequest {
    pub url: String,
    pub token: String,
    pub body: Value,
    pub headers: Vec<Header>,
}

pub struct Header {
    pub name: &'static str,
    pub value: String,
}

pub trait JevProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn default_model(&self) -> &'static str;
    fn required_env(&self) -> &'static [&'static str];
    /// Builds the provider request.
    ///
    /// # Errors
    /// Returns an error if credentials or provider configuration are invalid.
    fn request(&self, evaluation: Evaluation<'_>) -> Result<ProviderRequest>;

    /// Checks that required provider credentials are present.
    ///
    /// # Errors
    /// Returns an error if a required environment variable is missing or invalid.
    fn validate(&self) -> Result<()> {
        for name in self.required_env() {
            required_env(name)?;
        }
        Ok(())
    }

    /// Converts a provider response into the shared Jev response format.
    ///
    /// # Errors
    /// Implementations may reject malformed provider responses.
    fn normalize_response(&self, response: Value) -> Result<Value> {
        Ok(response)
    }

    /// Sends an evaluation request and normalizes its response.
    ///
    /// # Errors
    /// Returns an error for invalid credentials, transport failures, unsuccessful
    /// HTTP responses, or malformed response bodies.
    fn evaluate(&self, client: &Client, evaluation: Evaluation<'_>) -> Result<Value> {
        self.normalize_response(post(client, self.request(evaluation)?)?)
    }
}

/// Reads a required provider environment variable.
///
/// # Errors
/// Returns an error when the variable is absent or not valid Unicode.
pub fn required_env(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("{name} must be set for the selected provider"))
}

fn post(client: &Client, request: ProviderRequest) -> Result<Value> {
    let ProviderRequest {
        url,
        token,
        body,
        headers,
    } = request;
    let mut request = client.post(url).bearer_auth(token).json(&body);
    for header in headers {
        request = request.header(header.name, header.value);
    }
    let response = request.send().context("could not reach Jev provider")?;
    let status = response.status();
    let body: Value = response.json().context("provider returned invalid JSON")?;
    if !status.is_success() {
        bail!("provider returned HTTP {status}");
    }
    Ok(body)
}
