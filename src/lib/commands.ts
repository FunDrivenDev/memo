// Every key of the main window, in one table: the keydown handler, the palette, `?` and the README's keys table
// derive from it, so that one key keeps one action (docs/decisions/0001-one-key-one-action.md).

/**
 * When a command's keys work, the scopes nesting: `anywhere`, even over a dialog or while typing in a field;
 * `window`, while typing but not over a dialog; `note`, neither over a dialog nor while typing.
 */
export type Scope = "anywhere" | "window" | "note";

/** A palette entry's label, or one per pane: a note of the folders, or of the archive. */
export type Label = string | { notes: string; archive: string };

export interface Command {
  /** Names its handler, which App.svelte gives. */
  id: string;
  /** Its keys, written as in the README: `⌘` spelt out, `⇧` too for a named key (`⇧⇥`) but not in a character (`?`). */
  keys: readonly string[];
  scope: Scope;
  /** What it does, as the README's keys table and `?` say it, keys in backticks. */
  help: string;
  /** The palette's entry for each of its keys it lists. */
  palette?: Readonly<Record<string, Label>>;
  /** Whether its keys move through a list or the note read, which App.svelte and NoteView handle before the table. */
  moves?: true;
}

/** The key showing each tab. */
export const paneKeys = { notes: "⌘1", archive: "⌘2", comments: "⌘3" } as const;

export const commands = [
  {
    id: "palette",
    keys: ["⌘K"],
    scope: "anywhere",
    help: "Command palette: fuzzy search over titles and content; start with `>` for commands",
  },
  {
    id: "next",
    keys: ["↓", "↑"],
    scope: "note",
    help: "Next or previous note (in the Comments tab: comment)",
    moves: true,
  },
  { id: "ends", keys: ["⌘↑", "⌘↓"], scope: "note", help: "First or last note", moves: true },
  {
    id: "read",
    keys: ["↵"],
    scope: "note",
    help:
      "Read the note: `↓` `↑` move through its paragraphs, list items, code and tables, skipping headings, and keep the current one mid-height; `⌘↑` `⌘↓` go to the first or last; `⇧↓` `⇧↑` span more of them; `↵` edits the comment on them; `esc` goes back to the list",
  },
  {
    id: "fold",
    keys: ["←", "→"],
    scope: "note",
    help: "Fold the folder of the note, or unfold the folder selected",
    moves: true,
  },
  { id: "folder", keys: ["⇥", "⇧⇥"], scope: "note", help: "Next or previous folder" },
  {
    id: "pane",
    keys: Object.values(paneKeys),
    scope: "window",
    help: "Notes, Archive or Comments tab",
    palette: { "⌘1": "Show the notes", "⌘2": "Show the archive", "⌘3": "Show the comments" },
  },
  { id: "scroll", keys: ["space", "⇧space"], scope: "note", help: "Scroll the note by a page" },
  {
    id: "comment",
    keys: ["c"],
    scope: "note",
    help: "Comment on the selected text, or on the paragraphs read",
    palette: { c: "Comment on the selection or the item read" },
  },
  {
    id: "session",
    keys: ["s"],
    scope: "note",
    help: "Start a Claude Code session from the note",
    palette: { s: "Start a Claude Code session" },
  },
  {
    id: "resolve",
    keys: ["r"],
    scope: "note",
    help: "Resolve the focused comment",
    palette: { r: "Resolve the comment" },
  },
  {
    id: "search",
    keys: ["/"],
    scope: "note",
    help: "Search the comments",
    palette: { "/": "Search the comments" },
  },
  {
    id: "archive",
    keys: ["a"],
    scope: "note",
    help: "Archive the note (in the archive: restore it)",
    palette: { a: { notes: "Archive note", archive: "Restore note from the archive" } },
  },
  {
    id: "undo",
    keys: ["u"],
    scope: "note",
    help: "Undo the archive, restore, trash or resolve, while its message shows",
  },
  {
    id: "trash",
    keys: ["t"],
    scope: "note",
    help: "Move to the macOS Trash, after a 6-second countdown `u` can cancel",
    palette: { t: "Move note to the Trash" },
  },
  {
    id: "edit",
    keys: ["e"],
    scope: "note",
    help: "Open in the default editor",
    palette: { e: "Open in editor" },
  },
  { id: "reveal", keys: ["o"], scope: "note", help: "Reveal in Finder", palette: { o: "Reveal in Finder" } },
  {
    id: "reload",
    keys: ["⌘R"],
    scope: "anywhere",
    help: "Reload: reread the folders and watch them afresh, for a new note that does not show",
    palette: { "⌘R": "Reload the folders" },
  },
  {
    id: "settings",
    keys: ["⌘,"],
    scope: "anywhere",
    help: "Settings: folders and archive",
    palette: { "⌘,": "Settings: folders and archive" },
  },
  { id: "help", keys: ["?"], scope: "note", help: "Help", palette: { "?": "Keyboard shortcuts" } },
] as const satisfies readonly Command[];

export type CommandId = (typeof commands)[number]["id"];
/** The commands App.svelte runs, those whose keys don't move. */
export type RunId = Exclude<CommandId, Extract<(typeof commands)[number], { moves: true }>["id"]>;

/** Runs a command, given the key that ran it. */
export type Handler = (key: string) => void;

const byKey = new Map<string, Command>(commands.flatMap((c: Command) => c.keys.map((k) => [k, c] as const)));

const named: Record<string, string> = {
  ArrowDown: "↓",
  ArrowUp: "↑",
  ArrowLeft: "←",
  ArrowRight: "→",
  Enter: "↵",
  Escape: "esc",
  Tab: "⇥",
  " ": "space",
};

/** A key pressed, written as in the README: `⌘K`, `⇧⇥`, `space`, `?`. */
export function keyName(event: { key: string; metaKey: boolean; shiftKey: boolean }): string {
  const name = named[event.key];
  const key = name ?? (event.metaKey && event.key.length === 1 ? event.key.toUpperCase() : event.key);
  return (event.metaKey ? "⌘" : "") + (name && event.shiftKey ? "⇧" : "") + key;
}

/** The command a key runs, unless its keys move, which the table only documents. */
export function commandFor(key: string): (Command & { id: RunId }) | null {
  const command = byKey.get(key);
  return command && !command.moves ? (command as Command & { id: RunId }) : null;
}
