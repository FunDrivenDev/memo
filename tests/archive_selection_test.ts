/// <reference lib="dom" />
import { equal } from "node:assert/strict";
import type { Page } from "playwright";
import { browserTest, openApp } from "./app.ts";

const selectedTitle = (page: Page) => page.locator("aside .card.selected .title").textContent();

/** Has the watcher report a change in the folders, and waits for the front end to read the library again. */
async function libraryChanged(page: Page) {
  const before = await page.evaluate(() => (globalThis as unknown as { invoked: string[] }).invoked.length);
  await page.evaluate(() =>
    (globalThis as unknown as { emit: (event: string) => void }).emit("library-changed")
  );
  await page.waitForFunction(
    (before) => (globalThis as unknown as { invoked: string[] }).invoked.slice(before).includes("library"),
    before,
  );
}

browserTest("the archive keeps its selection across a reload while the Comments pane shows", async () => {
  const app = await openApp({ notes: 2, paragraphs: 2, archived: 2 });
  try {
    const { page } = app;
    await page.keyboard.press("Meta+2");
    await page.keyboard.press("ArrowDown");
    equal(await selectedTitle(page), "Archived 1");

    await page.keyboard.press("Meta+3");
    await libraryChanged(page);
    await page.keyboard.press("Meta+2");
    equal(await selectedTitle(page), "Archived 1");
    await page.keyboard.press("Meta+1");
    equal(await selectedTitle(page), "Note 0", "the notes keep theirs too");
  } finally {
    await app.close();
  }
});
