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
  folders: Folder[];
  notes: Note[];
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
export const render = (id: string) => invoke<Rendered>("render", { id });
export const search = (query: string) => invoke<Hit[]>("search", { query });
export const archive = (id: string) => invoke<Library>("archive", { id });
export const trash = (id: string) => invoke<Library>("trash", { id });
export const sessionDefaults = (id: string) => invoke<SessionDefaults>("session_defaults", { id });
export const startSession = (workdir: string, prompt: string) =>
  invoke<void>("start_session", { workdir, prompt });
export const reveal = (id: string) => invoke<void>("reveal", { id });
export const edit = (id: string) => invoke<void>("edit", { id });
export const openUrl = (url: string) => invoke<void>("open_url", { url });

export const onLibraryChanged = (handler: () => void): Promise<UnlistenFn> =>
  listen("library-changed", handler);
