'use client';

import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';

export interface Trace {
  times: number[];
  signals: Record<string, number[]>;
  dense?: boolean;
}

interface Common {
  children: ReactNode;
  hint: string;
  explanation: string;
}

interface NumericProps extends Common {
  formula: string;
  trace: Trace;
  at?: number;
  unbounded?: 'inf' | '-inf';
}

interface FormulaProps extends Common {
  answer: string;
  cases: Trace[];
  placeholder?: string;
}

interface WilsonProps extends Common {
  wilson: { successes: number; trials: number; level: number; end: 'lo' | 'hi' };
  places?: number;
}

type Props = NumericProps | FormulaProps | WilsonProps;

interface EngineResult {
  ok: boolean;
  value?: number | null;
  series?: (number | null)[];
  variables?: string[];
  lo?: number;
  hi?: number;
  error?: string | null;
}

type Grade = { state: 'right' } | { state: 'wrong'; note: string; unread?: true };

const TOLERANCE = 1e-6;

function isNumeric(props: Props): props is NumericProps {
  return 'formula' in props;
}

function isWilson(props: Props): props is WilsonProps {
  return 'wilson' in props;
}

function sameSeries(a: (number | null)[], b: (number | null)[]) {
  if (a.length !== b.length) return false;
  return a.every((x, i) => {
    const y = b[i];
    if (x === null || y === null) return x === y;
    return Math.abs(x - y) <= TOLERANCE;
  });
}

function readNumber(input: string): number | null {
  const text = input.trim().toLowerCase();
  if (/\s/.test(text)) return null;
  if (/^\+?inf(inity)?$/.test(text)) return Number.POSITIVE_INFINITY;
  if (/^-inf(inity)?$/.test(text)) return Number.NEGATIVE_INFINITY;
  const value = Number(text);
  return text.length > 0 && Number.isFinite(value) ? value : null;
}

export default function Exercise(props: Props) {
  const { children, hint, explanation } = props;
  const wantsNumber = !('answer' in props);
  const workerRef = useRef<Worker | null>(null);
  const seqRef = useRef(1);
  const [input, setInput] = useState('');
  const [grade, setGrade] = useState<Grade | null>(null);
  const [attempts, setAttempts] = useState(0);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    const handle = setTimeout(() => {
      workerRef.current ??= new Worker('/engine/check-worker.js', { type: 'module' });
      workerRef.current.postMessage({ op: 'warm' });
    }, 250);
    return () => {
      clearTimeout(handle);
      workerRef.current?.terminate();
    };
  }, []);

  const ask = useCallback((message: Record<string, unknown>): Promise<EngineResult> => {
    workerRef.current ??= new Worker('/engine/check-worker.js', { type: 'module' });
    const worker = workerRef.current;
    const id = seqRef.current++;
    return new Promise((resolve) => {
      const onMessage = (event: MessageEvent) => {
        if (event.data.id !== id) return;
        worker.removeEventListener('message', onMessage);
        resolve(event.data.result as EngineResult);
      };
      worker.addEventListener('message', onMessage);
      worker.postMessage({ id, ...message });
    });
  }, []);

  const check = useCallback(async () => {
    if (busy) return;
    setBusy(true);
    let next: Grade;

    if (isNumeric(props)) {
      const given = readNumber(input);
      const result = given === null ? null : await ask({ op: 'robustness', formula: props.formula, trace: props.trace });
      if (given === null) {
        next = { state: 'wrong', note: 'Robustness is one signed number. Write it as a number.', unread: true };
      } else if (!result?.ok) {
        next = { state: 'wrong', note: result?.error ?? 'The engine could not score that trace.' };
      } else {
        const index = props.at ?? 0;
        const series = result.series ?? [];
        const expected = index < series.length ? series[index] : undefined;
        if (expected === undefined) {
          next = { state: 'wrong', note: 'This exercise asks about a sample the trace does not carry.', unread: true };
        } else if (expected === null) {
          const wanted = props.unbounded === '-inf' ? Number.NEGATIVE_INFINITY : Number.POSITIVE_INFINITY;
          next = given === wanted ? { state: 'right' } : { state: 'wrong', note: hint };
        } else if (Math.abs(given - expected) <= TOLERANCE) {
          next = { state: 'right' };
        } else {
          const flipped = expected !== 0 && given !== 0 && Math.sign(given) !== Math.sign(expected);
          next = {
            state: 'wrong',
            note: flipped ? `The sign is the verdict, and yours is the other one. ${hint}` : hint,
          };
        }
      }
    } else if (isWilson(props)) {
      const given = readNumber(input);
      const result = given === null ? null : await ask({ op: 'wilson', ...props.wilson });
      const expected = result?.[props.wilson.end];
      if (given === null || expected === undefined) {
        next = { state: 'wrong', note: 'Write the bound as a decimal, for example 0.812.', unread: true };
      } else {
        const slack = 0.5 * 10 ** -(props.places ?? 3);
        next = Math.abs(given - expected) <= slack ? { state: 'right' } : { state: 'wrong', note: hint };
      }
    } else {
      const written = input.trim();
      const parsed = await ask({ op: 'parse', formula: written });
      if (!parsed?.ok) {
        next = { state: 'wrong', note: parsed?.error ?? 'That formula does not parse.' };
      } else {
        next = { state: 'right' };
        for (const trace of props.cases) {
          const [mine, reference] = await Promise.all([
            ask({ op: 'robustness', formula: written, trace }),
            ask({ op: 'robustness', formula: props.answer, trace }),
          ]);
          if (!mine.ok) {
            next = { state: 'wrong', note: mine.error ?? 'The engine could not score that formula.' };
            break;
          }
          if (!mine.series || !reference.series || !sameSeries(mine.series, reference.series)) {
            next = { state: 'wrong', note: hint };
            break;
          }
        }
      }
    }

    setGrade(next);
    if (!(next.state === 'wrong' && next.unread)) setAttempts((n) => n + 1);
    setBusy(false);
  }, [ask, busy, hint, input, props]);

  const solved = grade?.state === 'right';
  const showExplanation = solved || attempts >= 3;

  return (
    <div className="exercise not-prose">
      <p className="exercise-label">Exercise</p>
      <div className="exercise-question">{children}</div>
      <div className="exercise-row">
        <input
          type="text"
          value={input}
          spellCheck={false}
          autoComplete="off"
          aria-label={wantsNumber ? 'Your answer, as a number' : 'Your formula'}
          placeholder={wantsNumber ? 'a number' : ((props as FormulaProps).placeholder ?? 'G (x > 0)')}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              void check();
            }
          }}
        />
        <button type="button" onClick={() => void check()} disabled={busy}>
          {busy ? 'Checking' : 'Check'}
        </button>
      </div>
      {grade && (
        <p className={solved ? 'exercise-verdict is-right' : 'exercise-verdict'} role="status">
          {solved ? 'Correct.' : grade.note}
        </p>
      )}
      {showExplanation && <p className="exercise-explanation">{explanation}</p>}
    </div>
  );
}