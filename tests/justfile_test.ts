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

Deno.test("just publish --locally builds the app and installs it in ~/Applications", async () => {
  const publish = await dryRun("publish", "--locally");
  equal(publish.ok, true);
  match(publish.script, /^just _install-locally$/m);
  const { script } = await dryRun("_install-locally");
  match(script, /tauri build/);
  match(script, /cp -R .* ~\/Applications\/memo\.app/);
});

Deno.test("just publish <version> starts the Release workflow", async () => {
  const publish = await dryRun("publish", "0.3.0");
  equal(publish.ok, true);
  match(publish.script, /^just _release 0\.3\.0$/m);
  const { script } = await dryRun("_release", "0.3.0");
  match(script, /^gh workflow run release -f version=0\.3\.0$/m);
});

Deno.test("just publish refuses to run without a version or --locally", async () => {
  const { ok, script } = await dryRun("publish");
  equal(ok, false);
  match(script, /publish needs a version or --locally/);
});
