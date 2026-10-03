# The only entry point for developing memo. Every tool comes pinned from mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Exported, so the scripts under scripts/ read them too.
export MANIFEST := "src-tauri/Cargo.toml"
export APP := "src-tauri/target/release/bundle/macos/memo.app"

# List the recipes.
default:
    @just --list --unsorted

# Set up a fresh clone: the tools, the dependencies and the pre-push secrets hook. `mode` is empty, `frozen` (from the lockfiles, as in CI), `clean` (wipe, then frozen) or `update`.
install mode="":
    scripts/install.sh {{ quote(mode) }}

# Format everything, fixing what the formatters can.
fmt:
    cargo fmt --manifest-path {{ MANIFEST }}
    deno fmt --quiet
    taplo fmt --colors never 2>&1 | { grep -v ' INFO ' || true; }
    rumdl fmt --quiet .
    just --fmt --unstable

# Lint everything; `family` narrows it to rust, web, toml, markdown, just, shell, workflows, spelling or secrets.
lint family="":
    scripts/lint.sh {{ quote(family) }}

# Run the Rust tests, then the Deno ones, which drive the front end in headless Chromium.
test:
    cargo nextest run --manifest-path {{ MANIFEST }} --locked
    deno test --quiet --allow-all tests/

# Lint, then test.
check: lint test

# Run the app with hot reload of the front end, served on a free port.
dev:
    scripts/dev.sh

# Build the release app bundle, with home directory paths trimmed to `~` in the binary.
build:
    RUSTFLAGS="--remap-path-prefix=$HOME=~" deno task tauri build --bundles app
    @du -sh {{ APP }} | sed 's/\t/  /'

# Build the app into ~/Applications and open it.
app: build
    mkdir -p ~/Applications
    rm -rf ~/Applications/memo.app
    cp -R {{ APP }} ~/Applications/memo.app
    open ~/Applications/memo.app

# What CI runs on Linux: check, build for the host without bundling, and lint the macOS code (clang compiles its Objective-C without an SDK).
ci: check
    deno task tauri build --no-bundle
    CC_aarch64_apple_darwin=clang cargo clippy --manifest-path {{ MANIFEST }} --target aarch64-apple-darwin --all-targets --locked -- -D warnings

# Audit security: secrets or personal data anywhere in the history, vulnerable or unknown-source dependencies, unsafe workflows; `family` narrows it to secrets, rust, web or workflows.
audit family="":
    scripts/audit.sh {{ quote(family) }}

# Refuse an app bundle that carries a secret or a home directory path, before it goes public.
scan-app path=APP:
    gitleaks dir --config .gitleaks.toml --redact --no-banner --log-level warn "{{ path }}"
    strings -a -n 6 "{{ path }}/Contents/MacOS/memo" | gitleaks stdin --config .gitleaks.toml --redact --no-banner --log-level warn

# Set `version` in Cargo.toml and its lockfile.
set-version version:
    scripts/set-version.sh {{ quote(version) }}

# Zip the release bundle as src-tauri/target/dist/memo-<version>-macos-arm64.zip, keeping its signature.
package version:
    scripts/package.sh {{ quote(version) }}

# Check the cask on macOS against Homebrew's style and audit rules, where deprecations fail; given an app archive, also install it from the cask into a scratch folder and check the app lost its quarantine flag.
cask archive="":
    scripts/cask.sh {{ quote(archive) }}

# Print the size of the release binary and bundle, and of the front end.
size:
    @du -sh src-tauri/target/release/memo {{ APP }} dist 2>/dev/null | sed 's/\t/  /' || true
