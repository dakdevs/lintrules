---
title: Rust packages use edition 2024
globs: ["**/Cargo.toml"]
applies-to: Cargo manifests declaring a package or shared workspace package settings.
---

Every Rust package must use edition "2024". A package can declare
`edition = "2024"` in `[package]` or inherit it with `edition.workspace = true`
from `[workspace.package]` in the root Cargo manifest.

When inheritance is used, inspect the workspace manifest before deciding
whether the package complies. A virtual workspace without a `[package]`
section does not need a package declaration.

Source: rust-router, Default Project Settings.
