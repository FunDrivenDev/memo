# memo

A small macOS reader for the plans, handoffs and reports Claude Code writes. It is keyboard-driven, has fuzzy search over whole files, and can start a Claude session from a note.

It uses Tauri 2 (a Rust backend and the system WebView, so no bundled browser) and a Svelte 5 front end built by Vite, all run through Deno.

## Where the notes come from

By default memo shows Claude Code's plans folder: `plansDirectory` in `settings.local.json` or `settings.json` of the Claude Code configuration (`$CLAUDE_CONFIG_DIR`, else `~/.claude`), else `~/.claude/plans`.

The settings (`⌘,`, or memo › Settings…) replace it with folders of your choice, typed or picked in Finder, and choose the archive folder. Each change is saved as it is made. A typed folder must exist: for one that does not, memo asks before creating it. The settings are kept per installation in `~/Library/Application Support/dev.rlvdx.memo/settings.json`; the folders have a button back to the default.

Typing a path lists the existing folders that complete it, read from the disk: `↑` `↓` pick one, `⇥` or a click goes into it, `↵` takes it. memo is not sandboxed, so it reads most folders freely. macOS asks once before letting it into Desktop, Documents, Downloads or iCloud Drive; a folder it refused shows a Grant access… button that opens the right pane of System Settings (Files and Folders for those, Full Disk Access for the rest).

## Archive

`a` moves the note, without confirmation, to `<archive>/<folder name>/`; a taken name gets `-2`, `-3`... On first launch memo sets the archive to an `archive` folder beside Claude Code's plans folder (`~/Notes/claude/plans` → `~/Notes/claude/archive/plans/`) and saves it in its settings, so it stays there if `plansDirectory` changes. A message confirms it, and `u` moves the note back while the message shows.

`⇧A` switches the sidebar to the archive, one section per folder notes came from. There `a` restores the note to the watched folder of the same name, again with `u` to undo. Search covers the pane shown only; in the palette, `⇥` switches between the notes and the archive, keeping the query, and an empty result offers it.

Changing the archive folder leaves the notes already archived where they are: move them in Finder.

Each existing folder becomes a sidebar section. The section lists the `.md` files directly inside it, most recently modified first. The folders are watched, so new notes appear as Claude writes them.

## Keys

| Key | Action |
| --- | --- |
| `⌘K`, `/`, `:` | Command palette: fuzzy search over titles and content; start with `>` for commands |
| `j` `k`, `↓` `↑` | Next or previous note |
| `⇥` `⇧⇥` | Next or previous folder |
| `g` `G` | First or last note |
| `space` `⇧space`, `⌃d` `⌃u`, `J` `K` | Scroll the note |
| `s` | Start a Claude Code session from the note |
| `a` | Archive the note (in the archive: restore it) |
| `u` | Undo the archive, restore or trash, while its message shows |
| `⇧A` | Switch between the notes and the archive |
| `t`, `⌫` | Move to the macOS Trash, after a 6-second countdown `u` can cancel |
| `e` | Open in the default editor |
| `o` | Reveal in Finder |
| `r`, `⌘R` | Reload |
| `⌘,` | Settings: folders and archive |
| `?` | Help |

Every search word must match fuzzily, in the title or on one line; as in fzf, `'word` matches exactly, `^word` at the start, `word$` at the end, and `!word` excludes. Opening a search result scrolls the note to the matching line.

"Start a session" guesses the working directory from the git repository the note mentions most (home otherwise) and proposes a prompt; both can be edited. It then opens a Ghostty window (Terminal.app when Ghostty is missing) running `claude '<prompt>'` in your usual shell. The first time, macOS asks to let memo control the terminal.

## Security

- The front end touches no file directly: it has no filesystem or shell plugin, and its capabilities stop at events and window dragging. Every command takes a note path, and Rust checks it is a Markdown file directly inside a configured folder before anything else. The folders themselves are set only through the settings window, which stores them in memo's own settings file.
- Markdown is rendered by comrak without raw HTML, then sanitised by ammonia; the CSP allows only the app's own scripts.
- Only `http`, `https` and `mailto` links leave the app, and they open in the default browser. Terminal commands go to osascript as arguments and are shell-quoted, never spliced into the script.

## Development

Every tool is pinned in `mise.toml`, and the Justfile is the entry point:

```sh
just deps      # install the tools, the dependencies and the pre-push secrets hook
just dev       # run with hot reload
just check     # lint (Rust, Svelte/TypeScript, TOML, Markdown, Justfile, spelling, secrets), then test
just install   # build the release bundle into ~/Applications and open it
```

Deno installs the npm packages itself; npm is never used.
