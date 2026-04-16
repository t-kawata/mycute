# Tauri v2 Windows WebView2 透過ウィンドウ不可視問題：調査レポート

## 1. 問題の要約

Tauri v2 で `.transparent(true)` かつ `.visible(false)` で作成したサブウィンドウを、後から `show()` で表示しようとしても Windows 環境で画面上に何も表示されないという問題。`show()` は `Ok(())` を返すが、視覚的には何も描画されない。[^1]

***

## 2. 根本原因の分析

### 2.1 WebView2 の可視性依存初期化

この問題の最大の原因は、**WebView2 コントロールが UI 上で可視状態になるまで完全に初期化されない**という Microsoft WebView2 の仕様にある。Rick Strahl 氏のブログで詳細に文書化されているように、`EnsureCoreWebView2Async()` は WebView2 コントロールが**物理的に画面上に見える**状態にならないと完了しない。[^2]

> WebView2 は「非表示のウィンドウ」に配置された場合、ロードはされるが完全には初期化されず、コントロールが可視になるのを待つスリープ状態に入る。[^2]

つまり、Tauri 側で `.visible(false)` で作成したウィンドウの WebView2 は、内部的に**サスペンド状態**のままであり、その後 `show()` を呼んでも WebView2 の描画パイプラインが正しく起動しないケースがある。

### 2.2 Windows DWM コンポジターの透過ウィンドウ処理

Windows のデスクトップウィンドウマネージャー（DWM）は、完全に透明なウィンドウを「描画不要」と判断してスキップする場合がある。レポート執筆者がアルファ値を `1/255` に設定した対策は理にかなっているが、DWM レベルの問題がこれだけでは解消されない場合がある。[^1]

### 2.3 Tauri/tao の透過ウィンドウ既知バグ

Tauri の下層ライブラリ `tao`（ウィンドウ管理）には、**decorations を持つ透過ウィンドウが初期状態で正しく描画されない**という長年の既知バグ（tao#8632）が存在する。この問題は Tauri v1 から v2 にかけて継続しており、ウィンドウのリサイズ操作によって初めて透過が有効になるという症状を示す。[^3][^4]

***

## 3. 同様の事例（GitHub Issues）

以下は調査で見つかった、同一または密接に関連する報告である。

| Issue | 概要 | ステータス |
|---|---|---|
| tauri#4881 | `transparent: true` でも白背景。リサイズで解消 | Closed (upstream) [^3] |
| tauri#8308 | v2 で `transparent` が効かない（v1 では動作） | Needs triage [^5] |
| tauri#8133 | v2 移行後、透過が機能せずサイズ変更で解消 | Needs triage [^6] |
| tauri#11551 | JS API の `WebviewWindow()` で `transparent: true` が無効 | Closed [^7] |
| tao#8632 | decorations 付き透過ウィンドウが初期状態で描画されない | Open (長期) [^4] |
| WebView2Feedback#1118 | `AllowsTransparency = true` で WebView2 の内容が描画されない | Tracked [^8] |
| tauri#9286 | メインウィンドウが `visible: false` だと子ウィンドウが表示されない | Reported [^9] |

Microsoft の WebView2 側でも、`AllowsTransparency` を有効にした場合に**WebView2 のコンテンツが描画されない**という問題が報告されている。特に仮想マシン環境や旧世代のグラフィックハードウェアで頻発するとされる。[^8]

***

## 4. 推奨ワークアラウンド

以下に、優先度の高い順にワークアラウンドを提案する。

### ワークアラウンド 1: `set_size` トリック（リサイズ強制）⭐ 最推奨

tauri#8133 で報告されている**最も効果的なワークアラウンド**。ウィンドウのサイズを一度ゼロ（または微小値）に設定してから元のサイズに戻すことで、WebView2 の描画エンジンを強制的にウェイクアップする。[^6]

