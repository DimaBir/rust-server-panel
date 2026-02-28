# Stage 1: Build Vue frontend
FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust backend
FROM rust:1.83-bookworm AS backend-builder
WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock* ./
# Create dummy main to cache dependencies
RUN mkdir src && echo 'fn main() {}' > src/main.rs
RUN cargo build --release 2>/dev/null || true
# Now copy real source and build
COPY backend/src/ ./src/
RUN touch src/main.rs && cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy backend binary
COPY --from=backend-builder /app/backend/target/release/rust-server-panel ./rust-server-panel

# Copy frontend dist (served by backend or nginx)
COPY --from=frontend-builder /app/frontend/dist ./static/

# Copy example config
COPY backend/config.example.yaml ./config.example.yaml

EXPOSE 8443

CMD ["./rust-server-panel"]
