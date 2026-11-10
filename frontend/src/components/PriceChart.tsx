import { useEffect, useMemo, useRef, useState } from "react";
import { dateTime, money } from "../format";

export interface ChartPoint {
  t: number;
  v: number;
}

interface Props {
  /** Oldest first. */
  points: ChartPoint[];
  target: number;
  currency: string | null;
}

const HEIGHT = 260;
const M = { top: 20, right: 16, bottom: 28, left: 64 };

/** Round tick values (1/2/5 × 10ⁿ) covering [min, max]. */
function niceTicks(min: number, max: number, count = 4): number[] {
  const span = max - min || 1;
  const raw = span / count;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 5, 10].map((m) => m * mag).find((s) => s >= raw) ?? raw;
  const ticks: number[] = [];
  for (let v = Math.ceil(min / step) * step; v <= max + step * 1e-9; v += step) {
    ticks.push(Number(v.toFixed(6)));
  }
  return ticks;
}

export function PriceChart({ points, target, currency }: Props) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(600);
  const [hover, setHover] = useState<number | null>(null);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) return;
    const ro = new ResizeObserver(([entry]) => setWidth(Math.max(280, entry.contentRect.width)));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const geom = useMemo(() => {
    if (points.length === 0) return null;
    const values = points.map((p) => p.v);
    let lo = Math.min(...values, target);
    let hi = Math.max(...values, target);
    const pad = (hi - lo) * 0.12 || Math.max(hi * 0.05, 1);
    lo = Math.max(0, lo - pad);
    hi += pad;

    let t0 = points[0].t;
    let t1 = points[points.length - 1].t;
    if (t0 === t1) {
      t0 -= 3600_000;
      t1 += 3600_000;
    }

    const innerW = width - M.left - M.right;
    const innerH = HEIGHT - M.top - M.bottom;
    const x = (t: number) => M.left + ((t - t0) / (t1 - t0)) * innerW;
    const y = (v: number) => M.top + (1 - (v - lo) / (hi - lo)) * innerH;

    // Step line: a price holds until the next check finds a new one.
    let line = `M${x(points[0].t)},${y(points[0].v)}`;
    for (let i = 1; i < points.length; i++) line += `H${x(points[i].t)}V${y(points[i].v)}`;
    const baseline = M.top + innerH;
    const area = `${line}V${baseline}H${x(points[0].t)}Z`;

    const spanMs = t1 - t0;
    const xTickCount = Math.max(2, Math.min(5, Math.floor(innerW / 120)));
    const xTicks = Array.from({ length: xTickCount }, (_, i) => t0 + (spanMs * i) / (xTickCount - 1));
    const xLabel = (t: number) =>
      spanMs < 2 * 86400_000
        ? new Date(t).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })
        : new Date(t).toLocaleDateString(undefined, { month: "short", day: "numeric" });

    return { x, y, line, area, baseline, yTicks: niceTicks(lo, hi), xTicks, xLabel, innerW };
  }, [points, target, width]);

  if (!geom) {
    return (
      <div ref={wrapRef} className="chart-empty">
        No prices in this period yet.
      </div>
    );
  }

  const { x, y } = geom;
  const active = hover !== null && hover < points.length ? points[hover] : null;

  const nearest = (clientX: number, rect: DOMRect) => {
    const px = clientX - rect.left;
    let best = 0;
    for (let i = 1; i < points.length; i++) {
      if (Math.abs(x(points[i].t) - px) < Math.abs(x(points[best].t) - px)) best = i;
    }
    return best;
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    e.preventDefault();
    const cur = hover ?? points.length - 1;
    const next = cur + (e.key === "ArrowLeft" ? -1 : 1);
    setHover(Math.max(0, Math.min(points.length - 1, next)));
  };

  const targetY = y(target);
  // Put the target label at the end where the price line is farther away from it.
  const labelLeft =
    Math.abs(points[0].v - target) > Math.abs(points[points.length - 1].v - target);
  // Keep the tooltip inside the chart on both edges.
  const tipLeft = active ? Math.max(70, Math.min(width - 70, x(active.t))) : 0;

  return (
    <div ref={wrapRef} className="chart">
      <svg
        width={width}
        height={HEIGHT}
        role="img"
        aria-label={`Price history, ${points.length} checks. Use arrow keys to step through values.`}
        tabIndex={0}
        onKeyDown={onKey}
        onBlur={() => setHover(null)}
        onPointerMove={(e) => setHover(nearest(e.clientX, e.currentTarget.getBoundingClientRect()))}
        onPointerLeave={() => setHover(null)}
      >
        <defs>
          <linearGradient id="pw-area" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stopColor="var(--series-1)" stopOpacity="0.22" />
            <stop offset="1" stopColor="var(--series-1)" stopOpacity="0" />
          </linearGradient>
        </defs>

        {geom.yTicks.map((v) => (
          <g key={v}>
            <line className="chart-grid" x1={M.left} x2={width - M.right} y1={y(v)} y2={y(v)} />
            <text className="chart-tick" x={M.left - 10} y={y(v)} dy="0.32em" textAnchor="end">
              {money(v, null)}
            </text>
          </g>
        ))}
        {geom.xTicks.map((t, i) => (
          <text
            key={t}
            className="chart-tick"
            x={x(t)}
            y={HEIGHT - 8}
            textAnchor={i === 0 ? "start" : i === geom.xTicks.length - 1 ? "end" : "middle"}
          >
            {geom.xLabel(t)}
          </text>
        ))}

        <path d={geom.area} fill="url(#pw-area)" />

        {/* Target: a dashed reference line with a text label, so it never relies on color. */}
        <line className="chart-target" x1={M.left} x2={width - M.right} y1={targetY} y2={targetY} />
        <text
          className="chart-target-label"
          x={labelLeft ? M.left + 6 : width - M.right}
          y={targetY}
          dy={targetY < M.top + 18 ? 14 : -6}
          textAnchor={labelLeft ? "start" : "end"}
        >
          Target {money(target, currency)}
        </text>

        <path d={geom.line} fill="none" stroke="var(--series-1)" strokeWidth="2" strokeLinejoin="round" />

        {points.length <= 40 &&
          points.map((p) => (
            <circle key={p.t} cx={x(p.t)} cy={y(p.v)} r="3.5" fill="var(--series-1)" stroke="var(--surface)" strokeWidth="2" />
          ))}

        {active && (
          <g pointerEvents="none">
            <line className="chart-crosshair" x1={x(active.t)} x2={x(active.t)} y1={M.top} y2={geom.baseline} />
            <circle cx={x(active.t)} cy={y(active.v)} r="5" fill="var(--series-1)" stroke="var(--surface)" strokeWidth="2" />
          </g>
        )}
      </svg>

      {active && (
        <div className="chart-tip" style={{ left: tipLeft, top: Math.max(0, y(active.v) - 12) }}>
          <strong>{money(active.v, currency)}</strong>
          <span>{dateTime(active.t)}</span>
        </div>
      )}
    </div>
  );
}
