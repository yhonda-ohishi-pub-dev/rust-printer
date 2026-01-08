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

### POST /api/print

出張旅費精算書を印刷します。

```bash
curl -X POST http://localhost:3000/api/print \
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
  }'
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
