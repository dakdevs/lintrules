---
title: Rust packages enable unsafe-code diagnostics
globs: ["**/Cargo.toml"]
applies-to: Cargo manifests declaring packages or workspace lint settings.
---

Every Rust package must configure `unsafe_code` at "warn", "deny", or "forbid".
It can do so in `[lints.rust]`, or use `[lints] workspace = true` to inherit
`[workspace.lints.rust]` from the workspace root. `"allow"` and a missing
setting do not comply. A table with a `level` field is also valid.

Inspect the workspace manifest when evaluating inherited settings. A virtual
workspace that declares no package only needs to supply the shared settings
when packages inherit them; it does not need `[lints] workspace = true` itself.

Source: rust-router, Default Project Settings.
