#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = process.env.CARGO ?? "cargo";
const result = spawnSync(
  cargo,
  [
    "run",
    "--quiet",
    "--release",
    "--manifest-path",
    resolve(packageRoot, "Cargo.toml"),
    "-p",
    "lintrules",
    "--",
    ...process.argv.slice(2),
  ],
  { cwd: process.cwd(), stdio: "inherit" },
);

if (result.error?.code === "ENOENT") {
  console.error(
    "lintrules requires Cargo. Install Rust from https://rustup.rs/.",
  );
}

process.exit(result.status ?? 1);
