<script lang="ts" module>
  /** The open dialogs, the topmost last: Escape closes only that one. */
  const stack: symbol[] = [];
</script>

<script lang="ts">
  import type { Snippet } from "svelte";

  // A centred dialog over a dimmed window; Escape or a click outside closes it.
  let { title, onClose, children, small = false }: {
    title: string;
    onClose: () => void;
    children: Snippet;
    small?: boolean;
  } = $props();

  const id = Symbol();

  $effect(() => {
    stack.push(id);
    return () => stack.splice(stack.indexOf(id), 1);
  });

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && stack.at(-1) === id) {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="modal" class:small role="dialog" tabindex="-1" aria-label={title} onclick={(e) => e.stopPropagation()}>
    <h3>{title}</h3>
    {@render children()}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 16vh;
    background: rgb(0 0 0 / 0.22);
  }

  .modal {
    width: min(560px, 90vw);
    max-height: 72vh;
    overflow-y: auto;
    padding: 18px 20px;
    border-radius: 12px;
    border: 1px solid var(--surface0);
    background: var(--base);
    box-shadow: var(--shadow);
  }

  .small {
    width: min(400px, 90vw);
  }

  h3 {
    margin: 0 0 12px;
    font-size: 15px;
  }
</style>
