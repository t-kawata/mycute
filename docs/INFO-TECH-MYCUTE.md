# MYCUTE 技術情報

## プロジェクト概要

MYCUTE は、リアルタイム音声認識（STT）と LLM 連携を核としたマルチモーダル AI デスクトップアプリケーション。Tauri ベースの GUI と Axum ベースのバックエンドサーバーで構成される。複数のエディションを同一コードベースからビルドする。

- **バージョン**: 2.4.17
- **メインエディション**: `mycute`
- **メイン言語**: Rust
- **補助言語**: TypeScript (SDK/Web), Go (サーバー補完), Python (スクリプト), Swift (macOS Native), C# (Windows Native)
- **Rust ツールチェーン**: 1.92.0

---

## 1. アーキテクチャ概観

### 1.1 マルチプロセス構成

```
┌──────────────────────────────────────────────────┐
│                  ユーザー                          │
├──────────────────────────────────────────────────┤
│  Tauri GUI (CL)   ◄── IPC ──►  RT (Axum サーバー)  │
│  - Vue 3 / Quasar  │          - REST API          │
│  - 音声認識制御      │          - WebSocket (SSE)   │
│  - ホットキー       │          - プロキシ           │
│  - クリップボード    │          - DB 操作            │
├───────────────────┤          - Bifrost (LLM GW)   │
│  AM (Auto Migrate) │          - ZeroClaw (認証GW)   │
│  - DB Migration    │          - Cuber (投票/評価)    │
│  - スタンドアロン    │          - MyProxy (SSL Proxy) │
└───────────────────┴──────────────────────────────┘
```

### 1.2 4 つの動作モード

| モード | エントリポイント | 説明 |
|---|---|---|
| **CL** (Client) | `src/main.rs` → `mode/cl/main_of_cl.rs` | Tauri GUI プロセス。音声認識開始/停止、ホットキー監視、テキストコミット |
| **RT** (RealTime) | `src/launcher.rs` / `src/server.rs` | Axum バックエンドサーバー。REST API + SSE/WebSocket |
| **AM** (Auto Migration) | `src/main.rs` → `mode/am/main_of_am.rs` | DB マイグレーション専用モード |
| **OG** (One-shot Go) | `src/main.rs` → `mode/og/main_of_og.rs` | Go モジュールラッパー起動モード |

### 1.3 バイナリ構成

| バイナリ名 | エントリポイント | 説明 |
|---|---|---|
| `mycute` | `src/main.rs` | マルチモードランチャー (CL/AM/OG) |
| `mycute-server-core` | `src/server.rs` | GUI 非依存のスタンドアロンサーバー |
| `mycute-server` | `src/launcher.rs` | Core + ネイティブライブラリを内包した配布用ランチャー |

---

## 2. Rust プロジェクト構造

### 2.1 ディレクトリマップ

