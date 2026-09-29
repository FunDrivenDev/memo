#!/usr/bin/env bash
# Usage: cask.sh [archive]. macOS only.
set -euo pipefail

# Developer mode turns a deprecation into an error, and lets a plain untap leave installed casks alone.
export HOMEBREW_DEVELOPER=1 HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_ENV_HINTS=1
scratch=$(mktemp -d)
# A copy of the user's tap trust, so trusting the scratch tap leaves theirs as it was.
trust="${XDG_CONFIG_HOME:-}/homebrew/trust.json"
[[ -n "${XDG_CONFIG_HOME:-}" && -f "$trust" ]] || trust=~/.homebrew/trust.json
export XDG_CONFIG_HOME="$scratch/config"
mkdir -p "$XDG_CONFIG_HOME/homebrew"
[[ ! -f "$trust" ]] || cp "$trust" "$XDG_CONFIG_HOME/homebrew/trust.json"
tap=memo-check/scratch
installed=""
# Never `untap --force`: it uninstalls every installed cask sharing a token with the tap, the user's memo included.
cleanup() {
  [[ -z "$installed" ]] || brew uninstall --cask "$tap/memo-check" >/dev/null || true
  brew untap "$tap" >/dev/null 2>&1 || true
  rm -rf "$scratch"
}
trap cleanup EXIT
brew tap-new --no-git "$tap" >/dev/null
brew trust --tap "$tap" >/dev/null
casks="$(brew --repo "$tap")/Casks"
mkdir -p "$casks"
cp packaging/memo.rb "$casks/memo.rb"
brew style "$tap/memo"
brew audit --cask --strict "$tap/memo"
[[ -n "${1:-}" ]] || exit 0

# Its own token, so the install never touches an installed memo.
archive=$(realpath "$1")
sed -e 's/^cask "memo"/cask "memo-check"/' \
  -e "s|^  sha256 .*|  sha256 \"$(shasum -a 256 "$archive" | cut -d' ' -f1)\"|" \
  -e "s|^  url .*|  url \"file://$archive\"|" \
  packaging/memo.rb >"$casks/memo-check.rb"
installed=1
brew install --cask --appdir="$scratch/Applications" "$tap/memo-check"
if xattr -p com.apple.quarantine "$scratch/Applications/memo.app" >/dev/null 2>&1; then
  echo "memo.app still carries the quarantine flag after install" >&2
  exit 1
fi
