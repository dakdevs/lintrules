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
    fn request(&self, evaluation: Evaluation<'_>) -> Result<ProviderRequest>;

    fn validate(&self) -> Result<()> {
        for name in self.required_env() {
            required_env(name)?;
        }
        Ok(())
    }

    fn normalize_response(&self, response: Value) -> Result<Value> {
        Ok(response)
    }

    fn evaluate(&self, client: &Client, evaluation: Evaluation<'_>) -> Result<Value> {
        self.normalize_response(post(client, self.request(evaluation)?)?)
    }
}

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
