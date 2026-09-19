import { expect, test } from "bun:test";

test("the Rust command-line integration test passes", async () => {
  const process = Bun.spawn(["cargo", "test", "--test", "cli"], {
    cwd: import.meta.dir + "/..",
    stderr: "pipe",
    stdout: "pipe",
  });

  expect(await process.exited).toBe(0);
});
