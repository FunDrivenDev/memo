/// <reference lib="dom" />
import { deepEqual, equal } from "node:assert/strict";
import type { Page } from "playwright";
import { browserTest, openApp } from "./app.ts";

/** The titles of the notes the sidebar lists. */
const cards = (page: Page) => page.locator("aside .card .title").allTextContents();
const selectedTitle = (page: Page) => page.locator("aside .card.selected .title").textContent();

browserTest("a archives the note, u brings it back, and in the archive a restores one", async () => {
  const app = await openApp({ notes: 3, paragraphs: 2, archived: 1 });
  try {
    const { page } = app;
    await page.keyboard.press("a");
    await page.locator(".toast .undo").waitFor();
    deepEqual(await cards(page), ["Note 1", "Note 2"]);
    await page.keyboard.press("u");
    await page.locator(".toast .undo").waitFor({ state: "detached" });
    deepEqual(await cards(page), ["Note 0", "Note 1", "Note 2"]);
    equal(await selectedTitle(page), "Note 0");

    await page.keyboard.press("Meta+2");
    deepEqual(await cards(page), ["Archived 0"]);
    await page.keyboard.press("a");
    await page.locator(".toast .undo").waitFor();
    deepEqual(await cards(page), []);
    await page.keyboard.press("Meta+1");
    deepEqual(await cards(page), ["Archived 0", "Note 0", "Note 1", "Note 2"]);
  } finally {
    await app.close();
  }
});
