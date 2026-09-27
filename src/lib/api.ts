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
  /** The archive folder, as typed, and the default one beside Claude Code's plans folder. */
  archive: string;
  archive_default: string;
  archive_custom: boolean;
  file: string;
}

export interface Snippet {
  line: number;
  text: string;
  indices: number[];
}

export interface Hit {
  id: string;
  score: number;
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
}

export const library = () => invoke<Library>("library");
export const settings = () => invoke<Settings>("settings");
/** Saves the folders to show and the archive; `null` goes back to the default. */
export const saveSettings = (folders: string[] | null, archive: string | null) =>
  invoke<Library>("save_settings", { folders, archive });
export const chooseFolder = () => invoke<string | null>("choose_folder");
export const render = (id: string) => invoke<Rendered>("render", { id });
export const search = (query: string, archived: boolean) => invoke<Hit[]>("search", { query, archived });
export const archive = (id: string) => invoke<Moved>("archive", { id });
export const restore = (id: string) => invoke<Moved>("restore", { id });
/** Moves the last archived or restored note back. */
export const undo = () => invoke<Moved>("undo");
export const trash = (id: string) => invoke<Library>("trash", { id });
export const sessionDefaults = (id: string) => invoke<SessionDefaults>("session_defaults", { id });
export const startSession = (workdir: string, prompt: string) =>
  invoke<void>("start_session", { workdir, prompt });
export const reveal = (id: string) => invoke<void>("reveal", { id });
export const edit = (id: string) => invoke<void>("edit", { id });
export const openUrl = (url: string) => invoke<void>("open_url", { url });

export const onOpenSettings = (handler: () => void): Promise<UnlistenFn> => listen("open-settings", handler);

export const onLibraryChanged = (handler: () => void): Promise<UnlistenFn> =>
  listen("library-changed", handler);
