<script lang="ts">
  import Modal from "./Modal.svelte";

  let {
    title,
    message,
    action,
    onConfirm,
    onClose,
  }: { title: string; message: string; action: string; onConfirm: () => void; onClose: () => void } =
    $props();

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "y" || event.key === "Enter") {
      event.preventDefault();
      onConfirm();
    } else if (event.key === "n") {
      event.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<Modal {title} {onClose}>
  <p>{message}</p>
  <div class="actions">
    <button onclick={onClose}>Cancel <kbd>n</kbd></button>
    <button class="danger" onclick={onConfirm}>{action} <kbd>y</kbd></button>
  </div>
</Modal>

<style>
  p {
    margin: 0 0 16px;
    line-height: 1.5;
    color: var(--subtext);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  button {
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid var(--surface0);
    background: var(--mantle);
    cursor: pointer;
  }

  .danger {
    border-color: var(--red);
    color: var(--red);
  }
</style>
