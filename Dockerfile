# One image with everything: the Rust backend also serves the built frontend.

# ---- 1. Build the frontend ----
FROM node:22-slim AS frontend
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# ---- 2. Build the backend ----
FROM rust:1-bookworm AS backend
# cmake is needed by the TLS crypto library (aws-lc) on some platforms.
RUN apt-get update && apt-get install -y --no-install-recommends cmake \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY pricewatchproject/Cargo.toml pricewatchproject/Cargo.lock ./
COPY pricewatchproject/src ./src
# Migrations are embedded into the binary at compile time.
COPY pricewatchproject/migrations ./migrations
RUN cargo build --release --locked --bin pricewatchproject

# ---- 3. Small runtime image ----
FROM debian:bookworm-slim
# Root certificates, so the scraper can make HTTPS requests.
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /app/target/release/pricewatchproject ./pricewatch
COPY --from=frontend /app/dist ./dist
ENV STATIC_DIR=/app/dist
USER 1000
# The server listens on $PORT if the platform sets it, otherwise on 3000.
EXPOSE 3000
CMD ["./pricewatch"]
