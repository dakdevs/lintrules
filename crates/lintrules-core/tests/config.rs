use std::fs;

use lintrules_core::config::{ProviderName, discover_project, load_rules, write_init};
use tempfile::tempdir;

#[test]
fn init_creates_a_valid_starter() {
    let directory = tempdir().unwrap();
    write_init(directory.path(), ProviderName::Typesafe).unwrap();
    let rules = load_rules(directory.path()).unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].title, "Example rule");
    let project = discover_project(Some(&directory.path().join("lintrules.config.json"))).unwrap();
    assert_eq!(project.config.max_concurrency, 4);
}

#[test]
fn rules_require_title_and_globs() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join(".lintrules")).unwrap();
    fs::write(
        directory.path().join(".lintrules/missing.md"),
        "---\ntitle: \"\"\n---\n\nA rule.",
    )
    .unwrap();
    let errors = load_rules(directory.path()).unwrap_err();
    assert!(errors[0].contains("nonempty title"));
}

#[test]
fn rules_require_explicit_globs() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join(".lintrules")).unwrap();
    fs::write(
        directory.path().join(".lintrules/missing-globs.md"),
        "---\ntitle: A title\n---\n\nA rule.",
    )
    .unwrap();
    let errors = load_rules(directory.path()).unwrap_err();
    assert!(errors[0].contains("nonempty globs"));
}

#[test]
fn rules_require_a_closing_front_matter_delimiter() {
    let directory = tempdir().unwrap();
    fs::create_dir(directory.path().join(".lintrules")).unwrap();
    fs::write(
        directory.path().join(".lintrules/unclosed.md"),
        "---\ntitle: A title\nglobs: [\"**/*.rs\"]\n\nA rule.",
    )
    .unwrap();
    let errors = load_rules(directory.path()).unwrap_err();
    assert!(errors[0].contains("front matter must end"));
}

#[test]
fn repository_dogfood_rules_and_config_are_valid() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let project =
        lintrules_core::config::discover_project(Some(&root.join("lintrules.config.json")))
            .unwrap();
    let rules = lintrules_core::config::load_rules(root).unwrap();
    assert_eq!(rules.len(), 5);
    assert!(
        rules
            .iter()
            .all(|rule| !rule.title.is_empty() && !rule.globs.is_empty())
    );
    assert!(matches!(
        project.config.provider,
        lintrules_core::config::ProviderName::Vercel
    ));
    assert!(matches!(
        project.config.pr_report,
        lintrules_core::config::PrReport::Introduced
    ));
}
