#!/usr/bin/env bash
set -euo pipefail
args=(--format json --pr-report "${INPUT_PR_REPORT:-introduced}")
if [[ -n "${INPUT_CONFIG:-}" ]]; then args+=(--config "$INPUT_CONFIG"); fi
if [[ -n "${INPUT_BASE:-}" ]]; then args+=(--base "$INPUT_BASE"); fi
if [[ -n "${INPUT_WORKING_TREE:-}" ]]; then args+=(--working-tree "$INPUT_WORKING_TREE"); fi
if [[ "${INPUT_NO_CACHE:-false}" == "true" ]]; then args+=(--no-cache); fi

report="$RUNNER_TEMP/lintrules-report.json"
cd "${INPUT_WORKING_DIRECTORY:-.}"
status=0
lintrules "${args[@]}" > "$report" || status=$?
cat "$report"
echo "report=$report" >> "$GITHUB_OUTPUT"
echo "status=$status" >> "$GITHUB_OUTPUT"
