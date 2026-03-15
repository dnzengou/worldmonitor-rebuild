# Multi-stage build for WorldMonitor Agents

# Backend build stage
FROM rust:1.75-slim-bookworm AS backend-builder

WORKDIR /app/backend

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src

RUN cargo build --release

# Frontend build stage
FROM node:20-slim AS frontend-builder

WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install

COPY frontend/ ./
RUN npm run build

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/backend/target/release/worldmonitor-agents /app/worldmonitor-agents

# Copy frontend build
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Create data directory
RUN mkdir -p /app/data

ENV RUST_LOG=info
ENV DATABASE_URL=sqlite:/app/data/worldmonitor.db
ENV PORT=8080

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/intelligence || exit 1

CMD ["/app/worldmonitor-agents"]