```
src/
├── main.rs                      # エントリポイント (CL/AM/OG モード)
├── server.rs                    # mycute-server-core エントリポイント
├── launcher.rs                  # mycute-server エントリポイント (自己展開ランチャー)
├── lib.rs                       # ライブラリルート + CurrentTimestamp トレイト + マクロ
├── types.rs                     # 共通型定義 (enum, ペイロード構造体)
├── constants.rs                 # プロジェクト全体の共通定数 (TypeScript へ自動同期)
├── mycute_manager.rs            # CL のメインステートマシン
├── mycute_settings.rs           # 設定読み込み/管理 (ConfigManager)
│
├── mode/                        # 各モードの実装
│   ├── mod.rs                   # am / cl / og / rt (4 モード)
│   ├── cl/                      # Client モード (Tauri)
│   │   ├── main_of_cl.rs        # CL メインロジック
│   │   └── sw_server.rs         # Service Worker サーバー
│   ├── am/                      # Auto Migration
│   │   └── main_of_am.rs
│   ├── og/                      # One-shot Go
│   │   └── main_of_og.rs
│   └── rt/                      # RealTime サーバーモード (最大サブモジュール)
│       ├── main_of_rt.rs        # RT 起動シーケンス
│       ├── owner_secrets.rs     # Owner 秘密鍵管理
│       ├── req_map.rs           # リクエストID 管理
│       ├── client/              # セキュアクライアント
│       ├── middleware/          # P2P クロック同期ミドルウェア
│       ├── rtbl/                # ビジネスロジック層 (27 ファイル)
│       ├── rterr/               # エラー型定義
│       ├── rtevent/             # イベント (プロキシリーク / システムイベント)
│       ├── rthandler/           # REST API ハンドラー (22 ファイル)
│       ├── rtreq/               # リクエスト型定義
│       ├── rtres/               # レスポンス型定義
│       └── rtutils/             # ユーティリティ (DB, 投票, アプリタイプ)
│
├── tauri_cmd/                   # Tauri IPC コマンド
│   ├── llm.rs, recording.rs, settings.rs
│   ├── system.rs, util.rs, voice.rs
│   └── mod.rs
│
├── stt/                         # 音声認識エンジン
│   ├── mac.rs, win.rs, openai.rs, recognizer.rs
│   ├── resampler.rs, stats.rs, mod.rs
│
├── tools/                       # 音声処理ツール
│   ├── audio.rs, lindera_util.rs, post_correction_processor.rs
│   ├── pseudo_asr_streamer.rs, punctuation_machine.rs
│   ├── resampler.rs, text_cleanup.rs, vad_processor.rs
│   └── mod.rs
│
├── llm/                         # LLM クライアント
│   ├── client.rs, prompts.rs, mod.rs
│
├── bifrost/                     # Bifrost (LLM ゲートウェイ)
│   ├── assets.rs, error.rs, executor.rs, installer.rs, mod.rs
│
├── zeroclaw/                    # ZeroClaw (認証ゲートウェイ)
│   ├── assets.rs, error.rs, executor.rs, installer.rs, mod.rs
│
├── cuber/                       # Cuber (投票/評価/抽選エンジン)
│   ├── config.rs, consts.rs, error.rs, event.rs
│   ├── service.rs, tokenizer.rs, mod.rs
│   └── storage/
│
├── myproxy/                     # MyProxy (SSL MITM Proxy)
│   ├── myproxy_handler.rs, server.rs, utils.rs, mod.rs
│   ├── certs/
│   └── ssl/
│
├── nodejs/                      # Node.js ランタイム管理
│   ├── assets.rs, error.rs, executor.rs, installer.rs, mod.rs
│
├── migration/                   # Sea-ORM マイグレーション (40 ファイル)
├── entities/                    # Sea-ORM エンティティ (30 ファイル)
├── utils/                       # ユーティリティ (auth, crypto, db, jwt, process 等)
├── vo/                          # Value Objects (usrs_vo.rs)
├── config/                      # 設定 (settings.rs)
├── enums/                       # 列挙型 (mode.rs, usrtype.rs)
├── hotkey_mac.rs / hotkey_win.rs # プラットフォーム別ホットキー
└── wav/                         # サウンドファイル
```

### 2.2 主要依存関係

| カテゴリ | クレート | 用途 |
|---|---|---|
| 非同期 | `tokio` (full), `tokio-util`, `tokio-stream` | 非同期ランタイム |
| Web | `axum` (ws, multipart), `tower-http` (cors, fs) | REST API サーバー |
| GUI | `tauri` 2.9.5 (macos-private-api, devtools) | デスクトップ GUI |
| Tauri プラグイン | `tauri-plugin-process`, `tauri-plugin-clipboard-manager` | プロセス管理/クリップボード |
| ORM | `sea-orm` (MySQL/PostgreSQL/SQLite), `sea-orm-migration` | DB アクセス/マイグレーション |
| 音声認識 | `sherpa-rs` 0.6.8, `sherpa-rs-sys` | オンデバイス音声認識 |
| 音声処理 | `rubato` (リサンプル), `hound` (WAV), `rodio` | 音声入出力 |
| NLP | `lindera` (形態素解析 + IPADIC), `edit-distance` | 日本語テキスト処理 |
| LLM | `async-openai` 0.36.1 (audio, chat-completion) | OpenAI API クライアント |
| 認証 | `jsonwebtoken`, `bcrypt`, `argon2` | JWT/パスワード |
| 暗号 | `aes-gcm`, `ed448-goldilocks`, `sha3`, `sha2` | 暗号化/署名 |
| SSL/TLS | `rustls` (ring), `rcgen`, `fastcert`, `x509-parser` | 証明書管理 |
| プロキシ | `lol_html`, `tokio-tungstenite` (native-tls), `hyper` (full) | MITM プロキシ |
| ストレージ | `rust-s3`, `moka` (cache), `dashmap` | S3/キャッシュ |
| ベクトル検索 | `lbug` 0.15.1 (C++ ライブラリ, パッチ適用) | ベクトル類似検索 |
| macOS FFI | `objc`, `cocoa`, `core-foundation`, `block` | macOS Native 連携 |
| Windows FFI | `winapi` (winuser, libloaderapi) | Windows Native 連携 |
| 汎用 | `clap`, `chrono`, `serde`, `regex`, `uuid`, `indexmap`, `garde`, `thiserror` | ユーティリティ |

