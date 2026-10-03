import { deepEqual, equal } from "node:assert/strict";

/** What scripts/changes.sh decides CI checks for `paths`, or for everything with `--all`. */
async function changes(paths: string[], ...args: string[]): Promise<Record<string, string>> {
  const child = new Deno.Command("scripts/changes.sh", { args, stdin: "piped", stdout: "piped" }).spawn();
  const writer = child.stdin.getWriter();
  await writer.write(new TextEncoder().encode(paths.map((p) => `${p}\n`).join("")));
  await writer.close();
  const { success, stdout } = await child.output();
  equal(success, true);
  return Object.fromEntries(new TextDecoder().decode(stdout).trim().split("\n").map((l) => l.split("=", 2)));
}

Deno.test("a change to the docs lints the Markdown and the spelling, and scans for secrets", async () => {
  deepEqual(await changes(["README.md", "docs/decisions/0001-one-key-one-action.md"]), {
    check: "false",
    lint: "spelling markdown",
    audit: "secrets",
    cask: "false",
  });
});

Deno.test("a change to the code runs the whole check, without the cask", async () => {
  const out = await changes(["src/App.svelte", "src-tauri/src/lib.rs"]);
  equal(out.check, "true");
  equal(out.cask, "false");
  equal(out.audit, "secrets");
});

Deno.test("a change to a lockfile audits its dependencies", async () => {
  equal((await changes(["src-tauri/Cargo.lock"])).audit, "secrets rust");
  equal((await changes(["deno.lock"])).audit, "secrets web");
});

Deno.test("a change to the cask checks it on macOS only", async () => {
  deepEqual(await changes(["packaging/memo.rb"]), {
    check: "false",
    lint: "spelling",
    audit: "secrets",
    cask: "true",
  });
});

Deno.test("a change to another workflow lints and audits the workflows", async () => {
  const out = await changes([".github/workflows/release.yml"]);
  equal(out.check, "false");
  equal(out.lint, "spelling workflows");
  equal(out.audit, "secrets workflows");
});

Deno.test("CI's own workflow, the tools, an unknown path or --all check everything", async () => {
  const all = await changes([], "--all");
  deepEqual(all, {
    check: "true",
    lint: "markdown toml just shell workflows spelling",
    audit: "secrets rust web workflows",
    cask: "true",
  });
  for (const path of [".github/workflows/ci.yml", "mise.toml", "Justfile", "somewhere/new.txt"]) {
    deepEqual(await changes([path]), all, path);
  }
});
