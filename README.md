# memo

A small macOS reader for the plans, handoffs and reports Claude Code writes. It is keyboard-driven, has fuzzy search over whole files, and can start a Claude session from a note.

It uses Tauri 2 (a Rust backend and the system WebView, so no bundled browser) and a Svelte 5 front end built by Vite, all run through Deno.

## Where the notes come from

By default memo shows Claude Code's plans folder: `plansDirectory` in `settings.local.json` or `settings.json` of the Claude Code configuration (`$CLAUDE_CONFIG_DIR`, else `~/.claude`), else `~/.claude/plans`.

The settings (`⌘,`, or memo › Settings…) replace it with folders of your choice, typed or picked in Finder, and choose the archive folder. Each change is saved as it is made. A typed folder must exist: for one that does not, memo asks before creating it. The settings are kept per installation in `~/Library/Application Support/dev.rlvdx.memo/settings.json`; the folders have a button back to the default.

Typing a path lists the existing folders that complete it, read from the disk: `↑` `↓` pick one, `⌘↑` `⌘↓` the first or last, `⇥` or a click goes into it, `↵` takes it. memo is not sandboxed, so it reads most folders freely. macOS asks once before letting it into Desktop, Documents, Downloads or iCloud Drive; a folder it refused shows a Grant access… button that opens the right pane of System Settings (Files and Folders for those, Full Disk Access for the rest).

## Archive

`a` moves the note, without confirmation, to `<archive>/<folder name>/`; a taken name gets `-2`, `-3`... On first launch memo sets the archive to an `archive` folder beside Claude Code's plans folder (`~/Notes/claude/plans` → `~/Notes/claude/archive/plans/`) and saves it in its settings, so it stays there if `plansDirectory` changes. A message confirms it, and `u` moves the note back while the message shows.

The Archive tab above the sidebar shows the archive, one section per folder notes came from. There `a` restores the note to the watched folder of the same name, again with `u` to undo. Search covers the pane shown only; in the palette, `⇥` switches between the notes and the archive, keeping the query, and an empty result offers it.

Changing the archive folder leaves the notes already archived where they are: move them in Finder.

Each existing folder becomes a sidebar section. The section lists the `.md` files directly inside it, most recently modified first. The folders are watched, so new notes appear as Claude writes them.

## Comments

Comments are notes on a note, like the comments of a pull request review. Select text and press `c` (or the Comment button that shows by the selection), or read the note with `↵`, span more paragraphs with `⇧↓` `⇧↑` if needed, and press `c`; the `+` beside a paragraph comments on it with the mouse. `⌘↵` saves. The comment shows under what it is on, highlighted.

The note itself is never touched: comments live in `comments.json` beside memo's settings, and follow a note memo archives, restores or undoes. When the note changes, a comment finds its text again, preferring the place nearest its old lines; one whose text is gone shows at the top of the note, with what it was on.

A click on a highlight or a comment focuses it, as does the reading cursor on its paragraph: `r` then resolves it, which deletes it, and `u` brings it back while the message shows; `↵` edits it while reading. The Comments tab lists them all, one section per note, with a search field (`/` from anywhere): `↓` `↑` go through them and show each in its note, `⇥` goes to the next note, `↵` edits, `r` resolves. Trashing a note drops its comments.

## Keys

| Key | Action |
| --- | --- |
| `⌘K` | Command palette: fuzzy search over titles and content; start with `>` for commands |
| `↓` `↑` | Next or previous note (in the Comments tab: comment) |
| `⌘↑` `⌘↓` | First or last note |
| `↵` | Read the note: `↓` `↑` move through its paragraphs, list items, code and tables, skipping headings, and keep the current one mid-height; `⌘↑` `⌘↓` go to the first or last; `⇧↓` `⇧↑` span more of them; `↵` edits the comment on them; `esc` goes back to the list |
| `⇥` `⇧⇥` | Next or previous folder |
| `⌘1` `⌘2` `⌘3` | Notes, Archive or Comments tab |
| `space` `⇧space` | Scroll the note by a page |
| `c` | Comment on the selected text, or on the paragraphs read |
| `s` | Start a Claude Code session from the note |
| `r` | Resolve the focused comment |
| `/` | Search the comments |
| `a` | Archive the note (in the archive: restore it) |
| `u` | Undo the archive, restore, trash or resolve, while its message shows |
| `t` | Move to the macOS Trash, after a 6-second countdown `u` can cancel |
| `e` | Open in the default editor |
| `o` | Reveal in Finder |
| `⌘R` | Reload |
| `⌘,` | Settings: folders and archive |
| `?` | Help |

