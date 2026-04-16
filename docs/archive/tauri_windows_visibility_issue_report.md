# Tauri Windows WebView2 透過ウィンドウ初期化・表示に関する技術懸念レポート

## 1. 概要
本ドキュメントは、Tauri (v2.x) を用いたデスクトップアプリケーション開発において、Windows 環境でのみ発生している「透過ウィンドウが表示されない」という問題について、外部専門家へ情報提供および解決策の相談を行うために作成されました。

同様のロジックは macOS 環境では完全に正常動作（透過・表示・制御ともに期待通り）していますが、Windows (WebView2) 環境において特有のライフサイクル、あるいは描画エンジンの挙動に起因すると考えられる問題に直面しています。

## 2. アプリケーション構成
- **Framework**: Tauri v2
- **Backend**: Rust
- **Frontend**: Vue.js / Vite
- **Target OS**: macOS (Apple Silicon), Windows 11 (WebView2)
- **Window 特徴**: 
    - メインウィンドウに加えて、複数のサブウィンドウ（オーバーレイ、スナックバー等）を使用。
    - サブウィンドウはすべて `.transparent(true)` かつ初期状態 `.visible(false)` で作成。

## 3. 問題の変遷
現在までに、以下の2つのフェーズを経て現在の状況に至っています。

### フェーズ 1: 同期的問い合わせによるデッドロック（解決済み）
当初、ウィンドウ作成直後のセットアップ処理において、`window.is_visible()` を呼び出した際に Windows 環境でのみスレッドが永続的にハング（デッドロック）する現象が発生しました。
- **原因の推定**: WebView2 の初期化（あるいは内部的なスレッド間通信）が完了していない不安定なタイミングで、Rust 側の同期メソッドが呼び出されたため。
- **回避策**: ウィンドウ自体への同期的状態確認を廃止。メモリ上のアトミック変数 (`AtomicBool`) で可視性状態を管理し、処理の「一方通行化」を行うことでデッドロックを完全に排除しました。

### フェーズ 2: 描画エンジンの未起動・不可視問題（現在の課題）
デッドロックが解消され、命令が物理的に完走するようになりましたが、視覚的にウィンドウが現れないという問題が残っています。

**現在の状況:**
- ウィンドウの `.build()` は成功。
- 背景色設定（`set_background_color`）等の設定メソッドもすべて成功。
- `window.show()` 命令を発行すると、戻り値は `Ok(())` であり、ログ上では「表示成功」と記録される。
- **しかし、画面上には何も表示されない（透明なまま、あるいは存在自体が OS に認識されていないように見える）。**

## 4. これまでの調査と試行内容
問題解決のため、現在までに以下の対策を実施・検証しました。

1.  **初期化の遅延**: メインウィンドウの起動（`RunEvent::Ready`）から数秒の `sleep` を挟んでからサブウィンドウを作成。
2.  **完全非同期化**: ウィンドウ作成処理を `tauri::async_runtime::spawn` で実行し、メインスレッドへの影響を最小化。
3.  **アルファ値の微調整**: Windows のコンポジターが「完全に透明（Alpha: 0）」なウィンドウを「描画不要」として無視するのを防ぐため、背景色のアルファ値を `1/255` に設定。
4.  **フォーカス強制 (`set_focus`)**: `show()` の命令に加えて、フォーカスを強制的に要求することで描画プロセスのウェイクアップを試行。
5.  **デッドロック排除**: 前述の通り、`is_visible()` などの同期的プロパティ取得をすべて削除。

## 5. 現在の技術的懸念点
専門家の皆様に以下の点について知見を拝借したく考えております。

- **初期化順序の正解**: Windows WebView2 において、`transparent(true)` 且つ `visible(false)` で作成したウィンドウを、後から確実に（且つデッドロックさせずに）表示させるための作法や、呼び出し順序のベストプラクティスはあるか。
- **レンダリングスレッドの生存確認**: `show()` が `Ok` を返しているにもかかわらず、中身の HTML/CSS が描画（あるいはコンポジット）されない場合、WebView2 の内部プロセスが「サスペンド」状態にある可能性はあるか。その場合、外部（Rust）から強制的に叩き起こす方法はあるか。
- **Transparent 属性の影響**: Windows 11 のウィンドウマネージャーにおいて、Tauri (v2) の `transparent` 属性と、セカンダリディスプレイの解像度、あるいは OS のグラフィック設定が干渉してウィンドウが不可視になる既知の事象はあるか。

## 6. 捕捉情報：最新の動作ログ
```
[INFO] <Diagnostic> Overlay: Starting .build()...
[INFO] <Diagnostic> Overlay: .build() success.
[INFO] <Diagnostic> Overlay: background color set (alpha=1).
[INFO] <Diagnostic> Overlay: shadow set.
[INFO] <Diagnostic> Overlay: Requesting focus to wake up renderer...
[INFO] <Diagnostic> Overlay: cursor events set.
[INFO] <Diagnostic> toggle_overlay_visibility called
[INFO] <Diagnostic> Next visibility state (from atomic): true
[INFO] <Diagnostic> Calling ow.show()...
[INFO] Overlay toggled: shown
```
このように、ソフトウェアとしての命令パイプラインは最後まで正常に開通しています。

以上。
