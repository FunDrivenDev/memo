import { deepEqual, equal, ok } from "node:assert/strict";
import { Actions, type ActionsApi, type Host, type Timers, UNDO_MS } from "../src/lib/actions.ts";
import type { Comment, Library, Note } from "../src/lib/api.ts";

const note = (i: number): Note => ({
  id: `/notes/plans/note-${i}.md`,
  folder: "plans",
  file_name: `note-${i}.md`,
  title: `Note ${i}`,
  excerpt: "",
  modified: 0,
});

const library = (notes: Note[]): Library => ({
  claude_dir: "/claude",
  custom: false,
  folders: [{ name: "plans", path: "/notes/plans" }],
  notes,
  archive: "/notes/archive",
  archive_folders: [],
  archived: [],
  comments: [],
});

const comment: Comment = {
  id: "c1",
  note: note(0).id,
  body: "Look again",
  anchor: { kind: "block", start: 1, end: 1, quote: "Paragraph 0", prefix: "", suffix: "" },
  created: 1,
  updated: 1,
};

/** A clock that only moves when told to. */
class FakeTimers implements Timers {
  now = 0;
  #next = 1;
  #pending = new Map<number, { at: number; run: () => void }>();

  setTimeout(run: () => void, ms: number): number {
    this.#pending.set(this.#next, { at: this.now + ms, run });
    return this.#next++;
  }

  clearTimeout(id: number | undefined) {
    if (id !== undefined) this.#pending.delete(id);
  }

  /** Moves the clock on, running the timers due, then lets their promises settle. */
  async tick(ms: number) {
    this.now += ms;
    for (const [id, { at, run }] of [...this.#pending].sort(([, a], [, b]) => a.at - b.at)) {
      if (at > this.now) continue;
      this.#pending.delete(id);
      run();
    }
    await settle();
  }
}

const settle = () => new Promise((done) => setTimeout(done, 0));

/** The actions over a fake Rust side that logs its calls, and a host that logs what it is told. */
function setup() {
  const calls: string[] = [];
  const told: string[] = [];
  const api: ActionsApi = {
    archive: (
      id,
    ) => (calls.push(`archive ${id}`), Promise.resolve({ id: `/notes/archive/${id}`, library: library([]) })),
    restore: (id) => (calls.push(`restore ${id}`), Promise.resolve({ id, library: library([]) })),
    undo: () => (calls.push("undo"), Promise.resolve({ id: note(0).id, library: library([note(0)]) })),
    trash: (id) => (calls.push(`trash ${id}`), Promise.resolve(library([]))),
    resolveComment: (id) => (calls.push(`resolve ${id}`), Promise.resolve([])),
    restoreComment: (c) => (calls.push(`restore comment ${c.id}`), Promise.resolve([c])),
  };
  const host: Host = {
    library: (next) => told.push(`library of ${next.notes.length}`),
    comments: (next) => told.push(`comments ${next.map((c) => c.id).join(" ")}`.trim()),
    reopen: (target) =>
      told.push("note" in target ? `reopen note ${target.note}` : `reopen comment ${target.comment}`),
  };
  const timers = new FakeTimers();
  return { actions: new Actions(api, host, timers), calls, told, timers };
}

Deno.test("a trash undone before its countdown ends never reaches the Trash, and brings the note back", async () => {
  const { actions, calls, told, timers } = setup();
  actions.trash(note(0));
  ok(actions.hidden(note(0).id));
  equal(actions.toast?.undoable, true);

  await timers.tick(UNDO_MS - 1);
  await actions.undo();
  await timers.tick(UNDO_MS);

  deepEqual(calls, []);
  ok(!actions.hidden(note(0).id));
  deepEqual(told, [`reopen note ${note(0).id}`]);
  equal(actions.toast, null);
});

Deno.test("a trash reaches the Trash when its countdown ends, and u no longer undoes it", async () => {
  const { actions, calls, told, timers } = setup();
  actions.trash(note(0));
  await timers.tick(UNDO_MS);

  deepEqual(calls, [`trash ${note(0).id}`]);
  deepEqual(told, ["library of 0"]);
  ok(!actions.hidden(note(0).id));
  equal(actions.toast, null);
  await actions.undo();
  deepEqual(calls, [`trash ${note(0).id}`]);
});

Deno.test("a new message ends a pending trash at once", async () => {
  const { actions, calls, timers } = setup();
  actions.trash(note(0));
  actions.show("Session started");
  await settle();

  deepEqual(calls, [`trash ${note(0).id}`]);
  equal(actions.toast?.text, "Session started");
  equal(actions.toast?.undoable, false);
  await timers.tick(UNDO_MS);
  deepEqual(calls, [`trash ${note(0).id}`], "trashed once");
});

Deno.test("a second trash ends the first, and u brings back the second only", async () => {
  const { actions, calls, timers } = setup();
  actions.trash(note(0));
  actions.trash(note(1));
  await settle();
  await actions.undo();
  await timers.tick(UNDO_MS);

  deepEqual(calls, [`trash ${note(0).id}`]);
  ok(!actions.hidden(note(1).id));
});

Deno.test("undoing an archive has the Rust side move the note back, and reselects it", async () => {
  const { actions, calls, told } = setup();
  await actions.archive(note(0), false);
  equal(actions.toast?.text, "Archived “Note 0”");
  await actions.undo();

  deepEqual(calls, [`archive ${note(0).id}`, "undo"]);
  deepEqual(told, ["library of 0", "library of 1", `reopen note ${note(0).id}`]);
  equal(actions.toast?.text, "Undone");
  equal(actions.toast?.undoable, false);
});

Deno.test("restoring from the archive is undone the same way", async () => {
  const { actions, calls } = setup();
  await actions.archive(note(0), true);
  equal(actions.toast?.text, "Restored “Note 0” to plans");
  await actions.undo();
  deepEqual(calls, [`restore ${note(0).id}`, "undo"]);
});

Deno.test("undoing a resolve sends the comment back, and reselects it", async () => {
  const { actions, calls, told } = setup();
  equal(await actions.resolve(comment), true);
  equal(actions.toast?.text, "Resolved the comment");
  await actions.undo();

  deepEqual(calls, ["resolve c1", "restore comment c1"]);
  deepEqual(told, ["comments", "comments c1", "reopen comment c1"]);
  equal(actions.toast?.text, "Undone");
});

Deno.test("u undoes only while the message shows", async () => {
  const { actions, calls, timers } = setup();
  await actions.archive(note(0), false);
  await timers.tick(UNDO_MS);
  equal(actions.toast, null);
  await actions.undo();
  deepEqual(calls, [`archive ${note(0).id}`]);
});

Deno.test("a failing action shows its error, with nothing to undo", async () => {
  const failing = new Actions(
    { ...({} as ActionsApi), archive: () => Promise.reject(new Error("no such note")) },
    { library: () => {}, comments: () => {}, reopen: () => {} },
    new FakeTimers(),
  );
  await failing.archive(note(0), false);
  deepEqual(failing.toast, { text: "Error: no such note", error: true, undoable: false });
});
