# The only entry point for developing memo. Every tool comes pinned from mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]

manifest := "src-tauri/Cargo.toml"
app := "src-tauri/target/release/bundle/macos/memo.app"

# List the recipes.
default:
    @just --list --unsorted

alias dependencies := deps

# Install the dependencies: `install` (default), `clean` (wipe, then install from the lockfiles) or `update`.
deps mode="install":
    #!/usr/bin/env bash
    set -euo pipefail
    mise install --quiet
    case "{{ mode }}" in
        install) deno install --quiet && cargo fetch --manifest-path {{ manifest }} ;;
        clean) rm -rf node_modules && cargo clean --manifest-path {{ manifest }} && deno install --quiet --frozen && cargo fetch --locked --manifest-path {{ manifest }} ;;
        update) deno outdated --update --latest && cargo update --manifest-path {{ manifest }} ;;
        *) echo "unknown mode {{ mode }}: install, clean or update" >&2; exit 2 ;;
    esac

# Format everything, fixing what the formatters can.
fmt:
    cargo fmt --manifest-path {{ manifest }}
    deno fmt --quiet
    taplo fmt --colors never 2>&1 | { grep -v ' INFO ' || true; }
    rumdl fmt --quiet .
    just --fmt --unstable

# Lint everything; `family` narrows it to rust, web, toml, markdown, just or spelling.
lint family="":
    #!/usr/bin/env bash
    set -euo pipefail
    want() { [[ -z "{{ family }}" || "{{ family }}" == "$1" ]]; }
    case "{{ family }}" in ""|rust|web|toml|markdown|just|spelling) ;; *) echo "unknown family {{ family }}" >&2; exit 2 ;; esac
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
    if want spelling; then typos; fi

# Run the Rust tests.
test:
    cargo nextest run --manifest-path {{ manifest }} --locked

# Lint, then test.
check: lint test

# Run the app with hot reload of the front end.
dev:
    deno task tauri dev

# Build the release app bundle.
build:
    deno task tauri build --bundles app
    @du -sh {{ app }} | sed 's/\t/  /'

# Install the release app in ~/Applications and open it.
install: build
    mkdir -p ~/Applications
    rm -rf ~/Applications/memo.app
    cp -R {{ app }} ~/Applications/memo.app
    open ~/Applications/memo.app

# Print the size of the release binary and bundle, and of the front end.
size:
    @du -sh src-tauri/target/release/memo {{ app }} dist 2>/dev/null | sed 's/\t/  /' || true
