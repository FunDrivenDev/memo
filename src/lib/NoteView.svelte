<script lang="ts">
  import { tick } from "svelte";
  import * as api from "./api";
  import type { Note } from "./api";
  import { listStep } from "./keys";
  import { reveal } from "./scroll";
  import { age, clock, longDate } from "./time.svelte";

  let {
    note,
    archived = false,
    focusLine,
    onOpenNote,
    onError,
  }: {
    note: Note;
    /** Whether the note is in the archive. */
    archived?: boolean;
    /** A source line to scroll to once rendered, from a search hit. */
    focusLine: number | null;
    onOpenNote: (id: string) => boolean;
    onError: (message: string) => void;
  } = $props();

  let html = $state("");
  let words = $state(0);
  let scroller: HTMLElement | undefined = $state();
  let article: HTMLElement | undefined = $state();
  let shownId = "";
  /** The item under the reading cursor, if reading. */
  let current: HTMLElement | null = null;

  // Rerender when the note or its file changes; keep the scroll position on a mere edit.
  $effect(() => {
    const { id, modified } = note;
    const line = focusLine;
    let stale = false;
    api
      .render(id)
      .then(async (rendered) => {
        if (stale) return;
        html = rendered.html;
        words = rendered.words;
        await tick();
        // A mere edit keeps the cursor on the same item, when it is still there.
        const pos = shownId === id ? current?.dataset.sourcepos : undefined;
        current = null;
        if (pos) mark(article?.querySelector<HTMLElement>(`[data-sourcepos="${pos}"]`) ?? null, false);
        if (line !== null) scrollToLine(line);
        else if (shownId !== id) scroller?.scrollTo({ top: 0 });
        shownId = id;
      })
      .catch((e) => onError(String(e)));
    void modified;
    return () => (stale = true);
  });

  /** Scrolls to the innermost block whose `data-sourcepos` covers `line`, and flashes it. */
  function scrollToLine(line: number) {
    let best: HTMLElement | null = null;
    let bestSpan = Infinity;
    for (const el of article?.querySelectorAll<HTMLElement>("[data-sourcepos]") ?? []) {
      const m = /^(\d+):\d+-(\d+):\d+$/.exec(el.dataset.sourcepos ?? "");
      if (!m) continue;
      const start = Number(m[1]);
      const end = Number(m[2]);
      if (start <= line && line <= end && end - start <= bestSpan) {
        best = el;
        bestSpan = end - start;
      }
    }
    if (!best || !scroller) return;
    reveal(scroller, best, "center");
    best.classList.remove("flash");
    void best.offsetWidth;
    best.classList.add("flash");
  }

  /** Scrolls the note by `pages` pages. */
  export function scroll(pages: number) {
    scroller?.scrollBy({ top: pages * scroller.clientHeight * 0.85 });
  }

  /** The blocks the reading cursor steps through: paragraphs, list items, code and tables, not headings. */
  function items(): HTMLElement[] {
    const all = article?.querySelectorAll<HTMLElement>(":is(p, li, pre, table, dt, dd)[data-sourcepos]") ?? [];
    // A loose list wraps its items' text in paragraphs: the item itself is the stop.
    return [...all].filter((el) => !(el.tagName === "P" && el.parentElement?.matches("li, dd")));
  }

  function mark(el: HTMLElement | null, show = true) {
    current?.classList.remove("current");
    current = el;
    if (!el || !scroller) return;
    el.classList.add("current");
    // The item sits mid-height, bar the ends of the note; one taller than the view shows from its start.
    if (show) reveal(scroller, el, el.offsetHeight > scroller.clientHeight * 0.8 ? "start" : "center");
  }

  /** Puts the reading cursor on the first item in view. */
  export function startReading() {
    const top = (scroller?.getBoundingClientRect().top ?? 0) + 40;
    const all = items();
    mark(all.find((el) => el.getBoundingClientRect().bottom > top) ?? all[0] ?? null);
  }

  export function stopReading() {
    mark(null);
  }

  /** Moves the reading cursor for an arrow key, as in any list; false for any other key. */
  export function step(event: KeyboardEvent): boolean {
    const all = items();
    const next = listStep(event, current ? all.indexOf(current) : -1, all.length);
    if (next === null) return false;
    mark(all[next] ?? null);
    return true;
  }

  function onClick(event: MouseEvent) {
    const link = (event.target as HTMLElement).closest("a");
    const href = link?.getAttribute("href");
    if (!link || !href) return;
    event.preventDefault();
    if (href.startsWith("#")) {
      const target = article?.querySelector<HTMLElement>(`[id="${CSS.escape(href.slice(1))}"]`);
      if (scroller && target) {
        scroller.scrollTo({ top: target.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop - 56 });
      }
    } else if (/^(https?:|mailto:)/i.test(href)) {
      api.openUrl(href).catch((e) => onError(String(e)));
    } else {
      const dir = note.id.slice(0, note.id.lastIndexOf("/") + 1);
      const target = decodeURIComponent(new URL(href, `file://${dir}`).pathname);
      if (!onOpenNote(target)) onError(`${href} is not a note memo shows`);
    }
  }

  const ago = $derived.by(() => {
    const a = age(note.modified, clock.now);
    return a === "now" ? "just now" : `${a} ago`;
  });
  const minutes = $derived(Math.max(1, Math.round(words / 230)));
