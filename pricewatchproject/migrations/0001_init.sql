CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE products (
    id              UUID PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    url             TEXT NOT NULL,
    name            TEXT,
    -- Optional CSS selector for stores where automatic detection fails.
    css_selector    TEXT,
    target_price    NUMERIC(14, 2) NOT NULL CHECK (target_price > 0),
    currency        TEXT,
    last_price      NUMERIC(14, 2),
    last_checked_at TIMESTAMPTZ,
    last_error      TEXT,
    -- Set when a "price reached target" notification was sent; cleared when
    -- the price goes back above the target so the next drop alerts again.
    notified_at     TIMESTAMPTZ,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, url)
);

CREATE INDEX products_due_idx ON products (last_checked_at NULLS FIRST) WHERE active;

CREATE TABLE price_history (
    id         BIGSERIAL PRIMARY KEY,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    price      NUMERIC(14, 2) NOT NULL,
    currency   TEXT,
    scraped_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX price_history_product_idx ON price_history (product_id, scraped_at DESC);
