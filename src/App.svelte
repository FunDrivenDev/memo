<script lang="ts">
  import { tick } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { Actions, UNDO_MS } from "./lib/actions";
  import * as api from "./lib/api";
  import type { Anchor, Comment, Library, Note } from "./lib/api";
  import CommentList, { type CommentItem } from "./lib/CommentList.svelte";
  import Help from "./lib/Help.svelte";
  import {
    type Command as KeyCommand,
    commandFor,
    commands,
    type Handler,
    keyName,
    paneKeys,
    type RunId,
  } from "./lib/commands";
  import { foldStep, listStep } from "./lib/keys";
  import NoteView from "./lib/NoteView.svelte";
  import Palette, { type Command } from "./lib/Palette.svelte";
  import SessionDialog from "./lib/SessionDialog.svelte";
  import Settings from "./lib/Settings.svelte";
  import Sidebar, { type Pane } from "./lib/Sidebar.svelte";

  type Overlay = "palette" | "session" | "help" | "settings";
  /** A stop of the list: a note, or a folded folder. */
  type Row = { note: string } | { folder: string };

  /** The shortest spin of the reload icon, and how long its check shows after. */
  const SPIN_MS = 400;
  const DONE_MS = 1500;
  /** Where the folded folders are kept. */
  const FOLDED_KEY = "memo.folded";
  /** How often memo looks for a newer version, besides at launch. */
  const UPDATE_CHECK_MS = 6 * 60 * 60 * 1000;

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
  /** The path of the folded folder the list stops on instead of a note, null on a note. */
  let selectedFolder = $state<string | null>(null);
  /** The selection of the other pane of notes, restored when switching back. */
  let otherId: string | null = null;
  let otherFolder: string | null = null;
  /** The paths of the folders folded in the sidebar, kept across launches. */
  const folded = new SvelteSet<string>(JSON.parse(localStorage.getItem(FOLDED_KEY) ?? "[]"));
  /** The pane of notes `selectedId` belongs to, the last one shown: the list's, under the Comments pane too. */
  let notesPane = $state<Exclude<Pane, "comments">>("notes");
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
  let view: NoteView | undefined = $state();
  /** The message at the bottom, and what `u` undoes while it shows. */
  const actions = new Actions(api, {
    library: apply,
    comments: applyComments,
    reopen: (target) => {
      if ("note" in target) openNote(target.note);
      else if (pane === "comments") selectedCommentId = target.comment;
    },
  }, window);
  const show = (text: string, error = false) => actions.show(text, error);

  const shown = (notes: Note[]) => notes.filter((n) => !actions.hidden(n.id));
  const notes = $derived(shown(library.notes));
  const archived = $derived(shown(library.archived));
  const folders = $derived(notesPane === "archive" ? library.archive_folders : library.folders);
  /** The notes of the pane of notes in sidebar order: by folder, then most recent first. */
  const ordered = $derived.by(() => {
    const paneNotes = notesPane === "archive" ? archived : notes;
    return folders.flatMap((f) => paneNotes.filter((n) => n.folder === f.name));
  });
  /** What the arrows stop on, top to bottom: the notes of the unfolded folders, and the folded folders. */
  const rows = $derived(
    folders.flatMap((f): Row[] =>
      folded.has(f.path)
        ? [{ folder: f.path }]
        : ordered.filter((n) => n.folder === f.name).map((n) => ({ note: n.id }))
    ),
  );
  const rowIndex = (): number =>
    rows.findIndex((r) => ("note" in r ? r.note === selectedId : r.folder === selectedFolder));
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
      .filter((comment) => !actions.hidden(comment.note))
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

  /** Takes a new library, keeping the selection or, when its note left, the note now in its place. */
  function apply(next: Library) {
    const before = rowIndex();
    library = next;
    comments = next.comments;
    loaded = true;
    if (rowIndex() >= 0) return;
    selectRow(rows[Math.max(0, Math.min(before, rows.length - 1))] ?? null);
  }

  /** Takes new comments, keeping the selected one or, when it left, the comment now in its place. */
  function applyComments(next: Comment[]) {
    const before = commentItems.findIndex((i) => i.comment.id === selectedCommentId);
    comments = next;
    if (before < 0 || commentItems.some((i) => i.comment.id === selectedCommentId)) return;
    selectedCommentId = commentItems[Math.min(before, commentItems.length - 1)]?.comment.id ?? null;
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

  /** A newer memo, offered at the top of the sidebar. */
  let update = $state<api.Update | null>(null);

  async function checkUpdate() {
    try {
      update = await api.checkUpdate();
    } catch {
      // Offline, or GitHub refused: the next check tries again.
    }
  }

  $effect(() => {
    checkUpdate();
    const timer = setInterval(checkUpdate, UPDATE_CHECK_MS);
    return () => clearInterval(timer);
  });

  /** Has Homebrew upgrade memo, which quits for it; another install opens the release page. */
  function installUpdate() {
    if (!update) return;
    (update.homebrew ? api.installUpdate() : api.openUrl(update.url)).catch((e) => show(String(e), true));
  }

  // Reading ends when another note shows, whatever changed the selection.
  $effect(() => {
    void selectedId;
    void selectedFolder;
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

  /** Selects a note, unfolding its folder. */
  function select(id: string | null, line: number | null = null) {
    stopReading();
    focusLine = line;
    selectedId = id;
    selectedFolder = null;
    const folder = folders.find((f) => f.name === ordered.find((n) => n.id === id)?.folder);
    if (folder) folded.delete(folder.path);
  }

  /** Stops the list on a folder, which shows no note. */
  function selectFolder(path: string) {
    stopReading();
    selectedId = null;
    selectedFolder = path;
  }

  function selectRow(row: Row | null) {
    if (row && "folder" in row) selectFolder(row.folder);
    else select(row?.note ?? null);
  }

  /** Folds a folder (`unfold` false) or unfolds it, moving the selection onto it or into it. */
  function fold(path: string, unfold: boolean) {
    const name = folders.find((f) => f.path === path)?.name;
    const first = ordered.find((n) => n.folder === name);
    if (unfold) {
      folded.delete(path);
      if (selectedFolder === path && first) select(first.id);
    } else {
      folded.add(path);
      if (selected?.folder === name) selectFolder(path);
    }
  }

  // Kept on every change, for the next launch.
  $effect(() => localStorage.setItem(FOLDED_KEY, JSON.stringify([...folded])));

  function showPane(next: Pane) {
    if (next === pane) return;
    stopReading();
    if (next !== "comments") {
      if (next !== notesPane) {
        [selectedId, otherId] = [otherId, selectedId];
        [selectedFolder, otherFolder] = [otherFolder, selectedFolder];
      }
      notesPane = next;
    }
    pane = next;
    if (next === "comments") {
      if (!commentItems.some((i) => i.comment.id === selectedCommentId)) {
        selectedCommentId = commentItems[0]?.comment.id ?? null;
      }
    } else if (rowIndex() < 0) selectRow(rows[0] ?? null);
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
  const resolveComment = (comment: Comment) => void actions.resolve(comment);

  function resolveFocused() {
    const comment = focusedComment ?? selectedComment?.comment;
    if (comment) resolveComment(comment);
  }

  /** Jumps to the first note of the next (or previous) folder holding any, or onto it when folded. */
  function moveFolder(delta: 1 | -1) {
    const withNotes = folders.filter((f) => ordered.some((n) => n.folder === f.name));
    if (!withNotes.length) return;
    const current = withNotes.findIndex((f) => f.name === selected?.folder || f.path === selectedFolder);
    const target = withNotes[(current + delta + withNotes.length) % withNotes.length]!;
    if (folded.has(target.path)) selectFolder(target.path);
    else select(ordered.find((n) => n.folder === target.name)?.id ?? null);
  }

  function openNote(id: string): boolean {
    if (library.notes.some((n) => n.id === id)) showPane("notes");
    else if (library.archived.some((n) => n.id === id)) showPane("archive");
    else return false;
    select(id);
    return true;
  }

  /** Archives a note of the folders, or restores one of the archive; `u` undoes it for a while. */
  const archiveOrRestore = (note: Note) => void actions.archive(note, selectedArchived);

  /** Trashes a note, on screen at once and for real once `u` can no longer cancel it. */
  function trash(note: Note) {
    const before = rows.findIndex((r) => "note" in r && r.note === note.id);
    actions.trash(note);
    if (selectedId === note.id) selectRow(rows[Math.min(before, rows.length - 1)] ?? null);
  }

  const undo = () => void actions.undo();

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

  /** What each command of the table does, given the key that ran it. */
  const handlers: Record<RunId, Handler> = {
    palette: () => (overlay === "palette" ? (overlay = null) : openPalette()),
    read: () => (reading || pane === "comments" ? view?.editFocused() : startReading()),
    folder: (key) => (pane === "comments" ? moveCommentedNote : moveFolder)(key === "⇧⇥" ? -1 : 1),
    pane: (key) => showPane(panes[key]!),
    scroll: (key) => view?.scroll(key === "⇧space" ? -1 : 1),
    comment: () => view?.comment(),
    session: withNote(() => (overlay = "session")),
    resolve: resolveFocused,
    search: searchComments,
    archive: withNote(archiveOrRestore),
    undo,
    trash: withNote(trash),
    edit,
    reveal: revealNote,
    reload: () => reload(true),
    settings: () => (overlay = "settings"),
    help: () => (overlay = "help"),
  };
  const panes = Object.fromEntries(Object.entries(paneKeys).map(([pane, key]) => [key, pane as Pane]));

  /** The palette's commands: those of the table it lists, then the update when there is one. */
  const paletteCommands: Command[] = $derived([
    ...commands.flatMap((command: KeyCommand) =>
      Object.entries(command.palette ?? {}).map(([key, label]) => ({
        id: `${command.id}${command.keys.length > 1 ? `-${key}` : ""}`,
        label: typeof label === "string" ? label : label[selectedArchived ? "archive" : "notes"],
        keys: key,
        run: () => handlers[command.id as RunId](key),
      }))
    ),
    ...(update ? [{ id: "update", label: `Update memo to ${update.version}`, run: installUpdate }] : []),
  ]);

  function onKeydown(event: KeyboardEvent) {
    const key = keyName(event);
    const command = commandFor(key);
    // ⌘ keys act on memo itself: in a field too, and over a dialog for some.
    // The Settings… menu item normally takes ⌘, first; this covers a missing menu.
    if (command && (command.scope === "anywhere" || (command.scope === "window" && !overlay))) {
      event.preventDefault();
      handlers[command.id](key);
      return;
    }
    const target = event.target as HTMLElement;
    // A dialog's Escape has closed it by the time this runs, or will just after.
    if (overlay || event.defaultPrevented || event.ctrlKey || event.altKey || target.closest("input, textarea")) return;

    if (pane === "comments") {
      const ids = commentItems.map((i) => i.comment.id);
      const i = reading ? null : listStep(event, ids.indexOf(selectedCommentId ?? ""), ids.length);
      if (reading ? view?.step(event) : i !== null) {
        event.preventDefault();
        if (i !== null) selectComment(ids[i] ?? null);
        return;
      }
    } else if (reading) {
      if (view?.step(event)) {
        event.preventDefault();
        return;
      }
    } else {
      const i = listStep(event, rowIndex(), rows.length);
      const unfold = foldStep(event);
      const folder = selectedFolder ?? folders.find((f) => f.name === selected?.folder)?.path;
      if (i !== null || (unfold !== null && folder)) {
        event.preventDefault();
        if (i !== null) selectRow(rows[i] ?? null);
        else fold(folder!, unfold!);
        return;
      }
    }

    if (reading && key === "esc") {
      event.preventDefault();
      stopReading();
    } else if (command?.scope === "note") {
      event.preventDefault();
      handlers[command.id](key);
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
    {selectedFolder}
    {folded}
    active={!reading}
    onSelect={(id) => select(id)}
    onFold={(path) => fold(path, folded.has(path))}
    onPane={showPane}
    onPalette={openPalette}
    {update}
    onUpdate={installUpdate}
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
      {#if pane !== "comments" && selectedFolder}
        <h1>{folders.find((f) => f.path === selectedFolder)?.name} is folded</h1>
        <p><kbd>→</kbd> unfolds it, <kbd>←</kbd> on a note folds its folder.</p>
      {:else if pane === "comments"}
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
    commands={paletteCommands}
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

{#if actions.toast}
  {@const toast = actions.toast}
  {#key toast}
  <div class="toast" class:error={toast.error} role="status">
    <span>{toast.text}</span>
    {#if toast.undoable}
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
