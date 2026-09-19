import { readFileSync, writeFileSync } from "node:fs";

const manifest = JSON.parse(readFileSync("package.json", "utf8"));
const version = process.argv[2] || manifest.version;
if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
  throw new Error(
    "Release version must be a stable semantic version, such as 1.0.1",
  );
}
manifest.version = version;
writeFileSync("package.json", `${JSON.stringify(manifest, null, 2)}\n`);
const cargo = readFileSync("Cargo.toml", "utf8").replace(
  /(\[workspace\.package\]\s*\nversion = ")[^"]+/,
  (_, prefix) => `${prefix}${version}`,
);
writeFileSync("Cargo.toml", cargo);
const lock = readFileSync("Cargo.lock", "utf8").replace(
  /(name = "lintrules(?:-[^"]+)?"\nversion = ")[^"]+/g,
  (_, prefix) => `${prefix}${version}`,
);
writeFileSync("Cargo.lock", lock);
