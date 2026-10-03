/**
 * Where an arrow key moves the selection of a list, the same in every list of the app: `↓` `↑` one item, `⌘↓` `⌘↑`
 * the last or first, stopping at the ends. Null for any other key, or an empty list.
 */
export function listStep(event: KeyboardEvent, index: number, length: number): number | null {
  const down = event.key === "ArrowDown";
  if ((!down && event.key !== "ArrowUp") || event.altKey || event.ctrlKey || event.shiftKey || !length) {
    return null;
  }
  if (event.metaKey) return down ? length - 1 : 0;
  return index < 0 ? 0 : Math.max(0, Math.min(length - 1, index + (down ? 1 : -1)));
}

/** Whether `→` unfolds a folder of the sidebar (true) or `←` folds it (false). Null for any other key. */
export function foldStep(event: KeyboardEvent): boolean | null {
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return null;
  return event.key === "ArrowRight" ? true : event.key === "ArrowLeft" ? false : null;
}
