/** Tiny price trend for a product card. Decorative: exact values live in the detail view. */
export function Sparkline({ values, width = 120, height = 36 }: { values: number[]; width?: number; height?: number }) {
  if (values.length < 2) return <div className="sparkline-empty" style={{ width, height }} />;

  const pad = 4;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const x = (i: number) => pad + (i / (values.length - 1)) * (width - pad * 2);
  // A flat series sits in the middle instead of on the floor.
  const y = (v: number) =>
    max === min ? height / 2 : pad + (1 - (v - min) / span) * (height - pad * 2);

  // Prices hold until the next check, so draw steps rather than slopes.
  let d = `M${x(0)},${y(values[0])}`;
  for (let i = 1; i < values.length; i++) d += `H${x(i)}V${y(values[i])}`;

  const last = values.length - 1;
  return (
    <svg className="sparkline" width={width} height={height} aria-hidden="true">
      <path d={d} fill="none" stroke="var(--series-1)" strokeWidth="2" strokeLinejoin="round" />
      <circle cx={x(last)} cy={y(values[last])} r="3" fill="var(--series-1)" stroke="var(--surface)" strokeWidth="2" />
    </svg>
  );
}
