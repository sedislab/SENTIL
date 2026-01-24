'use client';

import { CodeBlock, Pre } from 'fumadocs-ui/components/codeblock';
import { buttonVariants } from 'fumadocs-ui/components/ui/button';
import { useCopyButton } from 'fumadocs-ui/utils/use-copy-button';
import { Check, Clipboard } from 'lucide-react';
import { useRef, type ComponentProps, type RefObject } from 'react';

function snippet(pre: HTMLPreElement): string {
  const lines = pre.querySelectorAll('code > .line');
  if (lines.length === 0) return pre.textContent ?? '';

  return Array.from(lines, (line) => {
    if (!line.querySelector('.copy-note')) return line.textContent ?? '';
    const kept = line.cloneNode(true) as HTMLElement;
    kept.querySelectorAll('.copy-note').forEach((note) => note.remove());
    return (kept.textContent ?? '').trimEnd();
  }).join('\n');
}

function CopyCode({ figure }: { figure: RefObject<HTMLElement | null> }) {
  const [copied, onClick] = useCopyButton(async () => {
    const pre = figure.current?.querySelector('pre');
    if (pre) await navigator.clipboard.writeText(snippet(pre)).catch(() => undefined);
  });

  return (
    <button
      type="button"
      data-copy=""
      onClick={onClick}
      data-checked={copied || undefined}
      aria-label={copied ? 'Copied' : 'Copy'}
      className={buttonVariants({ size: 'icon-xs' })}
    >
      {copied ? <Check /> : <Clipboard />}
    </button>
  );
}

export function CodeBlockWithNotes({ children, ...props }: ComponentProps<'pre'>) {
  const figure = useRef<HTMLElement>(null);

  return (
    <CodeBlock
      {...props}
      ref={figure}
      allowCopy={false}
      Actions={({ className }) => (
        <div className={className}>
          <CopyCode figure={figure} />
        </div>
      )}
    >
      <Pre>{children}</Pre>
    </CodeBlock>
  );
}