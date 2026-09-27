<script lang="ts">
  import * as api from "./api";
  import type { Completion } from "./api";

  // A text field for a folder path, completed from the disk as it is typed: ↑ ↓ pick a folder, ⇥ or a click
  // takes it and lists its subfolders, ↵ takes it as is, Escape closes the list.
  let { value = $bindable(""), input = $bindable(), placeholder = "" }: {
    value?: string;
    input?: HTMLInputElement;
    placeholder?: string;
  } = $props();

  let completion = $state<Completion>({ folders: [], denied: null });
  let highlighted = $state(-1);
  let focused = $state(false);
  let closed = $state(false);
  let list: HTMLUListElement | undefined = $state();
  /** Drops the answers to older values. */
  let request = 0;

  const options = $derived(completion.folders.filter((f) => f !== value));
  const open = $derived(focused && !closed && (options.length > 0 || completion.denied !== null));

  $effect(() => {
    if (!focused) return;
    const id = ++request;
    highlighted = -1;
    api
      .completeFolder(value)
      .then((next) => {
        if (id === request) completion = next;
      })
      .catch(() => {});
  });

  $effect(() => {
    list?.children[highlighted]?.scrollIntoView({ block: "nearest" });
  });

  function take(path: string) {
    value = path;
    closed = false;
    input?.focus();
  }

  function onKeydown(event: KeyboardEvent) {
    if (!open) return;
    const n = options.length;
    if (event.key === "ArrowDown" && n) highlighted = (highlighted + 1) % n;
    else if (event.key === "ArrowUp" && n) highlighted = highlighted <= 0 ? n - 1 : highlighted - 1;
    else if (event.key === "Tab" && n && !event.shiftKey) take(`${options[Math.max(highlighted, 0)]}/`);
    else if (event.key === "Enter" && highlighted >= 0 && !event.metaKey) take(options[highlighted] ?? value);
    else if (event.key === "Escape") closed = true;
    else return;
    event.preventDefault();
    event.stopPropagation();
  }
</script>

<div class="field">
  <input
    bind:this={input}
    bind:value
    {placeholder}
    spellcheck="false"
    autocomplete="off"
    oninput={() => (closed = false)}
    onfocus={() => (focused = true)}
    onblur={() => (focused = false)}
    onkeydown={onKeydown}
  />
  {#if open}
    <!-- Mousedown keeps the focus in the field. -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="menu" onmousedown={(e) => e.preventDefault()}>
      {#if completion.denied}
        <p class="denied">
          macOS keeps memo out of <code>{completion.denied}</code>.
          <button onclick={() => api.openPrivacySettings(completion.denied ?? "")}>Grant access…</button>
        </p>
      {:else}
        <ul bind:this={list} role="listbox">
          {#each options as option, i (option)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <li role="option" aria-selected={i === highlighted} class:highlighted={i === highlighted}
              onclick={() => take(`${option}/`)}>{option}</li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>

<style>
  .field {
    position: relative;
    flex: 1;
  }

  input {
    width: 100%;
    font-family: var(--mono);
    font-size: 12.5px;
  }

  .menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 1;
    border: 1px solid var(--surface0);
    border-radius: 6px;
    background: var(--base);
    box-shadow: var(--shadow);
  }

  ul {
    max-height: 168px;
    margin: 0;
    padding: 4px 0;
    overflow-y: auto;
    list-style: none;
  }

  li {
    padding: 4px 10px;
    font-family: var(--mono);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }

  li:hover,
  .highlighted {
    background: color-mix(in srgb, var(--accent) 14%, transparent);
  }

  .denied {
    margin: 0;
    padding: 8px 10px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--subtext);
  }

  .denied button {
    margin-left: 4px;
    padding: 2px 8px;
    border: 1px solid var(--surface0);
    border-radius: 6px;
    background: var(--mantle);
    cursor: pointer;
  }
</style>
