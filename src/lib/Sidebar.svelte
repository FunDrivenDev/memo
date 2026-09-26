<script lang="ts">
  import type { Folder, Note } from "./api";
  import { reveal } from "./scroll";
  import { age, clock, longDate } from "./time.svelte";

  let {
    folders,
    notes,
    selectedId,
    onSelect,
    onPalette,
  }: {
    folders: Folder[];
    notes: Note[];
    selectedId: string | null;
    onSelect: (id: string) => void;
    onPalette: () => void;
  } = $props();

  const sections = $derived(
    folders.map((folder) => ({ folder, notes: notes.filter((n) => n.folder === folder.name) })),
  );

  let list: HTMLElement | undefined = $state();

  // Keep the selected card in view as the keyboard moves through the list.
  $effect(() => {
    if (!selectedId || !list) return;
    const card = list.querySelector<HTMLElement>(`[data-id="${CSS.escape(selectedId)}"]`);
    if (card) reveal(list, card);
  });
</script>

<aside>
  <header data-tauri-drag-region>
    <button class="search" onclick={onPalette} title="Command palette">
      <span>Search notes…</span>
      <kbd>⌘K</kbd>
    </button>
  </header>

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
