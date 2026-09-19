use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn missing_rules_fail_before_provider_setup() {
    let directory = tempdir().unwrap();
    std::fs::write(
        directory.path().join("lintrules.config.json"),
        "{\"provider\":\"typesafe\"}",
    )
    .unwrap();
    Command::cargo_bin("lintrules")
        .unwrap()
        .current_dir(directory.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("configuration error"));
}

#[test]
fn init_creates_config_and_example_rule() {
    let directory = tempdir().unwrap();
    Command::cargo_bin("lintrules")
        .unwrap()
        .current_dir(directory.path())
        .args(["init", "--provider", "cloudflare"])
        .assert()
        .success()
        .stdout(predicate::str::contains("created lintrules.config.json"));
    assert!(directory.path().join("lintrules.config.json").is_file());
    assert!(directory.path().join(".lintrules/example.md").is_file());
}

#[test]
fn check_requires_the_selected_provider_credential() {
    let directory = tempdir().unwrap();
    std::fs::create_dir(directory.path().join(".lintrules")).unwrap();
    std::fs::write(
        directory.path().join("lintrules.config.json"),
        "{\"provider\":\"typesafe\"}",
    )
    .unwrap();
    std::fs::write(
        directory.path().join(".lintrules/rule.md"),
        "---\ntitle: Rule\nglobs: [\"**/*.rs\"]\n---\n\nCode must be valid.",
    )
    .unwrap();
    Command::cargo_bin("lintrules")
        .unwrap()
        .current_dir(directory.path())
        .env_remove("TYPESAFE_API_KEY")
        .assert()
        .failure()
        .stderr(predicate::str::contains("TYPESAFE_API_KEY must be set"));
}