```rust
// show() の後にリサイズを強制
window.show().unwrap();

// 元のサイズを保存
let original_size = window.outer_size().unwrap();

// サイズを一時的にゼロに
window.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(0, 0))).unwrap();

// 即座に元のサイズに戻す（描画パイプラインのリフレッシュを強制）
window.set_size(tauri::Size::Physical(original_size)).unwrap();
```

この手法は、DWM に対してウィンドウの再描画を要求するため、透過ウィンドウでもコンテンツが正しく合成される。

### ワークアラウンド 2: `decorations: false` + `shadow: false` の組み合わせ

複数のユーザーが確認している回避策。透過ウィンドウを作成する際、`decorations` と `shadow` の両方を無効にすることで初期描画が正しく行われる。[^10][^3]

```rust
let overlay = tauri::WebviewWindowBuilder::new(
    app,
    "overlay",
    tauri::WebviewUrl::App("overlay.html".into()),
)
.transparent(true)
.decorations(false)  // 必須
.shadow(false)       // 必須（これがないと1px白枠が出る）
.visible(false)
.build()?;
```

既にこれを適用している可能性があるが、`shadow(false)` が抜けている場合は追加する。Windows 11 では `shadow: true` の場合、未装飾ウィンドウに1ピクセルの白枠と丸角が付与される。[^11]

### ワークアラウンド 3: フロントエンド側からの `show()` 呼び出し

WebView2 の初期化完了を確実にするため、**Rust 側ではなくフロントエンド（Vue.js）の `onMounted` でウィンドウの表示を行う**パターン。[^12][^13]

```typescript
// Vue.js側 (overlay.vue)
import { getCurrentWindow } from '@tauri-apps/api/window';

onMounted(async () => {
  // DOMが描画された後にウィンドウを表示
  await nextTick();
  const appWindow = getCurrentWindow();
  await appWindow.show();
});
```

```rust
// Rust側: ウィンドウは visible(false) で作成するのみ
// show() は呼ばない
```

この方法が有効な理由は、フロントエンドの `onMounted` + `nextTick` のタイミングでは WebView2 の DOM レンダリングが完了しているため、`show()` を呼んだ時点でコンテンツが描画可能な状態にあるためである。

### ワークアラウンド 4: 画面外配置による「疑似不可視」初期化

WebView2 は UI 上で可視でないと初期化を完了しない。これを逆手に取り、ウィンドウを **visible(true) で作成するが、画面外の座標に配置する**方法。[^2]

```rust
let overlay = tauri::WebviewWindowBuilder::new(
    app,
    "overlay",
    tauri::WebviewUrl::App("overlay.html".into()),
)
.transparent(true)
.decorations(false)
.shadow(false)
.visible(true)           // 可視状態で作成
.position(-10000.0, -10000.0)  // 画面外に配置
.build()?;

// WebView2 が初期化されるのを待つ
// フロントエンドから準備完了通知を受けた後、位置を戻す
```

```rust
// Tauri コマンド: フロントエンドから呼ばれる
#[tauri::command]
fn overlay_ready(window: tauri::WebviewWindow) {
    // ウィンドウを非表示にし、正しい位置にリセット
    window.hide().unwrap();
    window.set_position(tauri::Position::Physical(
        tauri::PhysicalPosition::new(target_x, target_y)
    )).unwrap();
}
```

Rick Strahl 氏のブログでは、WPF アプリケーションでこの「画面外配置」手法が WebView2 の初期化問題の解決に有効であったと報告されている。[^2]

### ワークアラウンド 5: Opacity ハック

ウィンドウ自体は `visible(true)` で作成し、**CSS の `opacity: 0` でコンテンツを非表示**にする。表示時に `opacity: 1` に切り替える。

```css
/* overlay.html 初期状態 */
body {
  opacity: 0;
  transition: opacity 0.1s ease;
  background: transparent;
}
body.ready {
  opacity: 1;
}
```

```typescript
// フロントエンドから表示制御
async function showOverlay() {
  document.body.classList.add('ready');
}
```

この手法は、WebView2 にとってはウィンドウが常に「可視」であるため、初期化がブロックされない。DWM に対してもウィンドウ自体は存在するため、コンポジション処理に含まれる。

