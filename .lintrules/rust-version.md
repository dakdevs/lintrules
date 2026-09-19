---
title: Rust packages declare a supported minimum Rust version
globs: ["**/Cargo.toml"]
applies-to: Cargo manifests declaring a package or shared workspace package settings.
---

Every Rust package must declare `rust-version` in `[package]`, directly or
through `rust-version.workspace = true` with a value in `[workspace.package]`.
The declared version must be at least "1.85", the baseline for edition 2024.
A higher minimum is allowed when the project needs newer language features
or dependencies. Do not require exactly "1.85".

When inheritance is used, inspect the workspace manifest before deciding
whether the package complies. A virtual workspace without a `[package]`
section does not need a package declaration.

Source: rust-router, Default Project Settings, adapted to allow a newer MSRV.
