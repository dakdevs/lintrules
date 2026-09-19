use std::{path::PathBuf, process::ExitCode};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use lintrules_core::{
    config::{PrReport, ProviderName, WorkingTree, discover_project, load_rules, write_init},
    scan::{Report, check},
};

#[derive(Parser)]
#[command(
    name = "lintrules",
    version,
    about = "Evaluate Markdown code rules with Jev"
)]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    base: Option<String>,
    #[arg(long, global = true, value_enum)]
    working_tree: Option<WorkingTreeArg>,
    /// Report all findings or only findings introduced relative to --base.
    #[arg(long, global = true, value_enum)]
    pr_report: Option<PrReportArg>,
    #[arg(long, global = true)]
    no_cache: bool,
    #[arg(long, global = true, value_enum, default_value_t = Format::Terminal)]
    format: Format,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Init {
        #[arg(long, value_enum)]
        provider: ProviderArg,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum ProviderArg {
    Typesafe,
    Cloudflare,
    Vercel,
}

#[derive(Clone, Copy, ValueEnum)]
enum WorkingTreeArg {
    Include,
    Committed,
}

#[derive(Clone, Copy, ValueEnum)]
enum PrReportArg {
    Introduced,
    All,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Terminal,
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(success) => {
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("lintrules: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Init { provider }) => {
            write_init(
                &std::env::current_dir()?,
                match provider {
                    ProviderArg::Typesafe => ProviderName::Typesafe,
                    ProviderArg::Cloudflare => ProviderName::Cloudflare,
                    ProviderArg::Vercel => ProviderName::Vercel,
                },
            )?;
            println!("created lintrules.config.json and .lintrules/example.md");
            Ok(true)
        }
        None => {
            let mut project = discover_project(cli.config.as_deref())?;
            if let Some(report) = cli.pr_report {
                project.config.pr_report = match report {
                    PrReportArg::Introduced => PrReport::Introduced,
                    PrReportArg::All => PrReport::All,
                };
            }
            let rules = match load_rules(&project.root) {
                Ok(rules) => rules,
                Err(errors) => {
                    if matches!(cli.format, Format::Json) {
                        println!("{}", serde_json::json!({"configuration_errors": errors}));
                    } else {
                        for error in errors {
                            eprintln!("configuration error: {error}");
                        }
                    }
                    return Ok(false);
                }
            };
            let working_tree = cli.working_tree.map(|value| match value {
                WorkingTreeArg::Include => WorkingTree::Include,
                WorkingTreeArg::Committed => WorkingTree::Committed,
            });
            let report = check(
                &project,
                &rules,
                cli.base.as_deref(),
                working_tree,
                cli.no_cache,
            )?;
            print_report(&report, cli.format);
            Ok(!report.failed())
        }
    }
}

fn print_report(report: &Report, format: Format) {
    if matches!(format, Format::Json) {
        println!(
            "{}",
            serde_json::to_string_pretty(report).expect("report serializes")
        );
        return;
    }
    for finding in &report.findings {
        let location = match (&finding.path, finding.line) {
            (Some(path), Some(line)) => format!("{path}:{line}"),
            (Some(path), None) => path.to_owned(),
            (None, _) => "<rule>".to_owned(),
        };
        let probability = finding
            .probability
            .map(|value| format!(" ({value:.2})"))
            .unwrap_or_default();
        println!(
            "{}: {} [{}]{} — {}",
            location,
            finding.title,
            serde_json::to_string(&finding.kind)
                .unwrap()
                .trim_matches('"'),
            probability,
            finding.message
        );
    }
    for skipped in &report.skipped {
        println!("skipped: {} — {}", skipped.title, skipped.reason);
    }
    if report.findings.is_empty() {
        println!("lintrules: no findings");
    }
}
