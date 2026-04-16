# ウェブアプリケーション表示基盤 設計指針 (FOR_WEB_TYPE_APP_GREAT_DISPLAY)

## 1. コンセプト: OS on OS
MYCUTE は単なるアプリケーションではなく、**「OS の上で動作するもう一つの OS 層」**として機能します。
したがって、登録されたグローバルなウェブアプリケーションやウェブサイトは、MYCUTE というエコシステムの外（外部ブラウザ等）に出ることなく、その内部でシームレスに表示・完結されなければなりません。

## 2. 実装の基本方針
外部のウェブコンテンツを MYCUTE 内部の一つのアプリケーションとして扱うため、**iframe** を利用した表示を採用します。
ユーザー体験（UX）において、「内部アプリ」と「外部ウェブアプリ」の境界を感じさせない一貫性を提供します。

## 3. セキュリティ上の課題と解決策
通常のウェブブラウザ（WebView）では、セキュリティ上の理由（`X-Frame-Options` や `CSP`）により、多くのサイトが iframe 内での表示を拒否します。
本プロジェクトでは「あらゆるウェブサイトを1つも漏らさず表示できること」を最優先とし、Tauri（Rust）レイヤーで動的な通信介入を行います。

### 3.1. HTTP レスポンスヘッダーの介入
WebView にデータが届く直前で、iframe 表示を阻害する以下のヘッダーを動的に削除または書き換えます。

- **`X-Frame-Options`**: `DENY` や `SAMEORIGIN` の指定を完全に抹消します。
- **`Content-Security-Policy (CSP)`**: `frame-ancestors` 等の制限を削除、または MYCUTE オリジンを許可するように書き換えます。
- **`Set-Cookie`**: iframe 内でのセッション維持のため、`SameSite=None; Secure` への書き換えを検討します。

### 3.2. ローカルリクエストプロキシの活用
標準的なリクエストインターセプトで解決できない複雑なサイトや、最新ブラウザエンジンの厳格な制限をバイパスするため、Rust 側でローカルプロキシを構築します。

- 外部 URL を直接読み込むのではなく、ローカルプロキシを経由させることで、ブラウザに対して「安全なローカルコンテンツ」として認識させます。
- プロキシ側で通信を中継し、セキュリティヘッダーを剥ぎ取った状態で WebView へ提供します。

### 3.3. Tauri 設定の最適化 (`tauri.conf.json`)
ウェブビュー自体のセキュリティレベルと自由度を調整します。

- `dangerousDisableAssetCsp`: アセット制約の緩和。
- 外部ドメインのリソース取得（Mixed Content 等）に対する柔軟な許可設定。

### 3.4. リクエストの隠蔽（ユーザーエージェントの偽装）
- iframe 内であることを検知してブロックするサイトに対し、リクエストヘッダー（`User-Agent` や `Sec-Fetch-*`）をデスクトップブラウザのものに正規化・偽装し、遮断を回避します。

## 4. 開発フェーズの優先順位
現在は開発段階であるため、**「あらゆるウェブページを制限なく表示できるオープンなセキュリティポリシー」**を優先的に実装します。
プロダクションに近い段階で、利便性と安全性のバランスを考慮したより高度なセキュリティ対策へと段階的に移行します。

## 5. 完全実装ステップ

### Phase 1: 環境確認と基盤整備 (Steps 1-10)
1. [x] 現在の `Cargo.toml` の依存関係を確認し、`tauri` 関連のバージョンを記録する。
   - 1.1. [x] `src-tauri/Cargo.toml` をエディタで開く。
   - 1.2. [x] `[dependencies]` セクション内の `tauri` クレートのバージョンを確認する。
   - 1.3. [x] `[features]` セクション等に関連する設定がないか目視で確認し、記録する。
2. [x] `web/package.json` を確認し、フロントエンドの依存関係に不整合がないかチェックする。
   - 2.1. [x] `web/package.json` を開く。
   - 2.2. [x] `dependencies` と `devDependencies` に重複や、古いバージョンが残っていないか目視確認する。
   - 2.3. [x] `typescript` や `vue` のバージョンを今後の型定義の参考に記録する。
3. [x] `Makefile` のビルドコマンド（`make run-web` 等）が現状で正常に機能することを実行して確認する。
   - 3.1. [x] ターミナルで `make rw` (run-web) を実行する。
   - 3.2. [x] ウェブのビルドと起動がエラーなく完了することを確認する。
   - 3.3. [x] 現在の状態で WebView が正しく立ち上がることを確認する。
4. [x] `src/lib.rs` (または `main.rs`) の現在の構造を解析し、Tauri ビルダーの初期化ロジックの場所を特定する。
   - 4.1. [x] `src-tauri/src/lib.rs` を開き、`tauri::Builder::default()` を探す。
   - 4.2. [x] 現在登録されているコマンドハンドラやセットアップロジックの範囲を確認する。
   - 4.3. [x] `setup` クロージャがどのように使われているか確認する。
5. [x] Tauri の `allowlist` (許可リスト) 設定が記述されている `tauri.conf.json` の現状を記録する。
   - 5.1. [x] `src-tauri/tauri.conf.json` を開く。
   - 5.2. [x] `tauri.allowlist` セクションの現状の設定をコピーしてメモ帳等に控える。
   - 5.3. [x] 既存の `security.csp` の文字列を記録し、後で復元できるようにする。
6. [x] 新しい Rust モジュール `src/myproxy` ディレクトリを作成する。
   - 6.1. [x] `src-tauri/src/myproxy` ディレクトリをコマンドラインから作成する。
   - 6.2. [x] 作成されたディレクトリに書き込み権限があることを確認する。
   - 6.3. [x] ディレクトリ作成の完了を git status で確認する。
7. [x] `src/myproxy/mod.rs` を作成し、空のパブリック関数 `init` を定義する。
   - 7.1. [x] `src-tauri/src/myproxy/mod.rs` を新規作成する。
   - 7.2. [x] `pub fn init() {}` という空の関数を記述する。
   - 7.3. [x] ファイルを保存し、構文エラーがないことを目視確認する。
8. [x] `src/lib.rs` から `src/myproxy` モジュールを `mod myproxy;` として認識させる。
   - 8.1. [x] `src-tauri/src/lib.rs` の冒頭（mod宣言の集まり）に `mod myproxy;` を追加する。
   - 8.2. [x] `init` 関数の呼び出しはまだ行わず、宣言のみに留める。
   - 8.3. [x] 保存して、エディタ上でパス解決・オートコンプリートが効くか確認する。
9. [x] `cargo check` を実行し、モジュール追加によるエラーがないことを確認する。
   - 9.1. [x] ターミナルで `cargo check` を実行する。
   - 9.2. [x] コンパイルエラーが発生していないことを確認する。
   - 9.3. [x] 未使用関数の警告が出るかもしれないが、初期段階としては問題ないものとする。
10. [x] ここまでの環境確認と準備作業を git commit する。
    - 10.1. [x] `git add` を行う。
    - 10.2. [x] `git status` で意図しないファイルが含まれていないか確認する。
    - 10.3. [x] 「feat: initialize myproxy module structure」などのメッセージで commit する。

### Phase 2: フロントエンド表示コンテナの構築 (Steps 11-25)
11. [x] `web/src/components/apps` ディレクトリが存在するか確認し、なければ作成する。
    - 11.1. [x] `web/src/components` 内を調査する。
    - 11.2. [x] 必要に応じて `apps` フォルダを作成する。
12. [x] `web/src/components/apps/WebFrame.vue` ファイルを新規作成する。
    - 12.1. [x] 空のファイルを `web/src/components/apps/WebFrame.vue` として作成。
    - 12.2. [x] エディタで開き、Vueコンポーネントとしての枠組みを準備する。
13. [x] `WebFrame.vue` にシンプルな `div` を描画する最小構成を書く。
    - 13.1. [x] `<template>` ブロックを追加。
    - 13.2. [x] `<div class="__mycute-web-frame-container"></div>` を記述。
14. [x] `WebFrame.vue` に `url: string` を受け取るプロパティ定義を書く。
    - 14.1. [x] `<script setup lang="ts">` ブロックを追加。
    - 14.2. [x] `defineProps<{ url: string }>()` を記述。
