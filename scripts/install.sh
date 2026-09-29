#!/usr/bin/env bash
# Usage: install.sh [frozen|clean|update]. Run through `just install`, which sets MANIFEST.
set -euo pipefail

mode=${1:-}
mise install --quiet
git config core.hooksPath .githooks # the pre-push secrets check
case "$mode" in
  "") deno install --quiet && cargo fetch --manifest-path "$MANIFEST" ;;
  frozen) deno install --quiet --frozen && cargo fetch --locked --manifest-path "$MANIFEST" ;;
  clean) rm -rf node_modules && cargo clean --manifest-path "$MANIFEST" && deno install --quiet --frozen && cargo fetch --locked --manifest-path "$MANIFEST" ;;
  update) deno outdated --update --latest && cargo update --manifest-path "$MANIFEST" ;;
  *)
    echo "unknown mode $mode: frozen, clean or update" >&2
    exit 2
    ;;
esac
deno run --quiet --allow-all npm:playwright install --only-shell chromium # for the browser tests
