<script lang="ts" module>
  /** What the threads of a note share, changed without rebuilding them. */
  export interface ThreadState {
    /** The comment `r` resolves and `↵` edits. */
    focused: string | null;
    /** The comment being edited, if any. */
    editing: string | null;
    /** The text of the comment being written or edited. */
    draft: string;
  }
</script>

<script lang="ts">
  import type { Comment } from "./api";
  import { age, clock, longDate } from "./time.svelte";

  // The comments under one block of the note, mounted by NoteView into the rendered Markdown.
  let {
    comments,
    ui,
    composing = false,
    detached = false,
    onSave,
    onCancel,
    onEdit,
    onResolve,
    onFocus,
  }: {
    comments: Comment[];
    ui: ThreadState;
    /** Whether a new comment is being written here. */
    composing?: boolean;
    /** Whether these comments lost their place in the note, which then shows what they were on. */
    detached?: boolean;
    /** Saves the draft: as a new comment, or over the comment `id`. */
    onSave: (id: string | null) => void;
    onCancel: () => void;
    onEdit: (comment: Comment) => void;
    onResolve: (comment: Comment) => void;
    onFocus: (id: string) => void;
  } = $props();

  function focusOnMount(textarea: HTMLTextAreaElement) {
    textarea.focus();
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
  }

  function onKeydown(event: KeyboardEvent, id: string | null) {
    if (event.key === "Enter" && event.metaKey) {
      event.preventDefault();
      if (ui.draft.trim()) onSave(id);
    } else if (event.key === "Escape") {
      event.preventDefault();
      onCancel();
    }
  }
</script>

{#snippet composer(id: string | null)}
  <div class="composer">
    <textarea
      bind:value={ui.draft}
      {@attach focusOnMount}
      onkeydown={(e) => onKeydown(e, id)}
      rows="3"
      placeholder="Comment…"
      spellcheck="true"
    ></textarea>
    <div class="actions">
      <button onclick={onCancel}>Cancel <kbd>esc</kbd></button>
      <button class="primary" disabled={!ui.draft.trim()} onclick={() => onSave(id)}>
        {id ? "Save" : "Comment"} <kbd>⌘↵</kbd>
      </button>
    </div>
  </div>
{/snippet}

<div class="thread" class:detached>
  {#if detached}<div class="label">No longer found in the note</div>{/if}
  {#each comments as comment (comment.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="comment"
      class:focused={ui.focused === comment.id}
      data-comment={comment.id}
      onclick={() => onFocus(comment.id)}
    >
      {#if detached}<div class="quote">{comment.anchor.quote}</div>{/if}
      {#if ui.editing === comment.id}
        {@render composer(comment.id)}
      {:else}
        <div class="body">{comment.body}</div>
        <div class="meta">
          <time title={longDate(comment.updated)}>{age(comment.updated, clock.now)}</time>
          <button onclick={() => onEdit(comment)}>Edit</button>
          <button onclick={() => onResolve(comment)} title="Resolve: delete the comment">Resolve</button>
        </div>
      {/if}
    </div>
  {/each}
  {#if composing}{@render composer(null)}{/if}
</div>

<style>
  .thread {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0.4em 0 1em;
    font-size: 13px;
    line-height: 1.5;
    user-select: none;
    -webkit-user-select: none;
  }

  .label {
    font-size: 11px;
    font-weight: 650;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--overlay0);
  }

  .comment,
  .composer {
    padding: 8px 10px;
    border: 1px solid color-mix(in srgb, var(--yellow) 35%, var(--surface0));
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--yellow) 9%, var(--base));
  }

  .comment {
    cursor: pointer;
  }

  .comment.focused {
    border-color: var(--yellow);
    box-shadow: 0 0 0 1px var(--yellow);
  }

  .comment .composer {
    padding: 0;
    border: 0;
    background: none;
  }

  .quote {
    margin-bottom: 4px;
    padding-left: 8px;
    border-left: 2px solid var(--surface1);
    color: var(--subtext);
    font-size: 12px;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .body {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
    -webkit-user-select: text;
  }

  .meta {
    display: flex;
    gap: 10px;
    align-items: baseline;
    margin-top: 2px;
    font-size: 11px;
    color: var(--overlay0);
  }

  .meta time {
    margin-right: auto;
  }

  .meta button {
    padding: 0;
    border: 0;
    background: none;
    color: var(--overlay2);
    cursor: pointer;
  }

  .meta button:hover {
    color: var(--text);
  }

  textarea {
    display: block;
    width: 100%;
    resize: vertical;
    font-size: 13px;
    line-height: 1.5;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .actions button {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 3px 10px;
    border: 1px solid var(--surface0);
    border-radius: 6px;
    background: var(--base);
    font-size: 12px;
    cursor: pointer;
  }

  .actions button.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--base);
  }

  .actions button.primary kbd {
    background: none;
    color: inherit;
    border-color: color-mix(in srgb, var(--base) 50%, transparent);
  }

  .actions button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
