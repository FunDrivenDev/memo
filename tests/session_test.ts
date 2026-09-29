/// <reference lib="dom" />
import { deepEqual, equal } from "node:assert/strict";
import { browserTest, openApp } from "./app.ts";

const options = ".modal [role=option]";

browserTest(
  "s starts a session in a folder picked among the usual ones, or found by typing part of it",
  async () => {
    const app = await openApp({ notes: 1, paragraphs: 3 });
    try {
      const { page } = app;
      await page.keyboard.press("s");
      await page.locator(".modal textarea").waitFor();
      await page.keyboard.press("Tab");
      // The usual folders but the one filled in.
      deepEqual(await page.locator(options).allTextContents(), ["~/Code/site", "~/Notes"]);
      await page.keyboard.press("Meta+a");
      await page.keyboard.type("note");
      deepEqual(await page.locator(options).allTextContents(), ["~/Notes"]);
      await page.keyboard.press("ArrowDown");
      await page.keyboard.press("Enter");
      await page.keyboard.press("Meta+Enter");
      await page.locator(".modal").waitFor({ state: "detached" });
      const session = await page.evaluate(() => (globalThis as unknown as { session: unknown }).session);
      deepEqual(session, { workdir: "~/Notes", prompt: "Read the plan." });
    } finally {
      await app.close();
    }
  },
);

browserTest("c no longer starts a session", async () => {
  const app = await openApp({ notes: 1, paragraphs: 3 });
  try {
    await app.page.keyboard.press("c");
    equal(await app.page.locator(".modal").count(), 0);
  } finally {
    await app.close();
  }
});