15. [x] `WebFrame.vue` のテンプレート内に `<iframe>` タグを追加する。
    - 15.1. [x] コンテナ内に `<iframe>` を配置。
    - 15.2. [x] 必要な属性（sandbox、allow等）を記述する下地を作る。
16. [x] iframe の `src` 属性に、prop で受け取った `url` を動的にバインドする。
    - 16.1. [x] `:src="url"` を記述。
    - 16.2. [x] 初期値が空の場合の挙動を考慮し、v-if等の制御を検討する。
17. [x] iframe にクラス `__mycute-web-frame` を付与する。
    - 17.1. [x] `class="__mycute-web-frame"` を追加。
18. [x] 適切なスタイルファイルに `__mycute-web-frame` のスタイル定義枠を作成する。
    - 18.1. [x] コンポーネント内の `<style scoped>` ブロックを使用。
    - 18.2. [x] 親から受け取るサイズに追従するための基礎設定を行う。
19. [x] iframe が親コンテナを完全に埋めるように CSS を記述する。
    - 19.1. [x] width: 100%; height: 100%; を指定。
    - 19.2. [x] display: block; を指定して余計な余白を除去。
20. [x] iframe の `border` を `none` に設定し、境界線を消す。
    - 20.1. [x] border: none; を追加。
    - 20.2. [x] アウトライン等も必要に応じてリセットする。
21. [x] `MainLayout.vue` でこのコンポーネントをマウントする場所を決定する。
    - 21.1. [x] `router-view` との共存方法を検討。
    - 21.2. [x] ウェブアプリ表示時に `router-view` を隠すか、オーバーレイするか判断。
22. [x] `MainLayout.vue` に `WebFrame.vue` をインポートする。
    - 22.1. [x] import 文を追加。
    - 22.2. [x] 依存関係が解決されているかエディタで確認。
23. [x] `APP_TYPE.WEB` の場合に `WebFrame` を表示する `ref` を用意する。
    - 23.1. [x] `activeWebUrl` という `ref<string | null>` を追加。
    - 23.2. [x] 表示是非を判定する `computed` または `watch` ロジックを作る。
24. [x] フロントエンドのビルドが通るか確認する。
    - 24.1. [x] ターミナルでビルドコマンドを実行。
    - 24.2. [x] Vueコンパイラがエラーを吐かないことを確認。
25. [x] 変更内容を git commit する。
    - 25.1. [x] 「feat: add WebFrame component and UI logic」

### Phase 3: Tauri 設定のセキュリティ緩和 (Steps 26-38)
26. [x] `src-tauri/tauri.conf.json` を開く。
27. [x] `tauri.allowlist.protocol` セクションを確認する。
    - 27.1. [x] `all: true` になっていないか確認（推奨されないため、必要なものだけに絞る）。
    - 27.2. [x] `http` スコープを有効化する。
28. [x] `tauri.security.csp` の現在の設定値を確認する。
    - 28.1. [x] 既存のポリシーをコピーしてコメントに残す。
29. [x] `csp` 設定を一時的に緩和するための修正を行う。
    - 29.1. [x] `frame-src *;` または `frame-src 'self' mycute-proxy:;` を追加。
    - 29.2. [x] 既存の `default-src` との競合を調整。
30. [x] `tauri.security.dangerousDisableAssetCsp` を `true` に設定する。
    - 30.1. [x] 該当行を書き換える。
    - 30.2. [x] これによりサードパーティ製アセットが読み込めることを確認。
31. [x] `tauri.allowlist.webview` セクションを整備する。
    - 31.1. [x] 設定項目が存在するか見渡す。
32. [x] 外部ドメインへのナビゲーションを許可する設定を確認する。
    - 32.1. [x] `tauri.allowlist.shell` セクションではなく `webview` 側であることを確認。
33. [x] `Makefile` を使用して `cargo check` を実行し、JSON 構文を検証する。
    - 33.1. [x] 構文エラーがあれば修正。
34. [x] 実機でアプリを起動し、ホワイトスクリーンにならないか確認する。
    - 34.1. [x] 正常に起動することを確認。
35. [x] デモデータ（Google）をクリックし、既存の挙動を確認。
    - 35.1. [x] まだ変更前の動作が壊れていないことを保証。
36. [x] `tauri.conf.json` の変更が意図通りか再度 diff を確認する。
37. [x] 変更内容を git commit する。
    - 37.1. [x] 「chore: relax security settings for iframe support」
38. [x] Windows 環境特有の WebView2 設定が必要か調査を行う。
    - 38.1. [x] Edgeブラウザエンジンの制限事項を再確認。

### Phase 4: Rust 側カスタムプロトコルの登録 - 骨子 (Steps 39-50)
39. [x] `src/myproxy/mod.rs` に `setup_proxy` 関数のシグネチャを書く。
    - 39.1. [x] `pub fn setup_proxy(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry>` を定義。
40. [x] `tauri::Builder` を正しく受け取るように引数を調整。
    - 40.1. [x] コンパイルが通る型定義を使用。
41. [x] `src/lib.rs` のビルダーチェーンの中に登録場所を確保。
    - 41.1. [x] `.invoke_handler` の前後に配置。
42. [x] カスタムプロトコル名を `mycute-proxy` と決定。
43. [x] `Cargo.toml` に必要な依存関係を追加する。
    - 43.1. [x] `cargo add tauri --features protocol` が必要か確認。
44. [x] `ResponseBuilder` の型定義を確認。
    - 44.1. [x] `tauri::http::ResponseBuilder` などのパスを確認。
45. [x] ダミーレスポンスを返すシンプルなハンドラを書く。
    - 45.1. [x] `ResponseBuilder::new().status(200).body("OK".as_bytes().to_vec())`
46. [x] `src/lib.rs` でこのハンドラを実際に呼び出す。
    - 46.1. [x] `myproxy::setup_proxy(builder)` とチェーンさせる。
47. [x] コンパイル (`make check`) を実行。
48. [x] フロントエンド側で `mycute-proxy://test` を fetch するコードを一時的に配置。
    - 48.1. [x] console.log で結果を表示させる。
49. [x] アプリを起動し、Rust からのレスポンスを受信。
    - 49.1. [x] 疎通を確認。
50. [x] 確認後、テストコードを削除し git commit する。
    - 50.1. [x] 「feat: register mycute-proxy custom protocol」

### Phase 5: リクエスト転送ロジックの実装 (Steps 51-65)
51. [x] `reqwest` クレートを `cargo add` する。
    - 51.1. [x] `features = ["json", "blocking"]` 等を検討（今回は非同期が望ましい）。
52. [x] カスタムプロトコルハンドラ内で URL を解析。
    - 52.1. [x] `mycute-proxy://...` から実際のドメインを分離。
53. [x] 本来のアクセス先 URL を抽出する。
54. [x] `reqwest::Client` を初期化。
55. [x] メソッド（GET/POST）を維持した状態でリクエストを作成。
56. [x] フロントエンドからのヘッダーをループでコピー。
    - 56.1. [x] `header_name: value` のペアを写す。
57. [x] **重要**: `Host` と `Origin` を本来のサイトに合わせて書き換える。
    - 57.1. [x] これを行わないとサーバー側で拒否される可能性がある。
58. [x] リクエストを送信 (`send()`)。
59. [x] レスポンスのステータスコードを抽出。
60. [x] レスポンスボディを `Bytes` で取得。
61. [x] 通信失敗時のエラーハンドリングを記述。
62. [x] `make check` で非同期処理を確認。
63. [x] `src/myproxy/tests.rs` で URL 解析ロジックをテスト。
64. [x] `cargo test` を実行。
65. [x] git commit する。
    - 65.1. [x] 「feat: implement basic proxy request forwarding」

