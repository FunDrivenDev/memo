/// <reference lib="dom" />
import { deepEqual, equal } from "node:assert/strict";
import type { Page } from "playwright";
import { browserTest, openApp } from "./app.ts";

/** The titles of the notes the sidebar lists. */
const cards = (page: Page) => page.locator("aside .card .title").allTextContents();
const selectedTitle = (page: Page) => page.locator("aside .card.selected .title").textContent();
/** Whether the mocked Rust side was asked to trash a note. */
const trashed = (page: Page) =>
  page.evaluate(() => (globalThis as unknown as { invoked: string[] }).invoked.includes("trash"));

browserTest("t hides the note at once, and u during the countdown brings it back untrashed", async () => {
  const app = await openApp({ notes: 3, paragraphs: 3 });
  try {
    const { page } = app;
    await page.keyboard.press("t");
    deepEqual(await cards(page), ["Note 1", "Note 2"]);
    equal(await selectedTitle(page), "Note 1", "the next note takes its place");
    await page.locator(".toast .undo").waitFor();

    await page.keyboard.press("u");
    deepEqual(await cards(page), ["Note 0", "Note 1", "Note 2"]);
    equal(await selectedTitle(page), "Note 0");
    await page.waitForTimeout(6500);
    equal(await trashed(page), false);
  } finally {
    await app.close();
  }
});

browserTest("t trashes the note for real once its countdown ends", async () => {
  const app = await openApp({ notes: 3, paragraphs: 3 });
  try {
    const { page } = app;
    await page.keyboard.press("t");
    equal(await trashed(page), false);
    await page.locator(".toast").waitFor({ state: "detached", timeout: 7000 });
    equal(await trashed(page), true);
    deepEqual(await cards(page), ["Note 1", "Note 2"]);
  } finally {
    await app.close();
  }
});
