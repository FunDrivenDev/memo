/// <reference lib="dom" />
import { ok } from "node:assert/strict";
import type { Page } from "playwright";
import { browserTest, openApp } from "./app.ts";

/** The note's column as a share of the note pane's width. */
const share = (page: Page) =>
  page.locator("main").evaluate((main) => main.querySelector("article")!.offsetWidth / main.clientWidth);

const near = (actual: number, expected: number) => Math.abs(actual - expected) < 0.01;

browserTest("the note takes two thirds of its pane, and keeps them when the window resizes", async () => {
  const app = await openApp({ notes: 1, paragraphs: 5 }, { width: 1200, height: 700 });
  try {
    const before = await share(app.page);
    ok(near(before, 0.66), `66% of the pane, not ${(before * 100).toFixed(1)}%`);
    await app.page.setViewportSize({ width: 1800, height: 700 });
    const after = await share(app.page);
    ok(near(after, 0.66), `66% of the wider pane, not ${(after * 100).toFixed(1)}%`);
  } finally {
    await app.close();
  }
});

browserTest("the note's width comes from --note-width", async () => {
  const app = await openApp({ notes: 1, paragraphs: 5 }, { width: 1800, height: 700 });
  try {
    await app.page.evaluate(() => document.documentElement.style.setProperty("--note-width", "50%"));
    const width = await share(app.page);
    ok(near(width, 0.5), `50% of the pane, not ${(width * 100).toFixed(1)}%`);
  } finally {
    await app.close();
  }
});

browserTest("in the narrowest window the note keeps a readable width rather than two thirds", async () => {
  const app = await openApp({ notes: 1, paragraphs: 5 }, { width: 760, height: 700 });
  try {
    const width = await share(app.page);
    ok(width > 0.9, `most of the pane, not ${(width * 100).toFixed(1)}%`);
  } finally {
    await app.close();
  }
});