### ワークアラウンド 6: `set_background_color` + `navigate` の再発行

`show()` の後に WebView2 に対して再ナビゲーションまたは JavaScript 評価を行い、描画パイプラインを強制起動する。

```rust
window.show().unwrap();

// WebView2 の描画を強制起動
window.eval("document.body.style.display='none'; 
             requestAnimationFrame(() => { 
               document.body.style.display=''; 
             });").unwrap();
```

または、ウィンドウのバックグラウンドカラーを再設定することでも効果がある場合がある。WebView2 の `DefaultBackgroundColor` の変更は描画パイプラインの再初期化をトリガーする。[^14][^15]

### ワークアラウンド 7: `WS_CLIPCHILDREN` の除去（上級者向け）

tauri#12450 で報告された事例では、親ウィンドウの `WS_CLIPCHILDREN` 属性が透過子ウィンドウの描画を阻害していた。MYCUTEでサブウィンドウが独立ウィンドウ（parent 設定なし）であればこの問題は該当しないが、parent を設定している場合は確認する価値がある。[^16]

***

## 5. 推奨する組み合わせ戦略

上記のワークアラウンドを単独ではなく、以下の組み合わせで適用することを推奨する。

```rust
// ===== Rust側: ウィンドウ作成 =====
let overlay = tauri::WebviewWindowBuilder::new(
    app,
    "overlay",
    tauri::WebviewUrl::App("overlay.html".into()),
)
.transparent(true)
.decorations(false)      // WA#2: decorations 無効
.shadow(false)           // WA#2: shadow 無効
.visible(false)          // 初期非表示
.inner_size(400.0, 300.0)
.build()?;

// background color は alpha=1 で設定（既に実施済み）
overlay.set_background_color(Some(tauri::Color(0, 0, 0, 1)))?;
```

```rust
// ===== Rust側: 表示コマンド =====
#[tauri::command]
async fn show_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("overlay") {
        // WA#1: show() + resize trick
        window.show().map_err(|e| e.to_string())?;
        
        let size = window.outer_size().map_err(|e| e.to_string())?;
        window.set_size(tauri::Size::Physical(
            tauri::PhysicalSize::new(1, 1)
        )).map_err(|e| e.to_string())?;
        window.set_size(tauri::Size::Physical(size))
            .map_err(|e| e.to_string())?;
        
        // WA#6: 描画強制
        window.eval(
            "requestAnimationFrame(() => { document.body.offsetHeight; })"
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

```html
<!-- ===== overlay.html ===== -->
<!DOCTYPE html>
<html>
<head>
  <style>
    html, body {
      margin: 0;
      padding: 0;
      background: transparent;  /* 必須: CSS側も透過指定 */
    }
  </style>
</head>
<body>
  <!-- コンテンツ -->
  <script>
    // WA#3: DOMContentLoaded でRust側に準備完了を通知
    document.addEventListener('DOMContentLoaded', async () => {
      const { invoke } = await import('@tauri-apps/api/core');
      // フロントエンド準備完了を通知
      await invoke('overlay_frontend_ready');
    });
  </script>
