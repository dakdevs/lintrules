use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use git2::{DiffDelta, DiffFormat, DiffOptions, Repository};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{
    config::{PrReport, Project, Rule, Thresholds, WorkingTree},
    provider::{Jev, Question},
};

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FindingKind {
    Violation,
    Inconclusive,
    Incomplete,
}

#[derive(Debug, Serialize)]
pub struct Finding {
    pub kind: FindingKind,
    pub title: String,
    pub rule: String,
    pub path: Option<String>,
    pub line: Option<usize>,
    pub probability: Option<f64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SkippedRule {
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub skipped: Vec<SkippedRule>,
}

impl Report {
    #[must_use]
    pub fn failed(&self) -> bool {
        !self.findings.is_empty()
    }
}

#[derive(Debug, Clone)]
struct SourceFile {
    path: String,
    source: String,
}

/// Checks matching files against the rules and collects findings.
///
/// # Errors
/// Returns an error if provider setup, worker creation, Git comparison, source
/// discovery, or glob compilation fails. Evaluation failures become findings.
pub fn check(
    project: &Project,
    rules: &[Rule],
    base: Option<&str>,
    working_tree: Option<WorkingTree>,
    no_cache: bool,
) -> Result<Report> {
    let jev = Jev::new(&project.config, &project.root, no_cache)?;
    let pool = ThreadPoolBuilder::new()
        .num_threads(project.config.max_concurrency)
        .build()
        .context("could not create evaluation worker pool")?;
    let changed = base
        .filter(|_| matches!(project.config.pr_report, PrReport::Introduced))
        .map(|base| {
            changed_lines(
                &project.root,
                base,
                working_tree.unwrap_or(project.config.working_tree),
            )
        })
        .transpose()?;
    let files = source_files(&project.root)?;
    let mut report = Report::default();
    for rule in rules {
        let candidates = matching_files(rule, &files)?;
        if candidates.is_empty() {
            report.skipped.push(SkippedRule {
                title: rule.title.clone(),
                reason: "no files matched globs".to_owned(),
            });
            continue;
        }
        let applicable = qualify(rule, candidates, &project.config, &jev, &mut report);
        if applicable.is_empty() {
            report.skipped.push(SkippedRule {
                title: rule.title.clone(),
                reason: "no files qualified".to_owned(),
            });
            continue;
        }
        let cross_file = infer_cross_file(rule, &jev, &mut report);
        evaluate_rule(
            rule,
            &applicable,
            cross_file,
            &jev,
            &pool,
            &mut report,
            &EvaluateOptions {
                max_context_chars: project.config.max_context_chars,
                changed: changed.as_ref(),
                pr_report: &project.config.pr_report,
                thresholds: &project.config.thresholds,
            },
        );
    }
    Ok(report)
}

fn source_files(root: &Path) -> Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    let mut walker = WalkBuilder::new(root);
    walker
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| entry.file_name() != ".git");
    for entry in walker.build().flatten() {
        if !entry.file_type().is_some_and(|type_| type_.is_file()) {
            continue;
        }
        let Ok(source) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let path = entry
            .path()
            .strip_prefix(root)
            .context("file was outside project root")?
            .to_string_lossy()
            .replace('\\', "/");
        files.push(SourceFile { path, source });
    }
    Ok(files)
}

fn matching_files(rule: &Rule, files: &[SourceFile]) -> Result<Vec<SourceFile>> {
    let mut builder = GlobSetBuilder::new();
    for pattern in &rule.globs {
        builder.add(Glob::new(pattern).with_context(|| format!("invalid glob {pattern:?}"))?);
    }
    let matcher: GlobSet = builder.build()?;
    Ok(files
        .iter()
        .filter(|file| matcher.is_match(&file.path))
        .cloned()
        .collect())
}

