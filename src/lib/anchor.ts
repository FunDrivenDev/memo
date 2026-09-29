// Attaches comments to the rendered note and finds them again after it changes. A passage is found by its text and
// the few characters around it, a run of blocks by the text it starts with; both prefer the place nearest to the source
// lines they were written on. Whitespace is ignored throughout, as the DOM and a selection space blocks differently.
import type { Anchor } from "./api.ts";

/** The blocks the reading cursor steps through and a comment attaches to. */
export const BLOCKS = ":is(p, li, pre, table, dt, dd)[data-sourcepos]";
/** What memo adds to the note, never part of its text. */
export const ADDED = ".comment-thread, button.copy";

/** Characters kept around a passage to tell its repeats apart. */
const CONTEXT = 32;

const strip = (text: string) => text.replace(/\s+/g, "");
const collapse = (text: string) => text.replace(/\s+/g, " ").trim();

/** The source lines of an element, from comrak's `data-sourcepos`. */
export function lines(el: Element | null): [number, number] | null {
  const m = /^(\d+):\d+-(\d+):\d+$/.exec(el?.getAttribute("data-sourcepos") ?? "");
  return m ? [Number(m[1]), Number(m[2])] : null;
}

/** The blocks of the note, in order: paragraphs, list items, code and tables; a list item stands for its paragraphs. */
export function blocks(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(BLOCKS)].filter(
    (el) => !(el.tagName === "P" && el.parentElement?.matches("li, dd")) && !el.closest(ADDED),
  );
}

/** The block holding a node, else the nearest element with a source position, such as a heading. */
export function blockOf(node: Node, root: HTMLElement): HTMLElement | null {
  const el = node instanceof Element ? node : node.parentElement;
  let block = el?.closest<HTMLElement>(BLOCKS) ?? null;
  if (block?.tagName === "P" && block.parentElement?.matches("li, dd")) block = block.parentElement;
  block ??= el?.closest<HTMLElement>("[data-sourcepos]") ?? null;
  return block && block !== root && root.contains(block) ? block : null;
}

/** The text of a range, without what memo added. */
function textOf(range: Range): string {
  const copy = range.cloneContents();
  copy.querySelectorAll(ADDED).forEach((el) => el.remove());
  return copy.textContent ?? "";
}

/** Every visible character of the note, whitespace left out, with the text node and offset it sits at. */
interface Index {
  text: string;
  at: { node: Text; offset: number }[];
}

function index(root: HTMLElement): Index {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (
      n,
    ) => (n.parentElement?.closest(ADDED) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
  });
  let text = "";
  const at: Index["at"] = [];
  for (let node = walker.nextNode() as Text | null; node; node = walker.nextNode() as Text | null) {
    for (let offset = 0; offset < node.data.length; offset++) {
      const c = node.data[offset]!;
      if (/\s/.test(c)) continue;
      text += c;
      at.push({ node, offset });
    }
  }
  return { text, at };
}

