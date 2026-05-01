#!/usr/bin/env bash
# Check npm dependency licenses in the frontend directory.
# Usage: ./scripts/check-licenses.sh [--frontend-dir <path>]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FRONTEND_DIR="${FRONTEND_DIR:-$REPO_ROOT/frontend}"

ALLOWED_LICENSES="MIT;Apache-2.0;BSD-2-Clause;BSD-3-Clause;ISC;Unlicense;CC0-1.0;Python-2.0;BlueOak-1.0.0;0BSD;Artistic-2.0"

echo "Checking npm licenses in $FRONTEND_DIR..."

if [[ ! -d "$FRONTEND_DIR/node_modules" ]]; then
    echo "ERROR: node_modules not found. Run 'npm install' in $FRONTEND_DIR first." >&2
    exit 1
fi

cd "$FRONTEND_DIR"

# Use npx to run license-checker without a global install.
npx --yes license-checker \
    --onlyAllow "$ALLOWED_LICENSES" \
    --excludePrivatePackages \
    --summary

echo "All npm licenses OK."
