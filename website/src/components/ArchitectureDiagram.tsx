import { brandIcons } from '@/lib/brand-icons';
import { languageIcons } from '@/lib/language-icons';

const W = 760;
const H = 424;

const RULE = 'var(--sentil-figure-edge)';
const INK = 'var(--color-fd-foreground)';
const MUTED = 'var(--color-fd-muted-foreground)';
const ACCENT = 'var(--color-fd-primary)';
const LINE = 'var(--sentil-figure-line)';

const CORE = { x: 260, y: 40, w: 240, h: 60 };
const LINKED_Y = 152;
const LINKED_H = 64;
const LINKED_W = 168;
const LINKED_X = [20, 204, 388, 572];
const CHIP_Y = 272;
const CHIP_H = 84;
const CHIP_W = 84;
const CHIP_GAP = 12;
const CHIP_X0 = (W - (8 * CHIP_W + 7 * CHIP_GAP)) / 2;
const LEGEND_Y = 392;

const ICON = 22;

const LINKED = [
  { icon: 'python', label: 'sentil-py', sub: 'PyO3 extension' },
  { glyph: 'terminal', label: 'sentil-cli', sub: 'the sentil binary' },
  { label: 'sentil-ffi', sub: 'C ABI, libsentil', accent: true },
  { icon: 'arduino', label: 'sentil-embedded', sub: 'no_std subset', dashed: true },
];

const OVER_ABI = [
  { icon: 'c', label: 'C', how: 'sentil.h' },
  { icon: 'cpp', label: 'C++', how: 'wrapper' },
  { icon: 'java', label: 'Java', how: 'JNI' },
  { icon: 'julia', label: 'Julia', how: 'ccall' },
  { icon: 'matlab', label: 'MATLAB', how: 'MEX' },
  { icon: 'ros', label: 'ROS 2', how: 'node' },
  { icon: 'apollo', label: 'Apollo', how: 'Cyber RT' },
  { icon: 'autosar', label: 'AUTOSAR', how: 'Adaptive' },
];

const mid = (x: number, w: number) => x + w / 2;

// The shell mark is a near-black box that disappears on the dark card, so the
// terminal is drawn rather than borrowed.
function Glyph({ name, x, y, size }: { name: string; x: number; y: number; size: number }) {
  const s = size / 24;
  return (
    <g transform={`translate(${x} ${y}) scale(${s})`} fill="none" stroke={MUTED} strokeWidth={1.8}>
      {name === 'terminal' && (
        <>
          <rect x="1.6" y="3.6" width="20.8" height="16.8" rx="2.4" />
          <path d="M6.2 9.2 9.6 12l-3.4 2.8" strokeLinecap="round" strokeLinejoin="round" />
          <path d="M12.4 15.4h5.4" strokeLinecap="round" />
        </>
      )}
    </g>
  );
}

// The marks arrive centred and scaled to fill their viewBox, so placing one is
// just a matter of mapping that box onto the size wanted here.
function Mark({ name, cx, cy, size = ICON }: { name: string; cx: number; cy: number; size?: number }) {
  const { viewBox, markup } = languageIcons[name] ?? brandIcons[name];
  const [vx, vy, vw, vh] = viewBox.split(' ').map(Number);
  const scale = size / Math.max(vw, vh);
  return (
    <g
      transform={`translate(${cx - (vx + vw / 2) * scale} ${cy - (vy + vh / 2) * scale}) scale(${scale})`}
      dangerouslySetInnerHTML={{ __html: markup }}
    />
  );
}

function Elbow({
  x1,
  y1,
  x2,
  y2,
  dashed,
  turnAt,
}: {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  dashed?: boolean;
  turnAt?: number;
}) {
  return (
    <path
      d={`M${x1} ${y1} V${turnAt ?? (y1 + y2) / 2} H${x2} V${y2}`}
      fill="none"
      stroke={LINE}
      strokeWidth={1.25}
      strokeDasharray={dashed ? '3 4' : undefined}
    />
  );
}

