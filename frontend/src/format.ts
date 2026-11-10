import type { Product } from "./api";

export function money(value: string | number | null | undefined, currency?: string | null): string {
  if (value === null || value === undefined || value === "") return "—";
  const n = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(n)) return "—";
  const digits = Number.isInteger(n) ? 0 : 2;
  if (currency) {
    try {
      return new Intl.NumberFormat(undefined, {
        style: "currency",
        currency,
        minimumFractionDigits: digits,
        maximumFractionDigits: 2,
      }).format(n);
    } catch {
      /* unknown currency code: fall through */
    }
  }
  const plain = new Intl.NumberFormat(undefined, {
    minimumFractionDigits: digits,
    maximumFractionDigits: 2,
  }).format(n);
  return currency ? `${plain} ${currency}` : plain;
}

const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto", style: "short" });

export function timeAgo(iso: string | null): string {
  if (!iso) return "never";
  const seconds = (new Date(iso).getTime() - Date.now()) / 1000;
  const abs = Math.abs(seconds);
  if (abs < 45) return "just now";
  if (abs < 3600) return rtf.format(Math.round(seconds / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(seconds / 3600), "hour");
  return rtf.format(Math.round(seconds / 86400), "day");
}

export function dateTime(iso: string | number): string {
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

export function displayName(p: Product): string {
  return p.name || hostOf(p.url);
}

export type Status = "reached" | "tracking" | "error" | "paused" | "pending";

export function statusOf(p: Product): Status {
  if (!p.active) return "paused";
  if (p.last_error) return "error";
  if (p.last_price === null) return "pending";
  return Number(p.last_price) <= Number(p.target_price) ? "reached" : "tracking";
}

/** How far the current price is above the target, as a percentage of the current price. */
export function percentAboveTarget(p: Product): number | null {
  if (p.last_price === null) return null;
  const price = Number(p.last_price);
  const target = Number(p.target_price);
  if (!(price > 0)) return null;
  return ((price - target) / price) * 100;
}
