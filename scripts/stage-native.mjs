import { chmodSync, copyFileSync, mkdirSync } from "node:fs";
import { resolve } from "node:path";

const [source] = process.argv.slice(2);
if (!source) throw new Error("Usage: node scripts/stage-native.mjs <binary>");
const directory = resolve("native", `${process.platform}-${process.arch}`);
mkdirSync(directory, { recursive: true });
const destination = resolve(
  directory,
  process.platform === "win32" ? "lintrules.exe" : "lintrules",
);
copyFileSync(source, destination);
chmodSync(destination, 0o755);
