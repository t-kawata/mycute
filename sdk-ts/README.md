# MYCUTE Access SDK (Beta)

**MYCUTE Access SDK** は、Webアプリケーションを MYCUTE OS の「OS on OS」環境内でシームレスに動作させるための TypeScript ライブラリです。標準的な Web プロトコル（`http://`, `https://`）と、MYCUTE 内部のプロキシ通信プロトコル（`mycute://`, `mycutes://`）との間のブリッジとして機能します。

> [!CAUTION]
> **現在、この SDK は npm レジストリには公開されていません。**
> 使用するには、このリポジトリをビルドし、生成されたファイルをプロジェクトに手動でコピーする必要があります。

## 🎯 目的と機能

MYCUTE の `iframe` 環境内におけるセキュリティ制約（X-Frame-Options, Mixed Content）を回避するため、以下の3つの防御機能を提供します。

1.  **Fetch & XHR**: API 通信を自動的にプロキシ URL に書き換え。
2.  **Navigation**: リンククリック時のページ遷移をプロキシ内に留める。
3.  **Service Worker**: 画像や CSS などの静的リソースを含む全通信を捕捉し、プロキシ経由で取得。

## 📦 ビルド手順

まず、この `sdk-ts` ディレクトリ内で SDK をビルドし、アーティファクト（利用可能なファイル群）を生成します。パッケージマネージャには **pnpm** を使用します。

```bash
# sdk-ts ディレクトリにて
pnpm install
pnpm run build
```

ビルドが成功すると、`dist` ディレクトリに以下のファイルが生成されます（主要なもののみ記載）：

-   **`dist/index.js`**: SDK のメインエントリーポイント（Webアプリからインポートするファイル）。
-   **`dist/sw.js`**: Service Worker 本体（Webアプリの公開ディレクトリに配置するファイル）。
-   その他 (`dist/utils/`, `dist/interceptors/` 等): ライブラリの依存ファイル。

## 🚀 導入手順（手動統合）

既存の Web アプリケーション（Vue, React, Next.js 等）にこの SDK を組み込むには、以下の手順に従ってください。

### 1. SDK ファイルのコピー

ターゲットとなる Web アプリケーションのソースコードディレクトリ内に、SDK の `dist` フォルダの中身を丸ごとコピーします。
例として、`src/libs/mycute-sdk` というディレクトリを作成して配置する場合：

```bash
# ターゲットアプリのルートにて
mkdir -p src/libs/mycute-sdk
# sdk-ts/dist/* をそこにコピー
cp -r /path/to/mycute/sdk-ts/dist/* src/libs/mycute-sdk/
```

### 2. Service Worker の配置

`dist/sw.js` はブラウザが直接アクセスできる場所に配置する必要があります。多くのフレームワークでは `public` ディレクトリがこれに該当します。

```bash
# dist/sw.js を public フォルダ直下にコピー
cp src/libs/mycute-sdk/sw.js public/sw.js
```

> [!TIP]
> ファイル名は `sw.js` のままである必要はありませんが、後述の初期化オプションと一致させる必要があります。

### 3. アプリケーションでの初期化

アプリケーションのエントリーポイント（`main.ts`, `App.vue`, `index.tsx` など）で、コピーした SDK をインポートして初期化します。

```typescript
// パスはコピー先に合わせて調整してください
// .js 拡張子が必要な場合があります（プロジェクトの設定による）
import { initMycute } from './libs/mycute-sdk/index.js'; 

// アプリケーション起動時に実行
// MYCUTE (Tauri) 環境外では自動的にスキップされます
initMycute({
  // 配置した Service Worker ファイルへのパス (ルート相対)
  swPath: '/sw.js', 
  enableServiceWorker: true
});
```

### 4. 動作確認

MYCUTE 環境でアプリを起動し、コンソールを確認してください。成功していれば以下のログが表示されます。

```
[MYCUTE SDK] Initializing...
[MYCUTE SDK] Fetch & XHR interceptors active.
[MYCUTE SDK] Navigation interceptor active.
[MYCUTE SDK] Service Worker registered with scope: ...
```

## ⚙️ API リファレンス

### `initMycute(options?: MycuteSdkOptions)`
SDK の全機能を有効化します。

| オプション | 型 | デフォルト | 説明 |
| :--- | :--- | :--- | :--- |
| `swPath` | `string` | `'/sw.js'` | 公開ディレクトリに配置した Service Worker ファイルへのパス。 |
| `enableServiceWorker` | `boolean` | `true` | Service Worker の登録を行うかどうか。 |

### `toProxyUrl(url: string): string`
標準 URL (e.g. `https://google.com`) を MYCUTE プロキシ URL (e.g. `mycutes://google.com`) に変換するユーティリティ関数です。

---

## 🛠 開発者向け情報

### ディレクトリ構成
-   `src/index.ts`: エントリーポイント
-   `src/interceptors/`: Fetch, XHR, Click のフックロジック
-   `src/service-worker/`: Service Worker のロジック (`sw.ts`) 及び登録ヘルパー
-   `src/utils/`: URL 変換ユーティリティ
-   `tsconfig.worker.json`: Service Worker 用のビルド設定（WebWorker ライブラリ使用）
