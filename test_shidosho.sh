#!/bin/bash

cd /home/yutaka/rust/rust-printer/rust-app

# ビルド
cargo build 2>&1 | tail -2

# サーバー起動
FONT_PATH=/home/yutaka/rust/rust-printer/fonts/ipaexm.ttf ./target/debug/rust-pdf-printer &
PID=$!

sleep 2

# PDF生成 (print: false で印刷せずPDFのみ返す)
curl -X POST http://localhost:8081/print-shidosho \
  -H "Content-Type: application/json" \
  -d @../test_shidosho.json \
  -o ../test_output.pdf

# サーバー停止
kill $PID 2>/dev/null

echo "Output: /home/yutaka/rust/rust-printer/test_output.pdf"
