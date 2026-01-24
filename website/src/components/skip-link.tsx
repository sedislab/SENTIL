export function SkipLink({ href }: { href: string }) {
  return (
    <a
      href={href}
      className="sr-only z-50 rounded-lg border border-fd-border bg-fd-background px-4 py-2 text-sm font-medium text-fd-primary focus-visible:not-sr-only focus-visible:fixed focus-visible:left-4 focus-visible:top-4"
    >
      Skip to content
    </a>
  );
}