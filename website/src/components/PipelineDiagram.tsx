const W = 760;
const H = 268;

const EDGE = 'var(--sentil-figure-edge)';
const LINE = 'var(--sentil-figure-line)';
const INK = 'var(--color-fd-foreground)';
const MUTED = 'var(--color-fd-muted-foreground)';

const STAGE_Y = 96;
const STAGE_H = 74;
const STAGE_W = 150;
const DATUM_H = 46;
const DATUM_W = 100;
const DATUM_Y = STAGE_Y + (STAGE_H - DATUM_H) / 2;

const STAGES = [
  { x: 136, label: 'lifting', lines: ['fit a noise model,', 'draw N trajectories'] },
  { x: 310, label: 'robustness', lines: ['evaluate \u03c1 on', 'each trajectory'] },
  { x: 484, label: 'aggregation', lines: ['estimate Pr,', 'bound the error'] },
];

const GROUP = { x: 122, y: 78, w: 526, h: 110 };

const mid = (x: number, w: number) => x + w / 2;

function Arrow({ x1, y1, x2, y2 }: { x1: number; y1: number; x2: number; y2: number }) {
  return <path d={`M${x1} ${y1} L${x2} ${y2}`} stroke={LINE} strokeWidth={1.25} markerEnd="url(#pipeline-tip)" />;
}

function Datum({ x, y, lines }: { x: number; y: number; lines: string[] }) {
  return (
    <g>
      <rect x={x} y={y} width={DATUM_W} height={DATUM_H} rx={5} fill="var(--color-fd-card)" stroke={EDGE} />
      {lines.map((t, i) => (
        <text
          key={t}
          x={mid(x, DATUM_W)}
          y={y + DATUM_H / 2 + (i - (lines.length - 1) / 2) * 14 + 4}
          textAnchor="middle"
          fontSize={11}
          fill={MUTED}
        >
          {t}
        </text>
      ))}
    </g>
  );
}

export function PipelineDiagram() {
  return (
    <figure className="not-prose my-8">
        <svg
        viewBox={`0 0 ${W} ${H}`}
        className="w-full"
      >
        <defs>
          <marker id="pipeline-tip" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" markerHeight="6" orient="auto">
            <path d="M0 1 L7 4 L0 7 z" fill={LINE} />
          </marker>
        </defs>

        <rect
          x={GROUP.x}
          y={GROUP.y}
          width={GROUP.w}
          height={GROUP.h}
          rx={8}
          fill="none"
          stroke={EDGE}
          strokeWidth={1.25}
        />
        <text x={GROUP.x + 2} y={GROUP.y - 7} fontSize={10.5} fontStyle="italic" fill={MUTED}>
          sentil-core
        </text>

        <Arrow x1={110} y1={STAGE_Y + STAGE_H / 2} x2={STAGES[0].x - 3} y2={STAGE_Y + STAGE_H / 2} />
        <Arrow x1={mid(STAGES[0].x, STAGE_W)} y1={214} x2={mid(STAGES[0].x, STAGE_W)} y2={STAGE_Y + STAGE_H + 3} />
        <Arrow x1={mid(STAGES[1].x, STAGE_W)} y1={66} x2={mid(STAGES[1].x, STAGE_W)} y2={STAGE_Y - 3} />
        {[0, 1].map((i) => (
          <Arrow
            key={i}
            x1={STAGES[i].x + STAGE_W}
            y1={STAGE_Y + STAGE_H / 2}
            x2={STAGES[i + 1].x - 3}
            y2={STAGE_Y + STAGE_H / 2}
          />
        ))}
        <Arrow x1={STAGES[2].x + STAGE_W} y1={STAGE_Y + STAGE_H / 2} x2={657} y2={STAGE_Y + STAGE_H / 2} />

        {STAGES.map((s) => (
          <g key={s.label}>
            <rect
              x={s.x}
              y={STAGE_Y}
              width={STAGE_W}
              height={STAGE_H}
              rx={5}
              fill="var(--color-fd-card)"
              stroke={EDGE}
            />
            <text x={mid(s.x, STAGE_W)} y={STAGE_Y + 24} textAnchor="middle" fontSize={13.5} fontWeight={600} fill={INK}>
              {s.label}
            </text>
            {s.lines.map((t, i) => (
              <text key={t} x={mid(s.x, STAGE_W)} y={STAGE_Y + 43 + i * 14} textAnchor="middle" fontSize={11} fill={MUTED}>
                {t}
              </text>
            ))}
          </g>
        ))}

        <Datum x={8} y={DATUM_Y} lines={['calibration', 'pairs']} />
        <Datum x={660} y={DATUM_Y} lines={['Pr = 0.94', '[0.92, 0.96]']} />
        <Datum x={mid(STAGES[1].x, STAGE_W) - DATUM_W / 2} y={20} lines={['spec', 'P>=0.9 (...)']} />
        <Datum x={mid(STAGES[0].x, STAGE_W) - DATUM_W / 2} y={214} lines={['live reading']} />
        </svg>
      <figcaption className="mt-3 text-center text-sm text-fd-muted-foreground">
        The probabilistic pipeline. Deterministic monitoring is the middle stage running on its own: a formula and a
        trace go in, a robustness value comes out.
      </figcaption>
    </figure>
  );
}