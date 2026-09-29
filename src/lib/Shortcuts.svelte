<script lang="ts" module>
  /** A key and what it does now; `hot` marks the ones the moment calls for, such as `c` over a selection. */
  export interface Hint {
    keys: string;
    label: string;
    hot?: boolean;
  }
</script>

<script lang="ts">
  // A thin column beside the note recalling the keys that act on it, as they change with what is going on.
  let { groups }: { groups: { title: string; hints: Hint[] }[] } = $props();
</script>

<aside data-tauri-drag-region>
  {#each groups as group (group.title)}
    {#if group.hints.length}
      <h2>{group.title}</h2>
      <dl>
        {#each group.hints as hint (hint.keys + hint.label)}
          <dt class:hot={hint.hot}>
            {#each hint.keys.split("  ") as k (k)}<kbd>{k}</kbd>{/each}
          </dt>
          <dd class:hot={hint.hot}>{hint.label}</dd>
        {/each}
      </dl>
    {/if}
  {/each}
</aside>

<style>
  aside {
    width: 168px;
    height: 100%;
    overflow-y: auto;
    /* Level with the note's header. */
    padding: 44px 12px 16px 14px;
    border-left: 1px solid color-mix(in srgb, var(--crust) 60%, transparent);
    font-size: 11.5px;
    color: var(--overlay2);
  }

  /* A narrow window keeps its width for the note. */
  @media (max-width: 1000px) {
    aside {
      display: none;
    }
  }

  h2 {
    margin: 0 0 6px;
    font-size: 10px;
    font-weight: 650;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--overlay0);
  }

  h2:not(:first-child) {
    margin-top: 16px;
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 5px 8px;
    margin: 0;
    align-items: baseline;
  }

  dt {
    display: flex;
    gap: 2px;
    justify-content: flex-end;
  }

  dd {
    margin: 0;
    line-height: 1.3;
  }

  kbd {
    font-size: 10px;
    padding: 0 4px;
    opacity: 0.85;
  }

  .hot kbd {
    border-color: var(--yellow);
    color: var(--text);
    opacity: 1;
  }

  dd.hot {
    color: var(--text);
    font-weight: 600;
  }
</style>
