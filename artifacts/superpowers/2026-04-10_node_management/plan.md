# 実装計画書: Node.js 管理クレート `mc-nodejs` の実装

本計画書は、MYCUTE プロジェクトにおいて Node.js バイナリを各 OS 向けに最適化して管理・利用するための専用クレート `mc-nodejs` の実装手順を定めたものです。

## 概要
MYCUTE の Rust プログラム内から、外部の Node.js インストールに依存せず、同梱された Node.js (node, npm, npx) を OS 差分なく利用可能にします。

### 主要な要件
1.  **ビルド時**: ターゲット OS に応じた Node.js 圧縮バイナリ（tar.gz, xz, zip）を `include_bytes!` で静的に埋め込む。
2.  **ランタイム（初期化）**: 起動時に `MYCUTE_HOME/nodejs` ディレクトリを確認し、未展開またはバージョンが古い場合にバイナリを適切に展開する。
3.  **抽象化インターフェース**: `node`, `npm`, `npx` を呼び出すための共通 API を提供し、OS ごとの実行ファイルのパスや拡張子の違いを完全に隠蔽する。

---

## ユーザーレビューが必要な事項
> [!IMPORTANT]
> **バイナリのサイズと埋め込み方式について**
> 全OS向けの全バイナリ（合計約110MB超）を1つのバイナリに含めると、最終的な配布用バイナリのサイズが非常に大きくなります。
> 本計画では、**コンパイル時のターゲット OS に応じて、該当するバイナリのみを埋め込む**（例: macOS 向けビルドには `darwin-arm64.tar.gz` のみを含める）方式を採用します。これにより、各プラットフォーム向けの配布サイズを最小限に抑えます。

---

## 提案される変更点

### [Component] 新規クレート: `mc-nodejs` (`crates/mc-nodejs/`)
OS 差分の吸収とバイナリ管理の責務をこのクレートに集約します。

#### [NEW] [Cargo.toml](file:///Users/kawata/shyme/mycute/crates/mc-nodejs/Cargo.toml)
- 依存関係: `tar`, `flate2` (gz用), `xz2` (xz用), `zip` (zip用), `anyhow`, `cfg-if` 等を導入。

#### [NEW] [lib.rs](file:///Users/kawata/shyme/mycute/crates/mc-nodejs/src/lib.rs)
- 公開 API の定義。
- `NodeManager` 構造体を提供し、初期化・コマンド提供の窓口とする。

#### [NEW] [assets.rs](file:///Users/kawata/shyme/mycute/crates/mc-nodejs/src/assets.rs)
- `cfg-if!` マクロを使用。
- `#[cfg(all(target_os = "macos", target_arch = "aarch64"))]` 等を用い、ビルドターゲットに対応するアーカイブファイルを `include_bytes!` で保持。

#### [NEW] [installer.rs](file:///Users/kawata/shyme/mycute/crates/mc-nodejs/src/installer.rs)
- `MYCUTE_HOME/nodejs/[version]` への展開ロジックの実装。
- UNIX 系での `chmod +x` 処理の実施。
- バージョンチェック機能（`node --version` またはメタデータファイルによる確認）で、不要な再展開を防止。

#### [NEW] [executor.rs](file:///Users/kawata/shyme/mycute/crates/mc-nodejs/src/executor.rs)
- `std::process::Command` の生成をラップ。
- OS ごとのパス解決ロジックを実装：
    - macOS/Linux: `bin/node`, `bin/npm`
    - Windows: `node.exe`, `npm.cmd`
- 実行時に `PATH` 環境変数へ展開先の `bin` ディレクトリを一時的に追加し、npm 等が内部で `node` を呼び出せるように調整。

---

### [Component] メインアプリケーション連携

#### [MODIFY] [Cargo.toml](file:///Users/kawata/shyme/mycute/Cargo.toml)
- ワークスペースメンバーとして `crates/mc-nodejs` を追加。
- 必要とするバイナリ（`mycute` 等）の依存関係に `mc-nodejs` を追加。

#### [MODIFY] [main_of_cl.rs](file:///Users/kawata/shyme/mycute/src/mode/cl/main_of_cl.rs)
- アプリケーション初期化フェーズにて `NodeManager::install()` を呼び出し、Node.js 環境を準備。

---

## オープンな質問（解決済み）
> [!NOTE]
> 1. **対応アーキテクチャの範囲**: 現時点では現在 `nodejs/` ディレクトリに置いてあるもの（linux-x64, darwin-arm64, win-x64）のみを想定して実装します。
> 2. **展開先の詳細**: `MYCUTE_HOME` 直下の `nodejs` ディレクトリ（例: `~/.mycute/nodejs/v25.9.0/`）に配置します。

---

## 検証計画

### 自動テスト
- **ユニットテスト**: `cfg` モック環境下で OS ごとのパス解決ロジックが期待通り動作するかを検証。
- **インテグレーションテスト**: 実際にアーカイブの展開と `node --version` の実行が成功することを確認。

### 手動検証
1. `make build` 実行後、アプリを起動して `~/.mycute/nodejs` に正しくファイルが展開されることを確認。
2. 展開された環境下で `node`, `npm`, `npx` を呼び出し、正常に応答することを確認。
3. macOS と Windows いずれの環境でも同様に透過的な呼び出しが可能であることを確認。
