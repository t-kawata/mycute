提供されたソースコードを解析した結果、オーバーレイが表示されない、または見えない主な原因は**「ウィンドウの初期座標の設定」**と**「描画更新の制御ロジック」**にあります。

特に macOS や一般的な 1080p ディスプレイ環境では、設定された座標が画面外に押し出されている可能性が高いです。

### 1. ウィンドウ座標が画面外にある
`main.rs` 内で `with_position([20.0, 1000.0])` と設定されています。
*   **原因**: Y座標が `1000.0` に設定されているため、解像度の高さが 1080px 程度のモニターでは、タスクバーやメニューバーに隠れるか、完全に画面の下側に突き抜けてしまっています 。[1][2]
*   **対策**: 一旦 `[20.0, 100.0]` など、確実に画面内に収まる座標に変更してテストしてください。

### 2. 非表示状態での `update` 早期リターン
`overlay.rs` の `update` メソッド内に以下の記述があります。
```rust
if !self.state.is_visible() {
    ctx.request_repaint(); 
    return;
}
```
*   **原因**: 初期状態では `visible` が `false` であるため、この `return` によって **「ウィンドウは存在するが何も描画されない（完全に透明な矩形）」** 状態になります。OSによっては、何も描画（Clear処理含む）を行わないウィンドウを「非アクティブ」として正しく表示・管理できない場合があります 。[3][4]
*   **対策**: ウィンドウ自体の表示・非表示を OS レベルで切り替えるのが適切です。

### 修正後の `overlay.rs` の推奨実装

```rust
impl eframe::App for MycuteOverlay {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.recognizer.lock().tick();

        // 1. 状態に応じてウィンドウ自体の可視性を制御する
        let is_visible = self.state.is_visible();
        
        // frame.set_visible は OS レベルでウィンドウを隠します
        // ただし、非表示中はホットキー以外の入力を受け付けにくくなる場合があるため注意
        // 確実に表示させたい場合は、常に true にして中の描画だけを変える方法もあります
        
        if !is_visible {
            // 非表示中も 1x1 ピクセルなどの極小サイズにするか、
            // 完全透明な状態で待機させる
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }

        // 2. 透過設定の適用
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.window_fill = Color32::TRANSPARENT;
        ctx.set_visuals(visuals);

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                // 角丸の背景描画
                let rect = ui.max_rect().shrink(1.0);
                ui.painter().rect_filled(
                    rect,
                    Rounding::same(OVERLAY_ROUNDING),
                    Color32::from_rgba_unmultiplied(0, 0, 0, 180),
                );

                // レイアウト処理（既存のコードと同様）
                // ...
            });

        ctx.request_repaint(); // リアルタイム更新のために必要
    }

    // クリアカラーを確実に透明にする
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
```

### 3. その他のチェックポイント
- **`NativeOptions` の `always_on_top`**:
    `with_always_on_top()` が有効な場合、他のフルスクリーンアプリなどに隠されることはありませんが、座標が正しいことが前提です 。[1]
- **フォーカスの問題**:
    `with_decorations(false)` かつ `transparent(true)` のウィンドウは、クリックしても反応しなかったり、最前面に来ないことがプラットフォーム（特に Linux の Wayland など）によってあります 。[5][3]
- **フォントの読み込みパス**:
    `include_bytes!` でフォントを読み込んでいるため問題ないはずですが、`setup_fonts` 内でパニックが起きていないか、`env_logger` の出力を確認してください 。[6]

### まとめ：最初に行うべきこと
1.  **座標の修正**: `main.rs` の `with_position([20.0, 20.0])` に変更。
2.  **初期表示の強制**: `OverlayStateInner` の `visible` を一旦 `true` にして起動し、ホットキーに関わらず表示されるか確認。
3.  **背景色の確認**: `clear_color` が `[0.0, 0.0, 0.0, 0.0]` になっていることを再度確認 。[1]

