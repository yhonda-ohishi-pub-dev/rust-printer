# Stage 1: Build Rust app (static linking with musl)
FROM rust:1.88-alpine AS rust-builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy src for dependency caching
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs

# Build dependencies only (for caching)
RUN cargo build --release && \
    rm -rf src target/release/deps/rust_pdf_printer* target/release/rust-pdf-printer*

# Copy actual source
COPY src ./src

# Touch source files to force rebuild and build the application
RUN touch src/main.rs && cargo build --release

# Stage 2: Build ppm2pwg
FROM alpine:3.21 AS ppm2pwg-builder

RUN apk add --no-cache git make g++

WORKDIR /build
RUN git clone --recursive https://github.com/attah/ppm2pwg.git && \
    cd ppm2pwg && \
    make ppm2pwg

# Stage 3: Runtime (alpine for PDF conversion tools)
FROM alpine:3.21

# Install poppler-utils for pdftoppm (PDF to image conversion)
RUN apk add --no-cache poppler-utils libstdc++

WORKDIR /app

# Copy ppm2pwg binary
COPY --from=ppm2pwg-builder /build/ppm2pwg/ppm2pwg /usr/local/bin/ppm2pwg

# Copy Rust binary
COPY --from=rust-builder /app/target/release/rust-pdf-printer /app/rust-pdf-printer

# Create fonts directory
RUN mkdir -p /app/fonts

# Set environment variables
ENV RUST_LOG=info
ENV CUPS_SERVER=cups-sidecar
ENV CUPS_PORT=631
ENV DEFAULT_PRINTER=Canon_LBP221
ENV LISTEN_ADDR=0.0.0.0:8081
ENV FONT_PATH=/app/fonts/NotoSansJP-Regular.ttf

EXPOSE 8081

ENTRYPOINT ["/app/rust-pdf-printer"]