---

## 3. サーバーモード (RT) 詳細

### 3.1 起動シーケンス (`main_of_rt.rs`)

1. **my_base_url バリデーション** — 必須設定値チェック
2. **DB 接続確立** — MySQL/PostgreSQL/SQLite のいずれか
3. **ConfigManager ライブ化** — 設定を DB から読み込み、変更を永続化可能に
4. **Node.js ランタイム準備** — ビルドインの Node.js v25.9.0 バイナリを展開
5. **ZeroClaw 認証ゲートウェイ起動** — JWT 認証サブプロセス
6. **Bifrost (LLM Gateway) 起動** — LLM API 中継サブプロセス
7. **SSL 証明書準備** — MyProxy 用の自己署名 CA 証明書生成
8. **MyProxy (SSL MITM Proxy) 起動** — HTTPS インターセプトプロキシ
9. **Cuber (投票エンジン) 起動** — 評価・抽選システム
10. **Axum HTTP サーバー起動** — REST + SSE + WebSocket
11. **Fate-Sharing 監視開始** — 親プロセス消失時の自己終了機構
12. **ポートクリーンアップ** — 起動前に既存の全バックエンドポートを解放

### 3.2 REST API ハンドラー一覧 (22)

| ハンドラー | プレフィックス | 主なリソース |
|---|---|---|
| `bds_handler` | `/bds` | 掲示板 CRUD |
| `ca_handler` | `/ca` | 認証局操作 |
| `ca_apps_handler` | `/ca/apps` | CA アプリケーション |
| `ca_identities_handler` | `/ca/identities` | CA アイデンティティ |
| `ca_blacklists_handler` | `/ca/blacklists` | CA ブラックリスト |
| `cryptos_handler` | `/cryptos` | 暗号資産管理 |
| `cubes_handler` | `/cubes` | キューブ (投票単位) CRUD |
| `forums_handler` | `/forums` | フォーラム CRUD |
| `health_handler` | `/health` | ヘルスチェック |
| `lmgws_handler` | `/lmgws` | LLM プロバイダ管理 |
| `mycute_handler` | `/mycute` | システム設定 |
| `mycute_proxy_leaks_handler` | `/mycute/proxy-leaks` | プロキシリーク報告 |
| `node_apps_handler` | `/node/apps` | ノードアプリ管理 |
| `node_identities_handler` | `/node/identities` | ノードアイデンティティ |
| `node_blacklists_handler` | `/node/blacklists` | ノードブラックリスト |
| `nodejs_handler` | `/nodejs` | Node.js コード実行 |
| `osca_handler` | `/osca` | 証明書管理 (OSCA) |
| `owner_handler` | `/owner` | オーナー管理 |
| `pub_apps_handler` | `/pub/apps` | 公開アプリケーション |
| `replace_items_handler` | `/replace/items` | 辞書置換アイテム |
| `replaces_handler` | `/replaces` | 辞書置換管理 |
| `usrs_handler` | `/usrs` | ユーザー管理 |

### 3.3 リアルタイム通信

- **SSE** (Server-Sent Events): サーバー → クライアントへのイベント配信。5 秒間隔の heartbeat。チャンネル容量 256 イベント
- **WebSocket**: 双方向通信。チャレンジ/レスポンス認証方式、TLS 暗号化
- **SSE イベント種別**: `LocaleChanged`, `SttEngineChanged`, `OwnerStatusChanged`, `CaStatusChanged`, `LicensesChanged`, `LmgwProvidersChanged`, `SystemMessage`, `Heartbeat`

### 3.4 ミドルウェア

- **P2P Clock Sync Enforcement**: P2P ネットワークにおける時刻同期を強制検証するミドルウェア

---

## 4. データベース

### 4.1 サポートDB構成

| DB | 用途 | Docker イメージ |
|---|---|---|
| SQLite | デフォルト (開発/シングルユーザー) | 不要 |
| MySQL 9.5.0 | 本番マスター | `mysql:9.5.0` |
| PostgreSQL 18 | セカンダリ/読み取りレプリカ | `postgres:18-alpine` |

`Env` 構造体が `rw_db` (read-write) と `ro_dbs` (read-only replicas) を管理し、マルチDB構成を抽象化する。

### 4.2 エンティティ一覧 (40 テーブル)

