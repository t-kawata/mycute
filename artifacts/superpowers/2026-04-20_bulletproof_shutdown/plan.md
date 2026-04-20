# 運命共同体シャットダウンの防弾化（A+B+C 統合実装）計画書

## 目的
GUIアプリのシャットダウン時および再起動時に、`Bifrost` や `ZeroClaw` といったバックエンドの子プロセスがゾンビとして取り残される問題を解決します。
今回の調査で発覚した「起動順序の誤りによるクリーンアップの空振り」と「終了時の Watchdog 介入による競合状態」を同時に修正し、防弾（Bulletproof）設計を実現します。

## 変更内容の概要
1.  **[B] 委譲型 Graceful Shutdown**: `kill -9` による即死をやめ、標準入力（`stdin`）のクローズ（EOF）をトリガーとした `main_of_rt` 主導の自死シーケンスに変更。
2.  **[A] Watchdog の抑制**: 正常な終了プロセス中に、GUI 側の監視スレッドが誤ってプロセス全体を道連れ終了させないよう、シャットダウンフラグを導入。
3.  **[C] 並列高速クリーンアップ**: 終了時のフェイルセーフであるポートクリーンアップを直列から並列処理に変更し、OS による強制終了前に瞬時に全ポートを掃除させる。
4.  **[Fix] 起動時のクリーンアップ順序適正化**: `main_of_rt.rs` にて、DB から正しい設定（ポート番号など）をロードした**後**にクリーンアップを実行するように配置を修正。

---

## 具体的な修正タスク

### 1. GUI 側 (`src/mode/cl/main_of_cl.rs`)
*   `TauriState` 構造体に `is_shutting_down: Arc<AtomicBool>` を追加する。
*   `TauriState` の初期化時に `is_shutting_down` を `false` でインスタンス化する。
*   `manage_backend_server` 内の Watchdog ループにて、`is_shutting_down.load(Ordering::Relaxed)` が `true` の場合は、`token.cancel()` などの強制終了ロジックを発動せず、ループを終了（`return`）するようにする。
*   `app.run` の `RunEvent::ExitRequested | RunEvent::Exit` ハンドラ内において、`backend_guard` を破棄（`drop`）する**直前**に `state.is_shutting_down.store(true, Ordering::Relaxed)` を実行し、Watchdog に終了プロセスに入ったことを通知する。

### 2. バックエンド側 (`src/mode/rt/main_of_rt.rs`)
*   現在ファイルの冒頭（80行目付近）にある `config_manager.cleanup_all_backend_ports("Startup");` の呼び出しを削除する。
*   230行目付近の `config_manager.ensure_initialized_with_db().await?;` の呼び出しの**直後**に、削除した `cleanup_all_backend_ports` を移動する。
    *   これにより、DB から最新のポート番号（`3912`など）を取得した状態で正確なクリーンアップが可能となる。
*   ※この移動に伴い、`cleanup_all_backend_ports` が誤って新しく起動した直後の `Bifrost` や `ZeroClaw` を殺してしまわないよう、`bifrost_manager.spawn` や `zeroclaw_manager.spawn` の呼び出し位置も、`cleanup_all_backend_ports` の**後**に移動させる。
    *   順序：`ensure_initialized_with_db` -> `cleanup_all_backend_ports` -> `bifrost_manager.spawn` -> `zeroclaw_manager.spawn`

### 3. プロセス管理 (`src/utils/auth.rs`, `src/utils/process.rs`)
*   **`src/utils/auth.rs`**: `BackendProcessGuard::drop` の実装を以下のように変更する。
    *   まず `self.stdin.take()` を実行し、`main_of_rt` に対してパイプを閉じる（これで `main_of_rt` 側の `watch_stdin` スレッドが EOF を検知し、自死と子プロセスの道連れロジックを開始する）。
    *   `kill -9` の実行を即座に行わず、自死シーケンスが完遂するのを待つために `std::thread::sleep(std::time::Duration::from_millis(500))` 程度の猶予を設ける。
    *   その後に `config_mgr.cleanup_all_backend_ports("Fate-Sharing")` を呼び出し、最終的なフェイルセーフクリーンアップを実行する。
    *   （`kill -9` はバックエンドの確実な終了を保証するため、スリープ後にまだ生きていれば実行するようにフォールバック化する）
*   **`src/utils/process.rs`**: `kill_processes_on_ports` の実装を変更する。
    *   現状、配列で受け取ったポート番号に対して `for` ループで直列に `kill_process_on_port` を呼んでいるため、ポート解放待ち時間（最大2秒）が累積してしまう。
    *   これを `std::thread::spawn` 等を用いて全ポートに対して並列に（同時に） `kill_process_on_port` を実行するようにし、全体の所要時間を最短に抑える。

## 検証手順 (Verification)
1.  **コンパイルの確認**: `make check-be` を実行し、エラーがないことを確認する。
2.  **起動テスト**: アプリケーションを起動し、ログに `[Startup] Checking ports: 3910, 3912, 3913` などの値が出力され、かつ Bifrost/ZeroClaw が正常に立ち上がっていることを確認。
3.  **ゾンビテスト**: アプリケーションを終了し、直後にターミナルで `lsof -i :3912` 等を実行し、プロセスが残っていない（ゾンビ化していない）ことを確認する。
4.  **再起動テスト**: アプリを再起動し、新しい PID で正常に立ち上がり、エラーが出ないことを確認する。