</body>
</html>
```

***

## 6. 追加の調査推奨事項

- **WebView2 のバージョン確認**: Windows 環境の WebView2 ランタイムのバージョンを確認すること。古いバージョンでは透過関連のバグが多い。[^8]
- **グラフィックドライバーの影響**: 統合GPU（Intel UHD等）の環境では、WebView2 の透過描画に問題が発生しやすいことが報告されている。ターゲットマシンの GPU 構成を確認する。[^8]
- **Tauri / wry のバージョンアップ**: `wry` の最新版では WebView2 の子ウィンドウ Z-order 問題が修正されている（wry#1271）。最新の Tauri v2.x / wry にアップデートすることで、一部の問題が解消される可能性がある。[^17]
- **Windows Spy / Spy++ での確認**: `show()` 後に実際にウィンドウハンドル (HWND) が作成されているかを Spy++ で確認する。ウィンドウが存在するが描画されない場合と、そもそも OS レベルで認識されていない場合では対策が異なる。

***

## 7. 結論

この問題は、Tauri 固有のバグというよりも、**WebView2 コントロールの「可視性依存初期化」という設計上の特性**と、**Windows DWM の透過ウィンドウ処理**の2つが組み合わさった構造的な問題である。最も効果的なアプローチは「WebView2 を一度も非表示にしない」ことであり、画面外配置 + CSS opacity 制御（WA#4 + WA#5）の組み合わせが最も確実な回避策となる。それが不可能な場合は、`set_size` リサイズトリック（WA#1）が次善策として有効である。

---

## References

1. [tauri_windows_visibility_issue_report.md](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/26667977/91a969bb-942b-4281-b048-cb6e08c9adbd/tauri_windows_visibility_issue_report.md?AWSAccessKeyId=ASIA2F3EMEYES2FEQ26P&Signature=PjUhjQW23WqQRBKXaSQlwoQSgSY%3D&x-amz-security-token=IQoJb3JpZ2luX2VjEN7%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FwEaCXVzLWVhc3QtMSJHMEUCIBq3keuYNJD5qCcQ4ojQ6Nbh3q%2Flh3xAdJn%2FPQENrjdSAiEA3t3cu6OUlOkNJaxT6MadzvJc7PdPGf5IKsWJ9BQSv4sq%2FAQIpv%2F%2F%2F%2F%2F%2F%2F%2F%2F%2FARABGgw2OTk3NTMzMDk3MDUiDPP%2Fojo3feSjxAonQirQBBTz6JrNFbE%2FzfS7jeR8HwKy3DvuAnSHavgXyEviHX39kIpw0POC830t6DWQG9U8nQMeyJgnn0%2Bg8Ug5NM%2B71eExk2dQXqan%2FM8z1bt7bYwkhncjb7cig2en47Dp84T21EpbyFO9MK1XGK%2F4LlgZQEPfVwaE68CZCcyeJHBS6R39ifb%2BEQJJ%2BR4FimMFTZtgJXFz7MkjU%2FYD2XHinXwjWukwZX78h96bj%2FoGOObzqnUMhC6MLQ984uIxNd%2FxANCkvV7zY2D2nGkCJBRhwFIKRxBr3kvGGOe9%2B%2FoNq4MmiJm%2FAz5c05RnGKrg8sADtOqpo5JA5lw91AAOnrot%2F261F1qJsqDcVvqZIHyabbgQf8dZghIMqh%2BMdGK7CPm16lLhTRk5ffyknAsZMJiR5XSKTTgUfOAcKt4ZH29ThLARWvGQqVUihF6h9oTTCZyC3%2F%2B80soIRdt9BPOZmOA903mm2JYhDlt3em2k8e%2BXg3oxzKoOJ8g4k2OJ8DjlK3zHSSIwas3gBpjQePBArjSBnqVfu9U%2F6Y6U5VV1tLSZEUxu5GBCvlwOK6Un9otFBjOWGM5jaItx2SX8ZDW7z5qBBUMGZHKvceIccYI2MtKIgr6h04an%2BUHEWihbxMhQuQyI4x2Ax0fojO1s%2FZ9BclptmMdmH3IqnLFDAqfLxE3OklsbdWdWMR9f4pQWP66GP9nmXjz4a7CFeLNbHS9J1jwa8a4ePHF4Z49JKxgSx7%2FX6sS1u2Pz1v9lWdGew4S8mfevp%2FQn41pFDZX58vH63PzZHGKE5DowqoHlzAY6mAE5r9DbFt8yRObvr2c9%2BY9HLmyRD7P5oIO2heQ%2FpOUTg0wwVcHa5gLfHQI2yv3lvH1UiWWV9Sa%2FAWC84PpOrv813Gqc5tpADOs2H%2B%2F%2B4WZBMqSALnnfAX23pSQ3qbXQ4QUCvkLWb9HPsOQkWuj1Baz2KIDCQrtpgvW68hwoSY%2BDwETtWeTrhMzrlswwRxW2L2pyiMGMb4xz5g%3D%3D&Expires=1771657525) - # Tauri Windows WebView2 透過ウィンドウ初期化・表示に関する技術懸念レポート

## 1. 概要
本ドキュメントは、Tauri (v2.x) を用いたデスクトップアプリケーショ...

2. [Fighting WebView2 Visibility on Initialization - Rick Strahl](https://weblog.west-wind.com/posts/2022/Jul/14/Fighting-WebView2-Visibility-on-Initialization) - The WebView2 control has a clever 'feature' that doesn't fully initialize the WebView control if it'...

3. [[bug] Transparency issue on windows #4881 - tauri-apps ...](https://github.com/tauri-apps/tauri/issues/4881) - Settings "transparent": true to the WindowConfig object still creates a white background. You need t...

4. [A window with decorations can't be transparent initially ...](https://github.com/tauri-apps/tauri/issues/8632) - With the newly added with_undecorated_shadow for Windows and is planned to be active by default in t...

5. [[bug] V2 window.transparent not work #8308](https://github.com/tauri-apps/tauri/issues/8308) - The behaviors of v1 and v2 are inconsistent. With the same settings, in tauri v1, the window is tran...

6. [[bug][v2] Problem with transparent background when ...](https://github.com/tauri-apps/tauri/issues/8133) - I recently decided to migrate a project I made for testing to version v2 and found a problem with tr...

7. [true' configured via the 'new WebviewWindow()' method is ...](https://github.com/tauri-apps/tauri/issues/11551) - I thought that when I set the window transparent, the whole window would disappear, leaving only the...

8. [WebView2 is rendering the page but the content is ...](https://github.com/MicrosoftEdge/WebView2Feedback/issues/1118) - Setting AllowsTransparency to true on the window that hosts the WebView2 control stops some parts of...

9. [[bug] Child window failed to show if main window is not ...](https://github.com/tauri-apps/tauri/issues/9286) - Calling a Tauri command to create a child window results in the child window appearing and disappear...

10. [Problems with window customization's rounded corners and shadows](https://github.com/tauri-apps/tauri/issues/9287) - Describe the bug. Because tauri does not support the extension of native windows, I have to manually...

11. [https://schema.tauri.app/config/2](https://schema.tauri.app/config/2) - ... [`tauri init`](https://v2.tauri.app/reference/cli/#init) command that lives in\n your Tauri appl...

12. [true improves resize performance and eliminates artifacts (black ...](https://github.com/tauri-apps/tauri/issues/13270) - While working with a default Tauri app on Windows, I noticed that enabling transparent: true in the ...

13. [[bug] Blank white screen on load · Issue #5170 · tauri-apps/tauri](https://github.com/tauri-apps/tauri/issues/5170) - There's a blank white screen on load in dev and after build / install. I'm guessing this is just the...

14. [How to use transparency in Webview2?](https://stackoverflow.com/questions/67838739/how-to-use-transparency-in-webview2) - You can change the WebView2.DefaultBackgroundColor (which defaults to white) to transparent to have ...

15. [WebView2 Win32 C++ ICoreWebView2Controller2 | Microsoft Learn](https://learn.microsoft.com/ja-jp/microsoft-edge/webview2/reference/win32/icorewebview2controller2?view=webview2-1.0.1210.39) - The A represents an Alpha value, meaning DefaultBackgroundColor can be transparent. In the case of a...

16. [[bug] Transparent attribute on child window is not taking effect #12450](https://github.com/tauri-apps/tauri/issues/12450) - I've managed to make the tauri spawn a child window in the bevy window, but the WebviewWindowBuilder...

17. [[bug] new webview added to the window does not appear ...](https://github.com/tauri-apps/tauri/issues/9798) - To make the second webview visible on Windows, I need to manually set the size of the main webview t...

