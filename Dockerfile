# Build stage
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
FROM nginx:alpine

# Copy built files to nginx
COPY --from=builder /app/dist /usr/share/nginx/html

# Custom nginx config for SPA, compression, and caching
RUN echo 'server { \
    listen 80; \
    root /usr/share/nginx/html; \
    index index.html; \
    include mime.types; \
    types { \
        application/wasm wasm; \
    } \
    location / { \
        try_files $uri $uri/ /index.html; \
    } \
    gzip on; \
    gzip_vary on; \
    gzip_min_length 1024; \
    gzip_comp_level 6; \
    gzip_types application/javascript application/wasm text/css text/html application/json image/svg+xml; \
    location ~* \.(wasm|js|css|ttf|woff2?)$ { \
        expires 1y; \
        add_header Cache-Control "public, immutable"; \
    } \
    location = /index.html { \
        expires -1; \
        add_header Cache-Control "no-cache"; \
    } \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
