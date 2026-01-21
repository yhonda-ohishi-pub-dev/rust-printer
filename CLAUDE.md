# Project Guidelines

## Session Start

**At the start of a new session, always check for the latest handover note in the `handover/` folder first.**

## Plan Mode

When creating plan files in plan mode, save them to the `plan/` folder.

- Plan files should be saved as: `plan/<descriptive-name>.md`
- Create the `plan/` folder if it doesn't exist

## Handover Notes

When creating handover notes (引き継ぎ書), save them to the `handover/` folder with a datetime filename.

- Handover files should be saved as: `handover/YYYYMMDD-HHMMSS.md`
- Create the `handover/` folder if it doesn't exist
- Example: `handover/20260108-153000.md`

## Project Structure

```
rust-printer/
├── rust-app/                    # メインのRustアプリケーション
│   ├── Cargo.toml               # Rust依存関係定義
│   ├── Dockerfile               # Rustアプリ用Dockerfile
│   └── src/
│       ├── main.rs              # エントリーポイント
│       ├── api/                 # HTTP API層
│       │   ├── mod.rs
│       │   ├── routes.rs        # ルーティング定義
│       │   └── handlers.rs      # リクエストハンドラ
│       ├── models/              # データモデル
│       │   ├── mod.rs
│       │   ├── item.rs          # アイテムモデル
│       │   └── request.rs       # リクエストモデル
│       ├── pdf/                 # PDF生成
│       │   ├── mod.rs
│       │   ├── generator.rs     # PDF生成ロジック
│       │   └── layout.rs        # レイアウト定義
│       └── print/               # 印刷機能
│           ├── mod.rs
│           └── ipp_client.rs    # IPP/CUPSクライアント
│
├── cups-sidecar/                # CUPSサイドカーコンテナ
│   ├── Dockerfile               # CUPS用Dockerfile
│   ├── cupsd.conf               # CUPS設定ファイル
│   └── scripts/
│       ├── entrypoint.sh        # コンテナ起動スクリプト
│       └── setup-printer.sh     # プリンタセットアップスクリプト
│
├── print_pdf_reference/         # 参照用Goプロジェクト（旧実装）
│
├── .github/
│   └── workflows/
│       └── deploy.yml           # CI/CD デプロイワークフロー
│
├── fonts/
│   └── NotoSansJP-Regular.ttf   # 日本語フォント
│
├── docker-compose.yml           # Docker Compose設定（開発用）
├── docker-compose.prod.yml      # Docker Compose設定（本番用・ghcr.io）
├── .env.example                 # 環境変数サンプル
├── VERSION                      # バージョンファイル
├── README.md                    # プロジェクトドキュメント
└── CLAUDE.md                    # このファイル
```

## Technology Stack

- **言語**: Rust (Edition 2021)
- **HTTPサーバー**: Axum 0.7
- **非同期ランタイム**: Tokio
- **PDF生成**: printpdf 0.7
- **印刷プロトコル**: IPP (Internet Printing Protocol) via `ipp` crate
- **シリアライズ**: serde / serde_json
- **ログ**: tracing / tracing-subscriber
- **コンテナ**: Docker + Docker Compose
- **印刷サーバー**: CUPS (cups-sidecar)

## API Endpoints

| エンドポイント | メソッド | 用途 | リクエスト形式 |
|--------------|--------|------|--------------|
| `/` | GET | API情報 | - |
| `/health` | GET | ヘルスチェック | - |
| `/generate-pdf` | POST | PDF生成のみ | JSON (items配列) |
| `/print-pdf` | POST | PDF生成+印刷 | JSON (PrintRequest) |
| `/print` | POST | 既存PDF印刷（封筒印刷など） | Multipart form |

## Bash コマンド注意事項

- `sleep` コマンドの使用禁止（レスポンスが返ってこなくなる）
- バックグラウンドプロセス起動時は `&` で起動後、別のBashコマンドで確認

## Development Commands

```bash
# Rustアプリのビルド
cd rust-app && cargo build

# Rustアプリのテスト
cd rust-app && cargo test

# 指導書PDFテスト（サーバー起動→PDF生成→サーバー停止）
./test_shidosho.sh

# Docker Composeで起動（開発用）
docker-compose up --build

# 個別コンテナのビルド
docker build -t rust-pdf-printer ./rust-app
docker build -t cups-sidecar ./cups-sidecar
```

## CI/CD & Deployment

### フロー

```
git push
  ↓ [pre-push hook]
  │ Docker build → ghcr.io push
  ↓ [GitHub Actions - ohishi-data runner]
  │ checkout → sync files → docker pull → docker-compose up → health check
```

### 自動同期されるファイル

CI/CD が `/opt/rust-printer/` に以下を自動コピー:
- `docker-compose.prod.yml`
- `fonts/NotoSansJP-Regular.ttf`

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

### ネットワーク

- 本番環境は `nginx_default` ネットワークに接続
- php3 からは `http://rust-pdf-printer:8081` でアクセス可能
