import { deepEqual, equal } from "node:assert/strict";
import { commandFor, commands, keyName } from "../src/lib/commands.ts";

Deno.test("no two commands share a key, the scopes nesting so that any two are active together", () => {
  const owners = new Map<string, string[]>();
  for (const { id, keys } of commands) {
    for (const key of keys) owners.set(key, [...(owners.get(key) ?? []), id]);
  }
  deepEqual([...owners].filter(([, ids]) => ids.length > 1), []);
});

Deno.test("the README's keys table lists the commands, in their order, with the same keys and wording", async () => {
  const readme = await Deno.readTextFile("README.md");
  const table = readme.split("\n## Keys\n")[1]!.trim().split("\n\n")[0]!;
  const rows = table.split("\n").slice(2).map((row) => {
    const [, keys, help] = row.match(/^\| (.*?) \| (.*) \|$/)!;
    return { keys: keys!.split(" ").map((k) => k.replaceAll("`", "")), help };
  });
  deepEqual(rows, commands.map(({ keys, help }) => ({ keys: [...keys], help })));
});

Deno.test("a key pressed is named as the README writes it", () => {
  const press = (key: string, mods: { metaKey?: boolean; shiftKey?: boolean } = {}) =>
    keyName({ key, metaKey: false, shiftKey: false, ...mods });
  equal(press("k", { metaKey: true }), "⌘K");
  equal(press(",", { metaKey: true }), "⌘,");
  equal(press("Tab", { shiftKey: true }), "⇧⇥");
  equal(press(" ", { shiftKey: true }), "⇧space");
  equal(press("?", { shiftKey: true }), "?");
  equal(press("ArrowUp", { metaKey: true }), "⌘↑");
  equal(press("a"), "a");
  equal(press("A", { shiftKey: true }), "A");
});

Deno.test("the keys that move are only documented, every other key runs its command", () => {
  equal(commandFor("↓"), null);
  equal(commandFor("⌘↑"), null);
  equal(commandFor("A"), null);
  equal(commandFor("⇧⇥")?.id, "folder");
  equal(commandFor("⌘2")?.id, "pane");
  equal(commandFor("t")?.id, "trash");
});
