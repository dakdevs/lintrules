use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use gray_matter::{Matter, engine::YAML};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderName {
    Typesafe,
    Cloudflare,
    Vercel,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrReport {
    Introduced,
    All,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkingTree {
    Include,
    Committed,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thresholds {
    #[serde(default = "default_violation")]
    pub violation: f64,
    #[serde(default = "default_pass")]
    pub pass: f64,
    #[serde(default = "default_inapplicable")]
    pub inapplicable: f64,
}

fn default_violation() -> f64 {
    0.8
}
fn default_pass() -> f64 {
    0.2
}
fn default_inapplicable() -> f64 {
    0.1
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            violation: default_violation(),
            pass: default_pass(),
            inapplicable: default_inapplicable(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub provider: ProviderName,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub thresholds: Thresholds,
    #[serde(default = "default_pr_report")]
    pub pr_report: PrReport,
    #[serde(default = "default_working_tree")]
    pub working_tree: WorkingTree,
    #[serde(default = "default_cache")]
    pub cache: bool,
    #[serde(default = "default_context_chars")]
    pub max_context_chars: usize,
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: usize,
}

fn default_pr_report() -> PrReport {
    PrReport::Introduced
}
fn default_working_tree() -> WorkingTree {
    WorkingTree::Include
}
fn default_cache() -> bool {
    true
}
fn default_context_chars() -> usize {
    100_000
}
fn default_max_concurrency() -> usize {
    4
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.thresholds.pass)
            || !(0.0..=1.0).contains(&self.thresholds.violation)
            || !(0.0..=1.0).contains(&self.thresholds.inapplicable)
        {
            bail!("thresholds must be between 0 and 1");
        }
        if self.thresholds.pass >= self.thresholds.violation {
            bail!("thresholds.pass must be lower than thresholds.violation");
        }
        if self.max_context_chars == 0 {
            bail!("max_context_chars must be greater than zero");
        }
        if self.max_concurrency == 0 {
            bail!("max_concurrency must be greater than zero");
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub config: Config,
}

pub fn discover_project(config_path: Option<&Path>) -> Result<Project> {
    let config_path = match config_path {
        Some(path) => fs::canonicalize(path)
            .with_context(|| format!("could not read configuration at {}", path.display()))?,
        None => find_config(&std::env::current_dir()?)?,
    };
    let text = fs::read_to_string(&config_path)
        .with_context(|| format!("could not read {}", config_path.display()))?;
    let config: Config = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON", config_path.display()))?;
    config.validate()?;
    let root = config_path
        .parent()
        .context("configuration has no parent directory")?
        .to_path_buf();
    Ok(Project {
        root,
        config_path,
        config,
    })
}

fn find_config(start: &Path) -> Result<PathBuf> {
    for directory in start.ancestors() {
        let candidate = directory.join("lintrules.config.json");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!("could not find lintrules.config.json; run `lintrules init --provider <provider>`")
}

#[derive(Debug, Deserialize)]
struct RuleFrontmatter {
    title: Option<String>,
    globs: Option<Vec<String>>,
    #[serde(rename = "applies-to")]
    applies_to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Rule {
    pub id: String,
    pub path: PathBuf,
    pub title: String,
    pub globs: Vec<String>,
    pub applies_to: Option<String>,
    pub body: String,
}

pub fn load_rules(root: &Path) -> Result<Vec<Rule>, Vec<String>> {
    let directory = root.join(".lintrules");
    if !directory.is_dir() {
        return Err(vec![format!("{} is missing", directory.display())]);
    }
    let entries = match fs::read_dir(&directory) {
        Ok(entries) => entries,
        Err(error) => {
            return Err(vec![format!(
                "could not read {}: {error}",
                directory.display()
            )]);
        }
    };
    let mut rules = Vec::new();
    let mut errors = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|extension| extension != "md") {
            continue;
        }
        match parse_rule(&path) {
            Ok(rule) => rules.push(rule),
            Err(error) => errors.push(format!("{}: {error:#}", path.display())),
        }
    }
    if rules.is_empty() && errors.is_empty() {
        errors.push(format!(
            "{} contains no Markdown rules",
            directory.display()
        ));
    }
    if errors.is_empty() {
        Ok(rules)
    } else {
        Err(errors)
    }
}

fn parse_rule(path: &Path) -> Result<Rule> {
    let text = fs::read_to_string(path).with_context(|| "could not read rule")?;
    if text
        .lines()
        .next()
        .is_none_or(|line| line.trim_end() != "---")
    {
        bail!("front matter must begin with ---");
    }
    if !text.lines().skip(1).any(|line| line.trim_end() == "---") {
        bail!("front matter must end with ---");
    }
    let matter = Matter::<YAML>::new();
    let parsed = matter
        .parse::<RuleFrontmatter>(&text)
        .context("front matter must be valid YAML")?;
    let front_matter = parsed.data.context("front matter must not be empty")?;
    let title = front_matter.title.unwrap_or_default().trim().to_owned();
    if title.is_empty() {
        bail!("front matter requires a nonempty title");
    }
    let globs = front_matter.globs.unwrap_or_default();
    if globs.is_empty() || globs.iter().any(|glob| glob.trim().is_empty()) {
        bail!("front matter requires a nonempty globs array");
    }
    if parsed.content.trim().is_empty() {
        bail!("rule body must not be empty");
    }
    let id = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("rule")
        .to_owned();
    Ok(Rule {
        id,
        path: path.to_path_buf(),
        title,
        globs,
        applies_to: front_matter.applies_to,
        body: parsed.content.trim().to_owned(),
    })
}

pub fn write_init(root: &Path, provider: ProviderName) -> Result<()> {
    let config = root.join("lintrules.config.json");
    let rules = root.join(".lintrules");
    if config.exists() || rules.exists() {
        bail!("refusing to overwrite existing lintrules.config.json or .lintrules/");
    }
    fs::create_dir(&rules).context("could not create .lintrules")?;
    let config_content = serde_json::to_string_pretty(&Config {
        provider,
        model: None,
        thresholds: Thresholds::default(),
        pr_report: PrReport::Introduced,
        working_tree: WorkingTree::Include,
        cache: true,
        max_context_chars: default_context_chars(),
        max_concurrency: default_max_concurrency(),
    })?;
    fs::write(config, format!("{config_content}\n"))?;
    fs::write(
        rules.join("example.md"),
        "---\ntitle: \"Example rule\"\nglobs:\n  - \"src/**/*.rs\"\napplies-to: \"Rust source files that define public functions.\"\n---\n\nPublic functions should have documentation comments explaining their purpose.\n",
    )?;
    Ok(())
}
