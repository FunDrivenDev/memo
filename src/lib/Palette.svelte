<script lang="ts" module>
  import type { Note } from "./api";

  export interface Command {
    id: string;
    label: string;
    /** The key that runs it outside the palette, if any. */
    keys?: string;
    run: () => void;
  }
</script>

<script lang="ts">
  import * as api from "./api";
  import type { CommentHit, Hit } from "./api";
  import Highlight from "./Highlight.svelte";
  import { listStep } from "./keys";
  import { reveal } from "./scroll";
  import { age, clock } from "./time.svelte";

  let {
    notes: current,
    archived,
    archive = $bindable(),
    commands,
    onOpen,
    onOpenComment,
    onClose,
  }: {
    /** Every note of the folders, most recent first. */
    notes: Note[];
    /** Every archived note, most recent first. */
    archived: Note[];
    /** Whether to search the archive rather than the folders; Tab switches. */
    archive: boolean;
    commands: Command[];
    onOpen: (id: string, line: number | null) => void;
    onOpenComment: (id: string) => void;
    onClose: () => void;
  } = $props();

  type Item =
    | { kind: "note"; note: Note; hit: Hit | null }
    | { kind: "comment"; note: Note; hit: CommentHit }
    | { kind: "command"; command: Command };

  let query = $state("");
  let hits = $state<Hit[]>([]);
  let commentHits = $state<CommentHit[]>([]);
  let active = $state(0);
  let input: HTMLInputElement | undefined = $state();
  let list: HTMLElement | undefined = $state();
  let sequence = 0;

  const notes = $derived(archive ? archived : current);
  const byId = $derived(new Map(notes.map((n) => [n.id, n])));

  const items: Item[] = $derived.by(() => {
    if (query.startsWith(">")) {
      const words = query.slice(1).trim().toLowerCase().split(/\s+/).filter(Boolean);
      return commands
        .filter((c) => words.every((w) => c.label.toLowerCase().includes(w)))
        .map((command) => ({ kind: "command" as const, command }));
    }
    if (!query.trim()) return notes.map((note) => ({ kind: "note" as const, note, hit: null }));
    return [
      ...hits.flatMap((hit) => {
        const note = byId.get(hit.id);
        return note ? [{ kind: "note" as const, note, hit }] : [];
      }),
      ...commentHits.flatMap((hit) => {
        const note = byId.get(hit.note);
        return note ? [{ kind: "comment" as const, note, hit }] : [];
      }),
    ];
  });

  // Search as the query changes; a slower, older answer never replaces a newer one.
  $effect(() => {
    const q = query;
    const inArchive = archive;
    active = 0;
    if (q.startsWith(">") || !q.trim()) return;
    const mine = ++sequence;
    const timer = setTimeout(() => {
      api.search(q, inArchive).then((result) => {
        if (mine !== sequence) return;
        hits = result.notes;
        commentHits = result.comments;
      });
    }, 60);
    return () => clearTimeout(timer);
  });

  /** Where a divider goes: before the first content-only hit when title hits come before it, before the comments. */
  const dividers = $derived.by(() => {
    const at = new Map<number, string>();
    const content = items.findIndex((item) => item.kind === "note" && item.hit && !item.hit.in_title);
    if (content > 0) at.set(content, "In the content only");
    const comments = items.findIndex((item) => item.kind === "comment");
    if (comments >= 0) at.set(comments, "In the comments");
    return at;
  });

  const key = (item: Item) =>
    item.kind === "command" ? item.command.id : item.kind === "comment" ? `comment:${item.hit.id}` : item.note.id;

  $effect(() => {
    input?.focus();
  });

  $effect(() => {
    const item = list?.querySelector<HTMLElement>(`[data-index="${active}"]`);
    if (list && item) reveal(list, item);
  });

  function choose(item: Item | undefined) {
    if (!item) return;
    if (item.kind === "command") {
      onClose();
      item.command.run();
    } else if (item.kind === "comment") {
      onOpenComment(item.hit.id);
    } else {
      onOpen(item.note.id, item.hit?.snippets[0]?.line ?? null);
    }
  }

  function onKeydown(event: KeyboardEvent) {
    const next = listStep(event, active, items.length);
    if (next !== null) {
      event.preventDefault();
      active = next;
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(items[active]);
    } else if (event.key === "Tab") {
      event.preventDefault();
      archive = !archive;
    } else if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="palette" onclick={(e) => e.stopPropagation()}>
    <div class="field">
      {#if archive}<span class="scope">Archive</span>{/if}
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={onKeydown}
        placeholder={archive ? "Search the archive" : "Search titles and content — type > for commands"}
        spellcheck="false"
        autocomplete="off"
      />
    </div>
    <ul bind:this={list}>
      {#each items as item, i (key(item))}
        {#if dividers.has(i)}<li class="divider">{dividers.get(i)}</li>{/if}
        <li>
          <button
            class:active={i === active}
            data-index={i}
            onmousemove={() => (active = i)}
            onclick={() => choose(item)}
          >
            {#if item.kind === "command"}
              <span class="title">{item.command.label}</span>
              {#if item.command.keys}<kbd>{item.command.keys}</kbd>{/if}
            {:else if item.kind === "comment"}
              <span class="title">{item.note.title}</span>
              <span class="meta">comment · {item.note.folder}</span>
              {#each item.hit.snippets as snippet (snippet.line)}
                <span class="snippet comment"><Highlight text={snippet.text} indices={snippet.indices} /></span>
              {/each}
            {:else}
              <span class="title"><Highlight text={item.note.title} indices={item.hit?.title_indices} /></span>
              <span class="meta">{item.note.folder} · {age(item.note.modified, clock.now)}</span>
              {#each item.hit?.snippets ?? [] as snippet (snippet.line)}
                <span class="snippet"
                ><span class="line">{snippet.line}</span><Highlight
                    text={snippet.text}
                    indices={snippet.indices}
                  /></span>
              {/each}
            {/if}
          </button>
        </li>
      {:else}
        <li class="none">
          {#if query.startsWith(">")}
            No such command.
          {:else if archive}
            No match in the archive. <kbd>⇥</kbd> Search the notes
          {:else}
            No match. Maybe it is archived? <kbd>⇥</kbd> Search the archive
          {/if}
        </li>
      {/each}
    </ul>
    <footer>
      <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
      <span><kbd>⌘↑</kbd><kbd>⌘↓</kbd> first, last</span>
      <span><kbd>↵</kbd> open</span>
      <span><kbd>⇥</kbd> {archive ? "notes" : "archive"}</span>
      <span><kbd>&gt;</kbd> commands</span>
      <span><kbd>esc</kbd> close</span>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 10;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    background: rgb(0 0 0 / 0.18);
  }

  .palette {
    width: min(680px, 90vw);
    max-height: 70vh;
    display: flex;
    flex-direction: column;
    border-radius: 12px;
    border: 1px solid var(--surface0);
    background: var(--base);
    box-shadow: var(--shadow);
    overflow: hidden;
  }

  .field {
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--surface0);
  }

  .scope {
    margin-left: 14px;
    padding: 2px 8px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--yellow) 20%, transparent);
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
  }

  input {
    flex: 1;
    border: 0;
    border-radius: 0;
    padding: 14px 16px;
    font-size: 16px;
    background: transparent;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 6px;
    overflow-y: auto;
  }

  li button {
    width: 100%;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px 12px;
    padding: 8px 10px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
    cursor: pointer;
  }

  li button.active {
    background: var(--mantle);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .title {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 11px;
    color: var(--overlay0);
    white-space: nowrap;
  }

  .snippet {
    grid-column: 1 / -1;
    font-family: var(--mono);
    font-size: 11.5px;
    color: var(--subtext);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .snippet.comment {
    padding-left: 3.2em;
    font-family: inherit;
    font-style: italic;
  }

  .line {
    display: inline-block;
    min-width: 3.2em;
    color: var(--overlay0);
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 10px 4px;
    font-size: 11px;
    color: var(--overlay0);
  }

  .divider::after {
    content: "";
    flex: 1;
    border-top: 1px solid var(--surface0);
  }

  .none {
    padding: 14px;
    color: var(--overlay0);
  }

  footer {
    display: flex;
    gap: 16px;
    padding: 8px 14px;
    border-top: 1px solid var(--surface0);
    font-size: 11px;
    color: var(--overlay0);
  }

  footer kbd {
    margin-right: 3px;
  }
</style>
