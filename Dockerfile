# Build stage
FROM rust:1.84-slim-bookworm AS builder

# Install dependencies for trunk and wasm
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install wasm target and trunk
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk --locked

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

# Custom nginx config for SPA and proper MIME types
RUN echo 'server { \
    listen 80; \
    root /usr/share/nginx/html; \
    index index.html; \
    location / { \
        try_files $uri $uri/ /index.html; \
    } \
    types { \
        application/wasm wasm; \
    } \
    gzip on; \
    gzip_types application/javascript application/wasm text/css; \
}' > /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
