/// <reference lib="dom" />
import { equal, ok } from "node:assert/strict";
import type { Locator } from "playwright";
import { browserTest, openApp } from "./app.ts";

/** Whether the element ends above the bottom of the window. */
async function endsInWindow(el: Locator): Promise<boolean> {
  const box = await el.boundingBox();
  return !!box && box.y + box.height <= el.page().viewportSize()!.height;
}

for (const notes of [1, 60]) {
  browserTest(`a long note scrolls inside the window, beside ${notes} notes`, async () => {
    const app = await openApp({ notes, paragraphs: 200 });
    try {
      const main = app.page.locator("main");
      ok(await endsInWindow(main), "the note pane ends inside the window");
      await main.hover();
      await app.page.mouse.wheel(0, 3000);
      await app.page.waitForTimeout(500);
      ok(await main.evaluate((el) => el.scrollTop) > 0, "the note scrolled");
    } finally {
      await app.close();
    }
  });

  browserTest(`reading with ↓ keeps the current item in the window, beside ${notes} notes`, async () => {
    const app = await openApp({ notes, paragraphs: 200 });
    try {
      await app.page.keyboard.press("Enter");
      for (let i = 0; i < 60; i++) await app.page.keyboard.press("ArrowDown");
      await app.page.waitForTimeout(600);
      const current = app.page.locator("article .current");
      equal(await current.textContent(), "Paragraph 60");
      ok(await endsInWindow(current), "the current item shows");
    } finally {
      await app.close();
    }
  });
}

browserTest("↓ through a list longer than the window keeps the selected note in the window", async () => {
  const app = await openApp({ notes: 60, paragraphs: 5 });
  try {
    for (let i = 0; i < 50; i++) await app.page.keyboard.press("ArrowDown");
    await app.page.waitForTimeout(300);
    const selected = app.page.locator("aside .card.selected");
    equal(await selected.locator(".title").textContent(), "Note 50");
    ok(await endsInWindow(selected), "the selected note shows");
  } finally {
    await app.close();
  }
});