/** How many indexed characters come before a DOM point. */
function position(idx: Index, container: Node, offset: number): number {
  const point = document.createRange();
  point.setStart(container, offset);
  let lo = 0;
  let hi = idx.at.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    const { node, offset: o } = idx.at[mid]!;
    if (point.comparePoint(node, o) < 0) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

/** The anchor of a selected passage, or null when it holds no text. */
export function passage(root: HTMLElement, range: Range): Anchor | null {
  const idx = index(root);
  const s = position(idx, range.startContainer, range.startOffset);
  const e = position(idx, range.endContainer, range.endOffset);
  if (e <= s) return null;
  const first = lines(blockOf(idx.at[s]!.node, root));
  const last = lines(blockOf(idx.at[e - 1]!.node, root));
  return {
    kind: "text",
    start: first?.[0] ?? 0,
    end: last?.[1] ?? first?.[1] ?? 0,
    quote: collapse(textOf(range)),
    prefix: idx.text.slice(Math.max(0, s - CONTEXT), s),
    suffix: idx.text.slice(e, e + CONTEXT),
  };
}

/** The anchor of a run of blocks, from the first to the last. */
export function run(first: HTMLElement, last: HTMLElement): Anchor {
  const range = document.createRange();
  range.setStartBefore(first);
  range.setEndAfter(last);
  return {
    kind: "block",
    start: lines(first)?.[0] ?? 0,
    end: lines(last)?.[1] ?? 0,
    quote: collapse(textOf(range)),
    prefix: "",
    suffix: "",
  };
}

/** Where an anchor is in the note now: the range it covers and the blocks it touches. */
export interface Found {
  range: Range;
  blocks: HTMLElement[];
}

const sharedEnd = (a: string, b: string) => {
  let n = 0;
  while (n < a.length && n < b.length && a[a.length - 1 - n] === b[b.length - 1 - n]) n++;
  return n;
};

const sharedStart = (a: string, b: string) => {
  let n = 0;
  while (n < a.length && n < b.length && a[n] === b[n]) n++;
  return n;
};

/** Finds an anchor in the note, or null when its text is gone. */
export function locate(root: HTMLElement, anchor: Anchor): Found | null {
  return anchor.kind === "block" ? locateRun(root, anchor) : locatePassage(root, anchor);
}

function locatePassage(root: HTMLElement, anchor: Anchor): Found | null {
  const idx = index(root);
  const quote = strip(anchor.quote);
  if (!quote) return null;
  let best = -1;
  let bestScore = -Infinity;
  for (let i = idx.text.indexOf(quote); i >= 0; i = idx.text.indexOf(quote, i + 1)) {
    const around = sharedEnd(idx.text.slice(Math.max(0, i - CONTEXT), i), anchor.prefix) +
      sharedStart(idx.text.slice(i + quote.length, i + quote.length + CONTEXT), anchor.suffix);
    const line = lines(blockOf(idx.at[i]!.node, root))?.[0] ?? 0;
    // The context weighs most; the distance to the old lines only breaks ties.
    const score = around - Math.abs(line - anchor.start) / 10_000;
    if (score > bestScore) [best, bestScore] = [i, score];
  }
  if (best < 0) return null;
  const first = idx.at[best]!;
  const last = idx.at[best + quote.length - 1]!;
  const range = document.createRange();
  range.setStart(first.node, first.offset);
  range.setEnd(last.node, last.offset + 1);
  const all = blocks(root);
  const from = blockOf(first.node, root);
  const to = blockOf(last.node, root);
  const a = from ? all.indexOf(from) : -1;
  const b = to ? all.indexOf(to) : -1;
  const touched = a >= 0 && b >= a ? all.slice(a, b + 1) : [from ?? to].filter((el) => el !== null);
  return { range, blocks: touched };
}

function locateRun(root: HTMLElement, anchor: Anchor): Found | null {
  const all = blocks(root);
  const quote = strip(anchor.quote);
  let best = -1;
  for (const [i, el] of all.entries()) {
    const range = document.createRange();
    range.selectNodeContents(el);
    const text = strip(textOf(range));
    const n = Math.min(40, text.length, quote.length);
    if (!n || text.slice(0, n) !== quote.slice(0, n)) continue;
    const distance = Math.abs((lines(el)?.[0] ?? 0) - anchor.start);
    if (best < 0 || distance < Math.abs((lines(all[best]!)?.[0] ?? 0) - anchor.start)) best = i;
  }
  if (best < 0) return null;
  // The run keeps its length in lines, moved as much as its first block moved.
  const end = anchor.end + (lines(all[best]!)?.[0] ?? 0) - anchor.start;
  let last = best;
  while (last + 1 < all.length && (lines(all[last + 1]!)?.[0] ?? Infinity) <= end) last++;
  const range = document.createRange();
  range.setStartBefore(all[best]!);
  range.setEndAfter(all[last]!);
  return { range, blocks: all.slice(best, last + 1) };
}

/** Wraps the text of a range in `<mark class="comment-mark">`, for the comment `id`. */
export function highlight(range: Range, id: string) {
  const root = range.commonAncestorContainer;
  const nodes: Text[] = [];
  if (root instanceof Text) nodes.push(root);
  else {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode() as Text | null; node; node = walker.nextNode() as Text | null) {
      if (range.intersectsNode(node) && node.data.trim() && !node.parentElement?.closest(ADDED)) {
        nodes.push(node);
      }
    }
  }
  const spans = nodes.map((node) => ({
    node,
    start: node === range.startContainer ? range.startOffset : 0,
    end: node === range.endContainer ? range.endOffset : node.length,
  }));
  for (const span of spans) {
    let { node } = span;
    if (span.end <= span.start) continue;
    if (span.end < node.length) node.splitText(span.end);
    if (span.start > 0) node = node.splitText(span.start);
    const mark = document.createElement("mark");
    mark.className = "comment-mark";
    mark.dataset.comment = id;
    node.replaceWith(mark);
    mark.append(node);
  }
}

/** Undoes `highlight`, merging the text back. */
export function unhighlight(root: HTMLElement) {
  for (const mark of root.querySelectorAll("mark.comment-mark")) {
    const parent = mark.parentNode;
    mark.replaceWith(...mark.childNodes);
    parent?.normalize();
  }
}

/** Where the comments of a block go: after it, or inside a list item before its sublist. */
export function threadAfter(block: HTMLElement): HTMLElement {
  const thread = document.createElement("div");
  thread.className = "comment-thread";
  if (block.matches("li, dd")) block.insertBefore(thread, block.querySelector(":scope > ul, :scope > ol"));
  else block.after(thread);
  return thread;
}
