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

#[test]
fn pr_report_flag_overrides_config_in_both_directions() {
    let directory = tempdir().unwrap();
    std::fs::create_dir(directory.path().join(".lintrules")).unwrap();
    std::fs::write(
        directory.path().join(".lintrules/rule.md"),
        "---\ntitle: Rule\nglobs: [\"no-matching-files.rs\"]\n---\nCode must be valid.\n",
    )
    .unwrap();
    for (configured, flag, succeeds) in [("introduced", "all", true), ("all", "introduced", false)]
    {
        std::fs::write(
            directory.path().join("lintrules.config.json"),
            format!(r#"{{"provider":"typesafe","cache":false,"pr_report":"{configured}"}}"#),
        )
        .unwrap();
        let assertion = Command::cargo_bin("lintrules")
            .unwrap()
            .current_dir(directory.path())
            .env("TYPESAFE_API_KEY", "unused-no-matching-files")
            .args([
                "--pr-report",
                flag,
                "--base",
                "nonexistent-base",
                "--format",
                "json",
            ])
            .assert();
        if succeeds {
            assertion
                .success()
                .stdout(predicate::str::contains("\"findings\": []"));
        } else {
            assertion
                .failure()
                .stderr(predicate::str::contains("Git repository"));
        }
    }
}

#[test]
fn pr_report_rejects_unknown_mode() {
    Command::cargo_bin("lintrules")
        .unwrap()
        .args(["--pr-report", "everything"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}