```
ユーザー・掲示板系: usrs, bds, identities, settings
認証・セキュリティ系: tickets, verifications, blacklists, burned_keys, node_tickets
アプリケーション系: apps, node_apps, pub_apps
暗号資産系: cryptos, exports
評価・抽選系: cubes, cube_contributors, cube_lineages, cube_model_stats
                 works, pools, points, payouts, payments, jobs
バッジ・マッチング系: badges, usr_badges, belongs, matches, match_statuses, flushes
フォーラム・辞書系: forums, replaces, replace_items
CA 系: ca_vote_allocated_summaries, ca_vote_item_summaries
LLM 系: chat_models, lmgw_providers
```

### 4.3 マイグレーション方式

- Sea-ORM Migrator (`src/migration/`) によるマイグレーション
- AM (Auto Migration) モードで単独実行可能
- サーバー起動時に自動実行 (`--skip-rt-migration` でスキップ可能)
- SQLite/MySQL/PostgreSQL に対応したポータブルなマイグレーション定義

---

## 5. 音声認識パイプライン

### 5.1 ストリーム処理フロー

```
マイク入力
    ↓
VAD (Voice Activity Detection)
  ├── Silero VAD (ONNX, Int8/FP32) — 高精度
  └── TEN VAD (ONNX, Int8/FP32) — 軽量
    ↓
リサンプリング (rubato)
    ↓
STT Engine
  ├── OS Native (macOS: SFSpeechRecognizer / Windows: WinRT Speech)
  └── OpenAI Whisper API (疑似ストリーミング)
    ↓
テキスト後処理パイプライン
  ├── Lindera 形態素解析 (IPADIC 組込)
  ├── 句読点自動付与 (無音区間検出)
  ├── PostCorrection (LLM による最終補正)
  └── テキストクリーンアップ
    ↓
出力 (テキストコミット / 画面表示)
```

### 5.2 VAD 設定パラメータ

| パラメータ | デフォルト | 説明 |
|---|---|---|
| `vad_threshold` | 0.5 | VAD 発話判定閾値 |
| `vad_min_silence_duration` | 0.5s | 発話終了判定の最小無音時間 |
| `vad_min_speech_duration` | 0.3s | 発話開始判定の最小発話時間 |
| `vad_max_speech_duration` | 25.0s | 最大発話時間 (強制区切り) |
| `vad_pre_padding_ms` | 500ms | 発話区間の前方パディング |

### 5.3 入力モード

| モード | 説明 |
|---|---|
| **RealTime** | 認識結果を逐次確定・出力 (デフォルト) |
| **Buffered** | 認識結果をバッファリングし、フラッシュ時に一括出力 |

### 5.4 ホットキー

| 操作 | macOS | Windows |
|---|---|---|
| 録音開始/停止 | Ctrl+Option+Space | Ctrl+Alt+Space |
| バッファ開始 | なし (macOS) | 同左 |
| バッファフラッシュ | なし | 同左 |
| LLM 補正実行 | 割当可能 | 割当可能 |
| LLM 要約実行 | 割当可能 | 割当可能 |

---

## 6. サブシステム詳細

### 6.1 Bifrost (LLM ゲートウェイ)

- **位置**: `src/bifrost/`
- **バージョン**: v1.4.24
- **役割**: 外部 LLM API (OpenAI 等) への統合ゲートウェイ
- **方式**: 単一バイナリ (`bifrost-http-{platform}-v1.4.24.tar.gz`) を内包し、初回実行時に展開
- **対応プラットフォーム**: macOS (ARM64), Linux (AMD64), Windows (AMD64)
- **LMGW 連携**: LMGW プロバイダ設定に基づいてルーティング

### 6.2 ZeroClaw (認証ゲートウェイ)

- **位置**: `src/zeroclaw/`
- **役割**: JWT 発行・検証の認証ゲートウェイ
- **JWT クレーム**: `aid` (app_id), `uid` (user_id), `email`, `vid` (version_id)
- **トークン有効期限**: 設定可能 (デフォルト 24 時間)
- **方式**: 内包バイナリを初回展開
- **対応プラットフォーム**: macOS (ARM64), Windows (AMD64), Linux (AMD64)

### 6.3 Cuber (投票/評価/抽選エンジン)

- **位置**: `src/cuber/`
- **役割**: 分散合意に基づく評価・抽選システム
- **コア機能**:
  - キューブ作成・管理 (CRUD)
  - 投票集計 (重み付き)
  - 抽選トークナイザー
  - 評価モデル統計
  - イベント駆動型アーキテクチャ

