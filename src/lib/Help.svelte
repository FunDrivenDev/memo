<script lang="ts">
  import type { Folder } from "./api";
  import { commands } from "./commands";
  import Modal from "./Modal.svelte";

  let { folders, archive, onClose }: { folders: Folder[]; archive: string; onClose: () => void } = $props();

</script>

<Modal title="Keyboard" {onClose}>
  <dl>
    {#each commands as { id, keys, help } (id)}
      <dt>
        {#each keys as key (key)}<kbd>{key}</kbd>{/each}
      </dt>
      <!-- The keys the text names, in backticks as in the README, show as keys. -->
      <dd>
        {#each help.split("`") as part, i (i)}{#if i % 2}<kbd>{part}</kbd>{:else}{part}{/if}{/each}
      </dd>
    {/each}
  </dl>
  <h4>Comments</h4>
  <p>
    Kept beside the note, which stays as it is. A comment goes under the passage or paragraphs it is on, and follows
    them when the note changes; one whose text is gone shows at the top. <kbd>↵</kbd> edits the focused comment while
    reading or in the Comments tab, <kbd>⌘↵</kbd> saves, <kbd>esc</kbd> cancels. A click on a highlight or a comment
    focuses it; the <kbd>+</kbd> beside a paragraph comments on it.
  </p>
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
    align-items: flex-start;
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
