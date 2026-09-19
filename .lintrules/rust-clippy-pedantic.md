---
title: Rust packages enable the Clippy pedantic lint group
globs: ["**/Cargo.toml"]
applies-to: Cargo manifests declaring packages or workspace lint settings.
---

Every Rust package must set `pedantic` to "warn", "deny", or "forbid" in
`[lints.clippy]`, either directly or via `[lints] workspace = true` and a shared
`[workspace.lints.clippy]` declaration. A table with a `level` field is valid.
A missing setting or `"allow"` does not comply.

Inspect the workspace root to resolve inheritance. A virtual workspace does
not need its own `[lints] workspace = true` declaration. Targeted exceptions
for individual Clippy lints do not violate this rule.

Source: rust-router, Default Project Settings.
