import { type Browser, chromium, type Page } from "playwright";
import { createServer, type ViteDevServer } from "vite";

export interface Fixture {
  /** How many notes the one folder holds. */
  notes: number;
  /** How many paragraphs each note renders to. */
  paragraphs: number;
  /** HTML rendered after the paragraphs. */
  after?: string;
}

/** Stands in for the Rust commands: one folder of notes, each a column of numbered paragraphs. */
function mockTauri({ notes, paragraphs, after = "" }: Fixture) {
  const list = Array.from({ length: notes }, (_, i) => ({
    id: `/notes/plans/note-${i}.md`,
    folder: "plans",
    file_name: `note-${i}.md`,
    title: `Note ${i}`,
    excerpt: "",
    modified: 0,
  }));
  const html = Array.from(
    { length: paragraphs },
    (_, i) => `<p data-sourcepos="${2 * i + 1}:1-${2 * i + 1}:20">Paragraph ${i}</p>`,
  ).join("") + after;
  const replies: Record<string, unknown> = {
    library: {
      claude_dir: "/claude",
      custom: false,
      folders: [{ name: "plans", path: "/notes/plans" }],
      notes: list,
      archive: "/notes/archive",
      archive_folders: [],
      archived: [],
    },
    render: { html, words: paragraphs * 2 },
  };
  let callbacks = 0;
  Object.assign(globalThis, {
    __TAURI_INTERNALS__: {
      invoke: (cmd: string, args: Record<string, unknown>) => {
        if (cmd === "copy") Object.assign(globalThis, { copied: args.text });
        return Promise.resolve(replies[cmd] ?? null);
      },
      transformCallback: () => ++callbacks,
    },
  });
}

export interface App {
  page: Page;
  close(): Promise<void>;
}

/** Serves the front end with Vite and opens it in headless Chromium, the Rust side mocked by `fixture`. */
export async function openApp(
  fixture: Fixture,
  viewport = { width: 1200, height: 700 },
): Promise<App> {
  const server: ViteDevServer = await createServer({
    logLevel: "silent",
    server: { port: 0, strictPort: false },
  });
  await server.listen();
  const browser: Browser = await chromium.launch();
  const page = await browser.newPage({ viewport });
  await page.addInitScript(mockTauri, fixture);
  await page.goto(server.resolvedUrls!.local[0]!);
  await page.locator("article p").first().waitFor();
  return {
    page,
    async close() {
      await browser.close();
      await server.close();
    },
  };
}

/** A test that opens the app: Playwright's browser and Vite's server outlive Deno's per-test leak checks. */
export const browserTest = (name: string, fn: () => Promise<void>) =>
  Deno.test({ name, fn, sanitizeOps: false, sanitizeResources: false });
