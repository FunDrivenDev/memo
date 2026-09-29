<script lang="ts">
  import { mount, tick, unmount, untrack } from "svelte";
  import { ADDED, BLOCKS, blockOf, blocks, highlight, locate, passage, run, threadAfter, unhighlight } from "./anchor";
  import * as api from "./api";
  import type { Anchor, Comment, Note } from "./api";
  import CommentThread, { type ThreadState } from "./CommentThread.svelte";
  import { listStep } from "./keys";
  import { reveal } from "./scroll";
  import { age, clock, longDate } from "./time.svelte";

  let {
    note,
    archived = false,
    focusLine,
    comments,
    focusComment = null,
    focused = $bindable(null),
    onOpenNote,
    onError,
    onAddComment,
    onEditComment,
    onResolveComment,
  }: {
    note: Note;
    /** Whether the note is in the archive. */
    archived?: boolean;
    /** A source line to scroll to once rendered, from a search hit. */
    focusLine: number | null;
    /** The comments on this note. */
    comments: Comment[];
    /** A comment to scroll to and focus, from the list of comments. */
    focusComment?: string | null;
    /** Whether text of the note is selected, which `c` then comments. */
    /** The comment `r` resolves and `↵` edits, if any. */
    focused?: Comment | null;
    onOpenNote: (id: string) => boolean;
    onError: (message: string) => void;
    /** Saves a new comment, returning its id, or null when it failed. */
    onAddComment: (anchor: Anchor, body: string) => Promise<string | null>;
    onEditComment: (id: string, body: string) => Promise<boolean>;
    onResolveComment: (comment: Comment) => void;
  } = $props();

  let html = $state("");
  /** The note `html` belongs to. */
  let htmlFor = $state("");
  let words = $state(0);
  let scroller: HTMLElement | undefined = $state();
  let article: HTMLElement | undefined = $state();
  let shownId = "";
  /** The item under the reading cursor, if reading. */
  let current: HTMLElement | null = null;
  /** The other end of the items the reading cursor spans, which `⇧↓` `⇧↑` extend. */
  let pivot: HTMLElement | null = null;

  /** What the comment threads share. */
  const ui = $state<ThreadState>({ focused: null, editing: null, draft: "" });
  /** Where the comment being written goes. */
  let draft = $state<Anchor | null>(null);
  /** The threads mounted into the note, removed before it changes. */
  let threads: { target: HTMLElement; instance: Record<string, unknown> }[] = [];
  /** The blocks each comment touches in the note. */
  let anchored = new Map<string, HTMLElement[]>();
  /** Counts the times the comments were laid out, so what depends on them follows. */
  let decorated = $state(0);
  /** The comment last scrolled to for `focusComment`. */
  let revealed: string | null = null;
  /** The floating Comment button over a selection, and the + button beside the block under the pointer. */
  let selectionButton = $state<{ top: number; left: number } | null>(null);
  let gutter = $state<{ top: number; left: number; block: HTMLElement } | null>(null);

  const copyIcons =
    `<svg class="icon-copy" viewBox="0 0 24 24" aria-hidden="true"><rect x="9" y="9" width="12" height="12" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>` +
    `<svg class="icon-copied" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>`;

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
        htmlFor = id;
        words = rendered.words;
        await tick();
        addCopyButtons();
        // A mere edit keeps the cursor on the same item, when it is still there.
        const pos = shownId === id ? current?.dataset.sourcepos : undefined;
        current = null;
        if (pos) mark(article?.querySelector<HTMLElement>(`[data-sourcepos="${pos}"]`) ?? null, false);
        if (line !== null) scrollToLine(line);
        else if (shownId !== id && !focusComment) scroller?.scrollTo({ top: 0 });
        shownId = id;
      })
      .catch((e) => onError(String(e)));
    void modified;
    return () => (stale = true);
  });

  // Another note drops the comment being written or edited.
  $effect(() => {
    void note.id;
    untrack(cancel);
  });

  // Lay the comments out over the note, again whenever it or they change.
  $effect(() => {
    const list = comments;
    const pending = draft;
    if (!article || htmlFor !== note.id || !html) return;
    untrack(() => decorate(list, pending));
    return undecorate;
  });

  $effect(() => {
    focused = comments.find((c) => c.id === ui.focused) ?? null;
  });

  // The focused comment's passage or blocks stand out.
  $effect(() => {
    const id = ui.focused;
    void decorated;
    for (const el of article?.querySelectorAll(".comment-focus") ?? []) el.classList.remove("comment-focus");
    if (!id || !article) return;
    for (const el of article.querySelectorAll(`mark[data-comment="${CSS.escape(id)}"]`)) {
      el.classList.add("comment-focus");
    }
    for (const el of anchored.get(id) ?? []) if (el.classList.contains("commented")) el.classList.add("comment-focus");
  });

  $effect(() => {
    const id = focusComment;
    void decorated;
    if (!id) {
      revealed = null;
      return;
    }
    if (id !== revealed && untrack(() => showComment(id))) revealed = id;
  });

  function decorate(list: Comment[], pending: Anchor | null) {
    if (!article) return;
    const root = article;
    anchored = new Map();
    const hosts = new Map<HTMLElement, Comment[]>();
    const lost: Comment[] = [];
    const sorted = [...list].sort((a, b) => a.anchor.start - b.anchor.start || a.created - b.created);
    // One at a time: highlighting splits the text the next one is looked for in.
    for (const comment of sorted) {
      const found = locate(root, comment.anchor);
      const host = found?.blocks.at(-1);
      if (!found || !host) {
        lost.push(comment);
        continue;
      }
      anchored.set(comment.id, found.blocks);
      if (comment.anchor.kind === "text") highlight(found.range, comment.id);
      else found.blocks.forEach((el) => el.classList.add("commented"));
      hosts.set(host, [...(hosts.get(host) ?? []), comment]);
    }
    let draftHost: HTMLElement | null = null;
    if (pending) {
      const found = locate(root, pending);
      draftHost = found?.blocks.at(-1) ?? null;
      if (found && draftHost) {
        if (pending.kind === "text") highlight(found.range, "draft");
        else found.blocks.forEach((el) => el.classList.add("commented"));
        if (!hosts.has(draftHost)) hosts.set(draftHost, []);
      }
    }
    for (const [host, list] of hosts) mountThread(threadAfter(host), list, host === draftHost, false);
    if (lost.length || (pending && !draftHost)) {
      const target = document.createElement("div");
      target.className = "comment-thread";
      root.prepend(target);
      mountThread(target, lost, !!pending && !draftHost, true);
    }
    decorated++;
  }

  function mountThread(target: HTMLElement, list: Comment[], composing: boolean, detached: boolean) {
    const instance = mount(CommentThread, {
      target,
      props: {
        comments: list,
        ui,
        composing,
        detached,
        onSave: save,
        onCancel: cancel,
        onEdit: edit,
        onResolve: onResolveComment,
        onFocus: (id: string) => (ui.focused = id),
      },
    });
    threads.push({ target, instance });
  }

  function undecorate() {
    for (const { target, instance } of threads) {
      void unmount(instance);
      target.remove();
    }
    threads = [];
    if (!article) return;
    unhighlight(article);
    for (const el of article.querySelectorAll(".commented")) el.classList.remove("commented", "comment-focus");
  }

  async function save(id: string | null) {
    const body = ui.draft.trim();
    if (!body) return;
    if (id) {
      if (!(await onEditComment(id, body))) return;
      ui.editing = null;
    } else {
      if (!draft) return;
      const added = await onAddComment(draft, body);
      if (!added) return;
      draft = null;
      ui.focused = added;
    }
    ui.draft = "";
  }

  function cancel() {
    draft = null;
    ui.editing = null;
    ui.draft = "";
  }

  function edit(comment: Comment) {
    draft = null;
    ui.focused = comment.id;
    ui.editing = comment.id;
    ui.draft = comment.body;
  }

  /** Edits the focused comment; false when none is. */
  export function editFocused(): boolean {
    const comment = comments.find((c) => c.id === ui.focused);
    if (comment) edit(comment);
    return !!comment;
  }

  /** Scrolls to a comment's place and focuses it; false when it is not laid out yet. */
  function showComment(id: string): boolean {
    const card = article?.querySelector<HTMLElement>(`.comment[data-comment="${CSS.escape(id)}"]`);
    if (!card || !scroller) return false;
    ui.focused = id;
    const target = anchored.get(id)?.[0] ?? card;
    reveal(scroller, target, target.offsetHeight > scroller.clientHeight * 0.6 ? "start" : "center");
    return true;
  }

  /** The selected passage of the note, if any. */
  function selection(): Range | null {
    const sel = window.getSelection();
    if (!article || !sel || sel.isCollapsed || !sel.rangeCount) return null;
    const range = sel.getRangeAt(0);
    const common = range.commonAncestorContainer;
    const el = common instanceof Element ? common : common.parentElement;
    return article.contains(common) && !el?.closest(ADDED) ? range : null;
  }

  /**
   * Starts a comment on the selected passage or, while reading, on the items under the cursor; false when there is
   * neither.
   */
  export function comment(): boolean {
    const range = selection();
    const span = range ? [] : cursorSpan();
    const anchor = range && article ? passage(article, range) : span.length ? run(span[0]!, span.at(-1)!) : null;
    if (!anchor) return false;
    startDraft(anchor);
    return true;
  }

  function startDraft(anchor: Anchor) {
    window.getSelection()?.removeAllRanges();
    ui.editing = null;
    ui.draft = "";
    draft = anchor;
  }

  function onSelectionChange() {
    const range = selection();
    if (!range || !scroller) {
      selectionButton = null;
      return;
    }
    const rects = range.getClientRects();
    const end = rects[rects.length - 1] ?? range.getBoundingClientRect();
    const box = scroller.getBoundingClientRect();
    selectionButton = {
      top: end.bottom - box.top + scroller.scrollTop + 6,
      left: Math.max(8, Math.min(end.right - box.left - 40, scroller.clientWidth - 130)),
    };
  }

  /** Offers the + button beside the block under the pointer. */
  function onPointerMove(event: MouseEvent) {
    const target = event.target as HTMLElement;
    if (target.closest(".gutter")) return;
    const block = article?.contains(target) && !target.closest(ADDED) ? blockOf(target, article) : null;
    if (!block?.matches(BLOCKS) || !scroller || !article) {
      gutter = null;
      return;
    }
    const box = scroller.getBoundingClientRect();
    const top = block.getBoundingClientRect().top - box.top + scroller.scrollTop;
    gutter = { top, left: article.offsetLeft + 10, block };
  }
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

  function addCopyButtons() {
    for (const pre of article?.querySelectorAll("pre") ?? []) {
      const button = document.createElement("button");
      button.className = "copy";
      button.title = "Copy";
      button.ariaLabel = "Copy code";
      // Out of the tab order, so a click leaves the keys to the app: space still scrolls.
      button.tabIndex = -1;
      button.innerHTML = copyIcons;
      pre.append(button);
    }
  }

  function copyCode(button: HTMLElement) {
    const code = button.parentElement?.querySelector("code")?.textContent ?? "";
    api
      .copy(code.replace(/\n$/, ""))
      .then(() => {
        button.classList.add("copied");
        button.title = "Copied";
        setTimeout(() => {
          button.classList.remove("copied");
          button.title = "Copy";
        }, 1500);
      })
      .catch((e) => onError(`Could not copy the code: ${e}`));
  }

  /** Scrolls the note by `pages` pages. */
  export function scroll(pages: number) {
    scroller?.scrollBy({ top: pages * scroller.clientHeight * 0.85 });
  }

  /** The blocks the reading cursor steps through: paragraphs, list items, code and tables, not headings. */
  function items(): HTMLElement[] {
    return article ? blocks(article) : [];
  }

  /** The items under the reading cursor, from the pivot to the cursor. */
  function cursorSpan(): HTMLElement[] {
    if (!current) return [];
    const all = items();
    const a = pivot ? all.indexOf(pivot) : -1;
    const b = all.indexOf(current);
    if (a < 0 || b < 0) return [current];
    return all.slice(Math.min(a, b), Math.max(a, b) + 1);
  }

  /** Puts the reading cursor on `el`; `extend` keeps the pivot, so the cursor spans the items between. */
  function mark(el: HTMLElement | null, show = true, extend = false) {
    for (const item of article?.querySelectorAll(".current") ?? []) item.classList.remove("current");
    current = el;
    if (!extend) pivot = el;
    if (!el || !scroller) return;
    const span = cursorSpan();
    for (const item of span) item.classList.add("current");
    // The comment on the items under the cursor is the one `r` and `↵` act on.
    ui.focused = comments.find((c) => anchored.get(c.id)?.some((b) => span.includes(b)))?.id ?? null;
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

  /** Moves the reading cursor for an arrow key, as in any list, or extends it with ⇧; false for any other key. */
  export function step(event: KeyboardEvent): boolean {
    const all = items();
    const arrow = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (arrow && event.shiftKey && !event.metaKey && !event.altKey && !event.ctrlKey && current) {
      mark(all[Math.max(0, Math.min(all.length - 1, all.indexOf(current) + arrow))] ?? current, true, true);
      return true;
    }
    const next = listStep(event, current ? all.indexOf(current) : -1, all.length);
    if (next === null) return false;
    mark(all[next] ?? null);
    return true;
  }

  function onClick(event: MouseEvent) {
    const commented = (event.target as HTMLElement).closest<HTMLElement>("mark.comment-mark");
    if (commented?.dataset.comment && commented.dataset.comment !== "draft" && !selection()) {
      ui.focused = commented.dataset.comment;
    }
    const copy = (event.target as HTMLElement).closest<HTMLElement>("button.copy");
    if (copy) return copyCode(copy);
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

<svelte:document onselectionchange={onSelectionChange} />

<main bind:this={scroller} onmousemove={onPointerMove} onmouseleave={() => (gutter = null)}>
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
  {#if selectionButton}
    <button
      class="float"
      style:top="{selectionButton.top}px"
      style:left="{selectionButton.left}px"
      onmousedown={(e) => e.preventDefault()}
      onclick={comment}
    >
      Comment <kbd>c</kbd>
    </button>
  {:else if gutter && !draft}
    {@const block = gutter.block}
    <button
      class="gutter"
      style:top="{gutter.top}px"
      style:left="{gutter.left}px"
      title="Comment on this block"
      aria-label="Comment on this block"
      onmousedown={(e) => e.preventDefault()}
      onclick={() => startDraft(run(block, block))}
    >+</button>
  {/if}
</main>

<style>
  main {
    position: relative;
    height: 100%;
    overflow-y: auto;
    scroll-behavior: smooth;
  }

  .float,
  .gutter {
    position: absolute;
    z-index: 3;
    border: 1px solid var(--surface0);
    background: var(--base);
    box-shadow: 0 2px 8px rgb(0 0 0 / 0.12);
    cursor: pointer;
  }

  .float {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 12px;
  }

  .gutter {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-top: 3px;
    padding: 0;
    border-radius: 5px;
    color: var(--overlay2);
    font-size: 15px;
    line-height: 1;
  }

  .gutter:hover {
    color: var(--base);
    background: var(--yellow);
    border-color: var(--yellow);
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

  /* The code scrolls inside the block, so the copy button stays in its corner. */
  .prose :global(pre) {
    position: relative;
    border-radius: var(--radius);
    background: var(--mantle);
    border: 1px solid var(--crust);
    line-height: 1.55;
  }

  .prose :global(pre code) {
    display: block;
    padding: 14px 16px;
    overflow-x: auto;
    border: 0;
    background: none;
    font-size: 12.5px;
  }

  .prose :global(pre .copy) {
    position: absolute;
    top: 6px;
    right: 6px;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid var(--surface0);
    border-radius: 6px;
    background: var(--mantle);
    color: var(--overlay2);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .prose :global(pre:hover .copy),
  .prose :global(pre.current .copy),
  .prose :global(pre .copy.copied) {
    opacity: 1;
  }

  .prose :global(pre .copy:hover) {
    color: var(--text);
    background: var(--crust);
  }

  .prose :global(pre .copy svg) {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .prose :global(pre .copy.copied) {
    color: var(--green);
  }

  .prose :global(pre .copy .icon-copied),
  .prose :global(pre .copy.copied .icon-copy) {
    display: none;
  }

  .prose :global(pre .copy.copied .icon-copied) {
    display: block;
  }

  /* Code coloured by the scopes syntect marks up as `hl-` classes, after Catppuccin's own mapping. */
  .prose :global(.hl-comment) {
    color: var(--overlay2);
    font-style: italic;
  }

  .prose :global(.hl-keyword),
  .prose :global(.hl-storage) {
    color: var(--accent);
  }

  .prose :global(.hl-keyword.hl-operator) {
    color: var(--sky);
  }

  .prose :global(.hl-string),
  .prose :global(.hl-markup.hl-inserted) {
    color: var(--green);
  }

  .prose :global(.hl-constant) {
    color: var(--peach);
  }

  .prose :global(.hl-constant.hl-character.hl-escape),
  .prose :global(.hl-string.hl-regexp) {
    color: var(--pink);
  }

  .prose :global(.hl-entity.hl-name.hl-function),
  .prose :global(.hl-support.hl-function),
  .prose :global(.hl-variable.hl-function),
  .prose :global(.hl-entity.hl-name.hl-tag),
  .prose :global(.hl-meta.hl-mapping.hl-key .hl-string),
  .prose :global(.hl-markup.hl-heading) {
    color: var(--blue);
  }

  .prose :global(.hl-entity.hl-name.hl-type),
  .prose :global(.hl-entity.hl-name.hl-class),
  .prose :global(.hl-entity.hl-other.hl-inherited-class),
  .prose :global(.hl-support.hl-type),
  .prose :global(.hl-support.hl-class),
  .prose :global(.hl-entity.hl-other.hl-attribute-name) {
    color: var(--yellow);
  }

  .prose :global(.hl-variable.hl-parameter) {
    color: var(--maroon);
  }

  .prose :global(.hl-support.hl-macro),
  .prose :global(.hl-markup.hl-deleted) {
    color: var(--red);
  }

  .prose :global(.hl-meta.hl-diff) {
    color: var(--sky);
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

  /* A commented passage, and the blocks a comment is on. */
  .prose :global(mark.comment-mark) {
    color: inherit;
    font-weight: inherit;
    background: color-mix(in srgb, var(--yellow) 22%, transparent);
    border-bottom: 1px solid color-mix(in srgb, var(--yellow) 70%, transparent);
    cursor: pointer;
  }

  .prose :global(mark.comment-mark.comment-focus),
  .prose :global(mark.comment-mark[data-comment="draft"]) {
    background: color-mix(in srgb, var(--yellow) 45%, transparent);
  }

  .prose :global(.commented) {
    border-radius: 4px;
    background: color-mix(in srgb, var(--yellow) 10%, transparent);
    box-shadow: -3px 0 0 color-mix(in srgb, var(--yellow) 70%, transparent);
  }

  .prose :global(.commented.comment-focus) {
    background: color-mix(in srgb, var(--yellow) 22%, transparent);
    box-shadow: -3px 0 0 var(--yellow);
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
