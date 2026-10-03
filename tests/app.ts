import { type Browser, chromium, type Page } from "playwright";
import { createServer, type ViteDevServer } from "vite";
import type * as api from "../src/lib/api.ts";
import type { Comment, Library, Note, Update } from "../src/lib/api.ts";

type Reply<F extends (...args: never[]) => Promise<unknown>> = Awaited<ReturnType<F>>;

/** What each Rust command answers, as its wrapper in src/lib/api.ts types it. */
interface Replies {
  library: Reply<typeof api.library>;
  refresh: Reply<typeof api.refresh>;
  settings: Reply<typeof api.settings>;
  save_settings: Reply<typeof api.saveSettings>;
  choose_folder: Reply<typeof api.chooseFolder>;
  inspect_folder: Reply<typeof api.inspectFolder>;
  create_folder: Reply<typeof api.createFolder>;
  complete_folder: Reply<typeof api.completeFolder>;
  open_privacy_settings: Reply<typeof api.openPrivacySettings>;
  render: Reply<typeof api.render>;
  search: Reply<typeof api.search>;
  archive: Reply<typeof api.archive>;
  restore: Reply<typeof api.restore>;
  undo: Reply<typeof api.undo>;
  trash: Reply<typeof api.trash>;
  add_comment: Reply<typeof api.addComment>;
  edit_comment: Reply<typeof api.editComment>;
  resolve_comment: Reply<typeof api.resolveComment>;
  restore_comment: Reply<typeof api.restoreComment>;
  session_defaults: Reply<typeof api.sessionDefaults>;
  start_session: Reply<typeof api.startSession>;
  check_update: Reply<typeof api.checkUpdate>;
  install_update: Reply<typeof api.installUpdate>;
  reveal: Reply<typeof api.reveal>;
  edit: Reply<typeof api.edit>;
  open_url: Reply<typeof api.openUrl>;
  copy: Reply<typeof api.copy>;
}

/** How the mock answers a command, given its arguments. */
type Handlers = { [K in keyof Replies]?: (args: Record<string, unknown>) => Replies[K] };

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
  comments?: Comment[];
  /** How many notes land in the folder after launch, unseen by the watcher: only `refresh` lists them. */
  added?: number;
  /** The newer memo `check_update` finds, if any. */
  update?: Update | null;
}

/** Stands in for the Rust commands: folders of notes, each a column of numbered paragraphs. */
function mockTauri(
  { notes, folders = ["plans"], paragraphs, after = "", comments = [], added = 0, update = null }: Fixture,
) {
  const all: Note[] = Array.from({ length: notes + added }, (_, i) => ({
    id: `/notes/${folders[i % folders.length]}/note-${i}.md`,
    folder: folders[i % folders.length]!,
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
  // The comments as the Rust side keeps them, for the tests to read back.
  const saved = comments;
  const library = (notes: Note[]): Library => ({
    claude_dir: "/claude",
    custom: false,
    folders: folders.map((name) => ({ name, path: `/notes/${name}` })),
    notes,
    archive: "/notes/archive",
    archive_folders: [],
    archived: [],
    comments: saved,
  });
  // Through JSON, as Tauri sends them: the front end's objects may be Svelte proxies.
  const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value));
  let ids = 0;
  const handlers: Handlers = {
    library: () => library(list),
    refresh: () => library(all),
    render: () => ({ html, words: paragraphs * 2 }),
    // Every search finds the second comment, when there is one.
    search: () => ({
      notes: [],
      comments: saved.slice(1, 2).map(({ id, note, body }) => ({
        id,
        note,
        score: 1,
        snippets: [{ line: 1, text: body, indices: [0] }],
      })),
    }),
    trash: ({ id }) => library(list.filter((n) => n.id !== id)),
    add_comment: (args) => {
      const { note, body, anchor } = copy(args) as Pick<Comment, "note" | "body" | "anchor">;
      saved.push({ id: `c${++ids}`, note, body, anchor, created: ids, updated: ids });
      return copy(saved);
    },
    edit_comment: ({ id, body }) => {
      Object.assign(saved.find((c) => c.id === id)!, { body });
      return copy(saved);
    },
    resolve_comment: ({ id }) => {
      saved.splice(saved.findIndex((c) => c.id === id), 1);
      return copy(saved);
    },
    restore_comment: (args) => {
      saved.push(copy(args.comment as Comment));
      return copy(saved);
    },
    session_defaults: () => ({
      workdir: "~/Code/memo",
      prompt: "Read the plan.",
      recent: ["~/Code/memo", "~/Code/site", "~/Notes"],
    }),
    complete_folder: () => ({ folders: [], denied: null }),
    check_update: () => update,
    copy: ({ text }) => void Object.assign(globalThis, { copied: text }),
    start_session: (args) => void Object.assign(globalThis, { session: args }),
    install_update: (args) => void Object.assign(globalThis, { updated: { cmd: "install_update", ...args } }),
    open_url: (args) => void Object.assign(globalThis, { updated: { cmd: "open_url", ...args } }),
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
        const handler = handlers[cmd as keyof Replies];
        return Promise.resolve(handler ? handler(args) : null);
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
