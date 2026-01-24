'use client';

import { CodeBlock, Pre } from 'fumadocs-ui/components/codeblock';
import type { ComponentProps } from 'react';

export function CodeBlockWithNotes({ children, ...props }: ComponentProps<'pre'>) {
  return (
    <CodeBlock {...props}>
      <Pre>{children}</Pre>
    </CodeBlock>
  );
}