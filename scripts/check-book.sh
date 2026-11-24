#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
log_file="$(mktemp)"
trap 'rm -f "$log_file"' EXIT

cd "$repo_root"

echo "==> Building mdBook..."
if ! mdbook build book 2>&1 | tee "$log_file"; then
    echo "::error::mdbook build failed."
    exit 1
fi

if grep -Eiq 'unclosed HTML tag|HTML tags must be closed before exiting' "$log_file"; then
    echo "::error::mdBook reported an HTML tag parsing warning. Fix the Markdown before committing."
    exit 1
fi

echo "==> mdBook build passed; no unclosed HTML tag warnings detected."
