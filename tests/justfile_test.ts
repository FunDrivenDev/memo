import { doesNotMatch, equal, match } from "node:assert/strict";

/** What `just` would run for `args`, without running it. */
async function dryRun(...args: string[]): Promise<{ ok: boolean; script: string }> {
  const { success, stderr } = await new Deno.Command("just", { args: ["--dry-run", ...args] }).output();
  return { ok: success, script: new TextDecoder().decode(stderr) };
}

Deno.test("just install sets up a fresh clone without building the app", async () => {
  const { ok, script } = await dryRun("install");
  equal(ok, true);
  match(script, /mise install/);
  match(script, /deno install/);
  match(script, /cargo fetch/);
  doesNotMatch(script, /tauri build/);
});

Deno.test("just install frozen installs from the lockfiles, as CI does", async () => {
  const { script } = await dryRun("install", "frozen");
  match(script, /deno install --quiet --frozen/);
  match(script, /cargo fetch --locked/);
});

Deno.test("just app builds the app and installs it in ~/Applications", async () => {
  const { ok, script } = await dryRun("app");
  equal(ok, true);
  match(script, /tauri build/);
  match(script, /cp -R .* ~\/Applications\/memo\.app/);
});