export function ArchitectureDiagram() {
  const coreMid = mid(CORE.x, CORE.w);
  const coreBottom = CORE.y + CORE.h;
  const ffiMid = mid(LINKED_X[2], LINKED_W);
  const ffiBottom = LINKED_Y + LINKED_H;
  const chipX = (i: number) => CHIP_X0 + i * (CHIP_W + CHIP_GAP);

  return (
    <figure className="not-prose my-8">
      <div className="figure-scroll">
        <svg
        viewBox={`0 0 ${W} ${H}`}
        className="w-full"
      >
        {LINKED.map((n, i) => (
          <Elbow
            key={n.label}
            x1={coreMid}
            y1={coreBottom}
            x2={mid(LINKED_X[i], LINKED_W)}
            y2={LINKED_Y}
            dashed={n.dashed}
            turnAt={n.dashed ? coreBottom + 14 : undefined}
          />
        ))}
        {OVER_ABI.map((_, i) => (
          <Elbow key={i} x1={ffiMid} y1={ffiBottom} x2={mid(chipX(i), CHIP_W)} y2={CHIP_Y} />
        ))}

        <rect
          x={CORE.x}
          y={CORE.y}
          width={CORE.w}
          height={CORE.h}
          rx={5}
          fill="var(--color-fd-card)"
          stroke={ACCENT}
        />
        <g className="mark-rust">
          <Mark name="rust" cx={CORE.x + 30} cy={CORE.y + CORE.h / 2} size={22} />
        </g>
        <text x={coreMid + 15} y={CORE.y + CORE.h / 2 - 2} textAnchor="middle" fontSize={14} fontWeight={600} fill={INK}>
          sentil-core
        </text>
        <text x={coreMid + 15} y={CORE.y + CORE.h / 2 + 16} textAnchor="middle" fontSize={11.5} fill={MUTED}>
          the Rust engine
        </text>

        {LINKED.map((n, i) => {
          const x = LINKED_X[i];
          const textMid = n.icon || n.glyph ? x + (LINKED_W + 32) / 2 : mid(x, LINKED_W);
          return (
            <g key={n.label}>
              <rect
                x={x}
                y={LINKED_Y}
                width={LINKED_W}
                height={LINKED_H}
                rx={5}
                fill="var(--color-fd-card)"
                stroke={n.accent ? ACCENT : RULE}
              />
              {n.icon && <Mark name={n.icon} cx={x + 22} cy={LINKED_Y + LINKED_H / 2} size={19} />}
              {n.glyph && <Glyph name={n.glyph} x={x + 12} y={LINKED_Y + LINKED_H / 2 - 10} size={20} />}
              <text x={textMid} y={LINKED_Y + LINKED_H / 2 - 2} textAnchor="middle" fontSize={14} fontWeight={600} fill={INK}>
                {n.label}
              </text>
              <text x={textMid} y={LINKED_Y + LINKED_H / 2 + 16} textAnchor="middle" fontSize={11.5} fill={MUTED}>
                {n.sub}
              </text>
            </g>
          );
        })}

        {OVER_ABI.map((b, i) => {
          const x = chipX(i);
          const cx = mid(x, CHIP_W);
          return (
            <g key={b.label}>
              <rect x={x} y={CHIP_Y} width={CHIP_W} height={CHIP_H} rx={5} fill="var(--color-fd-card)" stroke={RULE} />
              {b.icon && <Mark name={b.icon} cx={cx} cy={CHIP_Y + 27} />}
              <text x={cx} y={CHIP_Y + 58} textAnchor="middle" fontSize={12} fontWeight={600} fill={INK}>
                {b.label}
              </text>
              <text x={cx} y={CHIP_Y + 73} textAnchor="middle" fontSize={10} fill={MUTED}>
                {b.how}
              </text>
            </g>
          );
        })}

        <line x1={CHIP_X0} y1={LEGEND_Y - 16} x2={W - CHIP_X0} y2={LEGEND_Y - 16} stroke={RULE} strokeWidth={1.25} />
        <Mark name="raspberrypi" cx={CHIP_X0 + 8} cy={LEGEND_Y} size={17} />
        <text x={CHIP_X0 + 22} y={LEGEND_Y + 4} fontSize={11} fill={MUTED}>
          the full build runs on a Linux board
        </text>
        <Mark name="arduino" cx={CHIP_X0 + 396} cy={LEGEND_Y} size={17} />
        <text x={CHIP_X0 + 410} y={LEGEND_Y + 4} fontSize={11} fill={MUTED}>
          the no_std build runs on a microcontroller
        </text>
        </svg>
      </div>
      <figcaption className="mt-3 text-center text-sm text-fd-muted-foreground">
        The solid edges are a link against compiled code and the dashed edge is the core recompiled as a <code>no_std</code> subset.
      </figcaption>
    </figure>
  );
}