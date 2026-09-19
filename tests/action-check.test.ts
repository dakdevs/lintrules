import { expect, test } from "bun:test";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");

test("a failing scan completes the action step and preserves every annotation", async () => {
  const directory = mkdtempSync(join(tmpdir(), "lintrules-action-"));
  const bin = join(directory, "bin");
  mkdirSync(bin);
  const executable = join(bin, "lintrules");
  writeFileSync(executable, '#!/usr/bin/env bash\ncat "$FIXTURE"\nexit 1\n');
  chmodSync(executable, 0o755);
  const output = join(directory, "output");
  const child = Bun.spawn(
    [
      "bash",
      "--noprofile",
      "--norc",
      "-e",
      "-o",
      "pipefail",
      "scripts/check-rules.sh",
    ],
    {
      cwd: root,
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH}`,
        RUNNER_TEMP: directory,
        GITHUB_OUTPUT: output,
        INPUT_WORKING_DIRECTORY: directory,
        FIXTURE: join(root, "tests/fixtures/github-report.json"),
      },
      stdout: "pipe",
      stderr: "pipe",
    },
  );
  expect(await child.exited).toBe(0);
  expect(readFileSync(output, "utf8")).toContain("status=1\n");
  const report = join(directory, "lintrules-report.json");
  expect(JSON.parse(readFileSync(report, "utf8")).findings).toHaveLength(2);
  const annotations = Bun.spawn(
    ["node", "scripts/report-github-annotations.mjs", report, "1"],
    { cwd: root, stdout: "pipe" },
  );
  const text = await new Response(annotations.stdout).text();
  expect(await annotations.exited).toBe(0);
  expect(text).toContain("file=src/effect.ts,line=42");
  expect(text).toContain("file=src/forms.tsx");
});

for (const reportScope of [undefined, "introduced", "all"]) {
  test(`report-scope=${reportScope ?? "default"} is passed literally to the CLI`, async () => {
    const directory = mkdtempSync(join(tmpdir(), "lintrules-scope-"));
    const bin = join(directory, "bin");
    mkdirSync(bin);
    const executable = join(bin, "lintrules");
    writeFileSync(
      executable,
      '#!/usr/bin/env bash\nprintf "%s\\n" "$@" > "$CAPTURE_ARGS"\nprintf \'{"findings":[],"skipped":[]}\\n\'\n',
    );
    chmodSync(executable, 0o755);
    const capture = join(directory, "args");
    const child = Bun.spawn(["bash", "scripts/check-rules.sh"], {
      cwd: root,
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH}`,
        RUNNER_TEMP: directory,
        GITHUB_OUTPUT: join(directory, "output"),
        INPUT_WORKING_DIRECTORY: directory,
        INPUT_BASE: "test-pr-base",
        INPUT_REPORT_SCOPE: reportScope,
        CAPTURE_ARGS: capture,
      },
      stdout: "pipe",
      stderr: "pipe",
    });
    expect(await child.exited).toBe(0);
    const args = readFileSync(capture, "utf8").trim().split("\n");
    expect(args).toEqual([
      "--format",
      "json",
      "--report-scope",
      reportScope ?? "introduced",
      "--base",
      "test-pr-base",
    ]);
  });
}
