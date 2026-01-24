import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import Link from 'next/link';
import { Download } from 'lucide-react';
import { appName } from './shared';

function Logo() {
  return (
    <span className="inline-flex items-center gap-2.5">
      <svg width="24" height="24" viewBox="0 0 24 24" aria-hidden="true" className="shrink-0">
        <rect x="1.5" y="1.5" width="21" height="21" rx="6" fill="var(--color-fd-primary)" />
        <path
          d="M4.5 14h3.2l1.8-6.4 2.7 9.6 1.7-4.4h4.1"
          fill="none"
          stroke="var(--color-fd-primary-foreground)"
          strokeWidth="1.7"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <span className="font-display text-[1.0625rem] font-bold tracking-tight">{appName}</span>
    </span>
  );
}

function InstallButton() {
  return (
    <span className="install-cta">
      <Link
        href="/docs/start/install"
        prefetch={false}
        className="inline-flex items-center gap-2 rounded-xl bg-fd-primary px-4 py-2 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
      >
        <Download className="size-4" />
        Install SENTIL
      </Link>
    </span>
  );
}

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: <Logo />,
    },
    githubUrl: 'https://github.com/sedislab/SENTIL',
    links: [
      { text: 'Cite', url: '/docs/reference/methods/citation' },
      { type: 'custom', children: <InstallButton /> },
    ],
  };
}