fn qualify(
    rule: &Rule,
    files: Vec<SourceFile>,
    config: &crate::config::Config,
    jev: &Jev,
    report: &mut Report,
) -> Vec<SourceFile> {
    let Some(applies_to) = &rule.applies_to else {
        return files;
    };
    let mut qualified = Vec::new();
    for file in files {
        if file.source.len() > config.max_context_chars {
            qualified.push(file);
            continue;
        }
        let mut questions = BTreeMap::new();
        questions.insert(
            "qualifies".to_owned(),
            noul(format!(
                "Does `file` meet this applicability criterion: {applies_to}"
            )),
        );
        match jev.ask(
            &json!({"file": {"path": file.path, "source": numbered(&file.source, 1)}}),
            questions,
        ) {
            Ok(answers) if answers["qualifies"] > config.thresholds.inapplicable => {
                qualified.push(file);
            }
            Ok(_) => {}
            Err(error) => {
                report.findings.push(incomplete(
                    rule,
                    Some(&file),
                    format!("could not qualify file: {error:#}"),
                ));
            }
        }
    }
    qualified
}

fn infer_cross_file(rule: &Rule, jev: &Jev, report: &mut Report) -> bool {
    let mut questions = BTreeMap::new();
    questions.insert("cross_file".to_owned(), noul("Does this rule require comparing or relating more than one file to determine compliance? Answer yes when uncertain."));
    match jev.ask(&json!({"rule": rule.body}), questions) {
        Ok(answers) => answers["cross_file"] >= 0.5,
        Err(error) => {
            report.findings.push(incomplete(
                rule,
                None,
                format!("could not infer rule scope: {error:#}"),
            ));
            true
        }
    }
}

struct EvaluateOptions<'a> {
    max_context_chars: usize,
    changed: Option<&'a ChangedLines>,
    pr_report: &'a PrReport,
    thresholds: &'a Thresholds,
}

fn evaluate_rule(
    rule: &Rule,
    files: &[SourceFile],
    cross_file: bool,
    jev: &Jev,
    pool: &ThreadPool,
    report: &mut Report,
    options: &EvaluateOptions<'_>,
) {
    let full_context = if cross_file {
        let context = files
            .iter()
            .map(|file| json!({"path": file.path, "source": numbered(&file.source, 1)}))
            .collect::<Vec<_>>();
        let state = json!({"rule": rule.body, "files": context});
        if state.to_string().len() <= options.max_context_chars {
            Some(state)
        } else {
            report.findings.push(incomplete(
                rule,
                None,
                "cross-file context exceeds max_context_chars; no source was silently discarded"
                    .to_owned(),
            ));
            None
        }
    } else {
        None
    };
    let findings = pool.install(|| {
        files
            .par_iter()
            .map(|file| {
                let targets = options.changed.map(|changed| changed.includes(&file.path));
                if matches!(options.pr_report, PrReport::Introduced)
                    && options.changed.is_some()
                    && !targets.unwrap_or(false)
                {
                    return Vec::new();
                }
                let state = if let Some(context) = &full_context {
                    context.clone()
                } else if file.source.len() > options.max_context_chars {
                    return vec![incomplete(
                        rule,
                        Some(file),
                        "file exceeds max_context_chars; no source was silently truncated"
                            .to_owned(),
                    )];
                } else {
                    json!({"rule": rule.body, "files": [{"path": file.path, "source": numbered(&file.source, 1)}]})
                };
                evaluate_file(rule, file, &state, jev, options)
            })
            .collect::<Vec<_>>()
    });
    for file_findings in findings {
        report.findings.extend(file_findings);
    }
}

