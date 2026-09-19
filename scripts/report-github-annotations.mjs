#!/usr/bin/env node

import { readFileSync } from "node:fs";

const [reportPath, exitStatus] = process.argv.slice(2);

function escapeProperty(value) {
  return String(value)
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A")
    .replaceAll(":", "%3A")
    .replaceAll(",", "%2C");
}

function escapeMessage(value) {
  return String(value)
    .replaceAll("%", "%25")
    .replaceAll("\r", "%0D")
    .replaceAll("\n", "%0A");
}

function annotation({ path, line, title, message }) {
  const properties = [];
  if (path) properties.push(`file=${escapeProperty(path)}`);
  if (Number.isInteger(line) && line > 0) properties.push(`line=${line}`);
  if (title) properties.push(`title=${escapeProperty(title)}`);
  const suffix = properties.length > 0 ? ` ${properties.join(",")}` : "";
  console.log(`::error${suffix}::${escapeMessage(message)}`);
}

try {
  const report = JSON.parse(readFileSync(reportPath, "utf8"));
  for (const finding of report.findings ?? []) {
    annotation({
      path: finding.path,
      line: finding.line,
      title: `${finding.title} [${finding.kind}]`,
      message: finding.message,
    });
  }
  for (const error of report.configuration_errors ?? []) {
    annotation({ title: "Lintrules configuration", message: error });
  }
} catch (error) {
  annotation({
    title: "Lintrules execution",
    message: `Lintrules did not produce a JSON report (exit ${exitStatus ?? "unknown"}): ${error.message}`,
  });
}