### Phase 5.5: 完全透過のための SDK 拡張と自動注入 (Steps 66-80)
66. [x] SDK 静的ファイル配信用の軽量 Web サーバー (`sw_server`) を `axum` で実装。
67. [x] SDK ファイル (`mycute_sdk.js`, `mycute_sw.js`) をバイナリに埋め込み (`include_bytes!`)、定数化 (`src/constants.rs`)。
68. [x] SDK に `WebSocket` インターセプターを実装し、URL を `mycute(s)://` に変換。
69. [x] **WebSocket 透過化**: プロキシで WebSocket アップグレードリクエストを外部サーバーへ完全リレー。
70. [x] SDK に `EventSource` (SSE) インターセプターを実装し、URL をプロキシ経由に書き換え。
71. [x] **ストリーミングの保証**: SSE/WS リクエストに対し、中継時のバッファリングを完全に無効化する判定ロジックを実装。
72. [x] SDK / Service Worker に CORS 書き換えロジックを追加。
73. [x] プロキシ (Rust) で `OPTIONS` プリフライトリクエストに対する自動許可応答を実装。
74. [x] **iFrame への強制自動注入戦略 (アプローチA)**
    - 74.1. [x] プロキシが HTML レスポンスをインターセプトし、サイトの改変を行う基盤を構築。
    - 74.2. [x] **動的デコード判定ロジック**: `Content-Encoding` ヘッダーを解析し、圧縮 (`gzip`/`br`) の有無を判定。圧縮されている場合のみ解凍処理を動的に挿入し、非圧縮（identity）の場合はそのままパッチ処理へ渡す条件分岐を実装。
75. [x] **HTML パッチの実装**: 解凍/取得した HTML の `<head>` 先頭に `https://mycute.app/mycute_sdk.js` を読み込むタグを注入。
76. [x] **透過的なエイリアス配信**: 
    - 76.1. [x] `myproxy_handler.rs` にて、`https://mycute.app/mycute_sdk.js` および `mycute_sw.js` へのリクエストを横取り。
    - 76.2. [x] 外部へ転送せず、内部の `sw_server` (localhost) からデータを直接返却。
77. [x] **セキュリティ・ホワイトリストと SW 権限の確立**:
    - 77.1. [x] `Content-Security-Policy` ヘッダーを改変し、`https://mycute.app` と `localhost` を例外許可。
    - 77.2. [x] `mycute_sw.js` 配信時に `Service-Worker-Allowed: /` を付与し、SW の制御スコープをサイト全体へ拡張。
78. [x] **パッチ適用の安全な実行とヘッダー整合性**: 
    - 78.1. [x] `Content-Type: text/html` かつレスポンス成功時のみパッチを実行し、それ以外（画像・ストリーム等）は即座にパススルーする判定。
    - 78.2. [x] パッチ適用後はデータサイズが変化するため、`Content-Encoding` ヘッダーを削除/更新し、`Content-Length` を正確に再計算してブラウザへ返却。
79. [x] 外部アプリを MYCUTE に追加し、iFrame 内のどの階層からでも SDK/SW が自動適用されることを徹底検証。
80. [x] git commit する。
    - 80.1. [x] 「feat: implement advanced SDK injection with decompression and unified origin aliasing」

### Phase 5.9: [Interrupt] 透過プロキシ通信の網羅性監視（プロキシ漏れ検知）基盤の構築
> [!WARNING]
> **フェーズ中断 & 分岐 (Phase 5.10 を参照)**
> *   **状況**: Rust側のエンドポイント実装 (`/v1/mycute_proxy_leak/*`) と `AppHandle` の統合まで完了。
> *   **中断理由**: 「情報の対称性（WebViewへの警告リレー）」を実装する過程で、`eval` による場当たり的な注入ではなく、堅牢な通信基盤（神経系）が必要であることが判明したため。
> *   **次のアクション**: Phase 5.10 を実行し、「MycuteEventBus」を確立する。**Phase 5.10 完了後、直ちに Phase 5.9 の Step 83 ("CSPヘッダーの注入") に戻らなければならない。**

