<script lang="ts" module>
  import type { Note } from "./api";

  export interface Command {
    id: string;
    label: string;
    keys: string;
    run: () => void;
  }
</script>

<script lang="ts">
  import * as api from "./api";
  import type { Hit } from "./api";
  import Highlight from "./Highlight.svelte";
  import { reveal } from "./scroll";
  import { age, clock } from "./time.svelte";

  let {
    notes: current,
    archived,
    archive = $bindable(),
    commands,
    onOpen,
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
    onClose: () => void;
  } = $props();

  type Item =
    | { kind: "note"; note: Note; hit: Hit | null }
    | { kind: "command"; command: Command };

  let query = $state("");
  let hits = $state<Hit[]>([]);
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
    return hits.flatMap((hit) => {
      const note = byId.get(hit.id);
      return note ? [{ kind: "note" as const, note, hit }] : [];
    });
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
        if (mine === sequence) hits = result;
      });
    }, 60);
    return () => clearTimeout(timer);
  });

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
    } else {
      onOpen(item.note.id, item.hit?.snippets[0]?.line ?? null);
    }
  }

  function onKeydown(event: KeyboardEvent) {
    const next = event.key === "ArrowDown" || (event.ctrlKey && (event.key === "n" || event.key === "j"));
    const prev = event.key === "ArrowUp" || (event.ctrlKey && (event.key === "p" || event.key === "k"));
    if (next || prev) {
      event.preventDefault();
      const n = items.length;
      if (n) active = (active + (next ? 1 : n - 1)) % n;
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
      {#each items as item, i (item.kind === "note" ? item.note.id : item.command.id)}
        <li>
          <button
            class:active={i === active}
            data-index={i}
            onmousemove={() => (active = i)}
            onclick={() => choose(item)}
          >
            {#if item.kind === "command"}
              <span class="title">{item.command.label}</span>
              <kbd>{item.command.keys}</kbd>
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

  .line {
    display: inline-block;
    min-width: 3.2em;
    color: var(--overlay0);
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