### 6.4 MyProxy (SSL MITM プロキシ)

- **位置**: `src/myproxy/`
- **役割**: Web トラフィックの暗号化/復号プロキシ
- **機能**:
  - 動的 SSL 証明書生成 (rcgen)
  - OS トラストストアへのルート CA 登録
  - 証明書期限管理・自動更新
  - プロキシリーク検出機構
- **構成**: `myproxy_handler.rs` (リクエスト処理) + `server.rs` (サーバー管理) + `ssl/` (TLS 設定)

### 6.5 LMGW (LLM Provider Gateway)

- **DB テーブル**: `lmgw_providers`
- **役割**: 外部 LLM API プロバイダの統合管理
- **管理画面**: Web UI の `LlmApp.vue` からプロバイダの追加/編集/削除
- **移行履歴**: 旧 `LlmEndpoint` → LMGW 移行済み (破棄)

### 6.6 Node.js ランタイム管理

- **位置**: `src/nodejs/`
- **Node.js バージョン**: v25.9.0
- **方式**: 各プラットフォームの Node.js バイナリをビルドインアセットとして内包
- **アセット**:
  - `node-v25.9.0-darwin-arm64.tar.gz` (macOS)
  - `node-v25.9.0-linux-x64.tar.gz` (Linux)
  - `node-v25.9.0-win-x64.zip` (Windows)
- **用途**: Service Worker サーバー、SDK 関連処理

---

## 7. 周辺プロジェクト

### 7.1 Web フロントエンド (Vue 3 + Quasar 2)

```
web/
├── quasar.config.ts              # Quasar ビルド設定 (Vite ベース)
├── index.html                    # HTML エントリポイント
├── postcss.config.js             # PostCSS 設定
├── package.json                  # 依存関係
├── src/
│   ├── App.vue                   # ルートコンポーネント
│   ├── apps/                     # アプリケーションページ
│   │   ├── HarunohiApp.vue       # 掲示板/タイムライン
│   │   ├── LlmApp.vue            # LLM チャットインターフェース
│   │   └── SettingsApp.vue       # 設定画面
│   ├── components/
│   │   ├── decorations/          # UI 装飾 (バッジ, カーブ, ロゴ, 星)
│   │   ├── dialogs/              # ダイアログ (15+ 種類)
│   │   ├── effects/              # 視覚効果 (WaterRipple)
│   │   ├── icons/                # SVG カスタムアイコン (30+ 種類)
│   │   ├── panels/index/         # インデックスパネル群
│   │   └── tools/                # 汎用ツール (Calendar, Tinder, SwipeActions)
│   ├── layouts/                  # 4 種のレイアウト
│   ├── pages/                    # 3 ページ (Splash, Login, ErrorNotFound)
│   ├── stores/                   # Pinia ストア (main, llm, example)
│   ├── router/                   # Vue Router (hash モード)
│   ├── i18n/                     # 国際化 (ja-JP / en-US)
│   ├── models/                   # 型定義 (app, lmgw, main, rtreq, rtres)
│   ├── enums/                    # Edition, TAB, usrtype
│   ├── consts/                   # 定数 (data.ts, generated_constants.ts)
│   ├── configs/                  # 設定 (settings.ts)
│   ├── boot/                     # Quasar ブート (i18n, axios)
│   └── utils/                    # ユーティリティ
├── tmp-assets/                   # 画像アセット
└── public/                       # 公開アセット
```

**技術スタック**:
- Vue 3 (Composition API) + Quasar 2 + Vite
- Pinia (状態管理), Vue Router (hash モード)
- vue-i18n (日本語/英語), axios (HTTP)
- @douxcode/vue-spring-bottom-sheet, @vueuse/core
- matter-js, planck (物理エンジン: アニメーション効果)
- zod (バリデーション), jose (JWT)
- @intlify/unplugin-vue-i18n (i18n プラグイン)

### 7.2 MyCute SDK (TypeScript)

