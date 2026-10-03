import { type Browser, chromium, type Page } from "playwright";
import { createServer, type ViteDevServer } from "vite";

export interface Fixture {
  /** How many notes the folders hold, dealt among them in turn. */
  notes: number;
  /** The folders' names, `plans` alone by default. */
  folders?: string[];
  /** How many paragraphs each note renders to. */
  paragraphs: number;
  /** HTML rendered after the paragraphs. */
  after?: string;
  /** The comments saved already, as the Rust side keeps them. */
  comments?: unknown[];
  /** How many notes land in the folder after launch, unseen by the watcher: only `refresh` lists them. */
  added?: number;
  /** The newer memo `check_update` finds, if any. */
  update?: unknown;
}

/** Stands in for the Rust commands: folders of notes, each a column of numbered paragraphs. */
function mockTauri(
  { notes, folders = ["plans"], paragraphs, after = "", comments = [], added = 0, update = null }: Fixture,
) {
  const all = Array.from({ length: notes + added }, (_, i) => ({
    id: `/notes/${folders[i % folders.length]}/note-${i}.md`,
    folder: folders[i % folders.length],
    file_name: `note-${i}.md`,
    title: `Note ${i}`,
    excerpt: "",
    modified: 0,
  }));
  const list = all.slice(0, notes);
  const html = Array.from(
    { length: paragraphs },
    (_, i) => `<p data-sourcepos="${2 * i + 1}:1-${2 * i + 1}:20">Paragraph ${i}</p>`,
  ).join("") + after;
  const library = (notes: unknown[]) => ({
    claude_dir: "/claude",
    custom: false,
    folders: folders.map((name) => ({ name, path: `/notes/${name}` })),
    notes,
    archive: "/notes/archive",
    archive_folders: [],
    archived: [],
    comments,
  });
  const replies: Record<string, unknown> = {
    library: library(list),
    refresh: library(all),
    render: { html, words: paragraphs * 2 },
    // Every search finds the second comment, when there is one.
    search: {
      notes: [],
      comments: (comments as { id: string; note: string; body: string }[]).slice(1, 2).map((
        { id, note, body },
      ) => ({
        id,
        note,
        score: 1,
        snippets: [{ line: 1, text: body, indices: [0] }],
      })),
    },
    session_defaults: {
      workdir: "~/Code/memo",
      prompt: "Read the plan.",
      recent: ["~/Code/memo", "~/Code/site", "~/Notes"],
    },
    complete_folder: { folders: [], denied: null },
    check_update: update,
  };
  // The comments as the Rust side keeps them, for the tests to read back.
  const saved = comments as Record<string, unknown>[];
  let ids = 0;
  const commentCommands: Record<string, (args: Record<string, unknown>) => void> = {
    add_comment: ({ note, body, anchor }) =>
      saved.push({ id: `c${++ids}`, note, body, anchor, created: ids, updated: ids }),
    edit_comment: ({ id, body }) => Object.assign(saved.find((c) => c.id === id)!, { body }),
    resolve_comment: ({ id }) => saved.splice(saved.findIndex((c) => c.id === id), 1),
    restore_comment: ({ comment }) => saved.push(comment as Record<string, unknown>),
  };
  let callbacks = 0;
  // The commands called, in order, for the tests to read back.
  const invoked: string[] = [];
  Object.assign(globalThis, {
    savedComments: saved,
    invoked,
    __TAURI_INTERNALS__: {
      invoke: (cmd: string, args: Record<string, unknown>) => {
        invoked.push(cmd);
        if (cmd === "copy") Object.assign(globalThis, { copied: args.text });
        if (cmd === "start_session") Object.assign(globalThis, { session: args });
        if (cmd === "install_update" || cmd === "open_url") {
          Object.assign(globalThis, { updated: { cmd, ...args } });
        }
        const comment = commentCommands[cmd];
        if (comment) {
          // Through JSON, as Tauri sends them: the front end's objects may be Svelte proxies.
          comment(JSON.parse(JSON.stringify(args)));
          return Promise.resolve(JSON.parse(JSON.stringify(saved)));
        }
        if (cmd === "trash") return Promise.resolve(library(list.filter((n) => n.id !== args.id)));
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
