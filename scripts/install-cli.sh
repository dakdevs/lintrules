#!/usr/bin/env bash
set -euo pipefail
install_root="$RUNNER_TEMP/lintrules"
npm install --prefix "$install_root" --ignore-scripts --no-audit --no-fund "$1"
echo "$install_root/node_modules/.bin" >> "$GITHUB_PATH"
