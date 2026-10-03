import { deepEqual, ok } from "node:assert/strict";

/** The commands `tauri::generate_handler!` registers in src-tauri/src/lib.rs, without their module paths. */
async function registered(): Promise<string[]> {
  const rust = await Deno.readTextFile("src-tauri/src/lib.rs");
  const block = rust.match(/generate_handler!\s*\[([^\]]*)\]/);
  ok(block, "lib.rs has a generate_handler! block");
  return block[1]!
    .replace(/\/\/.*$/gm, "")
    .split(",")
    .map((path) => path.trim().split("::").at(-1)!)
    .filter(Boolean)
    .sort();
}

/** The commands src/lib/api.ts invokes. */
async function invoked(): Promise<string[]> {
  const ts = await Deno.readTextFile("src/lib/api.ts");
  return [...ts.matchAll(/\binvoke\s*(?:<.*?>)?\(\s*"([^"]+)"/g)].map((m) => m[1]!).sort();
}

Deno.test("the front end invokes exactly the commands the Rust side registers", async () => {
  const rust = await registered();
  const ts = await invoked();
  ok(rust.length > 0 && ts.length > 0);
  deepEqual(
    { "registered but never invoked": rust.filter((c) => !ts.includes(c)) },
    { "registered but never invoked": [] },
  );
  deepEqual(
    { "invoked but never registered": ts.filter((c) => !rust.includes(c)) },
    { "invoked but never registered": [] },
  );
  deepEqual(ts, [...new Set(ts)], "each command has one wrapper");
});

Deno.test("the test mock types a reply for every command the front end invokes", async () => {
  const mock = await Deno.readTextFile("tests/app.ts");
  const replies = mock.match(/interface Replies \{([^}]*)\}/);
  ok(replies, "tests/app.ts has a Replies interface");
  const typed = [...replies[1]!.matchAll(/^\s*(\w+):/gm)].map((m) => m[1]!).sort();
  deepEqual(typed, await invoked());
});
