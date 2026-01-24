'use client';

import { useState } from 'react';
import { Tab, Tabs } from 'fumadocs-ui/components/tabs';

export { Tab as OsTab };

function detect(): string {
  const hints = `${navigator.platform ?? ''} ${navigator.userAgent ?? ''}`;
  if (/Mac|iPhone|iPad/i.test(hints)) return 'macos';
  if (/Win/i.test(hints)) return 'windows';
  return 'linux';
}

export function OsTabs({ children }: { children: React.ReactNode }) {
  useState(() => {
    if (typeof window === 'undefined') return;
    try {
      if (!localStorage.getItem('os') && !sessionStorage.getItem('os')) {
        localStorage.setItem('os', detect());
      }
    } catch {
      return;
    }
  });
  return (
    <Tabs groupId="os" persist items={['Linux', 'macOS', 'Windows']}>
      {children}
    </Tabs>
  );
}