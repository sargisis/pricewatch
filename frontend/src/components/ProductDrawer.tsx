import { useEffect, useMemo, useState, type FormEvent } from "react";
import { api, type PricePoint, type Product } from "../api";
import { dateTime, displayName, hostOf, money, statusOf, timeAgo } from "../format";
import { useToast } from "../toast";
import { CloseIcon, ExternalIcon, RefreshIcon } from "./Icons";
import { PriceChart } from "./PriceChart";
import { StatusBadge } from "./StatusBadge";

interface Props {
  product: Product;
  checking: boolean;
  onClose: () => void;
  onChanged: (p: Product) => void;
  onCheck: () => void;
}

const RANGES = [
  { id: "24h", label: "24 hours", ms: 86400_000 },
  { id: "7d", label: "7 days", ms: 7 * 86400_000 },
  { id: "30d", label: "30 days", ms: 30 * 86400_000 },
  { id: "all", label: "All time", ms: Infinity },
] as const;
type RangeId = (typeof RANGES)[number]["id"];

export function ProductDrawer({ product: p, checking, onClose, onChanged, onCheck }: Props) {
  const toast = useToast();
  const [points, setPoints] = useState<PricePoint[] | null>(null);
  const [range, setRange] = useState<RangeId>("all");

  useEffect(() => {
    let cancelled = false;
    api
      .history(p.id)
      .then((h) => !cancelled && setPoints(h.points))
      .catch(() => !cancelled && setPoints([]));
    return () => {
      cancelled = true;
    };
  }, [p.id, p.last_checked_at]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  // Everything below the range picker (chart, stats, table) uses the same slice.
  const slice = useMemo(() => {
    const ms = RANGES.find((r) => r.id === range)!.ms;
    const since = Date.now() - ms;
    return (points ?? []).filter((pt) => new Date(pt.scraped_at).getTime() >= since);
  }, [points, range]);

  const chartPoints = useMemo(
    () => slice.map((pt) => ({ t: new Date(pt.scraped_at).getTime(), v: Number(pt.price) })).reverse(),
    [slice],
  );

  const stats = useMemo(() => {
    if (chartPoints.length === 0) return null;
    const values = chartPoints.map((c) => c.v);
    const first = values[0];
    const last = values[values.length - 1];
    return {
      min: Math.min(...values),
      max: Math.max(...values),
      avg: values.reduce((a, b) => a + b, 0) / values.length,
      change: first > 0 ? ((last - first) / first) * 100 : 0,
    };
  }, [chartPoints]);

  const rangeLabel = RANGES.find((r) => r.id === range)!.label.toLowerCase();

  return (
    <div className="drawer-overlay" onClick={onClose}>
      <aside className="drawer" role="dialog" aria-modal="true" aria-label={displayName(p)} onClick={(e) => e.stopPropagation()}>
        <header className="drawer-head">
          <div>
            <h2>{displayName(p)}</h2>
            <a href={p.url} target="_blank" rel="noopener noreferrer" className="link">
              {hostOf(p.url)} <ExternalIcon width={13} height={13} />
            </a>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            <CloseIcon />
          </button>
        </header>

        <section className="hero">
          <div>
            <div className="hero-label">Current price</div>
            <div className="hero-price">{money(p.last_price, p.currency)}</div>
            <div className="hero-sub">
              <StatusBadge status={statusOf(p)} />
              {stats && chartPoints.length > 1 && <Change percent={stats.change} label={rangeLabel} />}
            </div>
          </div>
          <div className="hero-side">
            <button className="btn" onClick={onCheck} disabled={checking}>
              <RefreshIcon className={checking ? "spin" : undefined} />
              {checking ? "Checking…" : "Check now"}
            </button>
            <span className="muted">Checked {timeAgo(p.last_checked_at)}</span>
          </div>
        </section>

        {p.last_error && (
          <p className="banner-error" role="alert">
            Last check failed: {p.last_error}
          </p>
        )}

        <div className="filters" role="tablist" aria-label="Time range">
          {RANGES.map((r) => (
            <button key={r.id} role="tab" aria-selected={range === r.id} className={`chip ${range === r.id ? "active" : ""}`} onClick={() => setRange(r.id)}>
              {r.label}
            </button>
          ))}
        </div>

        <section className="card chart-card">
          <h3>Price history</h3>
          {points === null ? (
            <div className="chart-empty">Loading…</div>
          ) : (
            <PriceChart points={chartPoints} target={Number(p.target_price)} currency={p.currency} />
          )}
        </section>

        <section className="tiles tiles-4">
          <MiniTile label="Lowest" value={stats ? money(stats.min, p.currency) : "—"} />
          <MiniTile label="Average" value={stats ? money(stats.avg.toFixed(2), p.currency) : "—"} />
          <MiniTile label="Highest" value={stats ? money(stats.max, p.currency) : "—"} />
          <MiniTile label="Checks" value={String(slice.length)} />
        </section>

        <Settings key={p.id} product={p} onSaved={(np) => { onChanged(np); toast("ok", "Saved"); }} />

        <section className="card">
          <h3>Checks</h3>
          {slice.length === 0 ? (
            <p className="muted">No checks in this period.</p>
          ) : (
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>When</th>
                    <th className="num">Price</th>
                    <th className="num">Change</th>
                  </tr>
                </thead>
                <tbody>
                  {slice.slice(0, 100).map((pt, i) => {
                    const prev = slice[i + 1];
                    const diff = prev ? Number(pt.price) - Number(prev.price) : 0;
                    return (
                      <tr key={pt.id}>
                        <td>{dateTime(pt.scraped_at)}</td>
                        <td className="num">{money(pt.price, pt.currency ?? p.currency)}</td>
                        <td className="num">
                          {diff === 0 ? <span className="muted">—</span> : <Delta value={diff} currency={pt.currency ?? p.currency} />}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          )}
        </section>
      </aside>
    </div>
  );
}

/** A price going down is good news here, so "down" gets the good tone. */
function Change({ percent, label }: { percent: number; label: string }) {
  if (Math.abs(percent) < 0.05) return <span className="muted">No change over {label}</span>;
  const down = percent < 0;
  return (
    <span className={down ? "tone-good" : "tone-critical"}>
      {down ? "▼" : "▲"} {Math.abs(percent).toFixed(1)}% <span className="muted">over {label}</span>
    </span>
  );
}

function Delta({ value, currency }: { value: number; currency: string | null }) {
  const down = value < 0;
  return (
    <span className={down ? "tone-good" : "tone-critical"}>
      {down ? "▼" : "▲"} {money(Math.abs(value).toFixed(2), currency)}
    </span>
  );
}

function MiniTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="card tile">
      <span className="tile-label">{label}</span>
      <span className="tile-value small">{value}</span>
    </div>
  );
}

function Settings({ product: p, onSaved }: { product: Product; onSaved: (p: Product) => void }) {
  const toast = useToast();
  const [name, setName] = useState(p.name ?? "");
  const [target, setTarget] = useState(p.target_price);
  const [selector, setSelector] = useState(p.css_selector ?? "");
  const [busy, setBusy] = useState(false);

  const dirty = name !== (p.name ?? "") || target !== p.target_price || selector !== (p.css_selector ?? "");

  const save = async (e: FormEvent) => {
    e.preventDefault();
    setBusy(true);
    try {
      // Only send fields that changed: a new target re-arms the alert.
      const patch: Parameters<typeof api.updateProduct>[1] = {};
      if (name.trim() && name !== (p.name ?? "")) patch.name = name.trim();
      if (target !== p.target_price) patch.target_price = target.replace(",", ".").trim();
      if (selector.trim() && selector !== (p.css_selector ?? "")) patch.css_selector = selector.trim();
      onSaved(await api.updateProduct(p.id, patch));
    } catch (err) {
      toast("error", err instanceof Error ? err.message : "Could not save");
    } finally {
      setBusy(false);
    }
  };

  return (
    <form className="card settings" onSubmit={save}>
      <h3>Settings</h3>
      <div className="add-row">
        <label className="grow">
          <span>Name</span>
          <input value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="target">
          <span>Target price</span>
          <input inputMode="decimal" required pattern="[0-9]+([.,][0-9]{1,2})?" value={target} onChange={(e) => setTarget(e.target.value)} />
        </label>
      </div>
      <label>
        <span>CSS selector for the price</span>
        <input value={selector} onChange={(e) => setSelector(e.target.value)} placeholder="Found automatically when empty" spellCheck={false} />
      </label>
      <div className="settings-foot">
        <button className="btn btn-primary" disabled={!dirty || busy}>
          {busy ? "Saving…" : "Save changes"}
        </button>
      </div>
    </form>
  );
}