fn evaluate_file(
    rule: &Rule,
    file: &SourceFile,
    state: &Value,
    jev: &Jev,
    options: &EvaluateOptions<'_>,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines = file
        .source
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, _)| index + 1)
        .collect::<Vec<_>>();
    let chunk_size = (options.max_context_chars / 220).clamp(1, 300);
    let mut had_line_violation = false;
    let mut overall = None;
    for line_chunk in lines.chunks(chunk_size) {
        let mut questions = BTreeMap::new();
        if overall.is_none() {
            questions.insert("overall".to_owned(), noul("Does the file in `files` violate the rule in `rule`? Answer yes for any violation, including a required construct that is missing."));
        }
        for line in line_chunk {
            questions.insert(
                format!("line_{line}"),
                noul(format!("Does source line {line} in `files` entry with path `{}` violate the rule in `rule`? Answer yes only when that exact existing line is a violating location. Do not use yes for missing code.", file.path)),
            );
        }
        let answers = match jev.ask(state, questions) {
            Ok(answers) => answers,
            Err(error) => {
                findings.push(incomplete(
                    rule,
                    Some(file),
                    format!("provider evaluation failed: {error:#}"),
                ));
                return findings;
            }
        };
        if let Some(probability) = answers.get("overall") {
            overall = Some(*probability);
        }
        for line in line_chunk {
            let probability = answers[&format!("line_{line}")];
            if !should_report(*line, &file.path, options.changed, options.pr_report) {
                continue;
            }
            if probability >= options.thresholds.violation {
                had_line_violation = true;
                findings.push(finding(
                    FindingKind::Violation,
                    rule,
                    Some(file),
                    Some(*line),
                    Some(probability),
                    "Jev found a violation on this line.",
                ));
            } else if probability > options.thresholds.pass {
                findings.push(finding(
                    FindingKind::Inconclusive,
                    rule,
                    Some(file),
                    Some(*line),
                    Some(probability),
                    "Jev could not determine compliance for this line.",
                ));
            }
        }
    }
    if let Some(probability) = overall {
        if probability >= options.thresholds.violation
            && !had_line_violation
            && should_report_file(&file.path, options.changed, options.pr_report)
        {
            findings.push(finding(
                FindingKind::Violation,
                rule,
                Some(file),
                None,
                Some(probability),
                "Jev found a file-level violation, including a possible missing requirement.",
            ));
        } else if probability > options.thresholds.pass
            && probability < options.thresholds.violation
            && should_report_file(&file.path, options.changed, options.pr_report)
        {
            findings.push(finding(
                FindingKind::Inconclusive,
                rule,
                Some(file),
                None,
                Some(probability),
                "Jev could not determine file-level compliance.",
            ));
        }
    }
    findings
}

fn noul(instructions: impl Into<String>) -> Question {
    Question {
        instructions: instructions.into(),
        criteria: Some(
            json!({"true": "The proposition is true.", "false": "The proposition is false."}),
        ),
    }
}

fn numbered(source: &str, offset: usize) -> String {
    source
        .lines()
        .enumerate()
        .map(|(index, line)| format!("{:>6}: {line}", index + offset))
        .collect::<Vec<_>>()
        .join("\n")
}

fn finding(
    kind: FindingKind,
    rule: &Rule,
    file: Option<&SourceFile>,
    line: Option<usize>,
    probability: Option<f64>,
    message: impl Into<String>,
) -> Finding {
    Finding {
        kind,
        title: rule.title.clone(),
        rule: rule.id.clone(),
        path: file.map(|file| file.path.clone()),
        line,
        probability,
        message: message.into(),
    }
}

fn incomplete(rule: &Rule, file: Option<&SourceFile>, message: impl Into<String>) -> Finding {
    finding(FindingKind::Incomplete, rule, file, None, None, message)
}

fn should_report(
    line: usize,
    path: &str,
    changed: Option<&ChangedLines>,
    pr_report: &PrReport,
) -> bool {
    !matches!(pr_report, PrReport::Introduced)
        || changed.is_none_or(|changed| changed.includes_line(path, line))
}

fn should_report_file(path: &str, changed: Option<&ChangedLines>, pr_report: &PrReport) -> bool {
    !matches!(pr_report, PrReport::Introduced)
        || changed.is_none_or(|changed| changed.includes(path))
}

#[derive(Debug, Default)]
struct ChangedLines {
    lines: BTreeMap<String, BTreeSet<usize>>,
}

impl ChangedLines {
    fn includes(&self, path: &str) -> bool {
        self.lines.contains_key(path)
    }
    fn includes_line(&self, path: &str, line: usize) -> bool {
        self.lines
            .get(path)
            .is_some_and(|lines| lines.contains(&line))
    }
}

