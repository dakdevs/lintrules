#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const platform = `${process.platform}-${process.arch}`;
const executable = process.platform === "win32" ? "lintrules.exe" : "lintrules";
const binary = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "native",
  platform,
  executable,
);
const result = spawnSync(binary, process.argv.slice(2), {
  cwd: process.cwd(),
  env: process.env,
  stdio: "inherit",
});

if (result.error) {
  console.error(
    `lintrules: could not run the bundled binary for ${platform}: ${result.error.message}`,
  );
  console.error(
    "Supported platforms: macOS and Linux (x64/arm64), Windows (x64).",
  );
}
if (result.signal) process.kill(process.pid, result.signal);
process.exit(result.status ?? 1);
