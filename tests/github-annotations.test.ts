import { expect, test } from "bun:test";

test("the annotation reporter preserves line and file level findings", async () => {
  const report = `${import.meta.dir}/fixtures/github-report.json`;
  const process = Bun.spawn(
    ["node", "scripts/report-github-annotations.mjs", report, "1"],
    { cwd: import.meta.dir + "/..", stdout: "pipe", stderr: "pipe" },
  );

  expect(await process.exited).toBe(0);
  expect(await new Response(process.stdout).text()).toBe(
    "::error file=src/effect.ts,line=42,title=Functional Effect [violation]::Jev found a violation on this line.\n" +
      "::error file=src/forms.tsx,title=Inline validation [violation]::Jev found a file-level violation.\n",
  );
});

test("the annotation reporter reports configuration errors", async () => {
  const report = `${import.meta.dir}/fixtures/github-config-errors.json`;
  const process = Bun.spawn(
    ["node", "scripts/report-github-annotations.mjs", report, "1"],
    { cwd: import.meta.dir + "/..", stdout: "pipe", stderr: "pipe" },
  );

  expect(await process.exited).toBe(0);
  expect(await new Response(process.stdout).text()).toBe(
    "::error title=Lintrules configuration::rules/example.md is missing globs\n",
  );
});
