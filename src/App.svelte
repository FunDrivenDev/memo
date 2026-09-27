<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import * as api from "./lib/api";
  import type { Library, Moved, Note } from "./lib/api";
  import Help from "./lib/Help.svelte";
  import { listStep } from "./lib/keys";
  import NoteView from "./lib/NoteView.svelte";
  import Palette, { type Command } from "./lib/Palette.svelte";
  import SessionDialog from "./lib/SessionDialog.svelte";
  import Settings from "./lib/Settings.svelte";
  import Sidebar, { type Pane } from "./lib/Sidebar.svelte";

  type Overlay = "palette" | "session" | "help" | "settings";

  /** How long an archive, restore or trash can be undone with `u`. */
  const UNDO_MS = 6000;

  let library = $state<Library>({
    claude_dir: "",
    custom: false,
    folders: [],
    notes: [],
    archive: "",
    archive_folders: [],
    archived: [],
  });
  let loaded = $state(false);
  let pane = $state<Pane>("notes");
  let selectedId = $state<string | null>(null);
  /** The selection of the pane not shown, restored when switching back. */
  let otherId: string | null = null;
  let focusLine = $state<number | null>(null);
  /** Whether the keys move through the items of the note rather than through the list: `↵` in, `Esc` out. */
  let reading = $state(false);
  let overlay = $state<Overlay | null>(null);
  /** Whether the palette searches the archive; Tab switches it. */
  let paletteArchive = $state(false);
  let toast = $state<{ text: string; error: boolean; undo: (() => void) | null } | null>(null);
  let view: NoteView | undefined = $state();
  let toastTimer: ReturnType<typeof setTimeout> | undefined;
  /** The note trashed on screen only, until its countdown ends or `u` brings it back. */
  let pendingTrash: { note: Note; timer: ReturnType<typeof setTimeout> } | null = null;
  /** Notes hidden as trashed: the pending one, and those on their way to the Trash. */
  const trashing = new SvelteSet<string>();

  const shown = (notes: Note[]) => notes.filter((n) => !trashing.has(n.id));
  const notes = $derived(shown(library.notes));
  const archived = $derived(shown(library.archived));
  const folders = $derived(pane === "archive" ? library.archive_folders : library.folders);
  /** The notes of the pane in sidebar order: by folder, then most recent first. */
  const ordered = $derived.by(() => {
    const paneNotes = pane === "archive" ? archived : notes;
    return folders.flatMap((f) => paneNotes.filter((n) => n.folder === f.name));
  });
  const selected = $derived(ordered.find((n) => n.id === selectedId) ?? null);

  /** Shows a message; with `undo`, `u` runs it while the message shows. A new message ends a pending trash. */
  function show(text: string, error = false, undo: (() => void) | null = null) {
    void commitTrash();
    toast = { text, error, undo };
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), undo ? UNDO_MS : error ? 6000 : 3000);
  }

  /** Takes a new library, keeping the selection or, when its note left, the note now in its place. */
  function apply(next: Library) {
    const before = ordered.findIndex((n) => n.id === selectedId);
    library = next;
    loaded = true;
    if (selectedId && ordered.some((n) => n.id === selectedId)) return;
    selectedId = (ordered[Math.max(0, Math.min(before, ordered.length - 1))] ?? null)?.id ?? null;
  }

  async function reload() {
    try {
      apply(await api.library());
    } catch (e) {
      show(String(e), true);
    }
  }

  $effect(() => {
    reload();
    const unlisten = [api.onLibraryChanged(reload), api.onOpenSettings(() => (overlay = "settings"))];
    return () => unlisten.forEach((u) => void u.then((stop) => stop()));
  });

  // Reading ends when another note shows, whatever changed the selection.
  $effect(() => {
    void selectedId;
    reading = false;
  });

  function startReading() {
    if (!selected) return;
    reading = true;
    view?.startReading();
  }

  function stopReading() {
    reading = false;
    view?.stopReading();
  }

  function select(id: string | null, line: number | null = null) {
    stopReading();
    focusLine = line;
    selectedId = id;
  }

  function showPane(next: Pane) {
    if (next === pane) return;
    [selectedId, otherId] = [otherId, selectedId];
    pane = next;
    if (!ordered.some((n) => n.id === selectedId)) selectedId = ordered[0]?.id ?? null;
  }

  /** Jumps to the first note of the next (or previous) folder holding any. */
  function moveFolder(delta: 1 | -1) {
    const withNotes = folders.filter((f) => ordered.some((n) => n.folder === f.name));
    if (!withNotes.length) return;
    const current = withNotes.findIndex((f) => f.name === selected?.folder);
    const target = withNotes[(current + delta + withNotes.length) % withNotes.length];
    select(ordered.find((n) => n.folder === target?.name)?.id ?? null);
  }

  function openNote(id: string): boolean {
    if (library.notes.some((n) => n.id === id)) showPane("notes");
    else if (library.archived.some((n) => n.id === id)) showPane("archive");
    else return false;
    select(id);
    return true;
  }

  /** Archives a note of the folders, or restores one of the archive; `u` undoes it for a while. */
  async function archiveOrRestore(note: Note) {
    try {
      const restoring = pane === "archive";
      apply((restoring ? await api.restore(note.id) : await api.archive(note.id)).library);
      show(restoring ? `Restored “${note.title}” to ${note.folder}` : `Archived “${note.title}”`, false, undoMove);
    } catch (e) {
      show(String(e), true);
    }
  }

  function undo() {
    const run = toast?.undo;
    if (!run) return;
    toast = null;
    run();
  }

  async function undoMove() {
    try {
      const moved: Moved = await api.undo();
      apply(moved.library);
      openNote(moved.id);
      show("Undone");
    } catch (e) {
      show(String(e), true);
    }
  }

  /**
   * Trashes a note on screen at once, and for real when the countdown ends: macOS offers no undo, so `u` cancels it
   * before it happens.
   */
  function trash(note: Note) {
    const before = ordered.findIndex((n) => n.id === note.id);
    show(`Moved “${note.title}” to the Trash`, false, () => cancelTrash(note));
    trashing.add(note.id);
    pendingTrash = { note, timer: setTimeout(commitTrash, UNDO_MS) };
    if (selectedId === note.id) select(ordered[Math.min(before, ordered.length - 1)]?.id ?? null);
  }

  function cancelTrash(note: Note) {
    if (pendingTrash?.note !== note) return;
    clearTimeout(pendingTrash.timer);
    pendingTrash = null;
    trashing.delete(note.id);
    openNote(note.id);
  }

  async function commitTrash() {
    if (!pendingTrash) return;
    const { note, timer } = pendingTrash;
    clearTimeout(timer);
    pendingTrash = null;
    try {
      apply(await api.trash(note.id));
    } catch (e) {
      show(String(e), true);
    } finally {
      trashing.delete(note.id);
    }
  }

  function withNote(run: (note: Note) => void) {
    return () => {
      if (selected) run(selected);
    };
  }

  function openPalette() {
    paletteArchive = pane === "archive";
    overlay = "palette";
  }

  const edit = withNote((n) => api.edit(n.id).catch((e) => show(String(e), true)));
  const revealNote = withNote((n) => api.reveal(n.id).catch((e) => show(String(e), true)));

  const commands: Command[] = $derived([
    { id: "session", label: "Start a Claude Code session", keys: "c", run: withNote(() => (overlay = "session")) },
    {
      id: "archive",
      label: pane === "archive" ? "Restore note from the archive" : "Archive note",
      keys: "a",
      run: withNote(archiveOrRestore),
    },
    {
      id: "pane",
      label: pane === "archive" ? "Show the notes" : "Show the archive",
      run: () => showPane(pane === "archive" ? "notes" : "archive"),
    },
    { id: "trash", label: "Move note to the Trash", keys: "t", run: withNote(trash) },
    { id: "edit", label: "Open in editor", keys: "e", run: edit },
    { id: "reveal", label: "Reveal in Finder", keys: "o", run: revealNote },
    { id: "reload", label: "Reload", keys: "⌘R", run: reload },
    { id: "settings", label: "Settings: folders and archive", keys: "⌘,", run: () => (overlay = "settings") },
    { id: "help", label: "Keyboard shortcuts", keys: "?", run: () => (overlay = "help") },
  ]);

  function onKeydown(event: KeyboardEvent) {
    if (event.metaKey && event.key === "k") {
      event.preventDefault();
      if (overlay === "palette") overlay = null;
      else openPalette();
      return;
    }
    // The Settings… menu item normally takes ⌘, first; this covers a missing menu.
    if (event.metaKey && event.key === ",") {
      event.preventDefault();
      overlay = "settings";
      return;
    }
    if (event.metaKey && event.key === "r") {
      event.preventDefault();
      reload();
      return;
    }
    const target = event.target as HTMLElement;
    // A dialog's Escape has closed it by the time this runs, or will just after.
    if (overlay || event.defaultPrevented || event.ctrlKey || event.altKey || target.closest("input, textarea")) return;

    const i = reading ? null : listStep(event, ordered.findIndex((n) => n.id === selectedId), ordered.length);
    if (reading ? view?.step(event) : i !== null) {
      event.preventDefault();
      if (i !== null) select(ordered[i]?.id ?? null);
      return;
    }

    // One key per action: ⌘ is spelt out, ⇧ only in the character it gives (`?`, `A`), or in Tab and space.
    const key = event.metaKey ? `⌘${event.key}` : event.key;
    const actions: Record<string, () => void> = {
      Tab: () => moveFolder(event.shiftKey ? -1 : 1),
      " ": () => view?.scroll(event.shiftKey ? -1 : 1),
      "?": () => (overlay = "help"),
      c: withNote(() => (overlay = "session")),
      a: withNote(archiveOrRestore),
      u: undo,
      t: withNote(trash),
      e: edit,
      o: revealNote,
    };
    const noteKeys: Record<string, () => void> = reading
      ? {
          Escape: stopReading,
        }
      : { Enter: startReading };
    const action = noteKeys[key] ?? actions[key];
    if (action) {
      event.preventDefault();
      action();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="layout">
  <Sidebar
    {pane}
    {folders}
    notes={ordered}
    counts={{ notes: notes.length, archive: archived.length }}
    {selectedId}
    active={!reading}
    onSelect={(id) => select(id)}
    onPane={showPane}
    onPalette={openPalette}
  />

  {#if selected}
    <NoteView
      bind:this={view}
      note={selected}
      archived={pane === "archive"}
      {focusLine}
      onOpenNote={openNote}
      onError={(m) => show(m, true)}
    />
  {:else if loaded}
    <div class="blank" data-tauri-drag-region>
      {#if pane === "archive"}
        <h1>The archive is empty</h1>
        <p>
          <kbd>a</kbd> on a note moves it to <code>{library.archive}</code>. The Notes tab above goes back to them.
        </p>
      {:else if !library.folders.length}
        <h1>No folders found</h1>
        <p>
          {#if library.custom}
            None of the folders chosen in the settings exists.
          {:else}
            memo shows Claude Code's plans folder: <code>plansDirectory</code> in
            <code>{library.claude_dir}/settings.json</code>, else <code>{library.claude_dir}/plans</code>. It does not
            exist yet.
          {/if}
          Press <kbd>⌘,</kbd> to choose the folders.
        </p>
      {:else}
        <h1>Nothing to read</h1>
        <p>The folders hold no Markdown file yet. New ones show up as Claude Code writes them.</p>
      {/if}
    </div>
  {/if}
</div>

{#if overlay === "palette"}
  <Palette
    {notes}
    {archived}
    bind:archive={paletteArchive}
    {commands}
    onOpen={(id, line) => {
      overlay = null;
      openNote(id);
      focusLine = line;
    }}
    onClose={() => (overlay = null)}
  />
{:else if overlay === "session" && selected}
  <SessionDialog
    note={selected}
    onDone={(message, error) => {
      overlay = null;
      show(message, error);
    }}
    onClose={() => (overlay = null)}
  />
{:else if overlay === "help"}
  <Help folders={library.folders} archive={library.archive} onClose={() => (overlay = null)} />
{:else if overlay === "settings"}
  <Settings
    onSaved={apply}
    onError={(m) => show(m, true)}
    onClose={() => (overlay = null)}
  />
{/if}

{#if toast}
  {#key toast}
  <div class="toast" class:error={toast.error} role="status">
    <span>{toast.text}</span>
    {#if toast.undo}
      <button class="undo" onclick={undo}>Undo <kbd>u</kbd></button>
      <span class="countdown" style:animation-duration={`${UNDO_MS}ms`}></span>
    {/if}
  </div>
  {/key}
{/if}

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(260px, 320px) 1fr;
    height: 100%;
  }

  .blank {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    padding: 40px;
    text-align: center;
    color: var(--subtext);
  }

  .blank h1 {
    font-size: 18px;
    color: var(--text);
  }

  .blank p {
    max-width: 52ch;
    line-height: 1.6;
  }

  .toast {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 30;
    max-width: 70vw;
    padding: 8px 14px;
    border-radius: 8px;
    border: 1px solid var(--surface0);
    background: var(--mantle);
    box-shadow: var(--shadow);
    font-size: 13px;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    overflow: hidden;
    animation: rise 160ms ease-out;
  }

  .undo {
    padding: 2px 8px;
    border: 1px solid var(--surface0);
    border-radius: 6px;
    background: var(--base);
    cursor: pointer;
  }

  /* Shrinks while the undo stays possible. */
  .countdown {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 2px;
    width: 100%;
    background: var(--accent);
    transform-origin: left;
    animation: countdown linear forwards;
  }

  @keyframes countdown {
    to {
      transform: scaleX(0);
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
  }

  .toast.error {
    border-color: var(--red);
    color: var(--red);
  }
</style>
