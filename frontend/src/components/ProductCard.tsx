import { useEffect, useState } from "react";
import { api, type Product } from "../api";
import { displayName, hostOf, money, percentAboveTarget, statusOf, timeAgo } from "../format";
import { PauseIcon, PlayIcon, RefreshIcon, TrashIcon } from "./Icons";
import { Sparkline } from "./Sparkline";
import { StatusBadge } from "./StatusBadge";

interface Props {
  product: Product;
  checking: boolean;
  onOpen: () => void;
  onCheck: () => void;
  onToggle: () => void;
  onDelete: () => void;
}

export function ProductCard({ product: p, checking, onOpen, onCheck, onToggle, onDelete }: Props) {
  const [trend, setTrend] = useState<number[]>([]);
  const status = statusOf(p);
  const above = percentAboveTarget(p);

  // Reload the mini chart whenever a new check lands.
  useEffect(() => {
    let cancelled = false;
    api
      .history(p.id, 30)
      .then((h) => !cancelled && setTrend(h.points.map((pt) => Number(pt.price)).reverse()))
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [p.id, p.last_checked_at]);

  const stop = (fn: () => void) => (e: React.MouseEvent) => {
    e.stopPropagation();
    fn();
  };

  return (
    <article className={`card product status-${status}`}>
      <button className="product-main" onClick={onOpen} aria-label={`Open ${displayName(p)}`}>
        <header>
          <span className="avatar" aria-hidden="true">
            {hostOf(p.url).charAt(0).toUpperCase()}
          </span>
          <div className="product-title">
            <h3>{displayName(p)}</h3>
            <span className="muted">{hostOf(p.url)}</span>
          </div>
        </header>

        <div className="product-price">
          <div>
            <div className="price">{money(p.last_price, p.currency)}</div>
            <div className="muted">
              Target {money(p.target_price, p.currency)}
              {above !== null && above > 0 && <> · {above.toFixed(above < 10 ? 1 : 0)}% to go</>}
            </div>
          </div>
          <Sparkline values={trend} />
        </div>

        {status === "error" && <p className="product-error">{p.last_error}</p>}
      </button>

      <footer>
        <StatusBadge status={status} />
        <span className="muted checked" title={p.last_checked_at ?? undefined}>
          {p.last_checked_at ? timeAgo(p.last_checked_at) : ""}
        </span>
        <div className="actions">
          <button className="icon-btn" onClick={stop(onCheck)} disabled={checking} title="Check price now" aria-label="Check price now">
            <RefreshIcon className={checking ? "spin" : undefined} />
          </button>
          <button className="icon-btn" onClick={stop(onToggle)} title={p.active ? "Pause tracking" : "Resume tracking"} aria-label={p.active ? "Pause tracking" : "Resume tracking"}>
            {p.active ? <PauseIcon /> : <PlayIcon />}
          </button>
          <button className="icon-btn danger" onClick={stop(onDelete)} title="Stop tracking and delete" aria-label="Delete">
            <TrashIcon />
          </button>
        </div>
      </footer>
    </article>
  );
}
