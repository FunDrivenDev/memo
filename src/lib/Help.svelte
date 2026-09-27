<script lang="ts">
  import type { Folder } from "./api";
  import Modal from "./Modal.svelte";

  let { folders, archive, onClose }: { folders: Folder[]; archive: string; onClose: () => void } = $props();

  const keys: [string, string][] = [
    ["⌘K  /", "Command palette: search titles and content"],
    ["j  k  ↓  ↑", "Next or previous note"],
    ["⇥  ⇧⇥", "Next or previous folder"],
    ["g  G", "First or last note"],
    ["space  ⇧space", "Page down or up in the note"],
    ["J  K", "Scroll the note"],
    ["s", "Start a Claude Code session from the note"],
    ["a", "Archive the note; in the archive, restore it"],
    ["u", "Undo the archive or restore, while its message shows"],
    ["⇧A", "Switch between the notes and the archive"],
    ["d  ⌫", "Move the note to the Trash"],
    ["e", "Open the note in the default editor"],
    ["o", "Reveal the note in Finder"],
    ["r", "Reload"],
    ["⌘,", "Settings: folders and archive"],
    ["?", "This help"],
  ];
</script>

<Modal title="Keyboard" {onClose}>
  <dl>
    {#each keys as [key, what] (key)}
      <dt>
        {#each key.split("  ") as k (k)}<kbd>{k}</kbd>{/each}
      </dt>
      <dd>{what}</dd>
    {/each}
  </dl>
  <h4>Search</h4>
  <p>
    Every word must match, fuzzily, in the title or on one line. As in fzf: <code>'word</code> matches exactly,
    <code>^word</code> at the start, <code>word$</code> at the end, <code>!word</code> excludes. It covers the notes or
    the archive, not both: <kbd>⇥</kbd> switches.
  </p>
  <h4>Folders</h4>
  <p>
    Claude Code's plans folder unless others are chosen in the settings (<kbd>⌘,</kbd>). Archived notes move to
    <code>{archive}/&lt;folder&gt;</code>.
  </p>
  <ul>
    {#each folders as folder (folder.path)}
      <li><code>{folder.path}</code></li>
    {/each}
  </ul>
</Modal>

<style>
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 0;
  }

  dt {
    display: flex;
    gap: 4px;
    justify-content: flex-end;
  }

  dd {
    margin: 0;
    color: var(--subtext);
  }

  h4 {
    margin: 18px 0 6px;
    font-size: 13px;
  }

  p,
  ul {
    margin: 0 0 6px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--subtext);
  }

  code {
    font-family: var(--mono);
    font-size: 11.5px;
  }
</style>
