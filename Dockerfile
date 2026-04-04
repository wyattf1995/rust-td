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
RUN rustup target add wasm32-unknown-unknown
RUN curl -fsSL https://github.com/trunk-rs/trunk/releases/download/v0.21.14/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf - -C /usr/local/bin

WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./
COPY Trunk.toml ./

# Copy source and assets
COPY src ./src
COPY assets ./assets
COPY index.html ./

# Build release
RUN trunk build --release

# Production stage - serve static files with nginx
FROM nginx:1.29-alpine

# Copy built files to nginx
COPY --from=builder /app/dist /usr/share/nginx/html

# Custom nginx config — no SPA catch-all (prevents duplicate content indexing)
RUN echo 'server { \
    listen 80; \
    root /usr/share/nginx/html; \
    include mime.types; \
    types { \
        application/wasm wasm; \
    } \
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
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD wget -q --spider http://127.0.0.1:80/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
