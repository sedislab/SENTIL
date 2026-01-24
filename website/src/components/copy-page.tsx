'use client';
import { useEffect, useRef, useState } from 'react';
import { Check, ChevronDown, Clipboard, FileText, Pencil } from 'lucide-react';

export function CopyPage({ markdownUrl, editUrl }: { markdownUrl: string; editUrl?: string }) {
  const [copied, setCopied] = useState(false);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointer = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener('mousedown', onPointer);
    document.addEventListener('keydown', onKey);
    menuRef.current?.querySelector<HTMLElement>('a, button')?.focus();
    return () => {
      document.removeEventListener('mousedown', onPointer);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  async function copy() {
    await fetch(markdownUrl)
      .then((res) => res.text())
      .then((text) => navigator.clipboard.writeText(text))
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1800);
      })
      .catch(() => undefined);
    setOpen(false);
  }

  return (
    <div ref={ref} className="relative flex shrink-0 items-center">
      <div className="flex items-center overflow-hidden rounded-lg border border-[#e6e2e3] bg-white text-sm font-medium text-[#464243] shadow-sm dark:border-[#241420] dark:bg-[#0e0b0e] dark:text-[#d5d1d3]">
        <button
          type="button"
          onClick={copy}
          className="inline-flex items-center gap-2 px-3.5 py-1.5 transition-colors hover:bg-fd-accent"
        >
          {copied ? <Check className="size-3.5 text-fd-primary" /> : <Clipboard className="size-3.5" />}
          {copied ? 'Copied' : 'Copy page'}
        </button>
        <button
          ref={triggerRef}
          type="button"
          aria-label="More page options"
          aria-expanded={open}
          aria-haspopup="true"
          data-state={open ? 'open' : 'closed'}
          onClick={() => setOpen((v) => !v)}
          className="inline-flex items-center self-stretch border-l border-l-[#a5a3a4] px-4 transition-colors hover:bg-fd-accent dark:border-l-[#9c9c9c]"
        >
          <ChevronDown className={`size-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
        </button>
      </div>
      {open && (
        <div
          ref={menuRef}
          className="menu-in absolute right-0 top-full z-30 mt-1.5 min-w-[196px] rounded-lg border border-fd-border bg-fd-popover p-1 text-sm text-fd-foreground shadow-md"
        >
          <button
            type="button"
            onClick={copy}
            className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-black/[0.06] dark:hover:bg-white/10"
          >
            <Clipboard className="size-4 text-fd-muted-foreground" />
            Copy page
          </button>
          <a
            href={markdownUrl}
            target="_blank"
            rel="noreferrer noopener"
            className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 transition-colors hover:bg-black/[0.06] dark:hover:bg-white/10"
            onClick={() => setOpen(false)}
          >
            <FileText className="size-4 text-fd-muted-foreground" />
            View as Markdown
          </a>
          {editUrl && (
            <a
              href={editUrl}
              target="_blank"
              rel="noreferrer noopener"
              className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 transition-colors hover:bg-black/[0.06] dark:hover:bg-white/10"
              onClick={() => setOpen(false)}
            >
              <Pencil className="size-4 text-fd-muted-foreground" />
              Edit this page
            </a>
          )}
        </div>
      )}
    </div>
  );
}