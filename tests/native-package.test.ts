import { expect, test } from "bun:test";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const platform = `${process.platform}-${process.arch}`;
const executable = process.platform === "win32" ? "lintrules.exe" : "lintrules";

async function run(
  args: string[],
  options: { cwd: string; env?: Record<string, string | undefined> },
) {
  const child = Bun.spawn(args, { ...options, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, status] = await Promise.all([
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
    child.exited,
  ]);
  return { stdout, stderr, status };
}

test("npm package runs in the consumer project without Cargo or install scripts", async () => {
  expect(existsSync(join(root, "native", platform, executable))).toBe(true);
  const directory = mkdtempSync(join(tmpdir(), "lintrules-package-"));
  const packed = await run(
    ["npm", "pack", "--json", "--pack-destination", directory],
    { cwd: root },
  );
  expect(packed.status).toBe(0);
  const [archive] = JSON.parse(packed.stdout);
  const githubPath = join(directory, "github-path");
  const installed = await run(
    ["bash", "scripts/install-cli.sh", join(directory, archive.filename)],
    {
      cwd: root,
      env: { ...process.env, RUNNER_TEMP: directory, GITHUB_PATH: githubPath },
    },
  );
  expect(installed.status).toBe(0);
  const installRoot = join(directory, "lintrules");
  expect(readFileSync(githubPath, "utf8").trim()).toBe(
    join(installRoot, "node_modules", ".bin"),
  );
  const cli = join(
    installRoot,
    "node_modules",
    "@dakdevs",
    "lintrules",
    "bin",
    "lintrules.mjs",
  );
  const env = {
    ...process.env,
    PATH: "",
    CARGO: "missing-cargo",
    TYPESAFE_API_KEY: "test-key",
  };
  const version = JSON.parse(
    readFileSync(join(root, "package.json"), "utf8"),
  ).version;
  expect(
    await run([process.execPath, cli, "--version"], { cwd: directory, env }),
  ).toEqual({ stdout: `lintrules ${version}\n`, stderr: "", status: 0 });
  expect(
    (
      await run([process.execPath, cli, "init", "--provider", "typesafe"], {
        cwd: directory,
        env,
      })
    ).status,
  ).toBe(0);
  // No matching files: exercise config/rule discovery and credential validation without paid calls.
  writeFileSync(
    join(directory, ".lintrules", "example.md"),
    '---\ntitle: Test rule\nglobs: ["nothing-matches-*.rs"]\n---\nUse functional code.\n',
  );
  const nested = join(installRoot, "nested");
  mkdirSync(nested);
  const check = await run([process.execPath, cli, "--format", "json"], {
    cwd: nested,
    env,
  });
  expect(check.status).toBe(0);
  expect(JSON.parse(check.stdout).findings).toEqual([]);
  const missingKey = await run([process.execPath, cli], {
    cwd: nested,
    env: { ...env, TYPESAFE_API_KEY: undefined },
  });
  expect(missingKey.status).toBe(1);
  expect(missingKey.stderr).toContain("TYPESAFE_API_KEY");

  // Run the exact bunx command against the installed package. Cargo cannot be found.
  const runtimeDirectory = join(directory, "runtime");
  mkdirSync(runtimeDirectory);
  if (process.platform !== "win32") {
    symlinkSync(process.execPath, join(runtimeDirectory, "bun"));
    symlinkSync(process.execPath, join(runtimeDirectory, "node"));
    const bunx = await run(
      [
        process.execPath,
        "x",
        "--no-install",
        "@dakdevs/lintrules",
        "--format",
        "json",
      ],
      { cwd: nested, env: { ...env, PATH: runtimeDirectory } },
    );
    expect(bunx.status).toBe(0);
    expect(JSON.parse(bunx.stdout).findings).toEqual([]);
  }
}, 60_000);
