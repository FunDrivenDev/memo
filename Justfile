# The only entry point for developing memo. Every tool comes pinned from mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]

manifest := "src-tauri/Cargo.toml"
app := "src-tauri/target/release/bundle/macos/memo.app"

# List the recipes.
default:
    @just --list --unsorted

# Set up a fresh clone: the tools, the dependencies and the pre-push secrets hook. `mode` is empty, `frozen` (from the lockfiles, as in CI), `clean` (wipe, then frozen) or `update`.
install mode="":
    #!/usr/bin/env bash
    set -euo pipefail
    mise install --quiet
    git config core.hooksPath .githooks  # the pre-push secrets check
    case "{{ mode }}" in
        "") deno install --quiet && cargo fetch --manifest-path {{ manifest }} ;;
        frozen) deno install --quiet --frozen && cargo fetch --locked --manifest-path {{ manifest }} ;;
        clean) rm -rf node_modules && cargo clean --manifest-path {{ manifest }} && deno install --quiet --frozen && cargo fetch --locked --manifest-path {{ manifest }} ;;
        update) deno outdated --update --latest && cargo update --manifest-path {{ manifest }} ;;
        *) echo "unknown mode {{ mode }}: frozen, clean or update" >&2; exit 2 ;;
    esac

# Format everything, fixing what the formatters can.
fmt:
    cargo fmt --manifest-path {{ manifest }}
    deno fmt --quiet
    taplo fmt --colors never 2>&1 | { grep -v ' INFO ' || true; }
    rumdl fmt --quiet .
    just --fmt --unstable

# Lint everything; `family` narrows it to rust, web, toml, markdown, just, workflows, spelling or secrets.
lint family="":
    #!/usr/bin/env bash
    set -euo pipefail
    want() { [[ -z "{{ family }}" || "{{ family }}" == "$1" ]]; }
    case "{{ family }}" in ""|rust|web|toml|markdown|just|workflows|spelling|secrets) ;; *) echo "unknown family {{ family }}" >&2; exit 2 ;; esac
    if want rust; then
        cargo fmt --manifest-path {{ manifest }} --check
        cargo clippy --manifest-path {{ manifest }} --all-targets --locked -- -D warnings
    fi
    if want web; then
        deno fmt --check --quiet
        deno lint --quiet
        deno task --quiet web:check
    fi
    if want toml; then
        taplo fmt --colors never --check 2>&1 | { grep -v ' INFO ' || true; }
        taplo lint --colors never 2>&1 | { grep -v ' INFO ' || true; }
    fi
    if want markdown; then rumdl check --quiet .; fi
    if want just; then just --fmt --unstable --check; fi
    if want workflows; then actionlint; fi
    if want spelling; then typos; fi
    if want secrets; then gitleaks git --config .gitleaks.toml --redact --no-banner --log-level warn .; fi

# Run the Rust tests, then the Deno ones.
test:
    cargo nextest run --manifest-path {{ manifest }} --locked
    deno test --quiet --allow-run=just tests/

# Lint, then test.
check: lint test

# Run the app with hot reload of the front end.
dev:
    deno task tauri dev

# Build the release app bundle, with home directory paths trimmed to `~` in the binary.
build:
    RUSTFLAGS="--remap-path-prefix=$HOME=~" deno task tauri build --bundles app
    @du -sh {{ app }} | sed 's/\t/  /'

# Release `version` through the Release workflow, or with `--locally` build the app into ~/Applications and open it.
[arg("locally", long, value="true")]
publish version="" locally="false":
    just {{ if locally == "true" { "_install-locally" } else if version != "" { "_release " + version } else { error("publish needs a version or --locally") } }}

[private]
_install-locally: build
    mkdir -p ~/Applications
    rm -rf ~/Applications/memo.app
    cp -R {{ app }} ~/Applications/memo.app
    open ~/Applications/memo.app

[private]
_release version:
    gh workflow run release -f version={{ version }}

# What CI runs on Linux: check, build for the host without bundling, and lint the macOS code (clang compiles its Objective-C without an SDK).
ci: check
    deno task tauri build --no-bundle
    CC_aarch64_apple_darwin=clang cargo clippy --manifest-path {{ manifest }} --target aarch64-apple-darwin --all-targets --locked -- -D warnings

# Audit security: secrets or personal data anywhere in the history, vulnerable or unknown-source dependencies, unsafe workflows.
audit:
    gitleaks git --config .gitleaks.toml --redact --no-banner --log-level warn .
    cargo deny --manifest-path {{ manifest }} --config deny.toml check advisories sources
    deno audit
    zizmor --quiet .

# Refuse an app bundle that carries a secret or a home directory path, before it goes public.
scan-app path=app:
    gitleaks dir --config .gitleaks.toml --redact --no-banner --log-level warn "{{ path }}"
    strings -a -n 6 "{{ path }}/Contents/MacOS/memo" | gitleaks stdin --config .gitleaks.toml --redact --no-banner --log-level warn

# Set `version` in Cargo.toml and its lockfile.
set-version version:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "{{ version }}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "{{ version }} is not a semantic version" >&2; exit 2; }
    perl -0pi -e 's/(\[package\]\nname = "memo"\nversion = )"[^"]*"/$1"{{ version }}"/' {{ manifest }}
    cargo update --manifest-path {{ manifest }} --workspace --quiet

# Zip the release bundle as src-tauri/target/dist/memo-<version>-macos-arm64.zip, keeping its signature.
package version:
    #!/usr/bin/env bash
    set -euo pipefail
    archive="src-tauri/target/dist/memo-{{ version }}-macos-arm64.zip"
    mkdir -p "$(dirname "$archive")"
    rm -f "$archive"
    ditto -c -k --keepParent {{ app }} "$archive"

# Print the size of the release binary and bundle, and of the front end.
size:
    @du -sh src-tauri/target/release/memo {{ app }} dist 2>/dev/null | sed 's/\t/  /' || true