</script>

<main bind:this={scroller}>
  <header data-tauri-drag-region>
    {#if archived}<span class="archived">Archived</span>{/if}
    <span class="folder">{note.folder}</span>
    <span class="file" title={note.id}>{note.file_name}</span>
    <span class="meta" title={longDate(note.modified)}>
      {ago} · {words.toLocaleString()} words · {minutes} min
    </span>
  </header>
  <!-- The HTML is rendered and sanitised by ammonia in the Rust backend. -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
  <article class="prose" bind:this={article} onclick={onClick}>{@html html}</article>
</main>

<style>
  main {
    height: 100%;
    overflow-y: auto;
    scroll-behavior: smooth;
  }

  header {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    gap: 10px;
    align-items: baseline;
    padding: 12px 32px 10px;
    min-height: 38px;
    font-size: 12px;
    color: var(--overlay0);
    background: color-mix(in srgb, var(--base) 85%, transparent);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
  }

  .archived {
    padding: 0 7px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--yellow) 20%, transparent);
    color: var(--text);
    font-weight: 600;
  }

  .folder {
    text-transform: uppercase;
    letter-spacing: 0.06em;
    font-weight: 650;
    color: var(--accent);
    font-size: 11px;
  }

  .file {
    font-family: var(--mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    margin-left: auto;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  article {
    width: var(--note-width);
    min-width: min(100%, 60ch);
    margin: 0 auto;
    padding: 8px 40px 30vh;
    user-select: text;
    -webkit-user-select: text;
    cursor: auto;
  }

  /* The rendered Markdown. */
  .prose {
    font-size: 15px;
    line-height: 1.7;
  }

  .prose :global(h1),
  .prose :global(h2),
  .prose :global(h3),
  .prose :global(h4) {
    position: relative;
    line-height: 1.3;
    font-weight: 700;
    letter-spacing: -0.01em;
    margin: 1.8em 0 0.6em;
    scroll-margin-top: 60px;
  }

  .prose :global(h1) {
    font-size: 2em;
    margin-top: 0.4em;
    letter-spacing: -0.02em;
  }

  .prose :global(h2) {
    font-size: 1.45em;
    padding-bottom: 0.3em;
    border-bottom: 1px solid var(--surface0);
  }

  .prose :global(h3) {
    font-size: 1.2em;
  }

  .prose :global(h4) {
    font-size: 1em;
    color: var(--subtext);
  }

  .prose :global(a.anchor) {
    position: absolute;
    left: -1.1em;
    opacity: 0;
    text-decoration: none;
  }

  .prose :global(a.anchor)::before {
    content: "#";
    color: var(--overlay0);
  }

  .prose :global(h1:hover a.anchor),
  .prose :global(h2:hover a.anchor),
  .prose :global(h3:hover a.anchor),
  .prose :global(h4:hover a.anchor) {
    opacity: 1;
  }

  .prose :global(p),
  .prose :global(ul),
  .prose :global(ol),
  .prose :global(dl),
  .prose :global(pre),
  .prose :global(table),
  .prose :global(blockquote) {
    margin: 0 0 1em;
  }

  .prose :global(a) {
    color: var(--blue);
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, var(--blue) 35%, transparent);
    text-underline-offset: 3px;
    cursor: pointer;
  }

  .prose :global(strong) {
    font-weight: 650;
  }

  .prose :global(ul),
  .prose :global(ol) {
    padding-left: 1.5em;
  }

  .prose :global(li) {
    margin: 0.2em 0;
  }

  .prose :global(li::marker) {
    color: var(--overlay0);
  }

  .prose :global(li > ul),
  .prose :global(li > ol),
  .prose :global(li > p) {
    margin-bottom: 0.2em;
  }

  .prose :global(li:has(> input[type="checkbox"])) {
    list-style: none;
    margin-left: -1.4em;
  }

  .prose :global(input[type="checkbox"]) {
    accent-color: var(--green);
    margin: 0 0.4em 0 0;
    vertical-align: -1px;
  }

  .prose :global(code) {
    font-family: var(--mono);
    font-size: 0.85em;
    padding: 0.15em 0.35em;
    border-radius: 4px;
    background: var(--mantle);
    border: 1px solid var(--crust);
  }

  .prose :global(pre) {
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--mantle);
    border: 1px solid var(--crust);
    overflow-x: auto;
    line-height: 1.55;
  }

  .prose :global(pre code) {
    padding: 0;
    border: 0;
    background: none;
    font-size: 12.5px;
  }

  .prose :global(blockquote) {
    padding: 0.2em 1em;
    border-left: 3px solid var(--surface1);
    color: var(--subtext);
  }

  .prose :global(blockquote > :last-child) {
    margin-bottom: 0;
  }

  .prose :global(table) {
    display: block;
    overflow-x: auto;
    border-collapse: collapse;
    font-size: 0.92em;
    line-height: 1.5;
  }

  .prose :global(th),
  .prose :global(td) {
    padding: 6px 12px;
    border: 1px solid var(--surface0);
    text-align: left;
    vertical-align: top;
  }

  .prose :global(th) {
    background: var(--mantle);
    font-weight: 650;
  }

  .prose :global(tr:nth-child(2n) td) {
    background: color-mix(in srgb, var(--mantle) 50%, transparent);
  }

  .prose :global(hr) {
    border: 0;
    border-top: 1px solid var(--surface0);
    margin: 2em 0;
  }

  .prose :global(dt) {
    font-weight: 650;
  }

  .prose :global(dd) {
    margin: 0 0 0.5em 1.5em;
  }

  .prose :global(.markdown-alert) {
    padding: 0.5em 1em;
    margin: 0 0 1em;
    border-left: 3px solid var(--blue);
    border-radius: 0 var(--radius) var(--radius) 0;
    background: color-mix(in srgb, var(--blue) 8%, transparent);
  }

  .prose :global(.markdown-alert > :last-child) {
    margin-bottom: 0;
  }

  .prose :global(.markdown-alert-title) {
    font-weight: 650;
    margin-bottom: 0.2em;
  }

  .prose :global(.markdown-alert-tip) {
    border-color: var(--green);
    background: color-mix(in srgb, var(--green) 8%, transparent);
  }

  .prose :global(.markdown-alert-important) {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  .prose :global(.markdown-alert-warning) {
    border-color: var(--yellow);
    background: color-mix(in srgb, var(--yellow) 10%, transparent);
  }

  .prose :global(.markdown-alert-caution) {
    border-color: var(--red);
    background: color-mix(in srgb, var(--red) 8%, transparent);
  }

  .prose :global(.footnotes) {
    font-size: 0.88em;
    color: var(--subtext);
    border-top: 1px solid var(--surface0);
    margin-top: 2em;
  }

  .prose :global(.current) {
    border-radius: 4px;
    outline: 2px solid color-mix(in srgb, var(--accent) 45%, transparent);
    outline-offset: 5px;
  }

  .prose :global(.flash) {
    animation: flash 1.6s ease-out;
  }

  @keyframes flash {
    0%,
    30% {
      background: color-mix(in srgb, var(--yellow) 30%, transparent);
    }
    100% {
      background: transparent;
    }
  }
</style>
