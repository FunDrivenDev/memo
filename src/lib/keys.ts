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
