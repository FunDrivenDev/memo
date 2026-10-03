#!/usr/bin/env bash
# Usage: changes.sh [--all] < paths. Reads the paths a pull request changed, one per line, and
# prints what CI must check for them as GitHub step outputs: `check` (the whole `just ci`),
# `lint` and `audit` (the families of `just lint` and `just audit` to run) and `cask`. A path
# it does not know, or --all, checks everything.
set -euo pipefail

check=false
cask=false
lint=""
# Any change may carry a secret.
audit=" secrets"

everything() {
  check=true
  cask=true
  lint=" markdown toml just shell workflows spelling"
  audit=" secrets rust web workflows"
}
lint() { [[ "$lint " == *" $1 "* ]] || lint+=" $1"; }
audit() { [[ "$audit " == *" $1 "* ]] || audit+=" $1"; }

if [[ "${1:-}" == --all ]]; then
  everything
else
  while IFS= read -r path; do
    [[ -n "$path" ]] || continue
    lint spelling
    case "$path" in
      # What every job runs through.
      mise.toml | Justfile | .github/workflows/ci.yml | scripts/changes.sh | scripts/audit.sh) everything ;;
      src-tauri/Cargo.toml | src-tauri/Cargo.lock | deny.toml) check=true && audit rust ;;
      deno.json | deno.lock) check=true && audit web ;;
      scripts/cask.sh) check=true && cask=true ;;
      src-tauri/* | src/* | tests/* | scripts/* | assets/* | index.html | vite.config.ts | tsconfig.json | svelte.config.js)
        check=true
        ;;
      packaging/*) cask=true ;;
      .github/*) lint workflows && audit workflows ;;
      .githooks/*) lint shell ;;
      *.md) lint markdown ;;
      .rumdl.toml) lint toml && lint markdown ;;
      *.toml) lint toml ;;
      .gitignore) ;;
      *) everything ;;
    esac
  done
fi

echo "check=$check"
echo "lint=${lint# }"
echo "audit=${audit# }"
echo "cask=$cask"
