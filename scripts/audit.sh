#!/usr/bin/env bash
# Usage: audit.sh [family]. Run through `just audit`, which sets MANIFEST.
set -euo pipefail

family=${1:-}
want() { [[ -z "$family" || "$family" == "$1" ]]; }
case "$family" in
  "" | secrets | rust | web | workflows) ;;
  *)
    echo "unknown family $family" >&2
    exit 2
    ;;
esac

if want secrets; then gitleaks git --config .gitleaks.toml --redact --no-banner --log-level warn .; fi
if want rust; then cargo deny --manifest-path "$MANIFEST" --config deny.toml check advisories sources; fi
if want web; then deno audit; fi
if want workflows; then zizmor --quiet .; fi
