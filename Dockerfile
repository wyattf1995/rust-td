# Build stage
# Base images pinned 2026-04-01 — update intentionally
FROM rust:1.85-slim-bookworm AS builder

# Install dependencies for trunk and wasm
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install newer binaryen (wasm-opt) — Debian bookworm's v108 is too old
# for WASM sign-extension ops emitted by Rust 1.85
RUN curl -fsSL https://github.com/WebAssembly/binaryen/releases/download/version_121/binaryen-version_121-x86_64-linux.tar.gz \
    | tar -xzf - -C /usr/local --strip-components=1

# Install wasm target and trunk
RUN rustup target add wasm32-unknown-unknown \
    && curl -fsSL https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf - -C /usr/local/bin

WORKDIR /app

# Copy manifests and cargo config for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY .cargo ./.cargo

# Pre-compile dependencies with dummy source (cached until Cargo.toml/lock changes)
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --target wasm32-unknown-unknown \
    && rm -rf src

# Copy source and assets (only this layer invalidates on code changes)
COPY Trunk.toml ./
COPY src ./src
COPY assets ./assets
COPY index.html ./
COPY robots.txt sitemap.xml ./

# Build release (recompiles only game code, deps are cached)
RUN trunk build --release

# Production stage - serve static files with nginx (unprivileged)
FROM nginx:1.29-alpine

LABEL org.opencontainers.image.source="https://github.com/wyattf1995/rust-td"
LABEL org.opencontainers.image.description="Neon Command - Browser tower defense game"

# Custom nginx config with security headers
# Listens on 8080 (non-privileged port) so nginx can run as non-root
# No SPA catch-all (prevents duplicate content indexing)
RUN echo 'server { \
    listen 8080; \
    root /usr/share/nginx/html; \
    include mime.types; \
    types { \
        application/wasm wasm; \
    } \
    add_header X-Frame-Options "SAMEORIGIN" always; \
    add_header X-Content-Type-Options "nosniff" always; \
    add_header Referrer-Policy "strict-origin-when-cross-origin" always; \
    add_header Content-Security-Policy "default-src '"'"'self'"'"'; script-src '"'"'self'"'"' '"'"'unsafe-inline'"'"' https://analytics.wyatt-fleming.com; connect-src '"'"'self'"'"' https://analytics.wyatt-fleming.com; style-src '"'"'self'"'"' '"'"'unsafe-inline'"'"'; img-src '"'"'self'"'"' data:; font-src '"'"'self'"'"'" always; \
    gzip on; \
    gzip_vary on; \
    gzip_min_length 1024; \
    gzip_comp_level 6; \
    gzip_types application/javascript application/wasm text/css text/html application/json image/svg+xml text/xml; \
    location = / { \
        add_header Cache-Control "no-cache"; \
        try_files /index.html =404; \
    } \
    location = /robots.txt { try_files /robots.txt =404; } \
    location = /sitemap.xml { try_files /sitemap.xml =404; } \
    location /assets/ { \
        expires 1y; \
        add_header Cache-Control "public, immutable"; \
        try_files $uri =404; \
    } \
    location ~* \.(wasm|js|css)$ { \
        expires 1y; \
        add_header Cache-Control "public, immutable"; \
        try_files $uri =404; \
    } \
    location / { \
        return 404; \
    } \
}' > /etc/nginx/conf.d/default.conf \
    && sed -i 's/listen\s*80;/listen 8080;/' /etc/nginx/conf.d/default.conf \
    && chown -R nginx:nginx /var/cache/nginx /var/log/nginx /etc/nginx/conf.d \
    && touch /var/run/nginx.pid && chown nginx:nginx /var/run/nginx.pid

# Copy built files to nginx (owned by nginx user)
COPY --from=builder --chown=nginx:nginx /app/dist /usr/share/nginx/html

# Run as non-root nginx user
USER nginx

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget -q --spider http://127.0.0.1:8080/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
