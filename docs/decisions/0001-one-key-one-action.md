# One key, one action

Every action has a single key, without modifier when it acts on the note, and that key always does the same thing.

`c` once meant "comment" over a selection or while reading, and "start a Claude Code session" otherwise. Pressing it then took knowing which mode memo was in, and a stray `c` opened a session dialog. Now `c` only comments and `s` starts a session (**s**ession, not **c**laude).

Keys that act on the note are plain letters; keys that act on memo itself take `⌘`, as on macOS: `⌘K` the palette, `⌘,` the settings, `⌘1` `⌘2` `⌘3` the Notes, Archive and Comments tabs. They work even while typing in a field.

When a new action wants a taken key, it gets another letter, even a less obvious one, rather than sharing it by context. Every key is in one table, `src/lib/commands.ts`, which the key handler, the palette and `?` read; `tests/commands_test.ts` fails when two commands share a key or when the README's keys table drifts from it.
