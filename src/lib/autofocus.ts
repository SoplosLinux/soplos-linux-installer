// Svelte action: focuses the element as soon as it is mounted. Used on a
// popup's primary button so Enter accepts it immediately.
export function autofocus(node: HTMLElement) {
  node.focus();
}
