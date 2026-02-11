'use client';

import dynamic from 'next/dynamic';

const Playground = dynamic(() => import('./Playground'), {
  loading: () => (
    <div className="not-prose flex h-64 items-center justify-center rounded-2xl border border-fd-border text-sm text-fd-muted-foreground">
      Loading the editor
    </div>
  ),
});

export function PlaygroundLazy({
  code,
  example,
  compact = false,
}: {
  code?: string;
  example?: string;
  compact?: boolean;
}) {
  return <Playground code={code} example={example} compact={compact} />;
}