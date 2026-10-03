/// <reference lib="dom" />
import { deepEqual, equal } from "node:assert/strict";
import type { Page } from "playwright";
import { browserTest, openApp } from "./app.ts";

/** The titles of the notes the sidebar lists. */
const cards = (page: Page) => page.locator("aside .card .title").allTextContents();
const selectedTitle = (page: Page) => page.locator("aside .card.selected .title").textContent();
const selectedFolder = (page: Page) => page.locator("aside h2 button.selected .name").textContent();

browserTest("← folds the folder of the note onto its header, → unfolds it into its first note", async () => {
  const app = await openApp({ notes: 4, folders: ["plans", "reports"], paragraphs: 3 });
  try {
    const { page } = app;
    deepEqual(await cards(page), ["Note 0", "Note 2", "Note 1", "Note 3"]);
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("ArrowLeft");
    deepEqual(await cards(page), ["Note 1", "Note 3"], "the plans are folded");
    equal(await selectedFolder(page), "plans");
    equal(await page.locator("main .blank h1, .blank h1").textContent(), "plans is folded");

    await page.keyboard.press("ArrowDown");
    equal(await selectedTitle(page), "Note 1", "↓ goes from the folded folder to the next note");
    await page.keyboard.press("ArrowUp");
    equal(await selectedFolder(page), "plans", "↑ stops on the folded folder");

    await page.keyboard.press("ArrowRight");
    deepEqual(await cards(page), ["Note 0", "Note 2", "Note 1", "Note 3"]);
    equal(await selectedTitle(page), "Note 0");
  } finally {
    await app.close();
  }
});

browserTest("a click on a header folds it, and the folded folders stay so after a reload", async () => {
  const app = await openApp({ notes: 4, folders: ["plans", "reports"], paragraphs: 3 });
  try {
    const { page } = app;
    await page.locator("aside h2 button", { hasText: "reports" }).click();
    deepEqual(await cards(page), ["Note 0", "Note 2"]);
    equal(await selectedTitle(page), "Note 0", "folding another folder keeps the selection");

    await page.reload();
    await page.locator("article p").first().waitFor();
    deepEqual(await cards(page), ["Note 0", "Note 2"]);

    await page.locator("aside h2 button", { hasText: "reports" }).click();
    deepEqual(await cards(page), ["Note 0", "Note 2", "Note 1", "Note 3"]);
  } finally {
    await app.close();
  }
});

browserTest("⇥ stops on a folded folder", async () => {
  const app = await openApp({ notes: 4, folders: ["plans", "reports"], paragraphs: 3 });
  try {
    const { page } = app;
    await page.locator("aside h2 button", { hasText: "reports" }).click();
    await page.keyboard.press("Tab");
    equal(await selectedFolder(page), "reports");
    await page.keyboard.press("Tab");
    equal(await selectedTitle(page), "Note 0");
  } finally {
    await app.close();
  }
});
