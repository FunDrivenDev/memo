<script lang="ts" module>
  import type { Comment, Note } from "./api";

  /** A comment, with the note it is on when memo shows that note. */
  export interface CommentItem {
    comment: Comment;
    note: Note | null;
    archived: boolean;
  }
</script>

<script lang="ts">
  import { reveal } from "./scroll";
  import { age, clock, longDate } from "./time.svelte";

  // Every comment, one section per note, filtered by a search field.
  let {
    items,
    total,
    query = $bindable(),
    selectedId,
    active,
    onSelect,
  }: {
    /** The comments matching the query, in the order the keys move through them. */
    items: CommentItem[];
    /** How many comments there are, matching or not. */
    total: number;
    query: string;
    selectedId: string | null;
    /** Whether the keys move through the list, rather than through the note. */
    active: boolean;
    onSelect: (id: string) => void;
  } = $props();

  let list: HTMLElement | undefined = $state();
  let input: HTMLInputElement | undefined = $state();

  /** Consecutive comments on the same note, which the order keeps together. */
  const sections = $derived(
    items.reduce<{ key: string; items: CommentItem[] }[]>((all, item) => {
      const last = all.at(-1);
      if (last?.key === item.comment.note) last.items.push(item);
      else all.push({ key: item.comment.note, items: [item] });
      return all;
    }, []),
  );

  const fileName = (path: string) => path.slice(path.lastIndexOf("/") + 1);

  export function focusSearch() {
    input?.focus();
    input?.select();
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" || event.key === "Enter" || event.key === "ArrowDown") {
      event.preventDefault();
      input?.blur();
      const shown = items.some((i) => i.comment.id === selectedId);
      if (event.key !== "Escape" && !shown && items[0]) onSelect(items[0].comment.id);
    }
  }

  $effect(() => {
    if (!selectedId || !list) return;
    const card = list.querySelector<HTMLElement>(`[data-id="${CSS.escape(selectedId)}"]`);
    if (card) reveal(list, card);
  });
</script>

<div class="search">
  <input
    bind:this={input}
    bind:value={query}
    onkeydown={onKeydown}
    placeholder="Search comments…"
    spellcheck="false"
    autocomplete="off"
  />
  <kbd>/</kbd>
</div>

<nav bind:this={list} class:idle={!active}>
  {#each sections as section (section.key)}
    {@const first = section.items[0]!}
    <section>
      <h2 title={section.key}>
        <span class="note">{first.note?.title ?? fileName(section.key)}</span>
        {#if first.archived}<span class="tag">Archived</span>{/if}
        {#if !first.note}<span class="tag">Missing</span>{/if}
      </h2>
      {#each section.items as { comment } (comment.id)}
        <button
          class="card"
          class:selected={comment.id === selectedId}
          data-id={comment.id}
          onclick={() => onSelect(comment.id)}
        >
          <span class="body">{comment.body}</span>
          <time title={longDate(comment.updated)}>{age(comment.updated, clock.now)}</time>
          <span class="quote">{comment.anchor.quote}</span>
        </button>
      {/each}
    </section>
  {:else}
    <p class="empty">
      {#if total}
        No comment matches.
      {:else}
        No comment yet. Select text in a note, or read it with <kbd>↵</kbd>, then press <kbd>c</kbd>.
      {/if}
    </p>
  {/each}
</nav>

<style>
  .search {
    position: relative;
    padding: 0 12px 10px;
  }

  .search input {
    width: 100%;
    padding: 6px 30px 6px 10px;
  }

  .search kbd {
    position: absolute;
    right: 20px;
    top: 6px;
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
    gap: 6px;
    align-items: baseline;
    margin: 0;
    padding: 12px 6px 6px;
    background: var(--mantle);
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.04em;
    color: var(--subtext);
  }

  .note {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    flex: none;
    padding: 0 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--yellow) 20%, transparent);
    font-weight: 600;
    letter-spacing: 0;
  }

  .card {
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
    box-shadow: inset 3px 0 0 var(--yellow);
  }

  .idle .card.selected {
    box-shadow: inset 3px 0 0 var(--surface1);
  }

  .body {
    line-height: 1.35;
    white-space: pre-wrap;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  time {
    font-size: 11px;
    color: var(--overlay0);
    font-variant-numeric: tabular-nums;
    padding-top: 2px;
  }

  .quote {
    grid-column: 1 / -1;
    padding-left: 7px;
    border-left: 2px solid color-mix(in srgb, var(--yellow) 60%, transparent);
    font-size: 12px;
    line-height: 1.4;
    color: var(--subtext);
    display: -webkit-box;
    -webkit-line-clamp: 1;
    line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .empty {
    margin: 12px 10px;
    color: var(--overlay0);
    font-size: 12px;
    line-height: 1.6;
  }
</style>
