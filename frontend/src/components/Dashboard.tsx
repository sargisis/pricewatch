import { useCallback, useEffect, useMemo, useState } from "react";
import { api, type Product, type User } from "../api";
import { displayName, money, statusOf, type Status } from "../format";
import { useToast } from "../toast";
import { AddProduct } from "./AddProduct";
import { LogoMark } from "./Icons";
import { ProductCard } from "./ProductCard";
import { ProductDrawer } from "./ProductDrawer";
import { ThemeToggle } from "./ThemeToggle";

type Filter = "all" | "reached" | "tracking" | "problems";

const FILTERS: { id: Filter; label: string; match: (s: Status) => boolean }[] = [
  { id: "all", label: "All", match: () => true },
  { id: "reached", label: "Target reached", match: (s) => s === "reached" },
  { id: "tracking", label: "Tracking", match: (s) => s === "tracking" || s === "pending" },
  { id: "problems", label: "Needs attention", match: (s) => s === "error" || s === "paused" },
];

// Deals first, then problems, then the rest; newest first inside each group.
const ORDER: Record<Status, number> = { reached: 0, error: 1, pending: 2, tracking: 3, paused: 4 };

export function Dashboard({ user, onLogout }: { user: User; onLogout: () => void }) {
  const toast = useToast();
  const [products, setProducts] = useState<Product[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>("all");
  const [openId, setOpenId] = useState<string | null>(null);
  const [checking, setChecking] = useState<Set<string>>(new Set());

  const load = useCallback(async () => {
    try {
      setProducts(await api.products());
      setLoadError(null);
    } catch (err) {
      setLoadError(err instanceof Error ? err.message : "Could not load products");
    }
  }, []);

  // Poll so prices found by the background scheduler show up on their own.
  // Poll faster while a new product is waiting for its first price.
  const waiting = products?.some((p) => statusOf(p) === "pending") ?? false;
  useEffect(() => {
    load();
    const timer = setInterval(load, waiting ? 2000 : 15000);
    return () => clearInterval(timer);
  }, [load, waiting]);

  const replace = (p: Product) =>
    setProducts((list) => list?.map((x) => (x.id === p.id ? p : x)) ?? null);

  const check = async (p: Product) => {
    setChecking((s) => new Set(s).add(p.id));
    try {
      const res = await api.checkNow(p.id);
      replace(res.product);
      toast("ok", `${displayName(res.product)}: ${money(res.price.price, res.product.currency)}`);
    } catch (err) {
      toast("error", err instanceof Error ? err.message : "Check failed");
      load();
    } finally {
      setChecking((s) => {
        const next = new Set(s);
        next.delete(p.id);
        return next;
      });
    }
  };

  const toggle = async (p: Product) => {
    try {
      replace(await api.updateProduct(p.id, { active: !p.active }));
    } catch (err) {
      toast("error", err instanceof Error ? err.message : "Could not update");
    }
  };

  const remove = async (p: Product) => {
    if (!confirm(`Stop tracking "${displayName(p)}"? Its price history will be deleted.`)) return;
    try {
      await api.deleteProduct(p.id);
      setProducts((list) => list?.filter((x) => x.id !== p.id) ?? null);
      if (openId === p.id) setOpenId(null);
      toast("ok", "Deleted");
    } catch (err) {
      toast("error", err instanceof Error ? err.message : "Could not delete");
    }
  };

  const counts = useMemo(() => {
    const c: Record<Status, number> = { reached: 0, tracking: 0, error: 0, paused: 0, pending: 0 };
    products?.forEach((p) => c[statusOf(p)]++);
    return c;
  }, [products]);

  const visible = useMemo(() => {
    const match = FILTERS.find((f) => f.id === filter)!.match;
    return (products ?? [])
      .filter((p) => match(statusOf(p)))
      .sort(
        (a, b) =>
          ORDER[statusOf(a)] - ORDER[statusOf(b)] || b.created_at.localeCompare(a.created_at),
      );
  }, [products, filter]);

  const open = products?.find((p) => p.id === openId) ?? null;
  const total = products?.length ?? 0;

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <LogoMark />
          <span>PriceWatch</span>
        </div>
        <div className="topbar-right">
          <span className="muted email">{user.email}</span>
          <ThemeToggle />
          <button className="btn btn-ghost" onClick={onLogout}>
            Log out
          </button>
        </div>
      </header>

      <main>
        <section className="tiles">
          <Tile label="Products tracked" value={total} />
          <Tile label="At target price" value={counts.reached} tone={counts.reached ? "good" : undefined} />
          <Tile label="Still waiting" value={counts.tracking + counts.pending} />
          <Tile label="Needs attention" value={counts.error + counts.paused} tone={counts.error ? "critical" : undefined} />
        </section>

        <AddProduct onAdded={(p) => setProducts((list) => [p, ...(list ?? [])])} />

        {loadError && (
          <p className="banner-error" role="alert">
            {loadError}
          </p>
        )}

        {total > 0 && (
          <div className="filters" role="tablist" aria-label="Filter products">
            {FILTERS.map((f) => (
              <button
                key={f.id}
                role="tab"
                aria-selected={filter === f.id}
                className={`chip ${filter === f.id ? "active" : ""}`}
                onClick={() => setFilter(f.id)}
              >
                {f.label}
              </button>
            ))}
          </div>
        )}

        {products === null && !loadError ? (
          <div className="grid">
            {[0, 1, 2].map((i) => (
              <div key={i} className="card skeleton" />
            ))}
          </div>
        ) : total === 0 ? (
          <div className="empty">
            <h2>Nothing tracked yet</h2>
            <p>Paste a product link above and set the price you'd like to pay. The first price shows up in a few seconds.</p>
          </div>
        ) : visible.length === 0 ? (
          <div className="empty">
            <p>No products match this filter.</p>
          </div>
        ) : (
          <div className="grid">
            {visible.map((p) => (
              <ProductCard
                key={p.id}
                product={p}
                checking={checking.has(p.id)}
                onOpen={() => setOpenId(p.id)}
                onCheck={() => check(p)}
                onToggle={() => toggle(p)}
                onDelete={() => remove(p)}
              />
            ))}
          </div>
        )}
      </main>

      {open && (
        <ProductDrawer
          product={open}
          checking={checking.has(open.id)}
          onClose={() => setOpenId(null)}
          onChanged={replace}
          onCheck={() => check(open)}
        />
      )}
    </div>
  );
}

function Tile({ label, value, tone }: { label: string; value: number; tone?: "good" | "critical" }) {
  return (
    <div className="card tile">
      <span className="tile-label">{label}</span>
      <span className={`tile-value ${tone ? `tone-${tone}` : ""}`}>{value}</span>
    </div>
  );
}
