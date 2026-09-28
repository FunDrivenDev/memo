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
