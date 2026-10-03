/// <reference lib="dom" />
import { deepEqual, equal } from "node:assert/strict";
import { browserTest, openApp } from "./app.ts";

const update = { version: "9.9.9", url: "https://github.com/FunDrivenDev/memo/releases/tag/v9.9.9" };

browserTest("no update, no button", async () => {
  const app = await openApp({ notes: 1, paragraphs: 1 });
  try {
    await app.page.locator("button.sync:not(.busy)").waitFor();
    equal(await app.page.locator("button.update").count(), 0);
  } finally {
    await app.close();
  }
});

browserTest("an update installed with Homebrew has memo upgrade itself", async () => {
  const app = await openApp({ notes: 1, paragraphs: 1, update: { ...update, homebrew: true } });
  try {
    await app.page.locator("button.update", { hasText: "Update to 9.9.9" }).click();
    deepEqual(await app.page.evaluate(() => (globalThis as { updated?: unknown }).updated), {
      cmd: "install_update",
    });
  } finally {
    await app.close();
  }
});

browserTest("an update installed otherwise opens the release page, from the palette too", async () => {
  const app = await openApp({ notes: 1, paragraphs: 1, update: { ...update, homebrew: false } });
  try {
    await app.page.locator("button.update").waitFor();
    await app.page.keyboard.press("Meta+k");
    await app.page.keyboard.type(">update memo");
    await app.page.keyboard.press("Enter");
    deepEqual(await app.page.evaluate(() => (globalThis as { updated?: unknown }).updated), {
      cmd: "open_url",
      url: update.url,
    });
  } finally {
    await app.close();
  }
});
