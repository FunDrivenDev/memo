# The only entry point for developing memo. Every tool comes pinned from mise.toml.

set shell := ["bash", "-euo", "pipefail", "-c"]

manifest := "src-tauri/Cargo.toml"
app := "src-tauri/target/release/bundle/macos/memo.app"

# List the recipes.
default:
    @just --list --unsorted

alias dependencies := deps

# Install the dependencies: `install` (default), `frozen` (from the lockfiles, as in CI), `clean` (wipe, then frozen) or `update`.
deps mode="install":
    #!/usr/bin/env bash
    set -euo pipefail
    mise install --quiet
    case "{{ mode }}" in
        install) deno install --quiet && cargo fetch --manifest-path {{ manifest }} ;;
        frozen) deno install --quiet --frozen && cargo fetch --locked --manifest-path {{ manifest }} ;;
        clean) rm -rf node_modules && cargo clean --manifest-path {{ manifest }} && deno install --quiet --frozen && cargo fetch --locked --manifest-path {{ manifest }} ;;
        update) deno outdated --update --latest && cargo update --manifest-path {{ manifest }} ;;
        *) echo "unknown mode {{ mode }}: install, frozen, clean or update" >&2; exit 2 ;;
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

# What CI runs on Linux: check, build for the host without bundling, and lint the macOS code (clang compiles its Objective-C without an SDK).
ci: check
    deno task tauri build --no-bundle
    CC_aarch64_apple_darwin=clang cargo clippy --manifest-path {{ manifest }} --target aarch64-apple-darwin --all-targets --locked -- -D warnings

# Publish `version` from the pushed main once CI passed on it: set it, tag it `v<version>`, and attach the zipped app to a GitHub release, which the Publish workflow pushes to the Homebrew tap.
publish version:
    #!/usr/bin/env bash
    set -euo pipefail
    version="{{ version }}"
    tag="v$version"
    [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "$version is not a semantic version" >&2; exit 2; }
    [[ "$(git branch --show-current)" == main ]] || { echo "publish from main" >&2; exit 1; }
    [[ -z "$(git status --porcelain)" ]] || { echo "commit or stash the changes first" >&2; exit 1; }
    git fetch --quiet --tags origin main
    [[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || { echo "main differs from origin/main: push or pull first" >&2; exit 1; }
    if git rev-parse --quiet --verify "refs/tags/$tag" >/dev/null; then echo "$tag already exists" >&2; exit 1; fi
    ci=$(gh run list --commit "$(git rev-parse HEAD)" --workflow CI --json status,conclusion --jq '.[0] | "\(.status) \(.conclusion)"')
    [[ "$ci" == "completed success" ]] || { echo "CI has not passed on HEAD (${ci:-no run}): wait for it or fix it" >&2; exit 1; }
    if [[ "$(taplo get -f {{ manifest }} package.version)" != "$version" ]]; then
        perl -0pi -e 's/(\[package\]\nname = "memo"\nversion = )"[^"]*"/$1"'"$version"'"/' {{ manifest }}
        cargo update --manifest-path {{ manifest }} --workspace --quiet
        git commit --quiet -m "Publish $version" {{ manifest }} src-tauri/Cargo.lock
    fi
    git tag -a "$tag" -m "memo $version"
    git push --quiet origin main "$tag"
    just build
    dist=src-tauri/target/dist
    archive="$dist/memo-$version-macos-arm64.zip"
    mkdir -p "$dist"
    rm -f "$archive"
    ditto -c -k --keepParent {{ app }} "$archive"
    gh release create "$tag" --title "memo $version" --generate-notes "$archive"

# Print the size of the release binary and bundle, and of the front end.
size:
    @du -sh src-tauri/target/release/memo {{ app }} dist 2>/dev/null | sed 's/\t/  /' || true
