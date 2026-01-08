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
├── docker-compose.yml           # Docker Compose設定
├── .env.example                 # 環境変数サンプル
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

## Development Commands

```bash
# Rustアプリのビルド
cd rust-app && cargo build

# Rustアプリのテスト
cd rust-app && cargo test

# Docker Composeで起動
docker-compose up --build

# 個別コンテナのビルド
docker build -t rust-pdf-printer ./rust-app
docker build -t cups-sidecar ./cups-sidecar
```
