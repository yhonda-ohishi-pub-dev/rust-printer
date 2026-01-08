# Stage 1: Build
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy src for dependency caching
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies only (for caching)
RUN cargo build --release && \
    rm -rf src target/release/rust-pdf-printer*

# Copy actual source
COPY src ./src

# Build the application
RUN cargo build --release

# Stage 2: Runtime (distroless)
FROM gcr.io/distroless/cc-debian12

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/rust-pdf-printer /app/rust-pdf-printer

# Copy fonts directory (will be mounted or added at build time)
# Font files should be added via docker-compose volumes or build args

# Set environment variables
ENV RUST_LOG=info
ENV CUPS_SERVER=cups-sidecar
ENV CUPS_PORT=631
ENV DEFAULT_PRINTER=Canon_LBP221
ENV LISTEN_ADDR=0.0.0.0:8081
ENV FONT_PATH=/app/fonts/NotoSansJP-Regular.ttf

EXPOSE 8081

ENTRYPOINT ["/app/rust-pdf-printer"]
