'use client';
import dynamic from 'next/dynamic';
import NextLink from 'next/link';
import { RootProvider } from 'fumadocs-ui/provider/next';
import type { ComponentProps, FC, ReactNode } from 'react';

const SearchDialog = dynamic(() => import('@/components/search'));

const Link: FC<ComponentProps<'a'> & { prefetch?: boolean }> = ({
  href = '#',
  prefetch = false,
  ...props
}) => <NextLink href={href} prefetch={prefetch} {...props} />;

export function Provider({ children }: { children: ReactNode }) {
  return (
    <RootProvider search={{ SearchDialog }} components={{ Link }}>
      {children}
    </RootProvider>
  );
}