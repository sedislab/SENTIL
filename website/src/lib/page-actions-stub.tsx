// Load bearing. next.config.mjs swaps Fumadocs' shared page-actions module for
// this one; that module names outside services in its share links, and a static
// re-export keeps their strings in the bundle even unused.
export function MarkdownCopyButton(): null {
  return null;
}

export function ViewOptionsPopover(): null {
  return null;
}