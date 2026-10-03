// The message shown at the bottom, and the actions `u` undoes while it shows: archive or restore, trash, resolve.
import { createSubscriber, SvelteSet } from "svelte/reactivity";
import type { Comment, Library, Moved, Note } from "./api.ts";

/** How long an archive, restore, trash or resolve can be undone with `u`. */
export const UNDO_MS = 6000;
/** How long a message shows, and an error. */
const MESSAGE_MS = 3000;
const ERROR_MS = 6000;

/** The Rust commands the actions call, `./api` in the app. */
export interface ActionsApi {
  archive(id: string): Promise<Moved>;
  restore(id: string): Promise<Moved>;
  /** Moves the last archived or restored note back. */
  undo(): Promise<Moved>;
  trash(id: string): Promise<Library>;
  resolveComment(id: string): Promise<Comment[]>;
  restoreComment(comment: Comment): Promise<Comment[]>;
}

/** The clock the countdowns run on, the window's in the app. */
export interface Timers {
  setTimeout(run: () => void, ms: number): number;
  clearTimeout(id: number | undefined): void;
}

/** What the app does with the outcome of an action. */
export interface Host {
  /** Takes the library after an archive, restore, trash or undo. */
  library(next: Library): void;
  /** Takes the comments after a resolve or its undo. */
  comments(next: Comment[]): void;
  /** Shows what an undo brought back: a note, or a comment. */
  reopen(target: { note: string } | { comment: string }): void;
}

export interface Toast {
  text: string;
  error: boolean;
  /** Whether `u` undoes something while it shows. */
  undoable: boolean;
}

/**
 * Every reversible action, each undone its own way: an archive or restore by the Rust side moving the note back, a
 * resolve by sending the comment back, a trash by never happening, macOS offering no undo: the note only hides until
 * its countdown ends, any new message ending it at once. Reading `toast` or `hidden` in a component tracks them.
 */
export class Actions {
  #api: ActionsApi;
  #timers: Timers;
  #host: Host;
  #toast: Toast | null = null;
  #undo: (() => Promise<void> | void) | null = null;
  #toastTimer: number | undefined;
  /** The note trashed on screen only, until its countdown ends or `u` brings it back. */
  #pendingTrash: { note: Note; timer: number } | null = null;
  /** Notes hidden as trashed: the pending one, and those on their way to the Trash. */
  #trashing = new SvelteSet<string>();
  #changed = () => {};
  #subscribe = createSubscriber((update) => {
    this.#changed = update;
    return () => (this.#changed = () => {});
  });

  constructor(api: ActionsApi, host: Host, timers: Timers) {
    this.#api = api;
    this.#host = host;
    this.#timers = timers;
  }

  /** The message showing, if any. */
  get toast(): Toast | null {
    this.#subscribe();
    return this.#toast;
  }

  /** Whether a note is trashed already on screen. */
  hidden(id: string): boolean {
    return this.#trashing.has(id);
  }

  /** Shows a message, ending a pending trash. */
  show(text: string, error = false) {
    this.#show(text, error, null);
  }

  /** Archives a note of the folders, or restores one of the archive. */
  async archive(note: Note, restoring: boolean) {
    try {
      const moved = await (restoring ? this.#api.restore(note.id) : this.#api.archive(note.id));
      this.#host.library(moved.library);
      this.#show(
        restoring ? `Restored “${note.title}” to ${note.folder}` : `Archived “${note.title}”`,
        false,
        () => this.#undoMove(),
      );
    } catch (e) {
      this.show(String(e), true);
    }
  }

  /** Hides a note at once, and trashes it for real when the countdown ends. */
  trash(note: Note) {
    this.#show(`Moved “${note.title}” to the Trash`, false, () => this.#cancelTrash(note));
    this.#trashing.add(note.id);
    this.#pendingTrash = { note, timer: this.#timers.setTimeout(() => this.#commitTrash(), UNDO_MS) };
  }

  /** Resolves a comment, which deletes it; true once done. */
  async resolve(comment: Comment): Promise<boolean> {
    try {
      this.#host.comments(await this.#api.resolveComment(comment.id));
      this.#show("Resolved the comment", false, () => this.#restoreComment(comment));
      return true;
    } catch (e) {
      this.show(String(e), true);
      return false;
    }
  }

  /** Undoes the action of the message showing, if any. */
  async undo() {
    const run = this.#undo;
    if (!run) return;
    this.#set(null, null);
    await run();
  }

  #show(text: string, error: boolean, undo: (() => Promise<void> | void) | null) {
    void this.#commitTrash();
    this.#set({ text, error, undoable: !!undo }, undo);
    this.#timers.clearTimeout(this.#toastTimer);
    this.#toastTimer = this.#timers.setTimeout(
      () => this.#set(null, null),
      undo ? UNDO_MS : error ? ERROR_MS : MESSAGE_MS,
    );
  }

  #set(toast: Toast | null, undo: (() => Promise<void> | void) | null) {
    this.#toast = toast;
    this.#undo = undo;
    this.#changed();
  }

  async #undoMove() {
    try {
      const moved = await this.#api.undo();
      this.#host.library(moved.library);
      this.#host.reopen({ note: moved.id });
      this.show("Undone");
    } catch (e) {
      this.show(String(e), true);
    }
  }

  async #restoreComment(comment: Comment) {
    try {
      this.#host.comments(await this.#api.restoreComment(comment));
      this.#host.reopen({ comment: comment.id });
      this.show("Undone");
    } catch (e) {
      this.show(String(e), true);
    }
  }

  #cancelTrash(note: Note) {
    if (this.#pendingTrash?.note !== note) return;
    this.#timers.clearTimeout(this.#pendingTrash.timer);
    this.#pendingTrash = null;
    this.#trashing.delete(note.id);
    this.#host.reopen({ note: note.id });
  }

  async #commitTrash() {
    if (!this.#pendingTrash) return;
    const { note, timer } = this.#pendingTrash;
    this.#timers.clearTimeout(timer);
    this.#pendingTrash = null;
    try {
      this.#host.library(await this.#api.trash(note.id));
    } catch (e) {
      this.show(String(e), true);
    } finally {
      this.#trashing.delete(note.id);
    }
  }
}
