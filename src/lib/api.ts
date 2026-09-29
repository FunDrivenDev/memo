// Typed wrappers of the Rust commands; see src-tauri/src/lib.rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface Folder {
  name: string;
  path: string;
}

export interface Note {
  /** The absolute path. */
  id: string;
  folder: string;
  file_name: string;
  title: string;
  excerpt: string;
  /** Milliseconds since the Unix epoch. */
  modified: number;
}

export interface Library {
  claude_dir: string;
  /** Whether the folders come from memo's settings rather than Claude Code's plans folder. */
  custom: boolean;
  folders: Folder[];
  notes: Note[];
  /** The archive folder, which may not exist yet. */
  archive: string;
  /** The archive's sections, one per folder notes were archived from. */
  archive_folders: Folder[];
  archived: Note[];
  comments: Comment[];
}

/** What a comment is attached to. */
export interface Anchor {
  /** A selected passage, or whole paragraphs, list items, code blocks or tables. */
  kind: "text" | "block";
  /** The source lines it spanned when written. */
  start: number;
  end: number;
  /** The text it covers, whitespace collapsed. */
  quote: string;
  /** A few characters around a passage, whitespace removed, telling repeats apart. */
  prefix: string;
  suffix: string;
}

export interface Comment {
  id: string;
  /** The note's path. */
  note: string;
  body: string;
  anchor: Anchor;
  /** Milliseconds since the Unix epoch. */
  created: number;
  updated: number;
}

/** What an archive, restore or undo did: the note's new path, and the library after it. */
export interface Moved {
  id: string;
  library: Library;
}

export interface FolderSetting {
  /** As typed: absolute or under `~`. */
  path: string;
  exists: boolean;
}

export interface Settings {
  folders: FolderSetting[];
  /** Claude Code's plans folder, shown when no folder is chosen. */
  defaults: string[];
  custom: boolean;
  /** The archive folder, as typed. */
  archive: string;
  file: string;
}

export interface Completion {
  /** The subfolders completing the typed path, written the same way. */
  folders: string[];
  /** The folder macOS refused to list, as typed. */
  denied: string | null;
}

export interface Snippet {
  line: number;
  text: string;
  indices: number[];
}

export interface Hit {
  id: string;
  score: number;
  /** Whether the title matches; such hits come first, before the content-only ones. */
  in_title: boolean;
  title_indices: number[];
  snippets: Snippet[];
}

export interface Rendered {
  html: string;
  words: number;
}

export interface SessionDefaults {
  workdir: string;
  prompt: string;
  /** The existing folders sessions started in, the usual ones first. */
  recent: string[];
}

export const library = () => invoke<Library>("library");
export const settings = () => invoke<Settings>("settings");
/** Saves the folders to show, `null` following Claude Code's plans folder, and the archive. */
export const saveSettings = (folders: string[] | null, archive: string) =>
  invoke<Library>("save_settings", { folders, archive });
export const chooseFolder = () => invoke<string | null>("choose_folder");
/** The typed folder, cleaned, and whether it exists; an error for a path neither absolute nor under `~`. */
export const inspectFolder = (folder: string) => invoke<FolderSetting>("inspect_folder", { folder });
export const createFolder = (folder: string) => invoke<void>("create_folder", { folder });
export const completeFolder = (typed: string) => invoke<Completion>("complete_folder", { typed });
/** Opens System Settings where memo can be allowed into `folder`. */
export const openPrivacySettings = (folder: string) => invoke<void>("open_privacy_settings", { folder });
export const render = (id: string) => invoke<Rendered>("render", { id });
export const search = (query: string, archived: boolean) => invoke<Hit[]>("search", { query, archived });
export const archive = (id: string) => invoke<Moved>("archive", { id });
export const restore = (id: string) => invoke<Moved>("restore", { id });
/** Moves the last archived or restored note back. */
export const undo = () => invoke<Moved>("undo");
export const trash = (id: string) => invoke<Library>("trash", { id });
export const addComment = (note: string, body: string, anchor: Anchor) =>
  invoke<Comment[]>("add_comment", { note, body, anchor });
export const editComment = (id: string, body: string) => invoke<Comment[]>("edit_comment", { id, body });
/** Deletes the comment; `restoreComment` puts it back. */
export const resolveComment = (id: string) => invoke<Comment[]>("resolve_comment", { id });
export const restoreComment = (comment: Comment) => invoke<Comment[]>("restore_comment", { comment });
export const sessionDefaults = (id: string) => invoke<SessionDefaults>("session_defaults", { id });
export const startSession = (workdir: string, prompt: string) =>
  invoke<void>("start_session", { workdir, prompt });
export const reveal = (id: string) => invoke<void>("reveal", { id });
export const edit = (id: string) => invoke<void>("edit", { id });
export const openUrl = (url: string) => invoke<void>("open_url", { url });
export const copy = (text: string) => invoke<void>("copy", { text });

export const onOpenSettings = (handler: () => void): Promise<UnlistenFn> => listen("open-settings", handler);

export const onLibraryChanged = (handler: () => void): Promise<UnlistenFn> =>
  listen("library-changed", handler);