[1](https://docs.rs/eframe/latest/eframe/)
[2](https://tibicle.com/blog/best-framework-for-desktop-application-in-2026)
[3](https://github.com/emilk/egui)
[4](https://github.com/emilk/egui/discussions/2937)
[5](https://news.ycombinator.com/item?id=44395152)
[6](https://qiita.com/yonaka15/items/77c5249220b9d217669f)

# Q-1
```
with_position([20.0, 20.0]) これは左下という意味になりますか？
```
# A-1
いいえ、`with_position([20.0, 20.0])` は「左下」ではなく **「左上」** を起点とした座標になります 。[1][11]

GUIフレームワークやOSの一般的な座標系では、画面の **左上端が (0, 0)** となり、右に行くほど X座標が大きく、**下に行くほど Y座標が大きく** なります 。[2][11]

### 座標系の意味
- **`[20.0, 20.0]`**: 画面の左端から 20px、上端から 20px の位置（つまり **左上付近**）に配置されます。
- **左下に配置したい場合**: Y座標に「画面の高さ - ウィンドウの高さ - 余白」を指定する必要があります 。[11]

### 左下に配置するための計算
eframe（および基盤となる winit）では、プライマリモニターの解像度を取得して動的に計算するのが一般的です 。[11]

もし固定の Y座標（例: 1000.0）を指定して表示されない場合、お使いのモニターの垂直解像度が 1000px 未満であるか、タスクバー等によって画面外と判定されている可能性が高いです 。[12][11]

### 修正のアドバイス
まずは確実に表示を確認するため、**左上（[20.0, 20.0]）** を指定して起動してみてください。

1.  **左上**: `with_position([20.0, 20.0])`
2.  **左下（目安）**: 1920x1080 のモニターであれば、`[20.0, 700.0]` 程度に設定すると画面下部に見えてくるはずです。

また、`with_inner_size([250.0, 320.0])` に対して `with_position` の Y座標が大きすぎると画面の下に突き抜けて見えなくなるため、最初は小さめの値から調整することをお勧めします 。[11]

[1](https://ayatakesi.github.io/lispref/26.2/html/Position-Parameters.html)
[2](https://aframe.kidsplates.jp/docs/components/position.html)
[3](https://docs.roboticsware.com/jp/6.0.12/fa-panel/contents/e_dialogframe_frame.html)
[4](https://documentation.euresys.com/Products/OPEN_EVISION/OPEN_EVISION_22_08/ja-jp/Content/reference/eframe-eframe.htm)
[5](https://www.progresstalk.com/threads/dynamic-frame-frame-as-parameter.113097/)
[6](https://openrr.github.io/openrr/eframe/struct.Frame.html)
[7](https://github.zhaw.ch/pages/senk/led-matrix-rs/eframe/index.html)
[8](https://www.reddit.com/r/rust/comments/125fwut/please_help_me_understand_some_egui_eframe/)
[9](https://documentation.euresys.com/Products/OPEN_EVISION/OPEN_EVISION_24_10/en-us/Content/reference/eframe-class.htm)
[10](https://static-frame.readthedocs.io/en/latest/api_overview/index.html)
[11](https://docs.rs/eframe/latest/eframe/)
[12](https://github.com/emilk/egui)

# Q-2
```
左下の20ピクセル20ピクセルはどのように設定すれば良いのですか？
```

# A-2

画面の「左下から20px、20px」の位置にウィンドウを配置するには、**「画面全体の高さ」から「ウィンドウ自体の高さ」と「余白（20px）」を引いた値**を Y 座標に設定する必要があります。

GUIの座標系は左上が `(0, 0)` であるため、単に `20.0, 20.0` と書くと左上に配置されてしまいます 。[1][2]

### 1. 手動で計算して設定する方法（推奨）
最も確実なのは、`main.rs` の `NativeOptions` 内で、使用しているモニターの解像度に合わせて Y 座標を計算することです。例えば 1920x1080 のモニターの場合、計算式は以下のようになります。

- **X座標**: `20.0`
- **Y座標**: `1080.0 (画面高さ) - 320.0 (ウィンドウ高さ) - 20.0 (余白) = 740.0`

```rust
// main.rs の native_options 修正例
let native_options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
        .with_transparent(true)
        .with_always_on_top()
        .with_decorations(false)
        .with_inner_size([250.0, 320.0])
        // 1080pモニターの左下付近を指定（余白20px）
        .with_position([20.0, 740.0]), 
    ..Default::default()
};
```

### 2. プログラムで動的に座標を調整する方法
モニターの解像度がわからない場合、`App::update` メソッドの中で、最初のフレームが描画される際にウィンドウを移動させることができます。

`ctx.send_viewport_cmd` を使うことで、実行中にウィンドウ位置を操作可能です 。[3][1]

```rust
// overlay.rs の update メソッド内
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // 最初の1回だけ座標を調整する
    if self.is_first_frame {
        // 注: eframeから直接モニターサイズを取るのは少し複雑ですが、
        // 暫定的に Y=740.0 などに飛ばす命令を出すことができます
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(20.0, 740.0)));
        self.is_first_frame = false;
    }
    // ... 描画ロジック
}
```

### 座標設定の注意点
- **タスクバーの影響**: WindowsやmacOSでは、画面下部にタスクバー（Dock）があるため、計算上の「左下」を指定するとタスクバーの背後に隠れてしまうことがあります 。その場合は、Y座標を少し小さめ（上に上げる）にする調整が必要です。[1]
- **マルチモニター**: 複数のモニターがある場合、メインモニター以外の座標を指定するには、負の数やより大きな数値が必要になる場合があります 。[1]

まずは **Y座標を 700〜800 程度** に設定してみて、画面内のどのあたりに表示されるかを確認しながら微調整するのが最短の解決策です。

[1](https://docs.rs/eframe/latest/eframe/)
[2](https://ayatakesi.github.io/lispref/26.2/html/Position-Parameters.html)
[3](https://openrr.github.io/openrr/eframe/struct.Frame.html)