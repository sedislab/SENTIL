import type { Metadata } from 'next';
import Link from 'next/link';
import { PlaygroundLazy } from '@/components/playground/PlaygroundLazy';

export const metadata: Metadata = {
  title: 'Playground',
  description:
    'Write and run real Python against the real SENTIL engine in your browser. Parse a formula, score robustness, stream a monitor, and estimate satisfaction probability, with nothing installed.',
  alternates: { canonical: '/playground' },
};

export default function PlaygroundPage() {
  return (
    <div id="main-content" tabIndex={-1} className="mx-auto w-full max-w-5xl px-6 py-12 outline-none">
      <span className="text-sm font-semibold tracking-tight text-fd-primary">Playground</span>
      <h1 className="mt-2 font-display text-3xl font-bold tracking-tight">Run SENTIL in your browser</h1>
      <p className="mt-3 max-w-2xl text-lg leading-relaxed text-fd-muted-foreground">
        Use SENTIL in Python, run it and read the output.
      </p>
      <div className="mt-8">
        <PlaygroundLazy />
      </div>
      <p className="mt-6 text-sm text-fd-muted-foreground">
        For the theory behind STL and PrSTL, read <Link className="text-fd-primary" href="/docs/monitoring/concepts/what-is-stl">quantitative robustness</Link>, or install the library from the <Link className="text-fd-primary" href="/docs/start/install">quickstart</Link>.
      </p>
    </div>
  );
}