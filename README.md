# Lintrules

`lintrules` is a Rust command-line checker for semantic codebase rules. Put
Markdown rules in `.lintrules/`, configure a Jev provider in
`lintrules.config.json`, then run `lintrules` from the project or a subdirectory.

```sh
cargo install --path crates/lintrules
lintrules init --provider cloudflare
lintrules
```

Install directly from this repository with Cargo:

```sh
cargo install --git https://github.com/dakdevs/lintrules.git --locked --package lintrules
```

Run directly with Bun (no Rust or Cargo installation required):

```sh
bunx @dakdevs/lintrules
```

Run it from a project containing `lintrules.config.json` and `.lintrules/`, with
the provider credentials exported in your environment. The command also finds
the configuration from subdirectories. To create starter files, run
`bunx @dakdevs/lintrules init --provider vercel`.

The npm package includes prebuilt binaries for macOS and Linux (x64 and arm64)
and Windows (x64). Linux binaries use musl and also work on Alpine. No install
scripts or compiler are needed. You can also install it globally:

```sh
npm install --global @dakdevs/lintrules
lintrules
```

## GitHub Action

The repository root is a composite GitHub Action. It runs Lintrules once,
converts each finding into a GitHub error annotation, and preserves the precise
line when Jev identified one. File-level findings create file annotations.

```yaml
name: Lintrules

on:
  pull_request:

permissions:
  contents: read

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 0
      - uses: dakdevs/lintrules@main
        with:
          base: ${{ github.event.pull_request.base.sha }}
        env:
          AI_GATEWAY_API_KEY: ${{ secrets.AI_GATEWAY_API_KEY }}
```

Set `fail-on-findings: "false"` to keep annotations while allowing the job to
succeed. Use a release tag such as `dakdevs/lintrules@v1.0.0` once a release
exists instead of tracking `main`.

## Releases

Run **Publish npm release** from the Actions tab with the semantic version to
publish; it defaults to `1.0.1`. Add an `NPM_TOKEN` repository secret from the
npm account that owns `@dakdevs/lintrules`. The workflow builds and tests every supported binary, publishes, commits the
version, tags it, and creates the matching GitHub release.

The repository is a Cargo workspace: `crates/lintrules` contains the CLI and
`crates/lintrules-core` contains rule loading, scanning, caching, and provider
selection. The provider contract is `crates/lintrules-provider`; each adapter
has its own crate: `lintrules-provider-typesafe`,
`lintrules-provider-cloudflare`, and `lintrules-provider-vercel`.

To add a provider, create an adapter crate that implements `JevProvider`, add
its configured name to `ProviderName`, and register it in core's `provider()`.
Each adapter declares its default model and required environment variables,
builds its provider-specific request, and normalizes its response. The provider
contract owns HTTP status handling; the core crate owns caching and Jev
probability parsing.

Each rule requires `title` and `globs` in YAML front matter. `applies-to` is
optional. A rule body should normally describe one requirement, although the
checker accepts any Markdown prose.

```md
---
title: "Forms display inline validation errors"
globs:
  - "src/**/*.tsx"
applies-to: "Components that render forms."
---

Display validation errors beside the relevant fields.
```

```json
{
  "provider": "cloudflare",
  "pr_report": "introduced",
  "working_tree": "include",
  "max_concurrency": 4,
  "cache": true,
  "thresholds": {
    "violation": 0.8,
    "pass": 0.2,
    "inapplicable": 0.1
  }
}
```

Providers read credentials from the environment:

| Provider     | Required environment variables                  | Jev model         |
| ------------ | ----------------------------------------------- | ----------------- |
| `typesafe`   | `TYPESAFE_API_KEY`                              | `jev-latest`      |
| `cloudflare` | `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_API_TOKEN` | `typesafe/jev`    |
| `vercel`     | `AI_GATEWAY_API_KEY`                            | `typesafe-ai/jev` |

Use `--base origin/main` to report only introduced findings, `--working-tree
committed` to ignore staged and unstaged work, `--format json` for CI, and
`--no-cache` to force fresh provider calls.

`max_concurrency` bounds independent Jev evaluations and defaults to `4`.
Lintrules still waits for every scheduled evaluation and aggregates all findings.

The checker respects `.gitignore` and never scans `.git/`. It validates every
rule before any provider call, evaluates every valid check before returning, and
fails on violations, inconclusive checks, or incomplete provider calls.

Jev has a finite context window. Lintrules never truncates source invisibly:
when complete per-file or cross-file source cannot fit the configured
`max_context_chars`, it reports an incomplete check rather than a false pass.
