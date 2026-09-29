import { doesNotMatch, equal, match } from "node:assert/strict";

/** What `just` would run for `args`, without running it. */
async function dryRun(...args: string[]): Promise<{ ok: boolean; script: string }> {
  const { success, stderr } = await new Deno.Command("just", { args: ["--dry-run", ...args] }).output();
  return { ok: success, script: new TextDecoder().decode(stderr) };
}

/** The commands scripts/install.sh runs for `args`, each tool replaced by a stub that only logs its call. */
async function installCalls(...args: string[]): Promise<{ ok: boolean; calls: string }> {
  const stubs = await Deno.makeTempDir();
  const log = `${stubs}/calls`;
  try {
    for (const tool of ["mise", "git", "deno", "cargo", "rm"]) {
      await Deno.writeTextFile(`${stubs}/${tool}`, `#!/bin/sh\necho "${tool} $*" >> "${log}"\n`, {
        mode: 0o755,
      });
    }
    const { success } = await new Deno.Command("scripts/install.sh", {
      args,
      env: { PATH: `${stubs}:${Deno.env.get("PATH")}`, MANIFEST: "src-tauri/Cargo.toml" },
    }).output();
    return { ok: success, calls: await Deno.readTextFile(log) };
  } finally {
    await Deno.remove(stubs, { recursive: true });
  }
}

Deno.test("just install delegates to scripts/install.sh, passing the mode", async () => {
  const { ok, script } = await dryRun("install", "frozen");
  equal(ok, true);
  match(script, /scripts\/install\.sh 'frozen'/);
});

Deno.test("just dev and just dev-check go through scripts/dev.sh, which picks a free port", async () => {
  match((await dryRun("dev")).script, /^scripts\/dev\.sh$/m);
  match((await dryRun("dev-check")).script, /^scripts\/dev\.sh check$/m);
  match(await Deno.readTextFile("scripts/dev.sh"), /Deno\.listen\(\{ hostname: "127\.0\.0\.1", port: 0 \}\)/);
});

Deno.test("install.sh sets up a fresh clone without building the app", async () => {
  const { ok, calls } = await installCalls();
  equal(ok, true);
  match(calls, /^mise install/m);
  match(calls, /^deno install/m);
  match(calls, /^cargo fetch/m);
  doesNotMatch(calls, /tauri build/);
});

Deno.test("install.sh frozen installs from the lockfiles, as CI does", async () => {
  const { ok, calls } = await installCalls("frozen");
  equal(ok, true);
  match(calls, /^deno install --quiet --frozen$/m);
  match(calls, /^cargo fetch --locked/m);
});

Deno.test("just app builds the app and installs it in ~/Applications", async () => {
  const { ok, script } = await dryRun("app");
  equal(ok, true);
  match(script, /tauri build/);
  match(script, /cp -R .* ~\/Applications\/memo\.app/);
});
