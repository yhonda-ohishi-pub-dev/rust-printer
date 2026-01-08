# Rust PDF Printer

Canon LBP221プリンター向けのPDF生成・印刷システム。Rustで実装されたHTTPサーバーとCUPSサイドカーをDocker Composeで構成。

## 機能

- 出張旅費精算書のPDF生成（A5横向き）
- IPPプロトコルによるCUPS経由の印刷
- 日本語テキスト対応（Noto Sans JP）

## アーキテクチャ

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   HTTP Client   │────▶│   Rust App      │────▶│   CUPS Sidecar  │────▶ Canon LBP221
│                 │     │   (Axum)        │ IPP │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

## セットアップ

### 1. 環境変数の設定

```bash
cp .env.example .env
# .envを編集してPRINTER_IPを設定
```

### 2. フォントの配置

Google FontsからNoto Sans JPをダウンロードし、`fonts/`ディレクトリに配置：

```bash
mkdir -p fonts
# NotoSansJP-Regular.ttf を fonts/ に配置
```

### 3. Docker Composeで起動

```bash
docker-compose up --build
```

## API

### POST /generate-pdf

出張旅費精算書のPDFを生成して返します（印刷なし）。

```bash
curl -X POST http://localhost:3000/generate-pdf \
  -H "Content-Type: application/json" \
  -d '{
    "items": [{
      "name": "山田太郎",
      "car": "あ1234",
      "price": 15000,
      "start_date": "2026-01-08",
      "end_date": "2026-01-08",
      "purpose": "出張",
      "office": "本社",
      "pay_day": "2026-01-15",
      "ryohi": []
    }]
  }' --output output.pdf
```

### POST /print-pdf

出張旅費精算書のPDFを生成して印刷します。

```bash
curl -X POST http://localhost:3000/print-pdf \
  -H "Content-Type: application/json" \
  -d '{
    "items": [{
      "name": "山田太郎",
      "car": "あ1234",
      "price": 15000,
      "start_date": "2026-01-08",
      "end_date": "2026-01-08",
      "purpose": "出張",
      "office": "本社",
      "pay_day": "2026-01-15",
      "ryohi": []
    }],
    "print": true,
    "printerName": "LBP221"
  }'
```

### POST /print

既存のPDFファイルを印刷します（封筒印刷など）。

```bash
curl -X POST http://localhost:3000/print \
  -F "document=@/path/to/envelope.pdf" \
  -F "printer=LBP221-futo"
```

**レスポンス例:**
```json
{
  "status": "success",
  "message": "PDF printed successfully",
  "filename": "envelope.pdf",
  "printer": "LBP221-futo",
  "printed": true,
  "file_size": 12345
}
```

## 開発

### ビルドチェック

```bash
cd rust-app
cargo check
```

### ローカル実行（CUPSなし）

```bash
cd rust-app
cargo run
```

## 技術スタック

- **Rust**: Axum (HTTP), printpdf (PDF生成), ipp (印刷プロトコル)
- **Docker**: マルチコンテナ構成
- **CUPS**: 印刷サーバー

## ライセンス

MIT
