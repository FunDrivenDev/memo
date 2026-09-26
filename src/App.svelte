<script lang="ts">
  import * as api from "./lib/api";
  import type { Library, Note } from "./lib/api";
  import Confirm from "./lib/Confirm.svelte";
  import Help from "./lib/Help.svelte";
  import NoteView from "./lib/NoteView.svelte";
  import Palette, { type Command } from "./lib/Palette.svelte";
  import SessionDialog from "./lib/SessionDialog.svelte";
  import Sidebar from "./lib/Sidebar.svelte";

  type Overlay = "palette" | "trash" | "session" | "help";

  let library = $state<Library>({ claude_dir: "", folders: [], notes: [] });
  let loaded = $state(false);
  let selectedId = $state<string | null>(null);
  let focusLine = $state<number | null>(null);
  let overlay = $state<Overlay | null>(null);
  let toast = $state<{ text: string; error: boolean } | null>(null);
  let view: NoteView | undefined = $state();
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  /** The notes in sidebar order: by folder, then most recent first. */
  const ordered = $derived(library.folders.flatMap((f) => library.notes.filter((n) => n.folder === f.name)));
  const selected = $derived(ordered.find((n) => n.id === selectedId) ?? null);

  function show(text: string, error = false) {
    toast = { text, error };
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), error ? 6000 : 3000);
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
    const unlisten = api.onLibraryChanged(reload);
    return () => void unlisten.then((stop) => stop());
  });

  function select(id: string | null, line: number | null = null) {
    focusLine = line;
    selectedId = id;
  }

  function move(delta: number) {
    if (!ordered.length) return;
    const i = ordered.findIndex((n) => n.id === selectedId);
    const next = i < 0 ? 0 : Math.max(0, Math.min(ordered.length - 1, i + delta));
    select(ordered[next]?.id ?? null);
  }

  /** Jumps to the first note of the next (or previous) folder holding any. */
  function moveFolder(delta: 1 | -1) {
    const withNotes = library.folders.filter((f) => ordered.some((n) => n.folder === f.name));
    if (!withNotes.length) return;
    const current = withNotes.findIndex((f) => f.name === selected?.folder);
    const target = withNotes[(current + delta + withNotes.length) % withNotes.length];
    select(ordered.find((n) => n.folder === target?.name)?.id ?? null);
  }

  function openNote(id: string): boolean {
    if (!ordered.some((n) => n.id === id)) return false;
    select(id);
    return true;
  }

  async function archive(note: Note) {
    try {
      apply(await api.archive(note.id));
      show(`Archived “${note.title}”`);
    } catch (e) {
      show(String(e), true);
    }
  }

  async function trash(note: Note) {
    overlay = null;
    try {
      apply(await api.trash(note.id));
      show(`Moved “${note.title}” to the Trash`);
    } catch (e) {
      show(String(e), true);
    }
  }

  function withNote(run: (note: Note) => void) {
    return () => {
      if (selected) run(selected);
    };
  }

  const commands: Command[] = [
    { id: "session", label: "Start a Claude Code session", keys: "s", run: withNote(() => (overlay = "session")) },
    { id: "archive", label: "Archive note", keys: "a", run: withNote(archive) },
    { id: "trash", label: "Move note to the Trash", keys: "d", run: withNote(() => (overlay = "trash")) },
    {
      id: "edit",
      label: "Open in editor",
      keys: "e",
      run: withNote((n) => api.edit(n.id).catch((e) => show(String(e), true))),
    },
    {
      id: "reveal",
      label: "Reveal in Finder",
      keys: "o",
      run: withNote((n) => api.reveal(n.id).catch((e) => show(String(e), true))),
    },
    { id: "reload", label: "Reload", keys: "r", run: reload },
    { id: "help", label: "Keyboard shortcuts", keys: "?", run: () => (overlay = "help") },
  ];

  function onKeydown(event: KeyboardEvent) {
    if (event.metaKey && (event.key === "k" || event.key === "p")) {
      event.preventDefault();
      overlay = overlay === "palette" ? null : "palette";
      return;
    }
    if (event.metaKey && event.key === "r") {
      event.preventDefault();
      reload();
      return;
    }
    const target = event.target as HTMLElement;
    if (overlay || event.metaKey || event.altKey || target.closest("input, textarea")) return;

    const key = event.ctrlKey ? `C-${event.key}` : event.key;
    const actions: Record<string, () => void> = {
      j: () => move(1),
      ArrowDown: () => move(1),
      k: () => move(-1),
      ArrowUp: () => move(-1),
      g: () => select(ordered[0]?.id ?? null),
      Home: () => select(ordered[0]?.id ?? null),
      G: () => select(ordered.at(-1)?.id ?? null),
      End: () => select(ordered.at(-1)?.id ?? null),
      Tab: () => moveFolder(event.shiftKey ? -1 : 1),
      " ": () => view?.scroll(event.shiftKey ? -1 : 1, "page"),
      PageDown: () => view?.scroll(1, "page"),
      PageUp: () => view?.scroll(-1, "page"),
      "C-d": () => view?.scroll(0.5, "page"),
      "C-u": () => view?.scroll(-0.5, "page"),
      J: () => view?.scroll(120),
      K: () => view?.scroll(-120),
      "/": () => (overlay = "palette"),
      ":": () => (overlay = "palette"),
      "?": () => (overlay = "help"),
      r: reload,
      s: withNote(() => (overlay = "session")),
      a: withNote(archive),
      d: withNote(() => (overlay = "trash")),
      Backspace: withNote(() => (overlay = "trash")),
      e: commands.find((c) => c.id === "edit")!.run,
      o: commands.find((c) => c.id === "reveal")!.run,
    };
    const action = actions[key];
    if (action) {
      event.preventDefault();
      action();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="layout">
  <Sidebar
    folders={library.folders}
    notes={ordered}
    {selectedId}
    onSelect={(id) => select(id)}
    onPalette={() => (overlay = "palette")}
  />

  {#if selected}
    <NoteView
      bind:this={view}
      note={selected}
      {focusLine}
      onOpenNote={openNote}
      onError={(m) => show(m, true)}
    />
  {:else if loaded}
    <div class="blank" data-tauri-drag-region>
      {#if !library.folders.length}
        <h1>No folders found</h1>
        <p>
          memo shows the folders named by <code>plansDirectory</code> in
          <code>{library.claude_dir}/settings.json</code> and the directories listed in the code blocks of
          <code>{library.claude_dir}/CLAUDE.md</code>. None of them exists yet.
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
    notes={library.notes}
    {commands}
    onOpen={(id, line) => {
      overlay = null;
      select(id, line);
    }}
    onClose={() => (overlay = null)}
  />
{:else if overlay === "trash" && selected}
  {@const note = selected}
  <Confirm
    title="Move to the Trash?"
    message={`“${note.title}” goes to the macOS Trash, from where Finder can put it back.`}
    action="Move to Trash"
    onConfirm={() => trash(note)}
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
  <Help claudeDir={library.claude_dir} folders={library.folders} onClose={() => (overlay = null)} />
{/if}

{#if toast}
  <div class="toast" class:error={toast.error} role="status">{toast.text}</div>
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

  .toast.error {
    border-color: var(--red);
    color: var(--red);
  }
</style>
