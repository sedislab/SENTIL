import { source } from '@/lib/source';
import { DocsLayout } from 'fumadocs-ui/layouts/notebook';
import { baseOptions } from '@/lib/layout.shared';
import { SkipLink } from '@/components/skip-link';
import { Rocket, Activity, Sigma, Workflow, Braces, Book, Compass } from 'lucide-react';

const tabs = [
  { title: 'Get started', url: '/docs/start', icon: <Rocket className="size-4" /> },
  { title: 'Monitoring', url: '/docs/monitoring', icon: <Activity className="size-4" /> },
  { title: 'Probabilistic', url: '/docs/probabilistic', icon: <Sigma className="size-4" /> },
  { title: 'Synthesis', url: '/docs/synthesis', icon: <Workflow className="size-4" /> },
  { title: 'Languages', url: '/docs/languages', icon: <Braces className="size-4" /> },
  { title: 'Reference', url: '/docs/reference', icon: <Book className="size-4" /> },
  { title: 'Examples', url: '/docs/examples', icon: <Compass className="size-4" /> },
];

export default function Layout({ children }: LayoutProps<'/docs'>) {
  return (
    <>
      <SkipLink href="#nd-page" />
      <DocsLayout
        tree={source.getPageTree()}
        {...baseOptions()}
        tabMode="navbar"
        tabs={tabs}
        sidebar={{ collapsible: false, defaultOpenLevel: 4, prefetch: false }}
        nav={{ ...baseOptions().nav, mode: 'top' }}
      >
        {children}
      </DocsLayout>
    </>
  );
}