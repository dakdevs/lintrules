import { expect, test } from "bun:test";

test("the Rust command-line integration test passes", async () => {
  const child = Bun.spawn(["cargo", "test", "--test", "cli"], {
    cwd: import.meta.dir + "/..",
    stderr: "pipe",
    stdout: "pipe",
  });
  const [status, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  expect(status, `${stdout}\n${stderr}`).toBe(0);
}, 60_000);
