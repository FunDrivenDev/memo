/// <reference lib="dom" />
import { equal } from "node:assert/strict";
import { browserTest, openApp } from "./app.ts";

browserTest("⌘R lists a note the watcher missed, the icon spinning then showing a check", async () => {
  const app = await openApp({ notes: 2, paragraphs: 1, added: 1 });
  try {
    const notes = app.page.locator("nav .card");
    await app.page.locator("button.sync:not(.busy)").waitFor();
    equal(await notes.count(), 2);
    await app.page.keyboard.press("Meta+r");
    await app.page.locator("button.sync.busy").waitFor();
    await app.page.locator("button.sync.done").waitFor();
    equal(await notes.count(), 3);
    await app.page.locator("button.sync.idle").waitFor();
  } finally {
    await app.close();
  }
});

browserTest("a click on the icon reloads the folders too", async () => {
  const app = await openApp({ notes: 1, paragraphs: 1, added: 2 });
  try {
    await app.page.locator("button.sync").click();
    await app.page.locator("button.sync.done").waitFor();
    equal(await app.page.locator("nav .card").count(), 3);
  } finally {
    await app.close();
  }
});
