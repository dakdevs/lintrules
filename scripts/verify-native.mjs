import { accessSync, constants } from "node:fs";

for (const platform of [
  "darwin-arm64",
  "darwin-x64",
  "linux-arm64",
  "linux-x64",
  "win32-x64",
]) {
  const executable = platform.startsWith("win32")
    ? "lintrules.exe"
    : "lintrules";
  accessSync(`native/${platform}/${executable}`, constants.R_OK);
}
console.log("All release binaries are present.");