```
sdk-ts/
├── src/
│   ├── mycute_sdk.ts              # SDK エントリポイント: initMycute()
│   ├── generated_constants.ts      # Rust 定数からの自動生成
│   ├── events/event_bus.ts         # イベントバス
│   ├── interceptors/
│   │   ├── fetch.ts                # Fetch / XHR インターセプト
│   │   ├── dom.ts                  # DOM 要素インターセプト (img, iframe)
│   │   ├── eventsource.ts          # SSE インターセプト
│   │   ├── navigation.ts           # ナビゲーションインターセプト
│   │   ├── websocket.ts            # WebSocket インターセプト
│   │   └── worker.ts               # Web Worker インターセプト
│   ├── service-worker/
│   │   ├── mycute_sw.ts            # Service Worker (プロキシ通信)
│   │   └── register.ts             # Service Worker 登録
│   └── utils/
│       ├── url.ts                  # MYCUTE 環境判定
│       └── url_encoder.ts          # ホスト名エンコード
├── build.js                        # esbuild ビルド
└── package.json
```

**役割**: MYCUTE プロキシ環境下での Web アプリケーション統合 SDK。Fetch / XHR / WebSocket / SSE / DOM の各通信を自動インターセプトし、プロキシスキームに透過的に変換。Service Worker によるオフライン対応とプロキシ経由通信を実現。

**初期化**: `initMycute()` を呼び出すと全インターセプターが有効化される。MYCUTE 環境外では自動的に何もしない。

### 7.3 Go モジュール (`mycute-go/`)

```
mycute-go/
├── src/
│   ├── main.go                    # エントリポイント
│   ├── mode/
│   │   ├── rt/                    # RT 互換モード
│   │   └── am/                    # AM 互換モード
│   ├── lib/                       # 共通ライブラリ群
│   │   ├── common/                # 汎用ユーティリティ
│   │   ├── eventbus/              # イベントバス
│   │   ├── httpclient/            # HTTP クライアント
│   │   ├── logger/                # ロガー
│   │   ├── mycrypto/              # 暗号化ユーティリティ
│   │   └── s3client/             # S3 クライアント
│   ├── model/                     # データモデル
│   ├── pkg/                       # 公開パッケージ
│   ├── config/                    # 設定
│   ├── cutil/                     # C 言語連携
│   ├── enum/                      # 列挙型
│   ├── sql/                       # SQL クエリ
│   └── docs/                      # Swagger ドキュメント
├── cognee/                        # Python 知識グラフモジュール (cognee タスク)
│   └── tasks/
│       ├── codingagents/          # コーディングエージェントルール連携
│       ├── memify/                # メモリ化 (サブグラフ抽出)
│       ├── entity_completion/     # エンティティ補完 (LLM/Regex)
│       ├── ingestion/             # データ取り込み
│       ├── temporal_awareness/    # 時間認識グラフ (Graphiti)
│       ├── temporal_graph/        # 時間グラフ
│       └── web_scraper/           # Web スクレイピング
├── cmd/                           # CLI コマンド
├── sh/                            # ビルドスクリプト
└── docker/                        # Docker 設定
```

### 7.4 ネイティブコード

| 言語 | OS | ファイル | 役割 |
|---|---|---|---|
| Swift | macOS | `native/swift/SpeechHelper.swift` | SFSpeechRecognizer ラッパー |
| Objective-C | macOS | `native/swift/speech_helper.h` | Swift と Rust のブリッジヘッダ |
| C# | Windows | `native/cs/SpeechHelper/SpeechHelper.cs` | WinRT Speech ラッパー (.NET 10 Native AOT) |
| C# | Windows | `native/cs/SpeechHelper/Check.cs` | 権限チェック |
| C | 全般 | `crates/lbug/lbug-0.15.1/lbug-src/` | ベクトル検索エンジン本体 |
| C/C++ | 全般 | `crates/lbug/lbug-0.15.1/third_party/` | simsimd (類似度計算), yyjson (JSON) |

### 7.5 スクリプト類

| スクリプト | 言語 | 用途 |
|---|---|---|
| `scripts/release.sh` | Bash | GitHub Releases への自動リリース |
| `scripts/build-linux.sh` | Bash | Linux x86_64 クロスコンパイル |
| `scripts/macos-setup.command` | Bash | macOS 開発環境セットアップ |
| `scripts/apply-edition.js` | Node.js | エディション設定 (.env) 生成 |
| `scripts/gen-ts-constants.sh` / `.mjs` | Bash/JS | Rust const → TypeScript 定数自動同期 |
| `scripts/find-frontend-src.mjs` | Node.js | フロントエンドソース検索 (Tauri CSP 用) |
| `scripts/analyze_wav.py` | Python | WAV ファイル分析 |
| `scripts/trim_wav.py` | Python | WAV トリミング |
| `scripts/analyze_assets.py` | Python | ビルドインアセット分析 |
| `scripts/verify_assets.py` | Python | アセット整合性検証 |
| `scripts/migrate_replaces.py` | Python | 辞書置換データ移行 |
| `scripts/lmgw-chat.js` | Node.js | LMGW チャット動作確認 |
| `scripts/test_chat_models.sh` | Bash | チャットモデル結合テスト |
| `scripts/test_cube_crud.sh` | Bash | Cuber CRUD 結合テスト |
| `scripts/download_bifrost.sh` | Bash | Bifrost バイナリダウンロード |
| `scripts/resize-*.js` | Node.js | 画像リサイズ |
| `scripts/zed-deepseek-launch.sh` | Bash | Zed エディタ DeepSeek 連携 |

