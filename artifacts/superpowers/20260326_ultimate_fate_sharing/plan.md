# 実装計画: OSネイティブの運命共同体（Ultimate Fate-Sharing）の実装

## 1. 目的と背景
前回の「明示的なクリーンアップコマンド」による解決策は、Rustの `Drop` スキップ問題に対する場当たり的な処置であり、本質的な解決ではありませんでした。
本計画では、OSカーネルの仕組み（パイプのクローズ、kqueueのプロセス監視）を利用し、GUI（親）がどのような理由で終了した場合でも、バックエンド（子）が即座にかつ確実に自決する「本質的な運命共同体」を構築します。

## 2. 実装方針

### A. 無名パイプによる同期 (Stdin Monitoring)
- **GUI (親)**: バックエンド起動時に `stdin` を `Piped` に設定し、プロセスが生きている間そのパイプを維持します。
- **バックエンド (子)**: 標準入力を別スレッドで待ち受けます。親が死んでパイプが閉じられた（EOF）瞬間に `exit` します。

### B. macOSネイティブ監視 (kqueue Fallback)
- パイプの継承が難しい特権昇格時などのために、macOS独自の `kqueue` (EVFILT_PROC) を利用します。
- カーネルから「親PIDが終了した」という通知をイベント駆動で受け取り、即座に終了します。

## 3. 変更内容

### 3.1. [MODIFY] src/utils/auth.rs
- `spawn_normal_server` で `cmd.stdin(Stdio::piped())` を追加。
- `BackendProcessGuard` に `ChildStdin` を保持させ、GUIが生存している間パイプが開かれ続けるようにします。

### 3.2. [MODIFY] src/mode/rt/main_of_rt.rs (およびユーティリティ)
- バックエンド起動フローの初期段階で `spawn_fate_sharing_monitor` を呼び出します。
- `stdin` の EOF および `kqueue` (macOS用) による監視スレッドを開始します。

### 3.3. [DELETE] 不要になった場当たり的コードの削除
- `src/tauri_cmd/system.rs` の `cleanup_for_restart` を削除。
- `web/src/App.vue` の `invoke('cleanup_for_restart')` を削除。
- 以前の「不細工な解決」を完全に一掃します。

## 4. 検証計画
- `make check-all` にてビルドを確認。
- `make run ARGS="cl"` で起動。
- `relaunch` を実行し、バックエンドが即座に消滅し、新プロセスがポート 8888 を瞬時に掴めることを確認。
- `kill -9` で GUI を強制終了させても、バックエンドが道連れで消滅することを確認。