fn changed_lines(root: &Path, base: &str, working_tree: WorkingTree) -> Result<ChangedLines> {
    let root = fs::canonicalize(root).context("could not resolve project root")?;
    let repository = Repository::discover(&root).context("could not discover Git repository")?;
    let workdir = fs::canonicalize(
        repository
            .workdir()
            .context("bare Git repositories are not supported")?,
    )
    .context("could not resolve Git working directory")?;
    let head = repository
        .head()
        .context("could not resolve HEAD")?
        .peel_to_commit()
        .context("HEAD does not point to a commit")?;
    let base = repository
        .revparse_single(base)
        .with_context(|| format!("could not resolve base {base:?}"))?
        .peel_to_commit()
        .with_context(|| format!("base {base:?} does not point to a commit"))?;
    let merge_base = repository
        .merge_base(head.id(), base.id())
        .context("could not find a merge base")?;
    let merge_base_tree = repository.find_commit(merge_base)?.tree()?;
    let mut options = DiffOptions::new();
    if matches!(working_tree, WorkingTree::Include) {
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .show_untracked_content(true);
    }
    let diff = match working_tree {
        WorkingTree::Include => repository
            .diff_tree_to_workdir_with_index(Some(&merge_base_tree), Some(&mut options))?,
        WorkingTree::Committed => {
            let head_tree = head.tree()?;
            repository.diff_tree_to_tree(
                Some(&merge_base_tree),
                Some(&head_tree),
                Some(&mut options),
            )?
        }
    };
    let mut changed = ChangedLines::default();
    for delta in diff.deltas() {
        if let Some(path) = project_relative_path(&root, &workdir, &delta) {
            changed.lines.entry(path).or_default();
        }
    }
    diff.print(DiffFormat::Patch, |delta, _, line| {
        if line.origin() == '+'
            && let (Some(path), Some(line_number)) = (
                project_relative_path(&root, &workdir, &delta),
                line.new_lineno(),
            )
        {
            changed
                .lines
                .entry(path)
                .or_default()
                .insert(line_number as usize);
        }
        true
    })?;
    Ok(changed)
}

fn project_relative_path(root: &Path, workdir: &Path, delta: &DiffDelta<'_>) -> Option<String> {
    let path = delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())?;
    workdir
        .join(path)
        .strip_prefix(root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/")
        .into()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use git2::{Commit, Oid, Repository, Signature};
    use tempfile::tempdir;

    use super::{WorkingTree, changed_lines};

    fn commit_tracked_file(repository: &Repository, message: &str, parents: &[&Commit<'_>]) -> Oid {
        let mut index = repository.index().unwrap();
        index.add_path(Path::new("tracked.rs")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repository.find_tree(tree_id).unwrap();
        let signature = Signature::now("Lint Rules", "lint-rules@example.test").unwrap();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                parents,
            )
            .unwrap()
    }

    #[test]
    fn changed_lines_reads_working_tree_and_untracked_files() {
        let directory = tempdir().unwrap();
        let repository = Repository::init(directory.path()).unwrap();
        fs::write(directory.path().join("tracked.rs"), "let value = 1;\n").unwrap();
        commit_tracked_file(&repository, "initial", &[]);
        fs::write(
            directory.path().join("tracked.rs"),
            "let value = 2;\nlet extra = 3;\n",
        )
        .unwrap();
        fs::write(directory.path().join("new.rs"), "let new_file = true;\n").unwrap();

        let changed = changed_lines(directory.path(), "HEAD", WorkingTree::Include).unwrap();

        assert_eq!(changed.lines["tracked.rs"], [1, 2].into_iter().collect());
        assert_eq!(changed.lines["new.rs"], [1].into_iter().collect());
    }

    #[test]
    fn changed_lines_reads_committed_changes() {
        let directory = tempdir().unwrap();
        let repository = Repository::init(directory.path()).unwrap();
        fs::write(directory.path().join("tracked.rs"), "let value = 1;\n").unwrap();
        let initial = commit_tracked_file(&repository, "initial", &[]);
        fs::write(
            directory.path().join("tracked.rs"),
            "let value = 2;\nlet extra = 3;\n",
        )
        .unwrap();
        let initial_commit = repository.find_commit(initial).unwrap();
        commit_tracked_file(&repository, "change", &[&initial_commit]);

        let changed = changed_lines(
            directory.path(),
            &initial.to_string(),
            WorkingTree::Committed,
        )
        .unwrap();

        assert_eq!(changed.lines["tracked.rs"], [1, 2].into_iter().collect());
    }
}
