/// <reference lib="dom" />
import { deepEqual, equal } from "node:assert/strict";
import { commands } from "../src/lib/commands.ts";
import { browserTest, openApp } from "./app.ts";

browserTest("? lists every key of the table, the keys its text names shown as keys", async () => {
  const app = await openApp({ notes: 1, paragraphs: 1 });
  try {
    const { page } = app;
    await page.keyboard.press("?");
    const rows = page.locator(".modal dt");
    deepEqual(await rows.first().locator("kbd").allTextContents(), ["⌘K"]);
    equal(await rows.count(), commands.length);
    equal(
      await page.locator(".modal dd", { hasText: "goes back to the list" }).locator("kbd", { hasText: "esc" })
        .count(),
      1,
    );
  } finally {
    await app.close();
  }
});

browserTest("the palette lists the commands with their keys, and runs them", async () => {
  const app = await openApp({ notes: 1, paragraphs: 1 });
  try {
    const { page } = app;
    await page.keyboard.press("Meta+k");
    await page.keyboard.type(">show the");
    deepEqual(await page.locator(".palette li button .title").allTextContents(), [
      "Show the notes",
      "Show the archive",
      "Show the comments",
    ]);
    deepEqual(await page.locator(".palette li button kbd").allTextContents(), ["⌘1", "⌘2", "⌘3"]);
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    equal(
      await page.locator("[role=tab][aria-selected=true]").textContent().then((t) => t?.trim().split(" ")[0]),
      "Archive",
    );
  } finally {
    await app.close();
  }
});

browserTest("↵ reads the note, ↵ again edits the comment read, and esc goes back to the list", async () => {
  const app = await openApp({
    notes: 2,
    paragraphs: 3,
    comments: [{
      id: "a",
      note: "/notes/plans/note-0.md",
      body: "Look again",
      anchor: { kind: "block", start: 1, end: 1, quote: "Paragraph 0", prefix: "", suffix: "" },
      created: 1,
      updated: 1,
    }],
  });
  try {
    const { page } = app;
    await page.keyboard.press("Enter");
    await page.locator("aside.idle").waitFor();
    await page.keyboard.press("Enter");
    await page.locator(".comment-thread textarea").waitFor();
    await page.keyboard.press("Escape");
    await page.locator(".comment-thread textarea").waitFor({ state: "detached" });
    await page.keyboard.press("Escape");
    await page.locator("aside:not(.idle)").waitFor();
    await page.keyboard.press("ArrowDown");
    equal(await page.locator("aside .card.selected .title").textContent(), "Note 1");
  } finally {
    await app.close();
  }
});
