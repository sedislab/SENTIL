'use client';

import { useCallback, useEffect, useRef, useState } from 'react';

const EXAMPLES: { label: string; code: string }[] = [
  {
    label: 'First monitor',
    code: `import sentil
from sentil import Formula

trace = sentil.Trace([0, 1, 2, 3, 4], {"speed": [12, 9, 7, 4, 6]})
phi = Formula.parse("G (speed > 5)")

print("robustness:", phi.robustness(trace))
`,
  },
  {
    label: 'Signal and violations',
    code: `import sentil
from sentil import Formula

trace = sentil.Trace([0, 1, 2, 3, 4], {"speed": [12, 9, 7, 4, 6]})
phi = Formula.parse("G (speed > 5)")

print("per sample:", phi.robustness_signal(trace))
for v in phi.violations(trace):
    print(f"violated on [{v.start}, {v.end}]")
`,
  },
  {
    label: 'Streaming',
    code: `import math
import sentil

monitor = sentil.OnlineMonitor("G[0,10] (x > -0.9)")
for t in range(60):
    verdict = monitor.update(float(t), {"x": math.sin(t * 0.3)})
    if verdict.resolved and not verdict.satisfied:
        print(f"violated at t={t}, robustness {verdict.value:.3f}")
        break
else:
    print("held over the whole stream")
`,
  },
  {
    label: 'Probabilistic',
    code: `import sentil
from sentil import Formula, LiftingRegistry, NoiseModel, SmcConfig

trace = sentil.Trace(list(range(20)), {"distance": [3.4 + 0.02 * i for i in range(20)]})

lifting = LiftingRegistry()
lifting.register("distance", NoiseModel.gaussian(0.0, 0.2))

phi = Formula.parse("P>=0.9 (G (distance > 3))")
result = phi.check(trace, lifting, SmcConfig(samples=5000))

print(f"probability {result.probability:.3f}")
print(f"interval    {result.interval}")
print(f"holds       {result.holds}")
`,
  },
];

interface Line {
  stream: 'stdout' | 'stderr' | 'error';
  text: string;
}

export default function Playground({
  code,
  example,
  compact = false,
}: {
  code?: string;
  example?: string;
  compact?: boolean;
}) {
  const workerRef = useRef<Worker | null>(null);
  const gutterRef = useRef<HTMLDivElement>(null);
  const [source, setSource] = useState(
    code ?? (EXAMPLES.find((e) => e.label === example) ?? EXAMPLES[0]).code,
  );
  const [lines, setLines] = useState<Line[]>([]);
  const [status, setStatus] = useState<'idle' | 'booting' | 'running'>('idle');

  useEffect(() => {
    const handle = setTimeout(() => {
      workerRef.current ??= new Worker('/engine/py-worker.js', { type: 'module' });
      workerRef.current.postMessage({ op: 'warm' });
    }, 250);
    return () => {
      clearTimeout(handle);
      workerRef.current?.terminate();
    };
  }, []);

  const run = useCallback(() => {
    if (status !== 'idle') return;
    if (!workerRef.current) {
      workerRef.current = new Worker('/engine/py-worker.js', { type: 'module' });
    }
    const worker = workerRef.current;
    setLines([]);
    setStatus('running');
    const onMessage = (event: MessageEvent) => {
      const msg = event.data;
      if (msg.status === 'booting') setStatus('booting');
      if (msg.status === 'running') setStatus('running');
      if (msg.stream) setLines((prev) => [...prev, { stream: msg.stream, text: msg.text }]);
      if (msg.done) {
        if (msg.error) setLines((prev) => [...prev, { stream: 'error', text: msg.error }]);
        setStatus('idle');
        worker.removeEventListener('message', onMessage);
      }
    };
    worker.addEventListener('message', onMessage);
    worker.postMessage({ id, code: source });
  }, [source, status]);

  const onKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      event.preventDefault();
      run();
      return;
    }
    if (event.key === 'Escape') {
      event.currentTarget.blur();
      return;
    }
    if (event.key === 'Tab') {
      event.preventDefault();
      const el = event.currentTarget;
      const { selectionStart: start, selectionEnd: end, value } = el;
      setSource(value.slice(0, start) + '    ' + value.slice(end));
      requestAnimationFrame(() => el.setSelectionRange(start + 4, start + 4));
    }
  };

  return (
    <div className="not-prose overflow-hidden rounded-2xl border border-fd-border">
      <div className="flex items-center gap-3 border-b border-fd-border px-3 py-2">
        <button
          onClick={run}
          disabled={status !== 'idle'}
          className="inline-flex items-center gap-1.5 rounded-lg bg-fd-primary px-3.5 py-1.5 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90 disabled:opacity-60"
        >
          {status === 'booting' ? 'Loading runtime' : status === 'running' ? 'Running' : 'Run'}
        </button>
        {!code && (
          <select
            onChange={(e) => {
              setSource(EXAMPLES[Number(e.target.value)].code);
              setLines([]);
            }}
            className="rounded-lg border border-fd-border bg-fd-background px-2 py-1.5 text-sm text-fd-muted-foreground"
            aria-label="Example programs"
          >
            {EXAMPLES.map((ex, i) => (
              <option key={ex.label} value={i}>
                {ex.label}
              </option>
            ))}
          </select>
        )}
        <span className="ml-auto hidden text-xs text-fd-muted-foreground sm:inline">
          Ctrl+Enter runs, Esc exits the editor.
        </span>
      </div>
      <div className="flex">
        <div
          ref={gutterRef}
          aria-hidden="true"
          className="select-none overflow-hidden whitespace-pre py-3 pl-4 pr-3 text-right font-mono text-[0.8125rem] leading-6 text-fd-muted-foreground/60"
        >
          {Array.from({ length: source.split('\n').length }, (_, i) => i + 1).join('\n')}
        </div>
        <textarea
          value={source}
          onChange={(e) => setSource(e.target.value)}
          onKeyDown={onKeyDown}
          onScroll={(e) => {
            if (gutterRef.current) gutterRef.current.scrollTop = e.currentTarget.scrollTop;
          }}
          spellCheck={false}
          autoCapitalize="off"
          autoComplete="off"
          aria-label="Program"
          className="block w-full resize-y bg-transparent py-3 pr-4 font-mono text-[0.8125rem] leading-6 outline-none"
          style={{ minHeight: compact ? '14rem' : '22rem' }}
        />
      </div>
      <div className="border-t border-fd-border bg-fd-muted/60 px-4 py-3 dark:bg-black/20">
        {lines.length === 0 ? (
          <p className="font-mono text-[0.8125rem] leading-6 text-fd-muted-foreground">
            {status === 'booting'
              ? 'Fetching the Python runtime, about 12 MB, kept for the whole visit.'
              : status === 'running'
                ? 'Running.'
                : 'Output appears here.'}
          </p>
        ) : (
          <pre className="overflow-x-auto font-mono text-[0.8125rem] leading-6">
            {lines.map((line, i) => (
              <div key={i} className={line.stream === 'stdout' ? undefined : 'text-fd-error'}>
                {line.text}
              </div>
            ))}
          </pre>
        )}
      </div>
    </div>
  );
}