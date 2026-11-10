# PriceWatch

A price tracking service written in Rust. It monitors product pages on online stores, records price history in PostgreSQL, and sends a notification when a price drops to the user's target.

**Stack:** Axum · Tokio · SQLx (PostgreSQL) · reqwest · scraper · JWT + Argon2 · React + TypeScript + Vite

## Features

- **REST API.** Users register and log in with JWT auth, then add, edit and delete the products they track.
- **Scraping engine.** It finds the price on most store pages without any per-store code. It tries these sources in order:
  1. the product's own CSS selector, if one is set
  2. schema.org JSON-LD (`offers.price`, `AggregateOffer.lowPrice`, `@graph`)
  3. price meta tags (`product:price:amount`, `og:price:amount`, `itemprop="price"`)

  The parser reads US, European and Russian number formats (`$1,299.99`, `1.299,90 €`, `89 990 ₽`) and detects the currency.
- **Background scheduler.** It re-checks every product on an interval and scrapes several pages at once, up to a set limit. Failed checks are recorded on the product.
- **Notifications.** An alert goes out once when the price reaches the target. It goes out again only if the price rises back above the target and then drops again. Alerts are logged, and they're also POSTed to a Slack- or Discord-compatible webhook if you configure one.

- **Web app.** A React dashboard with light and dark themes that works on phones. It shows every tracked product with its current price, status and a mini trend chart. Opening a product shows its full price history chart against the target, min/average/max for the chosen period, and the list of checks.

## Project layout

```
pricewatchproject/
├── migrations/      SQL schema (users, products, price_history)
├── src/
│   ├── api/         HTTP handlers: auth, products, history, auth extractor
│   ├── domain/      data types
│   ├── repo/        database queries
│   ├── scraper/     fetch page → find price → parse number
│   ├── scheduler/   background loop + single-product check job
│   ├── notify.rs    alert delivery
│   └── main.rs      startup, migrations, graceful shutdown
└── tests/           price parsing + API tests
frontend/
└── src/
    ├── api.ts       typed API client
    ├── components/  dashboard, product card, detail drawer, price chart
    └── styles.css   theme tokens and layout
```

## Running

```bash
cd pricewatchproject
cp .env.example .env          # set DATABASE_URL and JWT_SECRET
createdb pricewatch           # or: docker run -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres:17
cargo run
```

Migrations run automatically on startup.

### Web app

For development, run the frontend dev server next to the backend. It proxies `/api` to `localhost:3000`:

```bash
cd frontend
npm install
npm run dev                   # http://localhost:5173
```

For production, build it once. The backend serves `frontend/dist` itself, so the whole app is on one address:

```bash
cd frontend && npm run build
cd ../pricewatchproject && cargo run    # http://localhost:3000
```

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | required | PostgreSQL connection string |
| `JWT_SECRET` | required | at least 16 characters |
| `BIND_ADDR` | `0.0.0.0:3000` | if unset, the `PORT` variable is used when the host provides one |
| `CHECK_INTERVAL_SECS` | `3600` | how often each product is re-checked |
| `SCHEDULER_TICK_SECS` | `60` | how often the scheduler looks for due products |
| `SCRAPE_CONCURRENCY` | `4` | pages scraped at the same time |
| `NOTIFY_WEBHOOK_URL` | none | Slack/Discord webhook for alerts |
| `STATIC_DIR` | `../frontend/dist` | built frontend to serve, if the folder exists |

## Deploying for free (Render + Neon)

The repository has a `Dockerfile` that builds the frontend and backend into one image, and a `render.yaml` blueprint for [Render](https://render.com).

1. Create a free PostgreSQL database at [neon.tech](https://neon.tech) and copy its connection string (it ends with `?sslmode=require`).
2. On Render choose **New + → Blueprint** and pick this repository.
3. Paste the connection string as `DATABASE_URL`. `JWT_SECRET` is generated for you.
4. After the build, the app is live at `https://<name>.onrender.com`. Every push to `main` redeploys it.

On the free plan Render puts the service to sleep after about 15 minutes without visits. While it sleeps the scheduler doesn't check prices, and the first visit takes up to a minute. A free uptime monitor that opens `/health` every few minutes keeps it awake.

## API

All `/api/products` routes need an `Authorization: Bearer <token>` header.

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/auth/register` | `{email, password}` → `{token, user}` |
| `POST` | `/api/auth/login` | `{email, password}` → `{token, user}` |
| `GET` | `/api/me` | current user |
| `GET` | `/api/products` | list tracked products |
| `POST` | `/api/products` | `{url, target_price, name?, css_selector?}`; the first check starts right away |
| `GET` | `/api/products/{id}` | one product |
| `PATCH` | `/api/products/{id}` | change `target_price`, `name`, `css_selector`, `active` |
| `DELETE` | `/api/products/{id}` | stop tracking |
| `POST` | `/api/products/{id}/check` | scrape now and return the new price |
| `GET` | `/api/products/{id}/history?limit=100` | price points (newest first) + min/max/avg |
| `GET` | `/health` | liveness check |

Example:

```bash
TOKEN=$(curl -s localhost:3000/api/auth/register -H 'content-type: application/json' \
  -d '{"email":"me@example.com","password":"supersecret"}' | jq -r .token)

curl localhost:3000/api/products -H "authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"url":"https://store.example/item/123","target_price":"499.00"}'
```

## Tests

```bash
cargo test
# end-to-end test against a real database (register → track → check → history):
TEST_DATABASE_URL=postgres://postgres:postgres@localhost/pricewatch_test cargo test
```

## Limitations

- Pages that render their price only with JavaScript can't be read, because the scraper doesn't run a browser.
- Some stores block automated requests or show a captcha. The check then fails, and the error is saved in the product's `last_error`.
