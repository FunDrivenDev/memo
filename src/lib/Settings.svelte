<script lang="ts">
  import * as api from "./api";
  import type { Library } from "./api";
  import FolderInput from "./FolderInput.svelte";
  import Modal from "./Modal.svelte";

  let { onSaved, onError, onClose }: {
    onSaved: (library: Library) => void;
    onError: (message: string) => void;
    onClose: () => void;
  } = $props();

  // `exists` is unknown (undefined) for a folder typed since the window opened.
  let folders = $state<{ path: string; exists?: boolean }[]>([]);
  let defaults = $state<string[]>([]);
  let custom = $state(false);
  let archive = $state("");
  let file = $state("");
  let typed = $state("");
  let loading = $state(true);
  let field: HTMLInputElement | undefined = $state();

  $effect(() => {
    api
      .settings()
      .then((settings) => {
        folders = settings.folders;
        defaults = settings.defaults;
        custom = settings.custom;
        archive = settings.archive;
        file = settings.file;
        loading = false;
      })
      .catch((e) => onError(String(e)));
  });

  $effect(() => {
    if (!loading) field?.focus();
  });

  function add(path: string, exists?: boolean) {
    path = path.trim();
    if (!path) return;
    if (!folders.some((f) => f.path === path)) folders.push({ path, exists });
    custom = true;
  }

  function remove(path: string) {
    folders = folders.filter((f) => f.path !== path);
    custom = true;
  }

  function useDefault() {
    folders = defaults.map((path) => ({ path }));
    custom = false;
  }

  async function choose(then: (path: string) => void) {
    try {
      const path = await api.chooseFolder();
      if (path) then(path);
    } catch (e) {
      onError(String(e));
    }
  }

  async function save() {
    if (typed.trim()) {
      add(typed);
      typed = "";
    }
    try {
      onSaved(
        await api.saveSettings(custom ? folders.map((f) => f.path) : null, archive),
      );
    } catch (e) {
      onError(String(e));
    }
  }

  function onKeydown(event: KeyboardEvent) {
    // A focused button keeps its own Enter.
    if (event.key !== "Enter" || !(event.metaKey || (event.target as HTMLElement).tagName === "INPUT")) return;
    event.preventDefault();
    if (event.metaKey || event.target !== field || !typed.trim()) {
      save();
    } else {
      add(typed);
      typed = "";
    }
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
            {#if folder.exists === false}<span class="missing">missing</span>{/if}
            <button class="remove" title="Remove" aria-label={`Remove ${folder.path}`} onclick={() => remove(folder.path)}>
              ×
            </button>
          </li>
        {:else}
          <li class="empty">No folder</li>
        {/each}
      </ul>

      <div class="add">
        <FolderInput bind:input={field} bind:value={typed} placeholder="~/Notes/claude/reports" />
        <button onclick={() => {
          add(typed);
          typed = "";
        }}>Add <kbd>↵</kbd></button>
        <button onclick={() => choose((path) => add(path, true))}>Choose…</button>
      </div>
      <button class="link" disabled={!custom} onclick={useDefault}>Use Claude Code's plans folder</button>
      <p class="hint">Typing a path lists the folders it can complete: <kbd>↑</kbd> <kbd>↓</kbd> pick, <kbd>⇥</kbd> goes in.</p>

      <h4>Archive</h4>
      <p>
        Where <kbd>a</kbd> moves notes, into a subfolder named after their folder. memo set it on first launch beside
        Claude Code's plans folder, and leaves it there if that folder moves.
      </p>
      <div class="add">
        <FolderInput bind:value={archive} />
        <button onclick={() => choose((path) => (archive = path))}>Choose…</button>
      </div>
      <p class="hint">Changing it leaves the notes already archived where they are.</p>

      <div class="actions">
        <span class="spacer"></span>
        <button onclick={onClose}>Cancel</button>
        <button class="primary" onclick={save}>Save <kbd>⌘↵</kbd></button>
      </div>
      <p class="hint">Kept in <code>{file}</code>.</p>
    </div>
  {/if}
</Modal>

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
