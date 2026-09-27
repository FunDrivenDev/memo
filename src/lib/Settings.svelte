<script lang="ts">
  import * as api from "./api";
  import type { FolderSetting, Library } from "./api";
  import FolderInput from "./FolderInput.svelte";
  import Modal from "./Modal.svelte";

  // Every change is saved as it is made: there is no Save button. A typed folder must
  // exist; one that does not is created only once confirmed.
  let { onSaved, onError, onClose }: {
    onSaved: (library: Library) => void;
    onError: (message: string) => void;
    onClose: () => void;
  } = $props();

  let folders = $state<FolderSetting[]>([]);
  let defaults = $state<string[]>([]);
  let custom = $state(false);
  let archive = $state("");
  let file = $state("");
  let typed = $state("");
  let typedArchive = $state("");
  let loading = $state(true);
  let field: HTMLInputElement | undefined = $state();
  let archiveField: HTMLInputElement | undefined = $state();
  /** A typed folder that does not exist, waiting for the confirmation to create it. */
  let missing = $state<{ path: string; then: (path: string) => Promise<void>; from: HTMLElement | null } | null>(
    null,
  );
  let createButton: HTMLButtonElement | undefined = $state();

  async function load() {
    const settings = await api.settings();
    folders = settings.folders;
    defaults = settings.defaults;
    custom = settings.custom;
    archive = settings.archive;
    typedArchive = settings.archive;
    file = settings.file;
    loading = false;
  }

  $effect(() => {
    load().catch((e) => onError(String(e)));
  });

  $effect(() => {
    if (!loading) field?.focus();
  });

  $effect(() => {
    createButton?.focus();
  });

  /** Saves the folders (`null` for Claude Code's plans folder) and the archive; true once saved. */
  async function save(next: string[] | null, nextArchive = archive): Promise<boolean> {
    try {
      onSaved(await api.saveSettings(next, nextArchive));
      await load();
      return true;
    } catch (e) {
      onError(String(e));
      return false;
    }
  }

  const paths = () => folders.map((f) => f.path);

  /** Runs `then` on the typed folder once it exists, asking first to create a missing one. */
  async function existing(folder: string, then: (path: string) => Promise<void>) {
    try {
      const { path, exists } = await api.inspectFolder(folder);
      if (exists) await then(path);
      else missing = { path, then, from: document.activeElement as HTMLElement | null };
    } catch (e) {
      onError(String(e));
    }
  }

  async function create() {
    if (!missing) return;
    const { path, then } = missing;
    closeMissing();
    try {
      await api.createFolder(path);
    } catch (e) {
      onError(String(e));
      return;
    }
    await then(path);
  }

  function closeMissing() {
    missing?.from?.focus();
    missing = null;
  }

  async function add(path: string) {
    if (!folders.some((f) => f.path === path)) await save([...paths(), path]);
  }

  function addTyped() {
    if (!typed.trim()) return;
    existing(typed, async (path) => {
      await add(path);
      typed = "";
    });
  }

  function setArchive() {
    if (!typedArchive.trim() || typedArchive === archive) return;
    existing(typedArchive, async (path) => {
      await save(custom ? paths() : null, path);
    });
  }

  async function choose(then: (path: string) => Promise<unknown>) {
    try {
      const path = await api.chooseFolder();
      if (path) await then(path);
    } catch (e) {
      onError(String(e));
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== "Enter") return;
    if (event.target === field) addTyped();
    else if (event.target === archiveField) setArchive();
    else return;
    event.preventDefault();
  }
</script>

<Modal title="Settings" {onClose}>
  {#if !loading}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="form" onkeydown={onKeydown}>
      <p class="intro">
        <strong>Folders</strong>: the ones memo lists and watches.
        {#if custom}
          Chosen for this installation.
        {:else}
          By default, Claude Code's plans folder.
        {/if}
      </p>

      <ul>
        {#each folders as folder (folder.path)}
          <li>
            <code>{folder.path}</code>
            {#if !folder.exists}<span class="missing">missing</span>{/if}
            <button class="remove" title="Remove" aria-label={`Remove ${folder.path}`}
              onclick={() => save(paths().filter((p) => p !== folder.path))}>
              ×
            </button>
          </li>
        {:else}
          <li class="empty">No folder</li>
        {/each}
      </ul>

      <div class="add">
        <FolderInput bind:input={field} bind:value={typed} placeholder="~/Notes/claude/reports" />
        <button onclick={addTyped}>Add <kbd>↵</kbd></button>
        <button onclick={() => choose(add)}>Choose…</button>
      </div>
      <button class="link" disabled={!custom} onclick={() => save(null)}>Use Claude Code's plans folder</button>
      <p class="hint">
        Typing a path lists the existing folders it can complete: <kbd>↑</kbd> <kbd>↓</kbd> pick, <kbd>⌘↑</kbd>
        <kbd>⌘↓</kbd> the first or last, <kbd>⇥</kbd> goes in.
      </p>

      <h4>Archive</h4>
      <p>
        Where <kbd>a</kbd> moves notes, into a subfolder named after their folder. memo set it on first launch beside
        Claude Code's plans folder, and leaves it there if that folder moves.
      </p>
      <div class="add">
        <FolderInput bind:input={archiveField} bind:value={typedArchive} />
        <button onclick={() => choose((path) => save(custom ? paths() : null, path))}>Choose…</button>
      </div>
      <p class="hint">
        {#if typedArchive !== archive}
          <kbd>↵</kbd> makes it the archive; closing keeps <code>{archive}</code>.
        {:else}
          Changing it leaves the notes already archived where they are.
        {/if}
      </p>

      <p class="hint">Every change is saved at once, in <code>{file}</code>.</p>
    </div>
  {/if}
</Modal>

{#if missing}
  <Modal title="Create the folder?" small onClose={closeMissing}>
    <div class="form">
      <p><code>{missing.path}</code> does not exist. memo can create it, with any missing parent.</p>
      <div class="actions">
        <span class="spacer"></span>
        <button onclick={closeMissing}>Cancel <kbd>esc</kbd></button>
        <button class="primary" bind:this={createButton} onclick={create}>Create <kbd>↵</kbd></button>
      </div>
    </div>
  </Modal>
{/if}

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--subtext);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--surface0);
    border-radius: 6px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 6px 6px 10px;
  }

  li + li {
    border-top: 1px solid var(--surface0);
  }

  li code {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    color: var(--overlay0);
    font-size: 12px;
  }

  .missing {
    font-size: 11px;
    color: var(--yellow);
  }

  code {
    font-family: var(--mono);
    font-size: 12.5px;
  }

  .add,
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  h4 {
    margin: 8px 0 -4px;
    font-size: 13px;
  }

  .link {
    align-self: flex-start;
  }

  .spacer {
    flex: 1;
  }

  button {
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--surface0);
    background: var(--mantle);
    cursor: pointer;
    white-space: nowrap;
  }

  button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .remove {
    padding: 0 8px;
    border: none;
    background: none;
    color: var(--overlay0);
    font-size: 16px;
  }

  .remove:hover {
    color: var(--red);
  }

  .link {
    padding: 6px 0;
    border: none;
    background: none;
    color: var(--accent);
    font-size: 12px;
  }

  .primary {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }

  .hint {
    color: var(--overlay0);
  }
</style>