---

## 8. エディションシステム

### 8.1 エディション定義 (`editions.json`)

`editions.json` にエディションごとの識別子、バンドル ID、データディレクトリ、リポジトリ等を定義する。`scripts/apply-edition.js` が `.env` を生成し、コンパイル時に `option_env!` で読み込まれる。

### 8.2 エディション切替フロー

```
make setup-edition EDITION=mycute
    ↓
scripts/apply-edition.js が .env を生成
    ↓
APP_EDITION, APP_SLUG, APP_BUNDLE_ID 等が設定される
    ↓
constants.rs が option_env! で APP_SLUG 等を読み込み
    ↓
tauri.conf.json が identifier 等を設定
```

---

## 9. ビルドシステム

### 9.1 ビルドツールチェーン

| ツール | バージョン |
|---|---|
| Rust | 1.92.0 (rust-toolchain.toml) |
| Node.js | v25.9.0 (内包バイナリ) |
| Swift | システム標準 (macOS) |
| .NET | 10.0 (Windows Native AOT) |
| Go | go.work 管理 |
| pnpm | SDK/Web パッケージ管理 |

### 9.2 主要 Makefile ターゲット

| ターゲット | 説明 |
|---|---|
| `all` | 全エディション一括デバッグビルド |
| `all-release` | 全エディション一括リリースビルド + GitHub リリース |
| `all-mycute` | メインエディションのデバッグビルド |
| `all-mycute-release` | メインエディションのリリースビルド |
| `server` | `mycute-server` バイナリビルド |
| `installer` | Tauri GUI インストーラビルド |
| `setup-edition EDITION=X` | エディション設定のみ適用 |
| `push` | バージョンインクリメント + git commit + push |
| `build-sdk-ts` | TypeScript SDK ビルド (esbuild) |
| `cl-dev` / `server-dev` | 開発モードビルド |
| `check` / `rh` | コードチェック / リソースハンドル確認 |
| `up-mysql` / `down-mysql` | Docker MySQL 起動/停止 |
| `release` | GitHub Releases へのアップロード |

### 9.3 ビルドフロー

```
make all
  → check-version (重複ビルド防止)
  → all-mycute:
       apply-edition.js mycute
       generate-icons (icongenie + tauri icon)
       make server (cargo build --release)
       make installer (tauri build)
  → record-version (dist/last_build_version.txt)
```

---

## 10. セキュリティ

### 10.1 暗号化スタック

| 目的 | アルゴリズム | クレート |
|---|---|---|
| JWT 署名 | Ed448-Goldilocks | `ed448-goldilocks` |
| データ暗号化 | AES-256-GCM | `aes-gcm` |
| パスワードハッシュ | bcrypt / Argon2id | `bcrypt` / `argon2` |
| TLS 1.2/1.3 | rustls + ring | `rustls` |
| 自己署名証明書 | rcgen (X.509 v3) | `rcgen` |
| 証明書パース | X.509 パーサー | `x509-parser` |
| ハッシュ関数 | SHA-3 / SHA-256 | `sha3` / `sha2` |
| 高速証明書検証 | 証明書チェーン検証 | `fastcert` |

### 10.2 認証フロー

1. **ユーザー登録/ログイン** → ZeroClaw が JWT 発行
2. **API リクエスト** → JWT 検証 (Authorization ヘッダ)
3. **CA (認証局)** → CA Token による相互認証
4. **ライセンス管理** → Owner/CA 間のライセンス発行・検証
5. **プロキシリーク検出** → プロキシ設定漏れの監視と通報

### 10.3 プラットフォームセキュリティ

- **macOS**: 音声認識権限 (SFSpeechRecognizer)、アクセシビリティ権限 (ホットキー)、Tauri `macos-private-api`
- **Windows**: WinRT Speech 認識権限
- **プロセス分離**: CL (GUI) と RT (サーバー) は別プロセス。Fate-Sharing で親プロセス消失時に子プロセスが自己終了

