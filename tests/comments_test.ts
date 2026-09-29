/// <reference lib="dom" />
import { deepEqual, equal, ok } from "node:assert/strict";
import type { Page } from "playwright";
import { browserTest, openApp } from "./app.ts";

/** The comments as the mocked Rust side keeps them. */
const saved = (page: Page) =>
  page.evaluate(() => (globalThis as unknown as { savedComments: Record<string, unknown>[] }).savedComments);

/** Selects the text of a paragraph from `from` to `to`. */
const select = (page: Page, paragraph: number, from: number, to: number) =>
  page.evaluate(([i, a, b]) => {
    const text = document.querySelectorAll("article p")[i!]!.firstChild!;
    const range = document.createRange();
    range.setStart(text, a!);
    range.setEnd(text, b!);
    getSelection()!.removeAllRanges();
    getSelection()!.addRange(range);
  }, [paragraph, from, to]);

async function write(page: Page, body: string) {
  await page.locator(".comment-thread textarea").fill(body);
  await page.keyboard.press("Meta+Enter");
}

const comment = (id: string, anchor: Record<string, unknown>, body = "Look again") => ({
  id,
  note: "/notes/plans/note-0.md",
  body,
  anchor: { prefix: "", suffix: "", ...anchor },
  created: 1,
  updated: 1,
});

browserTest("c comments the selected text, which stays highlighted with the comment under it", async () => {
  const app = await openApp({ notes: 1, paragraphs: 5 });
  try {
    await select(app.page, 2, 0, 9);
    await app.page.locator("button.float").waitFor();
    await app.page.keyboard.press("c");
    await write(app.page, "Why this one?");
    await app.page.locator(".comment .body").waitFor();

    const [stored] = await saved(app.page);
    deepEqual((stored as { anchor: unknown }).anchor, {
      kind: "text",
      start: 5,
      end: 5,
      quote: "Paragraph",
      prefix: "Paragraph0Paragraph1",
      suffix: "2Paragraph3Paragraph4",
    });
    equal(await app.page.locator("mark.comment-mark").textContent(), "Paragraph");
    equal(await app.page.locator(".comment .body").textContent(), "Why this one?");
    equal(await app.page.locator("article p").nth(2).textContent(), "Paragraph 2");
    ok(
      await app.page.locator("article p:nth-of-type(3) + .comment-thread").isVisible(),
      "the comment follows it",
    );
  } finally {
    await app.close();
  }
});

browserTest("while reading, ⇧↓ spans more items and c comments them all", async () => {
  const app = await openApp({ notes: 1, paragraphs: 5 });
  try {
    await app.page.keyboard.press("Enter");
    await app.page.keyboard.press("ArrowDown");
    await app.page.keyboard.press("Shift+ArrowDown");
    equal(await app.page.locator("article .current").count(), 2);
    await app.page.keyboard.press("c");
    await write(app.page, "These two");
    await app.page.locator(".comment .body").waitFor();

    const [stored] = await saved(app.page);
    deepEqual((stored as { anchor: unknown }).anchor, {
      kind: "block",
      start: 3,
      end: 5,
      quote: "Paragraph 1Paragraph 2",
      prefix: "",
      suffix: "",
    });
    equal(await app.page.locator("article .commented").count(), 2);
  } finally {
    await app.close();
  }
});

browserTest("a comment finds its passage again after lines are added above it", async () => {
  const app = await openApp({
    notes: 1,
    paragraphs: 6,
    comments: [comment("a", { kind: "text", start: 1, end: 1, quote: "Paragraph 4", prefix: "Paragraph3" })],
  });
  try {
    await app.page.locator("mark.comment-mark").waitFor();
    equal(
      await app.page.locator("mark.comment-mark").evaluate((m) => m.parentElement!.textContent),
      "Paragraph 4",
    );
  } finally {
    await app.close();
  }
});

browserTest("a comment whose text is gone shows at the top of the note", async () => {
  const app = await openApp({
    notes: 1,
    paragraphs: 3,
    comments: [comment("a", { kind: "block", start: 1, end: 1, quote: "Deleted since" })],
  });
  try {
    const thread = app.page.locator("article > .comment-thread").first();
    await thread.waitFor();
    ok((await thread.textContent())?.includes("No longer found"));
  } finally {
    await app.close();
  }
});

browserTest("r resolves the focused comment, and u brings it back", async () => {
  const app = await openApp({
    notes: 1,
    paragraphs: 3,
    comments: [comment("a", { kind: "block", start: 3, end: 3, quote: "Paragraph 1" })],
  });
  try {
    await app.page.locator(".comment").click();
    await app.page.locator(".comment.focused").waitFor();
    await app.page.keyboard.press("r");
    await app.page.locator(".comment").waitFor({ state: "detached" });
    equal((await saved(app.page)).length, 0);
    await app.page.keyboard.press("u");
    await app.page.locator(".comment").waitFor();
    equal((await saved(app.page)).length, 1);
  } finally {
    await app.close();
  }
});

browserTest("the Comments pane lists and searches the comments, and ↵ edits the one selected", async () => {
  const app = await openApp({
    notes: 1,
    paragraphs: 3,
    comments: [
      comment("a", { kind: "block", start: 1, end: 1, quote: "Paragraph 0" }, "First thought"),
      comment("b", { kind: "block", start: 5, end: 5, quote: "Paragraph 2" }, "Second thought"),
    ],
  });
  try {
    await app.page.keyboard.press("/");
    await app.page.keyboard.type("second");
    equal(await app.page.locator("aside nav .card").count(), 1);
    await app.page.keyboard.press("Enter");
    equal(await app.page.locator("aside nav .card.selected .body").textContent(), "Second thought");
    await app.page.locator(".comment.focused").waitFor();
    await app.page.keyboard.press("Enter");
    await write(app.page, "Second, edited");
    await app.page.locator(".comment.focused .body").waitFor();
    equal(((await saved(app.page))[1] as { body: string }).body, "Second, edited");
  } finally {
    await app.close();
  }
});

browserTest("the column of keys beside the note offers c over a selection", async () => {
  const app = await openApp({ notes: 1, paragraphs: 3 }, { width: 1400, height: 700 });
  try {
    const keys = app.page.locator("aside:has(dl)");
    ok((await keys.textContent())?.includes("Archive"));
    await select(app.page, 0, 0, 5);
    await app.page.locator("dd.hot", { hasText: "Comment on the selection" }).waitFor();
    await app.page.setViewportSize({ width: 900, height: 700 });
    ok(!(await keys.isVisible()), "a narrow window hides it");
  } finally {
    await app.close();
  }
});