**[憲法遵守 (Constitutional Mandate)]**
本フェーズ以降に実装される全ての REST API は、[REST API 開発厳格ルール](file:///Users/kawata/shyme/mycute/docs/00-REST_API_DEV_STRICT_RULES.md) を「プロジェクトの憲法」として扱い、その規約（命名・順序・バリデーション・出力構造等）に 100% 準拠しなければならない。

81. [x] **診断用APIエンドポイントの定義 (憲法準拠)**
    - [x] リクエスト構造体の定義: `src/mode/rt/rtreq/mycute_proxy_leaks_req.rs`
        - `CreateCspReportReq` (POST /v1/mycute_proxy_leak/csp)
        - `CreateSwLeakReq` (POST /v1/mycute_proxy_leak/sw)
    - [x] ハンドラの実装: `src/mode/rt/rthandler/mycute_proxy_leaks_handler.rs`
    - [x] ルーターへの登録: `req_map.rs` (ドキュメント用) および `sw_server.rs` (実動作マウント)

82. [x] **コンソールリレーの実装 (暫定版)**
    - [x] `sw_server.rs` が `AppHandle` を受け取れるように改修。
    - [x] `main_of_cl.rs` から `AppHandle` を渡すように修正 (起動タイミングを `setup` フック内に移動)。
    - [x] `window.eval` を使用してメッセージをリレー (Phase 5.10 で置換予定)。

83. [x] **[PAUSED] 監視と報告**
    - [x] Phase 5.10 の完了を待つ。
    - [x] 暫定的な `window.eval` を、正規の `MycuteEventBus.emit` に置き換える。

84. [x] **CSP ヘッダーの注入** (Phase 5.10 完了後に再開)
    - [x] `myproxy_handler.rs` にて、`Content-Security-Policy-Report-Only` ヘッダーを動的に注入する。
    - [x] `report-uri` を `http://localhost:{sw_port}/v1/mycute_proxy_leak/csp` に設定する。

85. [x] **SW/SDK インターセプターによる診断の実装**
    - [x] `mycute_sw.js` および SDK インターセプターを更新する。
    - [x] プロキシ対象外のオリジンへの `fetch` を検知するロジックを追加。
    - [x] `/v1/mycute_proxy_leak/sw` へレポートを POST 送信する。

---

### Phase 5.10: MycuteEventBus - OS神経系の構築
> [!IMPORTANT]
> **戦略目標**: MYCUTE を「単なるアプリ」から「OS on OS プラットフォーム」へと進化させる。
> 本フェーズでは、MYCUTE Kernel (Rust/Brain) と全てのフロントエンドアプリ（Organs）をつなぐ、統一された双方向通信プロトコル **MycuteEventBus** を確立する。

#### アーキテクチャ: ユニバーサル・ブリッジ (The "Universal Bridge")
1.  **下り通信 (Kernel -> App)**:
    - **プロトコル**: Tauri Events (`app.emit`)。
    - **ブリッジ機能**:
        - **内部アプリ (Internal)**: Tauri API を直接リッスンする。
        - **外部アプリ (External/iframe)**: 直接リッスンできないため、**シェル (親ウィンドウ)** がイベントを受信し、`postMessage` で iframe へ転送する。
    - **SDK 抽象化**: SDK は環境を自動判別し、アプリ開発者にはこの違いを隠蔽する。開発者は単に `Mycute.on('event', cb)` と書くだけで良い。

2.  **上り通信 (App -> Kernel)**:
    - **プロトコル**: REST API (厳格ルール準拠)。
    - **理由**: MYCUTE 内アプリだけでなく、外部スクリプトや他のツールからも等しくカーネル機能を利用可能にするため。

#### チェックリスト

86. [x] **イベントプロトコル定義 (Synapse)**
    - [x] `src/mode/rt/rtevent/` ディレクトリを作成。
    - [x] システムイベント `MycuteSystemEvent` を厳格に型定義する。
        - ペイロード定義: `ProxyLeak`, `SttStatus`, `NetworkState` 等。

87. [x] **SDK イベントレシーバーの実装 (Receptor)**
    - [x] SDK 内に `MycuteEventBus` クラスを実装する。
    - [x] **内部モード**: `window.__TAURI__.event.listen` を使用。
    - [x] **外部モード**: `window.addEventListener('message')` を使用。
    - [x] **抽象化**: `.on()` / `.off()` API を提供し、環境による差異を吸収する。

88. [x] **シェル・ブリッジ機能の実装 (Relay Station)**
    - [x] メインレイアウト（`WebFrame.vue` 相当）にて、`mycute://*` 以下の全イベントをリッスンする。
    - [x] 受信したイベントを、表示中の iframe に対して `postMessage` で安全に転送する。
    - [x] **セキュリティ**: オリジン検証を行い、信頼できる iframe に対してのみ転送する。

89. [x] **プロキシ漏れ通知の移行**
    - [x] `mycute_proxy_leaks_handler.rs` を更新する。
    - [x] 暫定的な `window.eval` を廃止し、正規の `app.emit("mycute://kernel/proxy-leak", payload)` に置き換える。

90. [x] **透過性の検証**
    - [x] 内部アプリ（設定画面等）でのイベント受信テスト。
    - [x] 外部アプリ（プロキシ経由の Google 等）でのイベント受信テスト。
    - [x] 両者が全く同じコード、全く同じペイロードで動作することを証明する。(コンパイル・実装確認済み)

91. [x] **Phase 5.10 完了 & Phase 5.9 への帰還**
    - [x] 本フェーズの完了をマークする。
    - [x] Phase 5.9 の Step 84 に戻り、作業を再開する。
---

### Phase 6: セキュリティヘッダー詳細加工 (Steps 92-105)

92. [x] **除去対象リスト（`x-frame-options` 等）を定義。**
93. [x] **条件分岐で特定のヘッダーをスキップ。**
94. [x] **`Set-Cookie` ヘッダーを特別扱いする分岐を追加。**
    - [x] トークンベースの堅牢な正規化ロジックの実装（SameSite=None, Secure, Partitioned）。
95. [x] **`SameSite` 属性を `None` に置換。**
96. [x] **修正後の `Set-Cookie` をビルダーに戻す。**
97. [x] **最終的な `ResponseBuilder` 処理を完了。**
98. [x] **`make check` を実行。**
99. [x] **Google等の URL をプロキシ経由で取得するテストを実施。**
100. [x] **ログでヘッダーが消えていことを確認。**
101. [x] **`WebFrame.vue` の `src` をプロキシ経由の URL に変換するロジックを実装。**
    - [x] 101.1. [x] `https://google.com` -> `mycute-proxy://google.com`
102. [x] **起動して Google が iframe に出ているか確認。**
103. [x] **ブラウザコンソールのエラーをチェック。**
104. [x] **除去リストを再調整。**
105. [x] **実装内容の最終確定。**
    - [x] 105.1. [x] 「feat: implement robust security header stripping and cookie normalization」

### Phase 7: 仕上げと統合検証 (Steps 106-115)
106. [-] `MainLayout.vue` で web アプリクリック時の挙動を切り替え。
    - 106.1. [-] `window.open` をコメントアウト。
    - 106.2. [-] `activeWebUrl` をセットして画面を切り替える。
107. [-] ボトムシート閉幕後の前面表示とレイアウトを調整。
108. [-] iframe 内のリンク遷移を確認。
109. [-] ナビゲーションバーの枠を配置。
    - 109.1. [-] 「戻る」「閉じる」等の UI コンポーネントの配置場所を決定。
110. [-] 複数の異なるサイトでの切り替え動作を確認。
111. [-] メモリリーク（特にWebView側の）がないか確認。
112. [-] 開発用ログをクリーンアップ。
113. [-] 最終ビルド確認 (`make check` etc)。
114. [-] `walkthrough.md` を更新。
115. [-] 最終 git commit と完了報告。
    - 115.1. [-] 「feat: complete OS-on-OS web app display foundation」

### 実装方針の変換について
docs/引き継ぎ書01.md にある通り、WebKit (Apple) の JS 層にハードコードされた「Service Worker は http/https プロトコルのみを許可する」という制約を突破するため、スキームベースのトリガー (`mycute(s)://`) を廃止し、**Domain Suffix Proxy 方式**へと移行します。

これは「失敗」による破棄ではなく、**「トリガー（点火スイッチ）の形状変更」**です。
Phase 1 〜 7 で構築した以下の強力な資産は、そのまま新方式へと継承・オーケストレーションされます。
- **MycuteEventBus**: iframe 越しの通信神経系（そのまま利用）
- **HTML Patching**: 解凍、SDK注入、ヘッダー再計算ロジック（URL変換規則のみ変更）
- **Cookie Normalization**: iframe 内セッション維持のためのクレンジング（そのまま利用）
- **Proxy Leak Detection**: 通信漏れの監視とレポート基盤（エンドポイントは維持）

これまでの作業で積み上げた「ロケットの機体（機能群）」に、新方式という「正しい点火装置」を取り付けるのが Phase 8 の役割です。

### Phase 8.0: SSL 実装の準備（詳細手順）
> [!IMPORTANT]
> **速度よりも安全性優先**: 本実装はシステムの信頼性の根幹に関わるため、全てのステップで動作確認を行いながら慎重に進める。
> **DB化への下準備**: 将来的なデータベース移行を見据え、生成された証明書は「ファイル」として出力せず、Base64 エンコードして `settings.json` (アプリケーション設定) 内に文字列として保持する「ファイルレス方式」を採用する。

8.0.1. [x] **依存関係の追加**
    - 8.0.1.1. [x] `cargo add fastcert` (最新安定版) を実行。
    - 8.0.1.2. [x] `cargo add rcgen` を実行。
    - 8.0.1.3. [x] `cargo add base64` を実行。
    - 8.0.1.4. [x] `make check` を実行し、既存の依存関係とのバージョン競合がないことを確認。

8.0.2. [x] **設定構造体 (Settings) の拡張**
    - 8.0.2.1. [x] `src/utils/init.rs` 等の `Settings` 構造体に、`proxy_certificate` および `proxy_private_key` フィールド (共に `Option<String>`) を追加。
    - 8.0.2.2. [x] JSON への保存・読み込みが正常に行えるか、既存の `settings.json` の仕組み（起動時の `-s` フラグ等）との整合性を確認。

8.0.3. [x] **SSL管理モジュール (`src/myproxy/ssl.rs`) の実装**
    - 8.0.3.1. [x] `src/myproxy/ssl.rs` を新規作成。
    - 8.0.3.2. [x] `ensure_certs() -> Result<ServerConfig>` 関数の実装。
        - 1. まず `Settings` 内に Base64 化された証明書が存在するかチェック。
        - 2. 存在しない場合は `fastcert` / `rcgen` を用いて新規生成。
        - 3. 生成したバイナリを Base64 エンコードし、`Settings` へ書き戻す（保存）。
    - 8.0.3.3. [x] メモリ上でデコードし、`rustls` 用の `ServerConfig` に変換するヘルパーを実装。

8.0.4. [x] **ルート証明書 (CA) の OS 登録**
    - 8.0.4.1. [x] `fastcert` を使用して、システムストアに「ShyMe Root CA」をインストールする処理を実装。
    - 8.0.4.2. **重要**: 初回実行時の管理者権限要求に対し、OS 側で適切に承認が行われるよう案内を出す。

8.0.5. [x] **サーバー証明書の動的発行 (Fileless)**
    - 8.0.5.1. [x] 生成した CA で `*.mc.local` および `.mc.local` 用のサーバー証明書を発行。
    - 8.0.5.2. [x] 発行されたバイナリを Base64 化し、Settings 内へ永続化する。**ファイルシステム上に直接証明書ファイルを置かないこと。**

8.0.6. [x] **モバイル配布用エクスポート (一時的)**
    - 8.0.6.1. [x] iOS/Android 向けに、CA 証明書のみを配布用ディレクトリ（`.mycute/mobile_export/` など）に一時的に取り出し可能にする、またはメモリから直接 HTTP 応答する仕組みを準備。

8.0.7. [x] **HTTPS サーバー (`server.rs`) への統合**
    - 8.0.7.1. [x] 静的な `include_bytes!` を廃止。
    - 8.0.7.2. [x] サーバー起動時に `ssl::ensure_certs()` から取得したオンメモリの証明書を使用するように改修。

8.0.8. [x] **動作確認**
    - 8.0.8.1. [x] `make run` 実行後、`settings.json` に Base64 文字列が追記されていることを確認。
    - 8.0.8.2. [x] 翌回起動時、新規生成が行われず、保存された Base64 から証明書が正しく復元されることを確認。
    - 8.0.8.3. [x] デスクトップブラウザで `.mc.shyme.net` が「安全な接続」と表示されることを確認。

### Phase 8: Domain Suffix Proxy への大移行 (Steps 116-150)
> [!IMPORTANT]
> **安全性の最優先**: 本フェーズはシステムの心臓部を入れ替える作業です。各ステップ完了ごとに `make check` および可能な限りの単体テストを実施し、デグレが発生していないことを確認しながら進めます。

#### 8.1. 基盤定数と環境の整備 (Steps 116-118)
116. [x] **プロキシサフィックスの定義**
    - 116.1. [x] `src/constants.rs` に `pub const MYCUTE_PROXY_SUFFIX: &str = ".mc.shyme.net";` を追加。
    - 116.2. [x] 既存の `MYCUTE_SCHEME_HTTP/HTTPS` 定数に `deprecated` アノテーション（コメント）を付与。
117. [x] **定数の同期と反映確認**
    - 117.1. [x] `make build-sdk-ts` を実行し、`sdk-ts/src/generated_constants.ts` に定数が書き出されたことを目視確認。
118. [x] **Rust 判定ユーティリティの作成**
    - 118.1. [x] `src/myproxy/utils.rs` (新規) または `mod.rs` に、ホスト名がサフィックスを持つか判定し、元のホスト名を抽出する純粋関数 `extract_original_host(host: &str) -> Option<String>` を作成。
    - 118.2. [x] 上記関数に対する単体テスト (`#[cfg(test)]`) を記述し、`example.com.mc.shyme.net` -> `example.com` の変換を多パターンで検証。

#### 8.2. SDK (TypeScript) 空間の刷新 (Steps 119-123)
119. [x] **URL 変換エンジンの論理変更**
    - 119.1. [x] `sdk-ts/src/utils/url.ts` の `toProxyUrl` を改修。
    - 119.2. [x] `replace('https://', ...)` 方式から、URL オブジェクトをパースして `hostname` にサフィックスを付与する方式に変更。
120. [x] **環境判定 (isMycuteEnvironment) の修正**
    - 120.1. [x] `location.protocol` チェックに加え、`location.hostname.endsWith(".mc.shyme.net")` を判定条件に追加。
121. [x] **Service Worker (mycute_sw.ts) の通信傍受ロジック更新**
    - 121.1. [x] `mycute_sw.ts` 内の fetch 書き換えロジックをサフィックス方式に同期。
122. [x] **SDK の再ビルドと整合性チェック**
    - 122.1. [x] `make build-sdk-ts` を再実行。
    - 122.2. [x] `sdk-ts/dist/mycute_sdk.js` および `mycute_sw.js` 内に `.mc.shyme.net` の文字列が含まれていることを検索して確認。
123. [x] **SDK ビルド成果物の Rust へ取り込み**
    - 123.1. [x] Rust 側で `include_bytes!` しているバイナリが最新であることを `make build` で保証。

#### 8.3. Rust プロキシプロトコルの大改造 (Steps 124-130)
124. [x] **Tauri カスタムプロトコル登録の「二重化」 (一時的な並行期間)**
    - 124.1. [x] ハンドラ関数をリファクタリングし、`scheme` (mycute/mycutes) だけでなく `host` (suffix) による判定分岐を導入。
    - 124.2. [x] 旧スキーマ (`mycute:`, `mycutes:`) も維持しつつ、新ハンドラへの移行準備を行う。
125. [x] **サフィックスベースのホスト復元ロジック実装**
    - 125.1. [x] `myproxy_handler.rs` 内で、`host.endsWith(".mc.shyme.net")` の場合にサフィックスを除去して真のホストを復元する処理を追加。
126. [x] **HTML パッチ機能の適応**
    - 126.1. [x] iframe 内への SDK 注入ロジックが、新方式の URL でも動作することを確認（または修正）。
127. [x] **CSP / Header 制御の緩和**
    - 127.1. [x] `.mc.shyme.net` からのリクエストや、`.mc.shyme.net` への fetch が CSP エラーにならないように `myproxy_handler.rs` のヘッダー注入ロジックを調整 (Report Only に `*.mc.shyme.net` を追加)。
128. [x] **SDK 注入タグの修正**
    - 128.1. [x] 注入される `<script src="/mycute_sdk.js">` 等のパスが、新方式のドメイン空間でも正しく Rust 側に解決されることを確認。
129. [x] **Cookie (Set-Cookie) 属性調整の再検証**
    - 129.1. [x] `.mc.local` ドメインにおいて、`Domain` 属性の書き換えが必要か（あるいは削除してホスト固定にするか）を検討・実装（既存の正規化でDomain属性削除を実施済み）。
130. [x] **CORS プリフライト応答 (`OPTIONS`) の緩和維持**
    - 130.1. [x] 新しいドメイン空間からの `OPTIONS` リクエストに対しても、既存の「全許可」応答が正しく機能することを確認（myproxy_handler.rs内で分岐前に処理されているためOK）。

#### 8.4. UI / 統合デプロイメント (Steps 131-135)
131. [x] **サフィックスを含むURLへの強制書き換え (Phase 8 Refactoring)**
    - 131.1. [x] `window.location` などをフックし、`mycute:` スキームの代わりに `https://<original>.mc.shyme.net` へ遷移させるロジックを検討・実装（既存の `mycute_sdk.js` 内フックを改修）。
132. [x] **WebView2 / WebKit のプロキシ設定 (名前解決のバイパス)**
    - 132.1. [x] `*.mc.shyme.net` へのアクセスが、OS の DNS 解決を行わず、ダイレクトに Rust 側プロキシサーバー (localhost) に向くように設定。
    - 132.2. [x] Tauri の `ProxyUrl` API または `Webview2` の `--proxy-server` フラグを使用。
133. [x] **SSL証明書の検証回避 (Self-signed)**
    - 133.1. [x] プロキシが返すオレオレ証明書 (`*.mc.shyme.net`) を WebView が受け入れるようにする（無視設定、またはルート証明書の動的インストール）。
134. [x] **Rust Proxy Server の本格稼働**
    - 134.1. [x] `axum` / `hyper` 等で HTTPS CONNECT を受けるサーバーを実装。
    - 134.2. [x] `*.mc.shyme.net` のリクエストに対してのみ MITM 復号を行い、内部ロジック (`myproxy_handler`) へ引き渡す。
135. [x] **デバッグログの強化**
    - 135.1. [x] 移行期特有の問題（URL 変換の失敗、DNS 未解決）を即座に特定できるよう、Rust/SDK 両方に詳細なトレースログを一時的に追加。

#### 8.5. 最終検証とクリーンアップ (Steps 136-150)
136. [x] **Service Worker 登録の正常性確認**
    - 136.1. [x] WebKit コンポーネントで `TypeError: protocol must be http or https` が消滅することを確認。
    - 136.2. [x] 実際に SW が `active` 状態になり、通信を掌握し始めることを確認。
    - 136.3. [x] Rust 側で `make build` が正常に通り、最新の SDK が埋め込まれていることを保証。
137. [x] **EventBus 疎通テスト**
    - 137.1. [x] 新方式のドメイン空間でも、`postMessage` による OS 神経系が死んでいないことを確認。
138. [x] **Cookie セッション維持テスト**
    - 138.1. [x] ログインが必要なサイトで、リフレッシュ後もセッションが維持されているかを確認。
139. [x] **レガシープロトコルの埋葬**
    - 139.1. [x] `native/swift/ProtocolHelper.swift` などのファイルをプロジェクトから物理的に削除。
    - 139.2. [x] `tauri.conf.json` の CSP および権限設定から `mycute:`, `mycutes:` を完全に削除。
140. [x] **REST API 憲法への再準拠確認**
    - 140.1. [x] `/v1/mycute_proxy_leak/*` へのレポートが、新方式のドメイン空間からでも憲法通りに送信・受信できているかを確認。
141. [x] **統合検証完了**
    - 141.1. [x] `make check` の最終パス。
    - 141.2. [x] `make test` による全テストの合格確認。
    - 141.3. [x] Phase 8 完了、および MYCUTE 通信基盤の完全オーケストレーションを宣言。

---

### Phase 8.7: 特権分離アーキテクチャ (Privilege Separation) - 超高解像度実装計画
> [!IMPORTANT]
> **戦略目標**: アプリ全体を管理者権限で動かす「全昇格」の方針を撤回し、**「GUI（Client）はUser権限、エンジン（Server）のみRoot権限」**で動作させる分離アーキテクチャへ移行する。
> これにより、macOS/Windows でのホットキー・D&D 阻害要因を完全に排除しつつ、ポートバインドや証明書管理に必要な特権を確保する。

#### 8.7.1. 全昇格ロジックの完全ロールバック (Steps 171-175)
171. [x] **Windows マニフェストの除去とビルド修正**
    - 171.1. [x] `src/mycute.exe.manifest` ファイルを物理削除。
    - 171.2. [x] `build.rs` から `embed-resource` クレートへの依存と、マニフェストコンパイル処理 (`compile("src/mycute.exe.manifest", ...)`) を削除。
    - 171.3. [x] `Cargo.toml` から `[build-dependencies]` の `embed-resource` を削除。
172. [x] **`main.rs` / `auth.rs` の自己昇格コードの除去**
    - 172.1. [x] `main.rs` 冒頭にある `auth::is_root()` チェックおよび「非rootなら終了/昇格」という分岐ロジックを全て削除。
    - 172.2. [x] `auth.rs` から `re_exec_with_gui_elevation` 関数（自分自身を再実行するコード）を削除。
    - 172.3. [x] 起動時引数 `--elevated-internal` の定義とパース処理を削除。

#### 8.7.2. クロスプラットフォーム昇格サーバー起動 (`spawn_elevated_server`) の実装 (Steps 176-185)
176. [x] **実行バイナリの特定とモバイルガード**
    - 176.1. [x] `std::env::current_exe()` を使用して、現在実行中のバイナリの絶対パスを `PathBuf` で取得するロジックを実装。これにより `PATH` 環境変数に依存せず、確実に「自分自身のバックエンド」を呼び出す。
    - 176.2. [x] モバイル環境 (`target_os = "ios"`, `target_os = "android"`) 向けには、この関数が `Ok(None)` などを返し、何もしない（No-op）ように `cfg` 属性で分岐する。

177. [x] **macOS 実装: `sudo` (CLI) / `osascript` (GUI) の二刀流**
    - 177.1. [x] **GUI判定**: `isatty` チェックなどを行い、ターミナルからの実行でない場合（`.app`起動時）は `osascript` パスを選択。
    - 177.2. [x] **`osascript` (GUI)**:
        - `do shell script "'/path/to/mycute' cl -r s" with administrator privileges` を実行。
        - **重要**: ログは標準出力に出ないので、`/tmp/mycute_server.log` 等へリダイレクトするようにコマンドを構築する。
    - 177.3. [x] **`sudo` (CLI)**:
        - `Command::new("sudo").arg("-S")...` を使用し、親プロセスの標準入出力を継承するか、パイプでパスワード入力を中継する。
        - これにより開発時のログ出力をリアルタイムで維持する。

178. [x] **Windows 実装: `runas` Verb**
    - 178.1. [x] `windows` crate の `ShellExecuteW` または `powershell` の `Start-Process -Verb RunAs` を使用。
    - 178.2. [x] 引数として `cl -r s` を指定し、対象実行ファイルに `current_exe()` のパスを渡す。
    - 178.3. [x] ウィンドウ表示フラグに `SW_HIDE` (0) を指定し、背後で黒い画面が一瞬出るのを防ぐ（UACダイアログのみ表示）。

179. [x] **Linux 実装: `pkexec` (GUI) 優先**
    - 179.1. [x] `pkexec` コマンドの存在を `which` 等で確認。
    - 179.2. [x] **`pkexec` (GUI)**:
        - `pkexec /path/to/mycute cl -r s` を実行。これにより Polkit の認証ダイアログが表示される。
    - 179.3. [x] **`sudo` (Fallback)**:
        - `pkexec` がない、またはヘッドレス環境で `sudo` が使える場合（`sudo -n true` チェック等）は `sudo` を使用。

180. [x] **プロセス管理と所有権**
    - 180.1. [x] 生成した子プロセスのハンドル（PID）を保持し、親（GUI）終了時に `kill` などを送って道連れにする（ゾンビプロセス防止）。
    - 180.2. [x] シグナルハンドリング (`Ctrl+C` 等) でサーバーも確実に落とす `Drop` トレイトの実装。

#### 8.7.3. クライアント(GUI)側のオーケストレーション (Steps 186-195)
186. [x] **サーバーヘルスチェック (Heartbeat)**
    - 186.1. [x] `Client` モード起動直後、設定ファイルにある `server_port` (例: 12345) に対して TCP 接続 (または HTTP GET `/health`) を試行。
    - 186.2. [x] 応答があれば「既存サーバーあり」とみなし、起動処理をスキップ。

187. [x] **オート・エレベーション・シーケンス**
    - 187.1. [x] サーバー不在と判断した場合、`auth::spawn_elevated_server()` を呼び出す。
    - 187.2. [x] ユーザーにOSのパスワードプロンプトが表示される。
    - 187.3. [x] **待機ループ**: サーバー起動には数秒かかるため、成功するまで 0.5秒おきに最大 10回程度ヘルスチェックをリトライする。
    - 187.4. [x] タイムアウトまたは承認キャンセル（プロセスの即死）を検知した場合、GUI 上に「サーバー起動に失敗しました。管理権限が必要です」といったエラーダイアログを出し、アプリを終了または機能制限モードで起動する。

188. [x] **設定ファイル (`settings.json`) の共有と権限**
    - 188.1. [x] Server (Root) は設定ファイルを読み書きする可能性がある（証明書保存など）。
    - 188.2. [x] macOS/Linux では、Root が作成したファイルは一般ユーザーが読めなくなる恐れがある。
    - 188.3. [x] Server 側でファイル保存時に `chown` を行うか、あるいは設定ファイルの保存場所を適切に分離（System vs User）するロジックを確認・実装する.
    - 188.4. [x] **推奨**: 証明書データ等は Root 権限でしか触れない場所に置くか、設定ファイル自体は User 権限で書き込み可能な場所に置き、Server は読み取り専用、または「User 権限に降格して書き込む」ロジックが必要か検討（今回は Root で書き込み、パーミッションを `644` にすることで User からの読み取りを許可する方針とする）.

#### 8.7.4. 統合検証とエッジケース確認 (Steps 196-200)
196. [x] **ホットキーおよび入力監視のテスト (macOS/Windows)**
    - 196.1. [x] アプリ起動 -> パスワード入力 -> サーバー起動。
    - 196.2. [x] この状態で、他のアプリにフォーカスを当てて `Option+S` を押下し、MYCUTE が前面に来るか確認（User権限であることの証明）。
197. [x] **証明書生成フローのテスト**
    - 197.1. [x] 証明書未生成（`settings.json` の該当項目削除）状態で起動。
    - 197.2. [x] 自動的に Server が昇格起動し、裏で証明書生成・信頼ストアへの登録（Root権限が必要）が成功するか確認。
198. [x] **多重起動防止の確認**
    - 198.1. [x] 既にアプリ起動中に、もう一度アイコンをクリックまたは `make run`。
    - 198.2. [x] 新しい GUI プロセスが立ち上がるが、サーバーは既存のものを使用し、ダブって起動しないことを確認。
199. [x] **強制終了時の挙動**
    - 199.1. [x] GUI を `Kill` した場合、裏の Server もタイムアウト等で終了するか、または次回の### Phase 8.8: スプラッシュ画面による起動シーケンス刷新 - Simplified High Resolution (Steps 201-250)
> [!IMPORTANT]
> **戦略目標**: アプリ起動時に即座にウィンドウ（スプラッシュ画面）を表示し、ユーザーに「起動した」という安心感を与える。既存の `isLoaderOn` (Pinia) を活用し、最小限の実装でサーバー接続待ちとセットアッププロセスを視覚化する。

#### 8.8.1. バックエンド実装: CA セットアップ API (Steps 201-215)
201. [x] **CA ロジックの共有ライブラリ化 (`src/myproxy/ssl/setup.rs`)**
    - 201.1. [x] `src/myproxy/ssl.rs` 内の `ensure_certs` 関数を、副作用（標準出力/プロセス終了）のない純粋な関数 `create_certs_if_missing` として `src/myproxy/ssl/setup.rs` (新規) に切り出す。
    - 201.2. [x] **戻り値の定義**: `Result<SetupStatus, anyhow::Error>` を返すようにする。`SetupStatus` 列挙型を定義し、`Created` (新規作成), `Existing` (既存あり), `Updated` (再作成) の状態を区別できるようにする。
    - 201.3. [x] **権限管理**: 生成された `settings.json` および内部の証明書データが、一般ユーザー権限（Client）から読み取り可能なパーミッション (`644` / `rw-r--r--`) で保存されるロジックを `std::os::unix::fs::PermissionsExt` 等を用いて明示的に実装する。

202. [x] **システム操作ハンドラの実装 (`src/mode/rt/rthandler/ca_handler.rs`)**
    - 202.1. [x] `src/mode/rt/rthandler/ca_handler.rs` を新規作成する。
    - 202.2. [x] **Request/Response 構造体の定義**:
        - `pub struct CaSetupRes { status: String, message: String }` (Derive: Serialize)
    - 202.3. [x] **ハンドラ関数 `setup_ca` の実装**:
        - シグネチャ: `pub async fn setup_ca() -> Result<Json<CaSetupRes>, ApiError>`
        - 処理: `myproxy::ssl::setup::create_certs_if_missing()` を呼び出す。
        - 成功時: `200 OK` と JSON を返す。
        - 失敗時: `500 Internal Server Error` とエラー詳細を返す。
    - 202.4. [x] **セキュリティガード**: 接続元 IP アドレスを確認し、`127.0.0.1` 以外からのリクエストは即座に `403 Forbidden` で拒否するロジックを挟む（ミドルウェアまたはハンドラ先頭）。

203. [x] **Axum ルーターへのエンドポイント登録 (`src/mode/rt/req_map.rs`)**
    - 203.1. [x] `api_v1` 関数内に新しいルートを追加: `post(rthandler::ca_handler::setup_ca)`。
    - 203.2. [x] パスは `/v1/ca/setup` とする。
    - 203.3. [x] Swagger (Utoipa) 用のコメント `#[utoipa::path(...)]` をハンドラに追加し、ドキュメントに反映されるようにする。

204. [x] **ヘルスチェックエンドポイントの仕様確認**
    - 204.1. [x] 既存の `/health` または `/` (root) が、認証なしで `200 OK` を返すことを再確認する。スプラッシュ画面からのポーリングに使用するため。

205. [x] **バックエンド単体検証**
    - 205.1. [x] `cargo check` を実行。
    - 205.2. [x] 手動で `rt` モードで起動し、`curl -X POST http://127.0.0.1:port/v1/ca/setup` を叩いて動作を確認する。

#### 8.8.2. フロントエンド実装: スプラッシュ画面とロジック (Steps 216-235)
> [!NOTE]
> 独自のスプラッシュコンポーネントは作成せず、既存の `MainStore.isLoaderOn` を利用して全画面ローダーを表示する。

216. [x] **Splash Layout/Page の最小実装 (`web/src/layouts/SplashLayout.vue`)**
    - 216.1. [x] `web/src/layouts/LoginLayout.vue` をコピーして `web/src/layouts/SplashLayout.vue` を作成。
    - 216.2. [x] `web/src/pages/LoginPage.vue` をコピーして `web/src/pages/SplashPage.vue` を作成。
    - 216.3. [x] `SplashPage.vue` のテンプレート内を空（または最小限のコンテナのみ）にし、フォーム類を全て削除する。
    - 216.4. [x] **Script Setup**:
        - `onMounted` で即座に `store.setIsLoaderOn(true)` を呼び出し、全画面ローダーを表示する。
        - 独自のメッセージ表示用ロジックは持たず、QSpinner 等のデフォルト挙動に任せる。

217. [x] **起動シーケンスロジック (`web/src/pages/SplashPage.vue`)**
    - 217.1. [x] **ロジック関数 `checkServerHealth`**:
        - `fetch('http://127.0.0.1:{rt_port}/')` を 500ms 間隔で実行。
        - 成功 -> `setupCa()` へ進む。
        - 失敗 -> リトライ (max 60回)。タイムアウト時は `store.setIsLoaderOn(false)` し、シンプルなエラーダイアログを表示（`$q.dialog`等）。
    - 217.2. [x] **ロジック関数 `setupCa`**:
        - `fetch('http://127.0.0.1:{rt_port}/v1/ca/setup', { method: 'POST' })` を実行。
        - 成功 -> `navigateNext()` へ進む。
        - 失敗 -> ローダー解除後、エラー表示。
    - 217.3. [x] **ロジック関数 `navigateNext`**:
        - `store.setIsLoaderOn(false)` を実行（ローダー解除）。
        - 認証トークンチェック等の既存ロジックに従い、`router.replace('/app')` または `router.replace('/auth/login')` を実行。
    - 217.4. [x] `onMounted` 内でこれらのシーケンスを開始する。

218. [x] **ルーティングの変更 (`web/src/router/routes.ts`)**
    - 218.1. [x] ルルート (`/`) を `SplashLayout` -> `SplashPage` に変更する。
    - 218.2. [x] 既存のルート（LoginやMain）は維持する。
    - 218.3. [x] **重要**: スプラッシュ画面は履歴に残さないため、遷移時は `router.replace` を使用することをコードコメントで明記。

219. [x] **環境変数の注入確認 (`web/src/config.ts` or `env`)**
    - 219.1. [x] SplashPage からアクセスする `RT_PORT` が取得可能か確認。

#### 8.8.3. クライアント(Rust)実装: 非同期起動への変更 (Steps 236-245)
236. [x] **`src/mode/cl/main_of_cl.rs` のクリーンアップ**
    - 236.1. [x] **Blocking Loop の削除**: `spawn_elevated_server` の呼び出し直後に存在していた `loop { check_health... }` ブロックを完全に削除する。
    - 236.2. [x] **Spawn の非同期化**: `spawn_elevated_server` はプロセスを起動して PID を返すだけなので、基本的にブロッキングしないが、念のため処理が軽快であることを確認。
    - 236.3. [x] **証明書チェックの緩和**: `load_settings` で証明書が見つからなくても `bail!` せず、`warn!` ログを出して処理を続行（Tauri ビルダーへ進む）ように変更。
    - 236.4. [x] **Tauri Build**: そのまま `tauri::Builder` を実行し、ウィンドウを表示させる。

#### 8.8.4. 統合検証とQA (Steps 246-250)
246. [x] **シナリオ A: 初回インストール (Clean Install)**
    - 246.1. [x] `rm -rf ~/.mycute` で環境削除。
    - 246.2. [x] `cargo run -- cl`
    - 246.3. [x] **期待値**:
        1. 即座にウィンドウ表示（ローダー回転）。
        2. OS のパスワードダイアログ出現。
        3. 承認後、ローダーが消えて Login 画面へ遷移。
        4. `~/.mycute/settings.json` に証明書が生成されている。

247. [x] **シナリオ B: サーバー起動失敗 (Server Failure)**
    - 247.1. [x] パスワードダイアログで「キャンセル」を押す。
    - 247.2. [x] **期待値**:
        1. ローダーがタイムアウトまで回転。
        2. タイムアウト後、エラーダイアログ表示。

248. [x] **シナリオ C: 既存環境 (Normal Boot)**
    - 248.1. [x] **期待値**: ローダー回転 -> (即応答 & Setup完了) -> Main画面へ。体感 1秒以内。

249. [x] **UI/Design Review**
    - 249.1. [x] `LoginLayout` ベースのため、テーマ逸脱がないことを確認。

250. [x] **完了コミット**
    - 250.1. [x] メッセージ: `feat: implement simplified splash orchestration using store loader`。

### Phase 8.9: Double-Hyphen Wildcard DNS Strategy Implementation (Steps 251-280)
> [!IMPORTANT]
> **戦略目標**: macOS/WebKit の DNS 挙動に対応するため、`socks5` プロトコルを廃止し、**「外部IPへのAレコード」+「ダブルハイフンエンコーディング」** を用いた HTTP プロキシ戦略へ移行する。
> これにより、OSのDNS解決をパスしつつ、プロキシサーバーへの確実なルーティングを実現する。

#### 8.9.1. エンコーディング仕様の確定 (Steps 251-252)
251. [x] **エンコーディングルールの定義**
    - 251.1. [x] **Encode**: `.` を `--` に置換し、末尾にサフィックス (`.mc.shyme.net`) を付与。
        - `google.com` -> `google--com.mc.shyme.net`
        - `api-server.org` -> `api-server--org.mc.shyme.net`
    - 251.2. [x] **Decode**: サフィックスを除去後、`--` を `.` に置換。単一の `-` は変更しない。
        - `google--com` -> `google.com`
        - `api-server--org` -> `api-server.org`

252. [x] **共有ライブラリの実装 (`utils/url_encoder.ts` & `src/myproxy/utils.rs`)**
    - 252.1. [x] Rust: `extract_original_host` にデコードロジックを実装（実装済み、再確認）。
    - 252.2. [x] TS: `sdk-ts/src/utils/url_encoder.ts` を新規作成し、`encodeHost(host: string): string` および `decodeHost(host: string): string` を実装。テストコードも含める。

#### 8.9.2. フロントエンド (Service Worker & SDK) の適応 (Steps 253-260)
253. [x] **Service Worker (`mycute_sw.ts`) の改修**
    - 253.1. [x] `fetch` ハンドラ内で、リクエスト先が「外部ドメイン」かつ「サフィックスを持たない」場合、**能動的にエンコードを行い、リダイレクトする** ロジックを追加。
    - 253.2. [x] `fetch('https://google.com/...')` -> `fetch('https://google--com.mc.shyme.net/...')`

254. [x] **SDK (`mycute_sdk.ts`) の URL 生成ロジック改修**
    - 254.1. [x] `window.open` や動的リンク生成時にもエンコーディングを適用するヘルパーを提供。

255. [x] **SDK ビルドと反映**
    - 255.1. [x] `make build-sdk-ts` を実行。
    - 255.2. [x] `make build` で Rust バイナリに最新の SDK を取り込む。

#### 8.9.3. バックエンド (Rust Proxy Server) の完全対応 (Steps 261-270)
261. [x] **リクエスト処理 (`myproxy_handler.rs`)**
    - 261.1. [x] `register_protocol` 内で、ターゲット URL 生成時のエンコード漏れがないか再確認。基本はデコード（受信）が主だが、リダイレクト追跡などで内部生成する場合はエンコードが必要。

262. [x] **レスポンス HTML のリライト (The Missing Piece)**
    - 262.1. [x] **HTML Rewriter の導入**: `lol_html` 等のストリーミングパーサー、または正規表現による置換処理を実装。
    - 262.2. [x] **リンク属性の書き換え**:
        - `<a href="...">`, `<img src="...">`, `<script src="...">`, `<form action="...">`
        - 属性値が絶対パスかつ外部ドメインの場合、エンコードされた形式 (`...--com.mc.shyme.net`) に書き換える。
    - 262.3. [x] **目的**: これにより、ユーザーがリンクをクリックした際もサフィックス環境内に留まれるようにする。

263. [x] **レスポンスヘッダーのリライト**
    - 263.1. [x] **`Location` ヘッダー**: 3xx リダイレクト時、リダイレクト先 URL をエンコードする。
        - `Location: https://accounts.google.com/` -> `Location: https://accounts--google--com.mc.shyme.net/`
    - 263.2. [x] **`Set-Cookie` Domain 属性**: 必要に応じてドメイン属性を削除またはエンコードされたドメインに書き換える（既に削除ロジックはあるが、整合性を確認）。

#### 8.9.4. 統合検証 (Steps 271-280)
271. [-] **`google.com` へのアクセス検証**
    - 271.1. [-] 検索ページが表示されること。
    - 271.2. [-] 検索結果のリンクをクリックしても、アプリ内に留まる（外部ブラウザに飛ばない、またはエラーにならない）こと。

272. [-] **ハイフン入りドメインの検証**
    - 272.1. [-] `api-server.org` 等、ハイフンを含むドメインが正しく復元され、アクセスできること。

273. [-] **認証フロー検証**
    - 273.1. [-] Google ログイン等の複雑なリダイレクトを含むフローが完走できること。

274. [-] **完了コミット**
    - 274.1. [-] `feat: implement complete double-hyphen wildcard dns strategy`

### 9. Direct Hosting Architecture Migration (Phase 8.20)
目標: WebKit のプロキシバイパス問題を打破するため、プロキシ構成を廃止し、ポート 58300 での直接 HTTPS ホスティング構成へ移行する。

#### 9.1. Frontend & SDK URL Logic Update
280. [ ] **URL Generation Strategy Update**
    - 280.1. [ ] `sdk-ts/src/utils/url.ts`: `toProxyUrl` 関数を修正。
        - 変更前: `https://${encoded}.mc.shyme.net/...`
        - 変更後: `https://${encoded}.mc.shyme.net:58300/...`
        - 目的: ブラウザにポート 58300 への直接接続を強制させる。
    - 280.2. [ ] `make build-sdk-ts` を実行し、変更を反映。

#### 9.2. Tauri Application Configuration
281. [ ] **Disable Native Proxy Settings**
    - 281.1. [ ] `src/mode/cl/main_of_cl.rs`: `window_builder.proxy_url(...)` の呼び出しを削除。
        - 目的: OS/ブラウザのプロキシ設定機能への依存を完全に排除する。

#### 9.3. Rust Backend Transformation (HTTPS Server)
282. [ ] **Server Binding & TLS Configuration (`src/myproxy/server.rs`)**
    - 282.1. [ ] **Binding Address**: `IP_LOCALHOST` (127.0.0.1) ではなく `0.0.0.0:58300` にバインド変更（念のため全インターフェース許可）。
    - 282.2. [ ] **TLS Setup**: `axum_server::bind_rustls` (または `rustls::ServerConfig` + `axum::serve`) を使用し、常に HTTPS で待ち受ける構成にする。
        - 証明書: `fastcert` でロードしたサーバー証明書 (`server_config`) を使用。

283. [ ] **Request Handling Logic (`src/myproxy/server.rs` & `handler`)**
    - 283.1. [ ] **Remove CONNECT Support**: HTTP CONNECT メソッドのハンドリング分岐を削除。
    - 283.2. [ ] **Direct Request Handling**: 全てのリクエストを `handle_proxy_request` に流し、そこで「Hostヘッダー解析 -> 転送」を行うパススルーリバースプロキシとして動作させる。

#### 9.4. API Integrity & CORS/Swagger
284. [ ] **CORS Configuration (`src/mode/rt/req_map.rs`)**
    - 284.1. [ ] `CorsLayer` の `allow_origin` に `https://*.mc.shyme.net:58300` を追加（動的判定ロジックの実装が必要な場合あり）。
        - 理由: WebView (Origin: `https://...:58300`) からの API コールを許可するため。

285. [ ] **Swagger UI Update (`src/mode/rt/req_map.rs`)**
    - 285.1. [ ] OpenAPI (`ApiDoc`) の `servers` 定義を追加。
        - URL: `https://127.0.0.1:58300` (または `https://api.mc.shyme.net:58300`)
        - 目的: Swagger UI 上の "Try it out" が正しいポートとプロトコルで実行されるようにする。

#### 9.5. Internal Health Check
286. [ ] **Health Check Update (`src/utils/init.rs` / `tauri_cmd`)**
    - 286.1. [ ] ヘルスチェックのターゲット URL を `https://127.0.0.1:58300/health` に変更。
    - 286.2. [ ] 自己署名証明書を許容する (`danger_accept_invalid_certs`) HTTP クライアントを使用するように修正。

#### 9.6. Verification
287. [ ] **Integration Test**
    - 287.1. [ ] `make rc` で起動し、スプラッシュ画面 -> アプリ画面への遷移を確認。
    - 287.2. [ ] 外部サイト (Google等) が SSL エラーなく表示されることを確認。
    - 287.3. [ ] Swagger UI が表示され、API 実行が成功することを確認。