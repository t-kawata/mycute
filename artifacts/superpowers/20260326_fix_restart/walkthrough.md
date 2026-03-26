# 実装完了報告（ウォークスルー）: 再起動・終了時のバックエンドプロセス残留バグ修正

## 実装内容
GUI の「再起動」や「シャットダウン」アクションが呼ばれた際に、Rust 側のバックグラウンド処理を手動で直接クリーンアップ（KILL）する処理を追加しました。
1. **`src/tauri_cmd/system.rs` と `src/mode/cl/main_of_cl.rs`**
   - 新規 Tauri コマンド `cleanup_for_restart` を実装し、ハンドラーに登録しました。
   - `BackendProcessGuard` を明示的に Take & Drop することで、子プロセスのバックエンドサーバーが安全・確実にキルされるようにしました。
2. **`web/src/App.vue`**
   - STT等フロント側の `shutdownMycute` と `restartMycute` 発火直前で `await invoke('cleanup_for_restart')` を実行するように修正しました。

## 検証結果
- `make check-all` を実行し、フロントエンドおよびバックエンド（Rust）ともにエラーなくコンパイルが通過することを確認しました。
- 次回GUI利用時において、「再起動」を押しアプリがリランチされても、バックグラウンドでの「ポート競合（8888等の占有）」や「ファイルロックの取得漏れ（Singleton の try_write）」による即死が発生しない仕組みが提供されます。