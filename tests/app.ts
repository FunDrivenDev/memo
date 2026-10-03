import { type Browser, chromium, type Page } from "playwright";
import { createServer, type ViteDevServer } from "vite";
import type * as api from "../src/lib/api.ts";
import type { Comment, Library, Moved, Note, Update } from "../src/lib/api.ts";

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
  /** How many notes the archive holds, dealt among the folders' sections in turn. */
  archived?: number;
  /** The newer memo `check_update` finds, if any. */
  update?: Update | null;
}

/** Stands in for the Rust commands: folders of notes, each a column of numbered paragraphs. */
function mockTauri(
  {
    notes,
    folders = ["plans"],
    paragraphs,
    after = "",
    comments = [],
    added = 0,
    archived = 0,
    update = null,
  }: Fixture,
) {
  const note = (dir: string, name: string, title: string, i: number): Note => {
    const folder = folders[i % folders.length]!;
    return {
      id: `${dir}/${folder}/${name}-${i}.md`,
      folder,
      file_name: `${name}-${i}.md`,
      title,
      excerpt: "",
      modified: 0,
    };
  };
  const all = Array.from({ length: notes + added }, (_, i) => note("/notes", "note", `Note ${i}`, i));
  // The notes listed, those the watcher has not seen yet, and the archived ones: archive, restore and undo move
  // notes between the first and the last.
  const listed = all.slice(0, notes);
  const unseen = all.slice(notes);
  const archive = Array.from(
    { length: archived },
    (_, i) => note("/notes/archive", "archived", `Archived ${i}`, i),
  );
  const byTitle = (a: Note, b: Note) => a.title.localeCompare(b.title, "en", { numeric: true });
  /** Moves the note `id` into the archive or out of it, and returns its new path. */
  const move = (id: string, into: "archive" | "folders"): string => {
    const [from, to, dir] = into === "archive"
      ? [listed, archive, "/notes/archive"]
      : [archive, listed, "/notes"];
    const at = from.findIndex((n) => n.id === id);
    if (at < 0) throw new Error(`${id} is not ${into === "archive" ? "in the folders" : "archived"}`);
    const [moved] = from.splice(at, 1);
    const path = `${dir}/${moved!.folder}/${moved!.file_name}`;
    to.push({ ...moved!, id: path });
    to.sort(byTitle);
    return path;
  };
  // The last archive or restore, which `undo` moves back.
  let last: { id: string; back: "archive" | "folders" } | null = null;
  const moved = (id: string, into: "archive" | "folders"): Moved => {
    const to = move(id, into);
    last = { id: to, back: into === "archive" ? "folders" : "archive" };
    return { id: to, library: library() };
  };
  const html = Array.from(
    { length: paragraphs },
    (_, i) => `<p data-sourcepos="${2 * i + 1}:1-${2 * i + 1}:20">Paragraph ${i}</p>`,
  ).join("") + after;
  // The comments as the Rust side keeps them, for the tests to read back.
  const saved = comments;
  const library = (): Library => ({
    claude_dir: "/claude",
    custom: false,
    folders: folders.map((name) => ({ name, path: `/notes/${name}` })),
    notes: listed,
    archive: "/notes/archive",
    archive_folders: folders.filter((name) => archive.some((n) => n.folder === name)).map((name) => ({
      name,
      path: `/notes/archive/${name}`,
    })),
    archived: archive,
    comments: saved,
  });
  let ids = 0;
  const handlers: Handlers = {
    library,
    refresh: () => {
      listed.push(...unseen.splice(0));
      listed.sort(byTitle);
      return library();
    },
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
    archive: ({ id }) => moved(id as string, "archive"),
    restore: ({ id }) => moved(id as string, "folders"),
    undo: () => {
      if (!last) throw new Error("Nothing to undo");
      const { id, back } = last;
      last = null;
      return { id: move(id, back), library: library() };
    },
    trash: ({ id }) => {
      listed.splice(listed.findIndex((n) => n.id === id), 1);
      return library();
    },
    add_comment: (args) => {
      const { note, body, anchor } = args as Pick<Comment, "note" | "body" | "anchor">;
      saved.push({ id: `c${++ids}`, note, body, anchor, created: ids, updated: ids });
      return saved;
    },
    edit_comment: ({ id, body }) => {
      Object.assign(saved.find((c) => c.id === id)!, { body });
      return saved;
    },
    resolve_comment: ({ id }) => {
      saved.splice(saved.findIndex((c) => c.id === id), 1);
      return saved;
    },
    restore_comment: ({ comment }) => {
      saved.push(comment as Comment);
      return saved;
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
    // Fire and forget: they only act on the Mac, and return nothing.
    reveal: () => {},
    edit: () => {},
    create_folder: () => {},
    open_privacy_settings: () => {},
  };
  const copy = <T>(value: T): T => JSON.parse(JSON.stringify(value));
  // The front end's callbacks, by the id `transformCallback` gives them, and the events they listen to.
  const callbacks = new Map<number, (event: unknown) => void>();
  const listeners: { event: string; id: number; handler: number }[] = [];
  let listened = 0;
  // The commands called, in order, and those the mock has no reply for, for the tests to read back.
  const invoked: string[] = [];
  const unmocked: string[] = [];
  Object.assign(globalThis, {
    savedComments: saved,
    invoked,
    unmocked,
    /** Emits `event` to the front end, as the Rust side does. */
    emit: (event: string, payload: unknown = null) => {
      for (const { id, handler } of listeners.filter((l) => l.event === event)) {
        callbacks.get(handler)?.({ event, id, payload });
      }
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
    __TAURI_INTERNALS__: {
      invoke: (cmd: string, args: Record<string, unknown>) => {
        invoked.push(cmd);
        // `listen` and its unlisten, for the events the front end subscribes to.
        if (cmd === "plugin:event|listen") {
          const id = ++listened;
          listeners.push({ event: args.event as string, id, handler: args.handler as number });
          return Promise.resolve(id);
        }
        if (cmd === "plugin:event|unlisten") {
          const at = listeners.findIndex((l) => l.id === args.eventId);
          if (at >= 0) listeners.splice(at, 1);
          return Promise.resolve();
        }
        const handler = handlers[cmd as keyof Replies];
        if (!handler) {
          unmocked.push(cmd);
          return Promise.reject(new Error(`unmocked command: ${cmd}`));
        }
        try {
          // Through JSON both ways, as Tauri sends them: the front end's objects may be Svelte proxies, and the
          // mock's state must not leak into them.
          const reply = handler(copy(args ?? {}));
          return Promise.resolve(reply === undefined ? reply : copy(reply));
        } catch (e) {
          return Promise.reject(e);
        }
      },
      transformCallback: (callback: (event: unknown) => void) => {
        callbacks.set(callbacks.size + 1, callback);
        return callbacks.size;
      },
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
      // The front end may swallow the rejection: a test still fails on a command the mock does not answer.
      const unmocked = await page.evaluate(() => (globalThis as unknown as { unmocked: string[] }).unmocked)
        .catch(() => []);
      await browser.close();
      await server.close();
      if (unmocked.length) throw new Error(`unmocked commands: ${unmocked.join(", ")}`);
    },
  };
}

/** A test that opens the app: Playwright's browser and Vite's server outlive Deno's per-test leak checks. */
export const browserTest = (name: string, fn: () => Promise<void>) =>
  Deno.test({ name, fn, sanitizeOps: false, sanitizeResources: false });
