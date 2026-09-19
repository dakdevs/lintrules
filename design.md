# Lintrules

Lintrules is a Rust command-line tool that evaluates Markdown rules against a
codebase using Jev. This document records its product behavior and current
implementation boundaries.

## Rules

Load Markdown files directly inside `.lintrules/`. Do not discover nested rules.
Recommend one independently checkable requirement per file, but accept arbitrary
rule prose without enforcing that recommendation.

```markdown
---
title: "Forms display inline validation errors"
globs:
  - "src/**/*.tsx"
applies-to: "Components that render forms."
---

Display validation errors beside the relevant fields.
```

- Require a nonempty `title` and a nonempty array of file globs.
- Require explicit globs even for universal rules, such as `["**/**"]`.
- Interpret globs relative to the project root.
- Support an optional natural-language `applies-to` field in front matter.
- Evaluate the Markdown body as the rule.
- Infer per-file or cross-file evaluation from rule text using Jev. If uncertain,
  use cross-file evaluation. Do not expose a scope field.
- Do not provide exclusions or inline suppression mechanisms in v1.

## Configuration and discovery

Use `lintrules.config.json`. Select the provider explicitly from `vercel`,
`cloudflare`, or `typesafe`. Read credentials and provider-specific connection
settings from environment variables; do not store API keys in configuration.

Search upward from the invocation directory for the nearest configuration file.
Its directory is the project root. Support `--config <path>` as an override.

Validate configuration and every rule before making provider calls. Report all
configuration errors together and stop without scanning if any are invalid.
A missing or empty rules directory is a setup error.

`lintrules init` creates configuration and an example rule, requires an explicit
provider choice, and never overwrites existing files.

The provider transports are explicit: TypeSafe uses `TYPESAFE_API_KEY` with
`https://api.typesafe.ai/v1/systemone`; Cloudflare uses
`CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_API_TOKEN` with the Workers AI
`typesafe/jev` endpoint; Vercel uses `AI_GATEWAY_API_KEY` and the
`typesafe-ai/jev` evaluation-model protocol.

## Scan scope

Running the CLI with no arguments scans the whole working codebase, including
committed files, staged and unstaged edits, and untracked files. Respect
`.gitignore` and always exclude `.git/`.

Support a committed-only setting in configuration and a CLI override. CLI
settings take precedence over configuration.

`--base <git-ref>` enables comparison against the merge base of HEAD and the
specified ref. Default to reporting introduced violations. Allow configuration
to report all violations in affected files instead.

Evaluate both baseline and current source using the same current rules. Adding
or changing a rule does not itself make an existing code violation introduced.
Comparisons must distinguish separate violations within a file; a boolean
before/after verdict per file is insufficient.

## Qualification and evaluation

1. Resolve a rule's globs in code to select candidate files.
2. If `applies-to` exists, use Jev to judge each candidate against it.
3. Exclude only clearly inapplicable files. Include uncertain files.
4. Run the rule against the qualifying subset, individually or with cross-file
   context according to inferred scope.

Files excluded by qualification are not retained as supporting context. Without
`applies-to`, all glob matches qualify.

A rule with no glob matches or no qualifying files is skipped with its reason;
that alone does not fail the run.

Use Jev only. Do not introduce a reasoning model, a text-generation model, or
automatic fixes. Code owns discovery, scheduling, comparison, aggregation,
source locations, and output formatting.

## Findings and completion

Report the rule title that failed and all discovered failure locations. For
violations in existing code, report exact source line numbers and file paths.
For missing-code requirements, flag the file where the unmet requirement was
discovered; do not invent a line for absent code.

Detected violations are errors. Uncertain compliance is inconclusive, separately
identified from violations. Request failures and other uncompleted evaluations
are incomplete checks. All three make the final run unsuccessful.

After valid configuration passes preflight, finish every independent check.
Do not stop at the first violation or provider failure. Aggregate results before
returning the final nonzero exit status.

Provide readable terminal output by default and `--format json` for integrations.
Include rule identity/title, locations, violation probabilities where available,
and incomplete or inconclusive statuses. Never represent unavailable judgments
as successful checks.

Make probability thresholds configurable. Calibrate them with labeled cases;
do not treat an arbitrary threshold as verified accuracy.

## Cache

Enable local evaluation caching. Key entries by the complete evaluation input,
provider, and model. Support `--no-cache` to force fresh evaluation. Do not cache
failed or inconclusive checks. Final cache storage, invalidation, model-alias
handling, and expiration details remain implementation decisions.

## Context and localization: unresolved engineering work

Oversized qualifying sets must have a considered evaluation strategy. Simply
marking every oversized rule incomplete is not an acceptable normal solution.
Neither is silently truncating input, splitting cross-file requirements into
unrelated fragments, or substituting lossy summaries for source evidence.

The implementation must investigate source-based decomposition and focused Jev
questions while preserving the logical scope of the entire qualifying subset.
Candidate source locations must be supplied by code because Jev returns typed
judgments, not generated lists of arbitrary locations.

Validate the strategy against both local and relational rules, including:

- Multiple violations within a file, including violations on adjacent lines.
- A new violation in a file that already violated the same rule at baseline.
- Renamed files, inserted lines, deleted files, and removed supporting tests.
- Supporting evidence located in a different context batch.
- Missing evidence, where all relevant candidates must be considered.
- Arbitrary prose that combines several requirements.
- Uncertain qualification, uncertain verdicts, and partial provider failures.

No general quality guarantee for arbitrary cross-file prose has been established.
Coverage accounting and labeled evaluations are required before claiming that an
oversized rule has been completely checked. Exhausted or unsupported evaluations
must remain explicitly incomplete, rather than being reported as passing.

## Research basis

Official documentation reviewed during design:

- [Jev introduction](https://docs.typesafe.ai/introduction): typed decisions rather
  than generated prose.
- [Primitives](https://docs.typesafe.ai/primitives): Noul probabilities,
  independent questions sharing state, batching, and the roughly 32,000-token
  shared state/question budget documented at the time of review.
- [Workflow design](https://docs.typesafe.ai/concepts/how-to-build-with-system-one):
  atomic questions, focused source context, and composition in code.
- [Confidence](https://docs.typesafe.ai/confidence): uncertainty handling and
  domain-specific threshold evaluation.

Provider compatibility and current limits must be verified before implementation;
the direct API's documentation is not proof that gateways use identical contracts.