Every search word must match fuzzily, in the title or on one line; notes whose title matches come first, above those matching in the content only; as in fzf, `'word` matches exactly, `^word` at the start, `word$` at the end, and `!word` excludes. Opening a search result scrolls the note to the matching line. The text of the comments is searched by the same rules, and its matches listed apart, under "In the comments", after the notes; opening one shows it in the Comments tab.

`s` starts a session: it proposes a prompt and a folder, both editable. The folder is the git repository the note mentions most, else the folder sessions usually start in, else home. The folder field lists the usual folders first, the most used lately at the top; typing filters them by any part of their path (`memo` finds `~/Code/rlvdx/memo`) and completes a path from the disk as in the settings. memo remembers the folders in `workdirs.json` beside its settings, 30 at most. The session opens a Ghostty window (Terminal.app when Ghostty is missing) running `claude '<prompt>'` in your usual shell. The first time, macOS asks to let memo control the terminal.

## Security

- The front end touches no file directly: it has no filesystem or shell plugin, and its capabilities stop at events and window dragging. Every command takes a note path, and Rust checks it is a Markdown file directly inside a configured folder before anything else. The folders themselves are set only through the settings window, which stores them in memo's own settings file. Comments go to memo's own `comments.json`, never into the note, and a new or restored one must name a note memo shows.
- Markdown is rendered by comrak without raw HTML, fenced code is coloured by syntect with bat's grammars, then everything is sanitised by ammonia; the CSP allows only the app's own scripts.
- The copy button of a code block hands its text to `pbcopy`: the front end can write to the clipboard, never read it.
- Only `http`, `https` and `mailto` links leave the app, and they open in the default browser. Terminal commands go to osascript as arguments and are shell-quoted, never spliced into the script.

## Install

```sh
brew install fundrivendev/tap/memo
```

Every merge to `main` is released once CI passes on it. The Release workflow waits 5 minutes and releases only `main`'s head, so pull requests merged close together ship as one release. It bumps the patch version, or the minor or major one when a pull request in the release carries the `minor` or `major` label. It builds the app on macOS and installs it there from the cask, then, on Linux, scans it for secrets and home directory paths, tags the commit and releases it. It then copies the app to [FunDrivenDev/homebrew-tap](https://github.com/FunDrivenDev/homebrew-tap) and updates its cask, so `brew upgrade` picks it up. The tag and the cask commit are signed as Fun Driven Stuff <stuff@fundriven.dev>. If a release fails, "Re-run failed jobs" finishes it.

Every job runs on GitHub-hosted runners, free and unlimited for a public repository, and on Linux except the app build and the cask check.

memo is ad-hoc signed, without an Apple Developer ID, so the cask lifts the quarantine flag Gatekeeper would otherwise block it on.

## Development

Every tool is pinned in `mise.toml`, and the Justfile is the entry point. On a fresh clone, run `just install` first:

```sh
just install             # install the tools, the dependencies, headless Chromium for the tests and the git hooks
just dev                 # run with hot reload, the dev server on a free port so other projects' can run alongside
just check               # lint (Rust, Svelte/TypeScript, TOML, Markdown, Justfile, shell scripts, workflows, spelling, secrets), then test
just audit               # secrets and personal data in the history, vulnerable dependencies, unsafe workflows
just app                 # build the release bundle into ~/Applications and open it
just cask                # macOS: check the cask against Homebrew's style, audit and deprecation rules
just cask <zip>          # the same, then install that archive from the cask into a scratch folder
```

Deno installs the npm packages itself; npm is never used.

The pre-commit hook runs `just check`, and the pre-push hook looks for secrets.

The note's column takes 66% of its pane, and at least 60 characters when the window is narrow; `--note-width` in `src/app.css` sets that share.
