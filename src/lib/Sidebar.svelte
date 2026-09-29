<script lang="ts" module>
  export type Pane = "notes" | "archive" | "comments";
</script>

<script lang="ts">
  import type { Snippet } from "svelte";
  import type { Folder, Note } from "./api";
  import { reveal } from "./scroll";
  import { age, clock, longDate } from "./time.svelte";

  let {
    pane,
    folders,
    notes,
    counts,
    selectedId,
    active,
    onSelect,
    onPane,
    onPalette,
    commentPane,
  }: {
    pane: Pane;
    /** The folders and notes of the pane shown. */
    folders: Folder[];
    notes: Note[];
    counts: Record<Pane, number>;
    selectedId: string | null;
    /** Whether the keys move through the list, rather than through the note. */
    active: boolean;
    onSelect: (id: string) => void;
    onPane: (pane: Pane) => void;
    onPalette: () => void;
    /** The list of comments, shown in their pane instead of the notes. */
    commentPane: Snippet;
  } = $props();

  const sections = $derived(
    folders.map((folder) => ({ folder, notes: notes.filter((n) => n.folder === folder.name) })),
  );

  /** The tabs, with the key that shows each. */
  const tabs = [["notes", "Notes", "⌘1"], ["archive", "Archive", "⌘2"], ["comments", "Comments", "⌘3"]] as const;

  let list: HTMLElement | undefined = $state();

  // Keep the selected card in view as the keyboard moves through the list.
  $effect(() => {
    if (!selectedId || !list) return;
    const card = list.querySelector<HTMLElement>(`[data-id="${CSS.escape(selectedId)}"]`);
    if (card) reveal(list, card);
  });
</script>

<aside class:archive={pane === "archive"} class:idle={!active}>
  <header data-tauri-drag-region>
    <button class="search" onclick={onPalette} title="Command palette">
      <span>{pane === "archive" ? "Search the archive…" : "Search notes…"}</span>
      <kbd>⌘K</kbd>
    </button>
    <div class="panes" role="tablist">
      {#each tabs as [id, label, key] (id)}
        <button role="tab" aria-selected={pane === id} class:active={pane === id} title={key} onclick={() => onPane(id)}>
          {label} <span class="count">{counts[id]}</span>
        </button>
      {/each}
    </div>
  </header>

  {#if pane === "comments"}
    {@render commentPane()}
  {:else}
  <nav bind:this={list}>
    {#each sections as { folder, notes: items } (folder.path)}
      <section>
        <h2 title={folder.path}>
          <span>{folder.name}</span>
          <span class="count">{items.length}</span>
        </h2>
        {#each items as note (note.id)}
          <button
            class="card"
            class:selected={note.id === selectedId}
            data-id={note.id}
            onclick={() => onSelect(note.id)}
          >
            <span class="title">{note.title}</span>
            {#if note.excerpt}<span class="excerpt">{note.excerpt}</span>{/if}
            <time title={longDate(note.modified)}>{age(note.modified, clock.now)}</time>
          </button>
        {:else}
          <p class="empty">Nothing here.</p>
        {/each}
      </section>
    {/each}
  </nav>
  {/if}
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--mantle);
    border-right: 1px solid var(--crust);
  }

  header {
    /* Room for the window's traffic lights. */
    padding: 38px 12px 10px;
  }

  .panes {
    display: flex;
    gap: 2px;
    margin-top: 8px;
    padding: 2px;
    border-radius: 7px;
    background: var(--crust);
  }

  .panes button {
    flex: 1;
    padding: 3px 8px;
    border: 0;
    border-radius: 5px;
    background: none;
    color: var(--subtext);
    font-size: 12px;
    cursor: pointer;
  }

  .panes button.active {
    background: var(--base);
    color: var(--text);
    font-weight: 600;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.12);
  }

  /* A tint tells the archive from the notes at a glance. */
  aside.archive {
    background: color-mix(in srgb, var(--yellow) 7%, var(--mantle));
  }

  aside.archive h2 {
    background: color-mix(in srgb, var(--yellow) 7%, var(--mantle));
  }

  .search {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 8px 6px 10px;
    border: 1px solid var(--surface0);
    border-radius: 6px;
    background: var(--base);
    color: var(--overlay0);
    cursor: pointer;
  }

  nav {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 16px;
  }

  h2 {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    justify-content: space-between;
    margin: 0;
    padding: 12px 6px 6px;
    background: var(--mantle);
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--subtext);
  }

  .count {
    color: var(--overlay0);
    font-weight: 500;
  }

  .card {
    position: relative;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 3px 8px;
    width: 100%;
    margin-bottom: 2px;
    padding: 8px 10px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: none;
    text-align: left;
    cursor: pointer;
  }

  .card:hover {
    background: color-mix(in srgb, var(--surface0) 45%, transparent);
  }

  .card.selected {
    background: var(--base);
    border-color: var(--surface0);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .idle .card.selected {
    box-shadow: inset 3px 0 0 var(--surface1);
  }

  .title {
    font-weight: 600;
    line-height: 1.3;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  time {
    font-size: 11px;
    color: var(--overlay0);
    font-variant-numeric: tabular-nums;
    padding-top: 2px;
  }

  .excerpt {
    grid-column: 1 / -1;
    font-size: 12px;
    line-height: 1.4;
    color: var(--subtext);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .empty {
    margin: 4px 10px 8px;
    color: var(--overlay0);
    font-size: 12px;
  }
</style>
