# memo

A small macOS reader for the plans, handoffs and reports Claude Code writes. It is keyboard-driven, has fuzzy search over whole files, and can start a Claude session from a note.

It uses Tauri 2 (a Rust backend and the system WebView, so no bundled browser) and a Svelte 5 front end built by Vite, all run through Deno.

## Where the notes come from

memo reads the Claude Code configuration (`$CLAUDE_CONFIG_DIR`, else `~/.claude`) each time it reloads:

- `plansDirectory` in `settings.json` and `settings.local.json`;
- every directory listed at the start of a line inside a fenced code block of `CLAUDE.md`, ending with `/`, such as `~/Notes/claude/reports/   what was found or done`.

Each existing directory becomes a sidebar section. The section lists the `.md` files directly inside it, most recently modified first. The folders are watched, so new notes appear as Claude writes them.

## Keys

| Key | Action |
| --- | --- |
| `⌘K`, `/`, `:` | Command palette: fuzzy search over titles and content; start with `>` for commands |
| `j` `k`, `↓` `↑` | Next or previous note |
| `⇥` `⇧⇥` | Next or previous folder |
| `g` `G` | First or last note |
| `space` `⇧space`, `⌃d` `⌃u`, `J` `K` | Scroll the note |
| `s` | Start a Claude Code session from the note |
| `a` | Archive: move it to `archive/<folder>/` beside the folder |
| `d`, `⌫` | Move to the macOS Trash (after confirmation) |
| `e` | Open in the default editor |
| `o` | Reveal in Finder |
| `r`, `⌘R` | Reload |
| `?` | Help |

Every search word must match fuzzily, in the title or on one line; as in fzf, `'word` matches exactly, `^word` at the start, `word$` at the end, and `!word` excludes. Opening a search result scrolls the note to the matching line.

"Start a session" guesses the working directory from the git repository the note mentions most (home otherwise) and proposes a prompt; both can be edited. It then opens a Ghostty window (Terminal.app when Ghostty is missing) running `claude '<prompt>'` in your usual shell. The first time, macOS asks to let memo control the terminal.

## Security

- The front end touches no file directly: it has no filesystem or shell plugin, and its capabilities stop at events and window dragging. Every command takes a note path, and Rust checks it is a Markdown file directly inside a configured folder before anything else.
- Markdown is rendered by comrak without raw HTML, then sanitised by ammonia; the CSP allows only the app's own scripts.
- Only `http`, `https` and `mailto` links leave the app, and they open in the default browser. Terminal commands go to osascript as arguments and are shell-quoted, never spliced into the script.

## Install without the sources

```sh
brew install rlvdx/tap/memo
```

`just publish <version>` releases from a Mac: once CI has passed on the pushed `main`, it sets the version, tags it, builds the app and attaches it to a GitHub release. The Publish workflow then copies the app to the public [rlvdx/homebrew-tap](https://github.com/rlvdx/homebrew-tap) and updates its cask, so `brew upgrade` picks it up. The sources stay private.

memo is ad-hoc signed, without an Apple Developer ID, so the cask lifts the quarantine flag Gatekeeper would otherwise block it on.

## Development

Every tool is pinned in `mise.toml`, and the Justfile is the entry point:

```sh
just deps      # install the tools and dependencies
just dev       # run with hot reload
just check     # lint (Rust, Svelte/TypeScript, TOML, Markdown, Justfile, spelling), then test
just install   # build the release bundle into ~/Applications and open it
```

Deno installs the npm packages itself; npm is never used.
