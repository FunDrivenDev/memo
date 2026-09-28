/// <reference lib="dom" />
import { equal, ok } from "node:assert/strict";
import { browserTest, openApp } from "./app.ts";

const code = `<pre data-sourcepos="3:1-6:3"><code class="language-sh"><span class="hl-comment">cd ~</span>
ls -la
</code></pre>`;

browserTest("the copy button of a code block copies its code, without the trailing newline", async () => {
  const app = await openApp({ notes: 1, paragraphs: 1, after: code });
  try {
    const button = app.page.locator("article pre button.copy");
    equal(await button.count(), 1);
    ok(
      !(await button.isVisible()) || Number(await button.evaluate((b) => getComputedStyle(b).opacity)) === 0,
    );
    await app.page.locator("article pre").hover();
    await button.click();
    equal(await app.page.evaluate(() => (globalThis as { copied?: string }).copied), "cd ~\nls -la");
    ok(await button.evaluate((b) => b.classList.contains("copied")), "the button shows the copy is done");
  } finally {
    await app.close();
  }
});

browserTest("a code block scrolls sideways under its copy button", async () => {
  const long = `<pre data-sourcepos="3:1-5:3"><code>${"x".repeat(2000)}\n</code></pre>`;
  const app = await openApp({ notes: 1, paragraphs: 1, after: long });
  try {
    const scroller = app.page.locator("article pre code");
    const button = app.page.locator("article pre button.copy");
    const before = await button.boundingBox();
    await scroller.evaluate((el) => (el.scrollLeft = 500));
    ok(await scroller.evaluate((el) => el.scrollLeft) > 0, "the code scrolled");
    equal((await button.boundingBox())?.x, before?.x);
  } finally {
    await app.close();
  }
});
