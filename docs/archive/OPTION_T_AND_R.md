# Option+T / Option+R による入力先固定機能（Anchored Input）実装計画書

## 0. 実装ステータス

> [!TIP]
> **実装完了** (2026-01-11)
> すべてのコンポーネントが実装され、ビルドに成功しました。

## 1. 背景と目的

現在の `mycute` は、macOS の `CGEvent` を利用して「現在カーソルがある場所（Active Focus）」に対して文字を入力する方式をとっています。この方式は汎用的ですが、以下のような課題があります。

- 音声入力中に誤って別のウィンドウをクリックしたり、フォーカスが外れたりすると、意図しない場所に入力が継続されてしまう。
- 背面にあるウィンドウや、特定の入力欄に対して入力を流し込みながら、手元では別の作業（ブラウザでの検索など）を並行して行いたいというニーズに対応できない。

本計画では、特定のテキストフィールドを「ターゲット」としてロックする機能を追加し、作業効率と入力の確実性を向上させることを目的とします。

---

## 2. 機能概要

- **Option + T (Target Lock)**: 現在フォーカスが当たっているテキスト入力欄を「入力先」として固定します。
- **Option + R (Release)**: 固定した入力先を解除し、通常の「現在フォーカスがある場所への入力」に戻します。
- **ブラウザ制限**: ブラウザ（Chrome, Safari等）の入力欄は、標準の Accessibility API による値の操作が不安定なため、本機能の対象外とし、通知を表示します。

---

## 3. 実装の設計方針

### 3.1 ターゲット要素の特定 (Target Detection)
macOS の Accessibility API (`HIServices`) を利用し、システムワイドで現在フォーカスされている要素を取得します。

```rust
// 概念図：フォーカス要素の取得
let system_wide = AXUIElementCreateSystemWide();
let mut focused_element: AXUIElementRef = std::ptr::null_mut();
AXUIElementCopyAttributeValue(system_wide, kAXFocusedUIElementAttribute, &mut focused_element);
```

### 3.2 ブラウザ判定
取得した要素を保持するプロセスの `Bundle Identifier` をチェックします。

- 判定対象: `com.google.Chrome`, `com.apple.Safari`, `org.mozilla.firefox` 等。
- ブラウザが検出された場合は `ui::notification` を通じて警告を表示し、ロック処理を中断します。

### 3.3 ターゲットへの入力 (Method A: AXValue Manipulation)
固定されたターゲットに対しては、キーイベントのシミュレーションではなく、属性値の直接書き換えを行います。

- **使用属性**: `kAXValueAttribute`
- **メリット**: ウィンドウが背面にあっても、フォーカスが他所に移っていても確実に入力されます。
- **課題**: 音声入力は「逐次的な追加」であるため、現在の入力欄にある「既存のテキスト」を壊さないように制御する必要があります。

---

## 4. 具体的なコードスニペット（実装イメージ）

### 4.1 ターゲット管理用データ構造
`src/input/keyboard.rs` または新規の `src/input/accessibility.rs` に定義します。

```rust
pub struct AnchoredTarget {
    element: AXUIElementRef,
    initial_value: String,
    process_name: String,
}

impl AnchoredTarget {
    /// 現在のフォーカス要素からターゲットを作成
    pub fn probe_current() -> Result<Self, String> {
        // 1. System Wide 要素から Focused UI Element を取得
        // 2. ブラウザ判定 (Bundle ID チェック)
        // 3. 現在のテキスト (kAXValueAttribute) を取得して initial_value に保持
        // 4. AnchoredTarget 構造体を返す
    }

    /// ターゲットに対してテキストをセット
    pub fn apply_text(&self, text: &str) {
        let full_text = format!("{}{}", self.initial_value, text);
        let cf_string = CFString::new(&full_text);
        unsafe {
            AXUIElementSetAttributeValue(self.element, kAXValueAttribute, cf_string.as_ptr());
        }
    }
}
```

### 4.2 main.rs での状態管理
メインループの中で、ターゲットが固定されているかどうかを判定し、入力処理を分岐させます。

```rust
// main.rs のループ内
let mut anchored_target: Option<AnchoredTarget> = None;

// ... イベントハンドリング ...
match action {
    HotkeyAction::LockTarget => { // Option+T
        match AnchoredTarget::probe_current() {
            Ok(target) => {
                crate::ui::notification::show_notification("mycute", "入力先を固定しました");
                anchored_target = Some(target);
            }
            Err(e) => {
                crate::ui::notification::show_notification("mycute", &format!("失敗: {}", e));
            }
        }
    }
    HotkeyAction::ReleaseTarget => { // Option+R
        anchored_target = None;
        crate::ui::notification::show_notification("mycute", "固定を解除しました");
    }
}

// ... STT結果受信時 ...
if let Some(target) = &anchored_target {
    target.apply_text(&new_stt_text);
} else {
    KeyboardInjector::input_diff(&last_text, &new_stt_text);
}
```

---

## 5. ユーザーへの通知

ブラウザ上で `Option+T` が押された際、以下のメッセージをポップアップ表示します。

> **「ブラウザの入力欄は固定できません」**
> ブラウザ上では Accessibility API の制約により入力先固定が不安定なため、通常のカーソル入力モードをご利用ください。

---

## 6. 今後の課題と検証

1. **メモリ管理**: `AXUIElementRef` は Core Foundation オブジェクトであるため、`Drop` トレイトで確実に `CFRelease` する必要があります。
2. **要素の有効性チェック**: ターゲットとなっているアプリが終了したり、ウィンドウが閉じられた場合、`AXUIElementSetAttributeValue` がエラーを返します。この際に自動で固定を解除する堅牢なエラーハンドリングを実装します。
3. **逐次入力の最適化**: 非常に長い文章を音声入力する場合、毎回全文を `AXValue` にセットするとパフォーマンスに影響が出る可能性があるため、検証が必要です。
