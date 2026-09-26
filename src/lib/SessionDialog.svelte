<script lang="ts">
  import * as api from "./api";
  import type { Note } from "./api";
  import Modal from "./Modal.svelte";

  let { note, onDone, onClose }: { note: Note; onDone: (message: string, error?: boolean) => void; onClose: () => void } =
    $props();

  let workdir = $state("");
  let prompt = $state("");
  let loading = $state(true);
  let promptField: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    api
      .sessionDefaults(note.id)
      .then((defaults) => {
        workdir = defaults.workdir;
        prompt = defaults.prompt;
        loading = false;
      })
      .catch((e) => onDone(String(e), true));
  });

  $effect(() => {
    if (!loading) promptField?.focus();
  });

  async function start() {
    try {
      await api.startSession(workdir, prompt);
      onDone(`Started a Claude Code session in ${workdir}`);
    } catch (e) {
      onDone(String(e), true);
    }
  }

  function onKeydown(event: KeyboardEvent) {
    const submit = event.key === "Enter" && (event.metaKey || (event.target as HTMLElement).tagName === "INPUT");
    if (submit) {
      event.preventDefault();
      start();
    }
  }
</script>

<Modal title="Start a Claude Code session" {onClose}>
  {#if !loading}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="form" onkeydown={onKeydown}>
      <label>
        <span>Working directory</span>
        <input bind:value={workdir} spellcheck="false" />
      </label>
      <label>
        <span>Prompt</span>
        <textarea bind:this={promptField} bind:value={prompt} rows="4" spellcheck="false"></textarea>
      </label>
      <div class="actions">
        <span class="hint">Opens a terminal window there and runs <code>claude</code> with this prompt.</span>
        <button onclick={start}>Start <kbd>⌘↵</kbd></button>
      </div>
    </div>
  {/if}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  label span {
    font-size: 12px;
    color: var(--subtext);
  }

  input,
  textarea {
    font-family: var(--mono);
    font-size: 12.5px;
    resize: vertical;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .hint {
    flex: 1;
    font-size: 12px;
    color: var(--overlay0);
  }

  button {
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    cursor: pointer;
  }
</style>
