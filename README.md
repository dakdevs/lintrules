# Lintrules

Write codebase rules in Markdown. Lintrules uses Jev to check matching files and
report violations with file paths and line numbers.

## Quick start

Create a config and an example rule:

```sh
bunx @dakdevs/lintrules init --provider vercel
```

Set `AI_GATEWAY_API_KEY` in your environment, edit the rule in `.lintrules/`, then
run:

```sh
bunx @dakdevs/lintrules
```

Lintrules finds `lintrules.config.json` in the current directory or a parent.
The npm package includes native binaries for macOS and Linux on x64 and arm64,
and Windows on x64. It does not require Cargo or install scripts.

For a global installation:

```sh
npm install --global @dakdevs/lintrules
lintrules
```

## Rules

Each Markdown file in `.lintrules/` needs a `title` and a nonempty `globs` array
in YAML front matter. Use `"**/**"` to match all files. Missing globs are an error.

```md
---
title: Forms display inline validation errors
globs:
  - "src/**/*.tsx"
applies-to: Components that render forms.
---

Display validation errors beside the relevant fields.
```

Lintrules matches the globs first. If a rule has `applies-to`, Jev uses it to
select which matching files to check. The Markdown body describes the requirement.
One requirement per file is recommended, but not required.

The scanner respects `.gitignore` and excludes `.git/`. It reports all findings
before exiting with an error. Violations, inconclusive results, and failed
provider calls all fail the check. If the required source exceeds
`max_context_chars`, the check is incomplete.

## Configuration

A minimal `lintrules.config.json`:

```json
{
  "provider": "vercel"
}
```

Providers read credentials from environment variables:

| Provider     | Required variables                              | Default model     |
| ------------ | ----------------------------------------------- | ----------------- |
| `vercel`     | `AI_GATEWAY_API_KEY`                            | `typesafe-ai/jev` |
| `cloudflare` | `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_API_TOKEN` | `typesafe/jev`    |
| `typesafe`   | `TYPESAFE_API_KEY`                              | `jev-latest`      |

Optional settings and their defaults:

```json
{
  "provider": "vercel",
  "pr_report": "introduced",
  "working_tree": "include",
  "max_concurrency": 4,
  "max_context_chars": 100000,
  "cache": true,
  "thresholds": {
    "violation": 0.8,
    "pass": 0.2,
    "inapplicable": 0.1
  }
}
```

Set `model` to override the provider's default model.

Without `--base`, Lintrules checks the whole repository. With `--base`,
`pr_report: "introduced"` limits findings to changed files and lines.
Set `pr_report` to `"all"` to include existing findings.

| CLI option                               | Purpose                                            |
| ---------------------------------------- | -------------------------------------------------- |
| `--pr-report all`                        | Report existing findings and override the config.  |
| `--base origin/main`                     | Compare against the merge base with `origin/main`. |
| `--working-tree committed`               | Compare committed changes when using `--base`.     |
| `--config path/to/lintrules.config.json` | Use a specific config.                             |
| `--format json`                          | Write a JSON report.                               |
| `--no-cache`                             | Make fresh provider calls.                         |

## GitHub action

Add `.lintrules/` and `lintrules.config.json` to your repository. For Vercel, add
an `AI_GATEWAY_API_KEY` repository secret, then create
`.github/workflows/lintrules.yml`:

```yaml
name: Lintrules

on:
  pull_request:

permissions:
  contents: read

jobs:
  check:
    if: github.event.pull_request.head.repo.full_name == github.repository && github.actor != 'dependabot[bot]'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: dakdevs/lintrules@v1
        env:
          AI_GATEWAY_API_KEY: ${{ secrets.AI_GATEWAY_API_KEY }}
```

This workflow skips fork and Dependabot PRs because they cannot access the
provider secret. The action installs a prebuilt binary and reports findings as
line or file annotations. By default, it uses the PR base to report introduced
findings.

To include existing violations across the repository:

```yaml
- uses: dakdevs/lintrules@v1
  with:
    pr_report: all
  env:
    AI_GATEWAY_API_KEY: ${{ secrets.AI_GATEWAY_API_KEY }}
```

`pr_report` accepts `introduced` or `all` and passes the value as `--pr-report`.
It defaults to `introduced` and overrides the project config. With `all`, the
CLI reports existing findings without a Git base comparison. Set
`fail-on-findings: "false"` to keep annotations without failing the job.

The action is available on [GitHub Marketplace](https://github.com/marketplace/actions/lintrules).
`@v1` tracks action updates. Pin a release tag or commit SHA to use a fixed version.

## Development

Build from source:

```sh
cargo install --git https://github.com/dakdevs/lintrules.git --locked --package lintrules
```

The Rust workspace lives in `crates/`. `lintrules` contains the CLI,
`lintrules-core` loads rules and runs checks, and `lintrules-provider` defines
`JevProvider`. Each provider has its own adapter crate.

To add a provider, implement `JevProvider` in a new crate, add its name to
`ProviderName`, and register it in the core crate's `provider()` function.
The adapter declares credentials and a default model, builds requests, and
normalizes responses.

To publish an npm release, set the repository's `NPM_TOKEN` secret and run
**Publish npm release** from the Actions tab with the next version. The workflow
builds and tests all supported binaries, publishes the package, and creates a
GitHub release.
