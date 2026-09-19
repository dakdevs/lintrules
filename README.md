# Lintrules

`lintrules` is a Rust command-line checker for semantic codebase rules. Put
Markdown rules in `.lintrules/`, configure a Jev provider in
`lintrules.config.json`, then run `lintrules` from the project or a subdirectory.

```sh
cargo install --path crates/lintrules
lintrules init --provider cloudflare
lintrules
```

Or install the npm package, which bundles the Cargo workspace and requires
[Cargo](https://rustup.rs/) when the command runs:

```sh
npm install --global lintrules
lintrules
```

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
