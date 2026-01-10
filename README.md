# Rust PDF Printer

Epson PX-M650F / Canon LBP221 プリンター向けのPDF生成・印刷システム。Rustで実装されたHTTPサーバーをDockerで構成。

## 機能

- 出張旅費精算書のPDF生成（A5横向き）
- RAW印刷（ポート9100）またはIPPプロトコルによる直接印刷
- 日本語テキスト対応（IPAexゴシック）

## アーキテクチャ

```
┌─────────────────┐     ┌─────────────────┐
│   HTTP Client   │────▶│   Rust App      │────▶ Printer (RAW/IPP)
│                 │     │   (Axum)        │
└─────────────────┘     └─────────────────┘
```

## セットアップ

### 1. 環境変数の設定

```bash
cp .env.example .env
# .envを編集してPRINTER_IPを設定
```

### 2. フォントの配置

`fonts/`ディレクトリにIPAexゴシックフォントを配置：

```bash
mkdir -p fonts
# ipaexg.ttf を fonts/ に配置
```

### 3. Docker Composeで起動

```bash
docker-compose up --build
```

## API

### GET /health

ヘルスチェック。

```bash
curl http://localhost:8081/health
```

### POST /generate-pdf

出張旅費精算書のPDFを生成して返します（印刷なし）。

**リクエスト形式: 配列を直接送信**

```bash
curl -X POST http://localhost:8081/generate-pdf \
  -H "Content-Type: application/json" \
  -d '[{
    "name": "山田太郎",
    "car": "あ1234",
    "price": 15000,
    "start_date": "2026-01-08",
    "end_date": "2026-01-08",
    "purpose": "出張",
    "office": "本社",
    "pay_day": "2026-01-15",
    "ryohi": []
  }]' --output output.pdf
```

### POST /print-pdf

出張旅費精算書のPDFを生成して印刷します。

**リクエスト形式: オブジェクト（itemsフィールド必須）**

```bash
# RAW印刷（デフォルト、ポート9100）
curl -X POST http://localhost:8081/print-pdf \
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
    "print": true
  }'

# Direct IPP印刷（PX-M650F等）
curl -X POST http://localhost:8081/print-pdf \
  -H "Content-Type: application/json" \
  -d '{
    "items": [...],
    "print": true,
    "use_direct_ipp": true,
    "printer_ip": "192.168.1.100",
    "paper_size": "iso_a5_148x210mm",
    "color_mode": "monochrome"
  }'
```

### POST /print

既存のPDFファイルを印刷します（封筒印刷など）。

```bash
# RAW印刷
curl -X POST http://localhost:8081/print \
  -F "document=@/path/to/envelope.pdf"

# Direct IPP印刷 - Epson PX-M650F（URF形式、デフォルト）
curl -X POST http://localhost:8081/print \
  -F "document=@/path/to/envelope.pdf" \
  -F "useDirectIpp=true" \
  -F "printerIp=172.18.21.70" \
  -F "paperSize=naga3"

# Direct IPP印刷 - Canon LBP221（PDF形式）
curl -X POST http://localhost:8081/print \
  -F "document=@/path/to/envelope.pdf" \
  -F "useDirectIpp=true" \
  -F "printerIp=172.18.21.60" \
  -F "paperSize=naga3" \
  -F "documentFormat=pdf"
```

#### ドキュメントフォーマット（documentFormat）

| 値 | 説明 | 推奨プリンタ |
|----|------|-------------|
| `urf` | URF (Apple Raster) 形式に変換（デフォルト） | Epson PX-M650F |
| `pdf` | PDFをそのまま送信 | Canon LBP221 |
| `pwg` | PWG Raster 形式に変換 | 一般的なIPPプリンタ |

#### 対応用紙サイズ

| 指定値 | IPP media | サイズ |
|-------|-----------|--------|
| `a4` | iso_a4_210x297mm | A4 |
| `a5` | iso_a5_148x210mm | A5 |
| `a3` | iso_a3_297x420mm | A3 |
| `b5` | iso_b5_176x250mm | B5 |
| `letter` | na_letter_8.5x11in | レター |
| `naga3`, `cho3`, `長3` | om_cho-3_120x235mm | 長3封筒 |
| `naga4`, `cho4`, `長4` | om_cho-4_90x205mm | 長4封筒 |

IPP形式（`_`を含む文字列）はそのまま使用可能。

**レスポンス例:**
```json
{
  "status": "success",
  "message": "PDF printed successfully",
  "filename": "envelope.pdf",
  "printer": "192.168.1.100 (RAW)",
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

### ローカル実行

```bash
cd rust-app
cargo run
```

### 開発用スクリプト

```bash
./dev.sh        # docker-compose up --build
./dev.sh down   # docker-compose down
./dev.sh logs   # docker-compose logs -f
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
gh auth refresh -s write:packages
gh auth token | docker login ghcr.io -u USERNAME --password-stdin

# pre-pushフックをインストール
cp scripts/pre-push .git/hooks/pre-push && chmod +x .git/hooks/pre-push
```

### リリース手順

1. `VERSION`ファイルのバージョンを更新
2. `git push` を実行（pre-pushフックが自動でビルド・プッシュ）
3. GitHub Actions が本番サーバーで自動デプロイ

### 自動同期されるファイル

CI/CD が `/opt/rust-printer/` に以下を自動コピー:
- `docker-compose.prod.yml`
- `fonts/ipaexg.ttf`

### 本番サーバー（ohishi-data）初回セットアップ

```bash
# ディレクトリ作成（runner ユーザーで書き込み可能にする）
sudo mkdir -p /opt/rust-printer/fonts
sudo chown -R $(whoami):$(whoami) /opt/rust-printer

# プリンタIP設定
echo "PRINTER_IP=192.168.x.x" > /opt/rust-printer/.env
```

### 本番起動（手動）

```bash
cd /opt/rust-printer
docker-compose -f docker-compose.prod.yml up -d
```

### イメージ

- `ghcr.io/yhonda-ohishi-pub-dev/rust-pdf-printer`

### ネットワーク

本番環境は `nginx_default` ネットワークに接続されます。
php3 からは `http://rust-pdf-printer:8081` でアクセス可能。

## 技術スタック

- **Rust**: Axum (HTTP), printpdf (PDF生成), ipp (印刷プロトコル)
- **Docker**: シングルコンテナ構成
- **印刷**: RAW (ポート9100) / Direct IPP

## ライセンス

MIT
