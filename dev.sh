#!/bin/bash
# 開発用スクリプト - ビルド、起動、テスト、停止を一括管理

set -e

PROJECT_DIR="/home/yutaka/rust/rust-printer"
RUST_APP_DIR="$PROJECT_DIR/rust-app"
BINARY="$RUST_APP_DIR/target/debug/rust-pdf-printer"
DEV_PORT=8084
FONT_PATH="$PROJECT_DIR/fonts/ipaexg.ttf"
PRINTER_IP="${PRINTER_IP:-192.168.0.76}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_port() {
    if lsof -i :$DEV_PORT >/dev/null 2>&1; then
        log_warn "Port $DEV_PORT is in use:"
        lsof -i :$DEV_PORT
        return 1
    fi
    return 0
}

stop_dev() {
    log_info "Stopping dev server..."
    pkill -f "target/debug/rust-pdf-printer" 2>/dev/null && log_info "Stopped" || log_info "Not running"
}

build() {
    log_info "Building..."
    cd "$RUST_APP_DIR"
    cargo build 2>&1
    log_info "Build complete"
}

rebuild() {
    log_info "Clean rebuild..."
    cd "$RUST_APP_DIR"
    cargo clean
    cargo build 2>&1
    log_info "Rebuild complete"
}

start_dev() {
    if ! check_port; then
        log_error "Port $DEV_PORT is busy. Run: $0 stop"
        exit 1
    fi

    if [ ! -f "$BINARY" ]; then
        log_error "Binary not found. Run: $0 build"
        exit 1
    fi

    log_info "Starting on port $DEV_PORT..."
    cd "$PROJECT_DIR"
    FONT_PATH="$FONT_PATH" PRINTER_IP="$PRINTER_IP" LISTEN_ADDR="0.0.0.0:$DEV_PORT" "$BINARY" &

    # Wait for startup
    for i in {1..10}; do
        if curl -s "http://localhost:$DEV_PORT/health" >/dev/null 2>&1; then
            log_info "Server started successfully"
            return 0
        fi
        sleep 0.5
    done
    log_error "Server failed to start"
    exit 1
}

health() {
    log_info "Health check..."
    if curl -s "http://localhost:$DEV_PORT/health" | head -c 100; then
        echo ""
        log_info "Server is healthy"
    else
        log_error "Server not responding"
        exit 1
    fi
}

generate_pdf() {
    log_info "Generating test PDF..."
    curl -s -X POST "http://localhost:$DEV_PORT/generate-pdf" \
        -H "Content-Type: application/json" \
        -d '[{"name":"山田太郎","car":"あ1234","price":15000,"startDate":"2026-01-06","endDate":"2026-01-08","payDay":"2026-01-10","purpose":"営業","office":"福岡営業所","ryohi":[{"date":"2026-01-06","dest":"博多","detail":["交通費","宿泊"],"price":5000,"vol":1.0}]}]' \
        -o "$PROJECT_DIR/test.pdf"

    if [ -f "$PROJECT_DIR/test.pdf" ]; then
        local mod_time=$(stat -c '%y' "$PROJECT_DIR/test.pdf" | cut -d'.' -f1)
        log_info "PDF generated: $PROJECT_DIR/test.pdf ($(du -h "$PROJECT_DIR/test.pdf" | cut -f1)) - $mod_time"
    else
        log_error "PDF generation failed"
        exit 1
    fi
}

status() {
    echo "=== Port Status ==="
    lsof -i :$DEV_PORT 2>/dev/null || echo "Port $DEV_PORT: free"
    lsof -i :8081 2>/dev/null || echo "Port 8081: free"
    echo ""
    echo "=== Processes ==="
    pgrep -a -f "rust-pdf-printer" 2>/dev/null || echo "No rust-pdf-printer running"
}

# メインコマンド処理
case "${1:-}" in
    build)
        build
        ;;
    rebuild)
        rebuild
        ;;
    start)
        start_dev
        ;;
    stop)
        stop_dev
        ;;
    restart)
        stop_dev
        sleep 1
        start_dev
        ;;
    health)
        health
        ;;
    pdf)
        generate_pdf
        ;;
    status)
        status
        ;;
    test)
        # フルテストサイクル: build -> start -> health -> pdf -> stop
        build
        stop_dev
        sleep 1
        start_dev
        health
        generate_pdf
        log_info "Test complete! PDF at: $PROJECT_DIR/test.pdf"
        ;;
    cycle)
        # 開発サイクル: rebuild -> start -> health -> pdf (サーバーは起動したまま)
        rebuild
        stop_dev
        sleep 1
        start_dev
        health
        generate_pdf
        log_info "Dev cycle complete! Server running on port $DEV_PORT"
        ;;
    *)
        echo "Usage: $0 {build|rebuild|start|stop|restart|health|pdf|status|test|cycle}"
        echo ""
        echo "Commands:"
        echo "  build   - Build the project"
        echo "  rebuild - Clean and rebuild"
        echo "  start   - Start dev server (port $DEV_PORT)"
        echo "  stop    - Stop dev server"
        echo "  restart - Stop and start"
        echo "  health  - Health check"
        echo "  pdf     - Generate test PDF"
        echo "  status  - Show port and process status"
        echo "  test    - Full cycle: build -> start -> health -> pdf -> stop"
        echo "  cycle   - Dev cycle: rebuild -> start -> health -> pdf (server keeps running)"
        exit 1
        ;;
esac