---

## 11. 配布パッケージ

### 11.1 配布構造

```
dist/
├── mac/v{VERSION}/
│   ├── mycute-server              # ランチャーバイナリ (自己展開型)
│   └── MYCUTE_x64.dmg              # Tauri インストーラ
├── win/v{VERSION}/
│   ├── mycute-server.exe          # ランチャーバイナリ
│   └── MYCUTE_x64.msi             # Tauri インストーラ
└── last_build_version.txt         # ビルド済バージョン記録
```

### 11.2 ランチャー内蔵リソース

`mycute-server` (launcher) は以下のリソースを `include_bytes!` で静的に内包し、実行時にカレントディレクトリに展開する:

- `mycute-server-core` (サーバー本体バイナリ)
- `libsherpa-onnx-c-api.dylib` (macOS) / `sherpa-onnx-c-api.dll` (Windows)
- `libonnxruntime.1.17.1.dylib` (macOS) / `onnxruntime.dll` (Windows)
- `SpeechHelper.dll` (Windows .NET Native AOT)
- `vcruntime140.dll`, `vcruntime140_1.dll`, `msvcp140.dll` (Windows VC++ ランタイム)

---

## 12. 定数管理と自動同期

### 12.1 Single Source of Truth

`src/constants.rs` の `pub const` 定数が唯一の真実源。

```
Makefile (make build-sdk-ts)
    ↓
scripts/gen-ts-constants.sh
    ↓
sdk-ts/src/generated_constants.ts  (自動生成)
    ↓ (include_bytes!)
Rust バイナリに組込み
```

同期対象: プロキシドメイン、ポート番号、スキームプレフィックス、API パス等

### 12.2 バージョン管理

- `src/constants.rs` の `MYCUTE_VERSION` が唯一の真実源
- `make push` により自動インクリメント (major.minor.patch, 99 で桁上がり)
- バージョン重複ビルドを `check-version` / `record-version` で防止

---

## 13. テスト

| テスト種別 | 場所 | 内容 |
|---|---|---|
| Rust ユニットテスト | `tests/pkg_test.rs` | パッケージテスト |
| Rust 暗号テスト | `tests/crypto_test.rs` | 暗号化/復号テスト |
| シェル結合テスト | `scripts/test_chat_models.sh` | LLM チャット結合テスト |
| シェル結合テスト | `scripts/test_cube_crud.sh` | Cuber CRUD 結合テスト |
| DB テスト用 | SQLite (dev-dependencies) | 軽量テスト用 DB |

---

## 14. Docker 開発環境

`docker/docker-compose.yml`:

| サービス | イメージ | ポート | 認証情報 |
|---|---|---|---|
| MySQL | `mysql:9.5.0` | 3306 | user=`asterisk`, password=`yu51043chie3` |
| PostgreSQL | `postgres:18-alpine` | 5432 | user=`asterisk`, password=`yu51043chie3` |

---

## 15. 主要用語集

| 用語 | 説明 |
|---|---|
| **CL** | Client — Tauri GUI プロセス |
| **RT** | RealTime — Axum バックエンドサーバー |
| **AM** | Auto Migration — DB マイグレーション |
| **OG** | One-shot Go — Go モジュールラッパー起動 |
| **Bifrost** | LLM ゲートウェイ (内包サブプロセス) |
| **ZeroClaw** | 認証ゲートウェイ (JWT 発行/検証) |
| **Cuber** | 投票・評価・抽選エンジン |
| **MyProxy** | SSL MITM プロキシ |
| **LMGW** | LLM Provider Gateway |
| **OSCA** | 自己署名証明書管理 |
| **CA** | 認証局 (相互認証) |
| **VAD** | Voice Activity Detection (音声区間検出) |
| **STT** | Speech-to-Text (音声認識) |
| **Cube** | Cuber における投票/評価の基本単位 |
| **SW** | Service Worker (プロキシ通信) |
| **SDK** | TypeScript SDK (MYCUTE 環境統合) |
| **Fate-Sharing** | 運命共同体 (親死→子死のプロセス監視) |
| **Proxy Leak** | プロキシ設定漏れ検出 |
| **P2P Clock Sync** | P2P 時刻同期強制ミドルウェア |
| **Edition** | エディション切替機能 (editions.json で定義) |
| **Ed448** | Ed448-Goldilocks 署名アルゴリズム |

---

*Document generated: 2026-05-07*
*Based on codebase analysis of MYCUTE v2.4.17*
*Source: https://github.com/mycute-os/mycute*
