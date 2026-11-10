// Typed client for the PriceWatch REST API.

export interface User {
  id: string;
  email: string;
  created_at: string;
}

export interface Product {
  id: string;
  url: string;
  name: string | null;
  css_selector: string | null;
  /** Decimal values come from the API as strings, e.g. "499.00". */
  target_price: string;
  currency: string | null;
  last_price: string | null;
  last_checked_at: string | null;
  last_error: string | null;
  notified_at: string | null;
  active: boolean;
  created_at: string;
}

export interface PricePoint {
  id: number;
  product_id: string;
  price: string;
  currency: string | null;
  scraped_at: string;
}

export interface PriceStats {
  min_price: string | null;
  max_price: string | null;
  avg_price: string | null;
  samples: number;
}

export interface History {
  product_id: string;
  stats: PriceStats;
  /** Newest first. */
  points: PricePoint[];
}

export interface NewProduct {
  url: string;
  target_price: string;
  name?: string;
  css_selector?: string;
}

export type ProductPatch = Partial<Pick<Product, "target_price" | "name" | "css_selector" | "active">>;

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

const TOKEN_KEY = "pw-token";

export const token = {
  get: (): string | null => {
    try {
      return localStorage.getItem(TOKEN_KEY);
    } catch {
      return null;
    }
  },
  set: (value: string | null) => {
    try {
      if (value) localStorage.setItem(TOKEN_KEY, value);
      else localStorage.removeItem(TOKEN_KEY);
    } catch {
      /* storage unavailable: session lasts until reload */
    }
  },
};

/** Called when the server rejects our token, so the app can show the login screen. */
let onUnauthorized: () => void = () => {};
export function setUnauthorizedHandler(fn: () => void) {
  onUnauthorized = fn;
}

async function request<T>(method: string, path: string, body?: unknown, auth = true): Promise<T> {
  const headers: Record<string, string> = {};
  if (body !== undefined) headers["Content-Type"] = "application/json";
  const t = token.get();
  if (auth && t) headers["Authorization"] = `Bearer ${t}`;

  let resp: Response;
  try {
    resp = await fetch(`/api${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });
  } catch {
    throw new ApiError(0, "Can't reach the server. Is the backend running?");
  }

  if (resp.status === 204) return undefined as T;

  const data = await resp.json().catch(() => null);
  if (!resp.ok) {
    if (resp.status === 401 && auth) onUnauthorized();
    const message =
      (data && typeof data.error === "string" && data.error) || `Request failed (${resp.status})`;
    throw new ApiError(resp.status, message);
  }
  return data as T;
}

interface AuthResponse {
  token: string;
  user: User;
}

export const api = {
  register: (email: string, password: string) =>
    request<AuthResponse>("POST", "/auth/register", { email, password }, false),
  login: (email: string, password: string) =>
    request<AuthResponse>("POST", "/auth/login", { email, password }, false),
  me: () => request<User>("GET", "/me"),

  products: () => request<Product[]>("GET", "/products"),
  createProduct: (p: NewProduct) => request<Product>("POST", "/products", p),
  updateProduct: (id: string, patch: ProductPatch) =>
    request<Product>("PATCH", `/products/${id}`, patch),
  deleteProduct: (id: string) => request<void>("DELETE", `/products/${id}`),
  checkNow: (id: string) =>
    request<{ price: PricePoint; product: Product }>("POST", `/products/${id}/check`),
  history: (id: string, limit = 1000) =>
    request<History>("GET", `/products/${id}/history?limit=${limit}`),
};
