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

## CI/CD & デプロイ

### フロー

```
git push
  ↓ [pre-push hook] Docker build → ghcr.io push
  ↓ [GitHub Actions] sync files → docker pull → docker-compose up → health check
```

### 開発者セットアップ

```bash
# GitHub Container Registryにログイン
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# pre-pushフックをインストール
cp scripts/pre-push .git/hooks/pre-push
```

### リリース手順

1. `VERSION`ファイルのバージョンを更新
2. `git push` を実行（pre-pushフックが自動でビルド・プッシュ）
3. GitHub Actions が本番サーバーで自動デプロイ

### 自動同期されるファイル

CI/CD が `/opt/rust-printer/` に以下を自動コピー:
- `docker-compose.prod.yml`
- `fonts/NotoSansJP-Regular.ttf`

### 本番サーバー（ohishi-data）初回セットアップ

```bash
# ディレクトリ作成と.env設定のみ
sudo mkdir -p /opt/rust-printer
echo "PRINTER_IP=192.168.x.x" > /opt/rust-printer/.env
```

### 本番起動（手動）

```bash
cd /opt/rust-printer
docker-compose -f docker-compose.prod.yml up -d
```

### イメージ

- `ghcr.io/yhonda-ohishi-pub-dev/rust-pdf-printer`
- `ghcr.io/yhonda-ohishi-pub-dev/cups-sidecar`

### ネットワーク

本番環境は `nginx_default` ネットワークに接続されます。
php3 からは `http://rust-pdf-printer:8081` でアクセス可能。

## 技術スタック

- **Rust**: Axum (HTTP), printpdf (PDF生成), ipp (印刷プロトコル)
- **Docker**: マルチコンテナ構成
- **CUPS**: 印刷サーバー

## ライセンス

MIT
