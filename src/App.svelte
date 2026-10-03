<script lang="ts">
  import { tick } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import * as api from "./lib/api";
  import type { Anchor, Comment, Library, Moved, Note } from "./lib/api";
  import CommentList, { type CommentItem } from "./lib/CommentList.svelte";
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
  /** The shortest spin of the reload icon, and how long its check shows after. */
  const SPIN_MS = 400;
  const DONE_MS = 1500;

  let library = $state<Library>({
    claude_dir: "",
    custom: false,
    folders: [],
    notes: [],
    archive: "",
    archive_folders: [],
    archived: [],
    comments: [],
  });
  let loaded = $state(false);
  let pane = $state<Pane>("notes");
  let selectedId = $state<string | null>(null);
  /** The selection of the other pane of notes, restored when switching back. */
  let otherId: string | null = null;
  /** The pane of notes `selectedId` belongs to, the last one shown. */
  let notesPane: Exclude<Pane, "comments"> = "notes";
  let comments = $state<Comment[]>([]);
  let commentQuery = $state("");
  let selectedCommentId = $state<string | null>(null);
  /** The comment `r` would resolve. */
  let focusedComment = $state<Comment | null>(null);
  let commentList: CommentList | undefined = $state();
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
  const shownNotes = $derived(
    new Map<string, { note: Note; archived: boolean }>([
      ...notes.map((note) => [note.id, { note, archived: false }] as const),
      ...archived.map((note) => [note.id, { note, archived: true }] as const),
    ]),
  );
  /** The comments matching the search, by note, the note last commented first, then in the note's order. */
  const commentItems: CommentItem[] = $derived.by(() => {
    const words = commentQuery.toLowerCase().split(/\s+/).filter(Boolean);
    const items = comments
      .filter((comment) => !trashing.has(comment.note))
      .map((comment) => ({ comment, ...(shownNotes.get(comment.note) ?? { note: null, archived: false }) }))
      .filter(({ comment, note }) => {
        const text = `${comment.body} ${comment.anchor.quote} ${note?.title ?? comment.note}`.toLowerCase();
        return words.every((w) => text.includes(w));
      });
    const latest = new Map<string, number>();
    for (const { comment } of items) latest.set(comment.note, Math.max(latest.get(comment.note) ?? 0, comment.updated));
    return items.sort(
      ({ comment: a }, { comment: b }) =>
        latest.get(b.note)! - latest.get(a.note)! || a.note.localeCompare(b.note) ||
        a.anchor.start - b.anchor.start || a.created - b.created,
    );
  });
  const selectedComment = $derived(
    pane === "comments" ? (commentItems.find((i) => i.comment.id === selectedCommentId) ?? null) : null,
  );
  const selected = $derived(
    pane === "comments" ? (selectedComment?.note ?? null) : (ordered.find((n) => n.id === selectedId) ?? null),
  );
  /** Whether the note shown is in the archive. */
  const selectedArchived = $derived(pane === "comments" ? !!selectedComment?.archived : pane === "archive");

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
    comments = next.comments;
    loaded = true;
    if (selectedId && ordered.some((n) => n.id === selectedId)) return;
    selectedId = (ordered[Math.max(0, Math.min(before, ordered.length - 1))] ?? null)?.id ?? null;
  }

  /** What the icon in the top right corner shows: the folders being read, just read, or failing. */
  let sync = $state<"idle" | "busy" | "done" | "error">("idle");
  let syncError = $state("");
  let syncTimer: ReturnType<typeof setTimeout> | undefined;
  /** The latest reload, the only one that sets the icon. */
  let reloads = 0;

  /** Rereads the folders; `force` also watches them afresh, for a change the watcher missed. */
  async function reload(force = false) {
    const run = ++reloads;
    clearTimeout(syncTimer);
    sync = "busy";
    // Spins long enough to be seen, the read itself often taking a few milliseconds.
    const spin = new Promise((done) => setTimeout(done, SPIN_MS));
    try {
      const next = await (force ? api.refresh() : api.library());
      if (run !== reloads) return;
      apply(next);
      await spin;
      if (run !== reloads) return;
      sync = "done";
      syncTimer = setTimeout(() => (sync = "idle"), DONE_MS);
    } catch (e) {
      if (run !== reloads) return;
      sync = "error";
      syncError = String(e);
      show(syncError, true);
    }
  }

  $effect(() => {
    reload();
    const unlisten = [api.onLibraryChanged(() => reload()), api.onOpenSettings(() => (overlay = "settings"))];
    return () => unlisten.forEach((u) => void u.then((stop) => stop()));
  });

  // Reading ends when another note shows, whatever changed the selection.
  $effect(() => {
    void selectedId;
    void selectedCommentId;
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
    stopReading();
    if (next !== "comments") {
      if (next !== notesPane) [selectedId, otherId] = [otherId, selectedId];
      notesPane = next;
    }
    pane = next;
    if (next === "comments") {
      if (!commentItems.some((i) => i.comment.id === selectedCommentId)) {
        selectedCommentId = commentItems[0]?.comment.id ?? null;
      }
    } else if (!ordered.some((n) => n.id === selectedId)) selectedId = ordered[0]?.id ?? null;
  }

  function selectComment(id: string | null) {
    stopReading();
    selectedCommentId = id;
  }

  /** Jumps to the first comment on the next (or previous) note. */
  function moveCommentedNote(delta: 1 | -1) {
    const noteIds = [...new Set(commentItems.map((i) => i.comment.note))];
    if (!noteIds.length) return;
    const current = noteIds.indexOf(selectedComment?.comment.note ?? "");
    const target = noteIds[(current + delta + noteIds.length) % noteIds.length];
    selectComment(commentItems.find((i) => i.comment.note === target)?.comment.id ?? null);
  }

  /** Shows a comment in its note, from the Comments tab, whatever its search filtered out. */
  function openComment(id: string) {
    commentQuery = "";
    showPane("comments");
    selectComment(id);
  }

  async function searchComments() {
    showPane("comments");
    await tick();
    commentList?.focusSearch();
  }

  /** Saves a new comment on the note shown, returning its id. */
  async function addComment(anchor: Anchor, body: string): Promise<string | null> {
    if (!selected) return null;
    try {
      const before = new Set(comments.map((c) => c.id));
      comments = await api.addComment(selected.id, body, anchor);
      const added = comments.find((c) => !before.has(c.id))?.id ?? null;
      if (pane === "comments") selectedCommentId = added;
      return added;
    } catch (e) {
      show(String(e), true);
      return null;
    }
  }

  async function editComment(id: string, body: string): Promise<boolean> {
    try {
      comments = await api.editComment(id, body);
      return true;
    } catch (e) {
      show(String(e), true);
      return false;
    }
  }

  /** Resolving deletes the comment; `u` brings it back for a while. */
  async function resolveComment(comment: Comment) {
    try {
      const at = commentItems.findIndex((i) => i.comment.id === comment.id);
      comments = await api.resolveComment(comment.id);
      if (selectedCommentId === comment.id) {
        selectedCommentId = commentItems[Math.min(at, commentItems.length - 1)]?.comment.id ?? null;
      }
      show("Resolved the comment", false, () => restoreComment(comment));
    } catch (e) {
      show(String(e), true);
    }
  }

  async function restoreComment(comment: Comment) {
    try {
      comments = await api.restoreComment(comment);
      if (pane === "comments") selectedCommentId = comment.id;
      show("Undone");
    } catch (e) {
      show(String(e), true);
    }
  }

  function resolveFocused() {
    const comment = focusedComment ?? selectedComment?.comment;
    if (comment) resolveComment(comment);
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
      const restoring = selectedArchived;
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
    { id: "session", label: "Start a Claude Code session", keys: "s", run: withNote(() => (overlay = "session")) },
    {
      id: "archive",
      label: selectedArchived ? "Restore note from the archive" : "Archive note",
      keys: "a",
      run: withNote(archiveOrRestore),
    },
    { id: "notes", label: "Show the notes", keys: "⌘1", run: () => showPane("notes") },
    { id: "archived", label: "Show the archive", keys: "⌘2", run: () => showPane("archive") },
    { id: "comments", label: "Show the comments", keys: "⌘3", run: () => showPane("comments") },
    { id: "search-comments", label: "Search the comments", keys: "/", run: searchComments },
    { id: "comment", label: "Comment on the selection or the item read", keys: "c", run: () => view?.comment() },
    { id: "resolve", label: "Resolve the comment", keys: "r", run: resolveFocused },
    { id: "trash", label: "Move note to the Trash", keys: "t", run: withNote(trash) },
    { id: "edit", label: "Open in editor", keys: "e", run: edit },
    { id: "reveal", label: "Reveal in Finder", keys: "o", run: revealNote },
    { id: "reload", label: "Reload the folders", keys: "⌘R", run: () => reload(true) },
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
    const panes: Record<string, Pane> = { "1": "notes", "2": "archive", "3": "comments" };
    if (event.metaKey && panes[event.key] && !overlay) {
      event.preventDefault();
      showPane(panes[event.key]!);
      return;
    }
    if (event.metaKey && event.key === "r") {
      event.preventDefault();
      reload(true);
      return;
    }
    const target = event.target as HTMLElement;
    // A dialog's Escape has closed it by the time this runs, or will just after.
    if (overlay || event.defaultPrevented || event.ctrlKey || event.altKey || target.closest("input, textarea")) return;

    const ids = pane === "comments" ? commentItems.map((i) => i.comment.id) : ordered.map((n) => n.id);
    const at = ids.indexOf((pane === "comments" ? selectedCommentId : selectedId) ?? "");
    const i = reading ? null : listStep(event, at, ids.length);
    if (reading ? view?.step(event) : i !== null) {
      event.preventDefault();
      if (i !== null) (pane === "comments" ? selectComment : select)(ids[i] ?? null);
      return;
    }

    // One key per action: ⌘ is spelt out, ⇧ only in the character it gives (`?`, `A`), or in Tab and space.
    const key = event.metaKey ? `⌘${event.key}` : event.key;
    const actions: Record<string, () => void> = {
      Tab: () => (pane === "comments" ? moveCommentedNote : moveFolder)(event.shiftKey ? -1 : 1),
      " ": () => view?.scroll(event.shiftKey ? -1 : 1),
      "?": () => (overlay = "help"),
      c: () => view?.comment(),
      s: withNote(() => (overlay = "session")),
      r: resolveFocused,
      "/": searchComments,
      a: withNote(archiveOrRestore),
      u: undo,
      t: withNote(trash),
      e: edit,
      o: revealNote,
    };
    const noteKeys: Record<string, () => void> = reading
      ? {
          Escape: stopReading,
          Enter: () => view?.editFocused(),
        }
      : { Enter: pane === "comments" ? () => view?.editFocused() : startReading };
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
    counts={{ notes: notes.length, archive: archived.length, comments: comments.length }}
    {selectedId}
    active={!reading}
    onSelect={(id) => select(id)}
    onPane={showPane}
    onPalette={openPalette}
  >
    {#snippet commentPane()}
      <CommentList
        bind:this={commentList}
        items={commentItems}
        total={comments.length}
        bind:query={commentQuery}
        selectedId={selectedCommentId}
        active={!reading}
        onSelect={selectComment}
      />
    {/snippet}
  </Sidebar>

  {#if selected}
    <NoteView
      bind:this={view}
      note={selected}
      archived={selectedArchived}
      {focusLine}
      comments={comments.filter((c) => c.note === selected.id)}
      focusComment={pane === "comments" ? selectedCommentId : null}
      bind:focused={focusedComment}
      onOpenNote={openNote}
      onError={(m) => show(m, true)}
      onAddComment={addComment}
      onEditComment={editComment}
      onResolveComment={resolveComment}
    />
  {:else if loaded}
    <div class="blank" data-tauri-drag-region>
      {#if pane === "comments"}
        <h1>{comments.length ? "No comment selected" : "No comment yet"}</h1>
        <p>
          Select text in a note, or read it with <kbd>↵</kbd>, then press <kbd>c</kbd>. Comments stay beside the note,
          which memo never changes.
        </p>
      {:else if pane === "archive"}
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
    onOpenComment={(id) => {
      overlay = null;
      openComment(id);
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

<button
  class="sync {sync}"
  title={{ idle: "Reload the folders (⌘R)", busy: "Reading the folders…", done: "Up to date", error: syncError }[sync]}
  aria-label="Reload the folders"
  onclick={() => reload(true)}
>
  {#if sync === "done"}
    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>
  {:else}
    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.5v2.5h-2.5" /></svg>
  {/if}
</button>

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
    grid-template-columns: minmax(260px, 320px) minmax(0, 1fr);
    /* Holds the row to the window: a sidebar longer than it would otherwise stretch both panes past the bottom. */
    grid-template-rows: minmax(0, 1fr);
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

  /* Sits in the note header's right padding, clear of the traffic lights. */
  .sync {
    position: fixed;
    top: 11px;
    right: 9px;
    z-index: 5;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: none;
    color: var(--overlay0);
    opacity: 0.55;
    cursor: pointer;
    transition: opacity 200ms, color 200ms;
  }

  .sync:hover,
  .sync.busy,
  .sync.done,
  .sync.error {
    opacity: 1;
  }

  .sync svg {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .sync.busy svg {
    color: var(--accent);
    animation: spin 700ms linear infinite;
  }

  .sync.done {
    color: var(--green);
    animation: pop 200ms ease-out;
  }

  .sync.error {
    color: var(--red);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes pop {
    from {
      transform: scale(0.6);
    }
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
