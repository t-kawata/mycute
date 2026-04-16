Windows 11での音声認識テキスト入力アプリ開発において、予測変換ウィンドウがカーソル位置を奪ってしまう問題の解決策をまとめました。

## 問題の概要と原因
Windows 11では、ハードウェアキーボードでの入力に対しても「テキスト候補（予測入力）」を表示する機能が標準で有効になっています。
C#からキーボードイベント（`SendKeys`など）で日本語を流し込む際、このIME（入力システム）が「ユーザーが今から文字を入力・変換しようとしている」と勘違いして候補ウィンドウを表示してしまいます。その結果、フォーカスが候補ウィンドウに移り、アプリ側で制御しているカーソル位置と実際の入力位置にズレが生じます。 [elevenforum](https://www.elevenforum.com/t/enable-or-disable-text-suggestions-in-windows-11.5550/)

***

## 解決策：IMEの一時的な無効化
レジストリ等のシステム設定を書き換えずに、入力操作の直前に対象ウィンドウのIMEを「オフ（英数モード）」に切り替える方法が最も安全で効果的です。既に確定している日本語テキストを送る場合、IMEがオフであっても正しく入力されます。 [learn.microsoft](https://learn.microsoft.com/en-us/windows/win32/api/imm/nf-imm-immdisableime)

### 推奨される処理フロー
1.  **音声認識:** `SpeechRecognizer` でテキストを確定させる。
2.  **IME制御:** Win32 API を呼び出し、入力先ウィンドウのIMEを「オフ」にする。
3.  **テキスト送出:** `SendKeys` 等で確定済みテキストを送信する。
4.  **完了:** IMEはオフのまま（または必要に応じてオンに戻す）入力を終える。

***

## 実装例 (C#)

Win32 APIの `imm32.dll` を使用して、最前面のウィンドウのIMEを強制的にオフにする実装です。

```csharp
using System;
using System.Runtime.InteropServices;

public static class InputHelper
{
    // Win32 APIのインポート
    [DllImport("imm32.dll")]
    private static extern IntPtr ImmGetContext(IntPtr hWnd);

    [DllImport("imm32.dll")]
    private static extern bool ImmSetOpenStatus(IntPtr hIMC, bool fOpen);

    [DllImport("imm32.dll")]
    private static extern bool ImmReleaseContext(IntPtr hWnd, IntPtr hIMC);

    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    /// <summary>
    /// 現在のフォーカスウィンドウのIMEをオフにし、予測入力を抑制します。
    /// </summary>
    public static void SuppressImePrediction()
    {
        // 1. 入力対象（最前面）のウィンドウハンドルを取得
        IntPtr hwnd = GetForegroundWindow();
        if (hwnd == IntPtr.Zero) return;

        // 2. IMEコンテキスト（制御用ハンドル）を取得
        IntPtr hIMC = ImmGetContext(hwnd);
        if (hIMC != IntPtr.Zero)
        {
            // 3. IMEをClose状態（false）に設定 = 英数モード強制
            // これにより予測変換ウィンドウの割り込みを物理的に防ぎます
            ImmSetOpenStatus(hIMC, false);

            // 4. コンテキストの解放
            ImmReleaseContext(hwnd, hIMC);
        }
    }
}
```

***

## 期待される効果
| 項目 | 従来の挙動（IMEオン） | 対策後の挙動（IMEオフ） |
| :--- | :--- | :--- |
| **予測候補表示** | 入力中にウィンドウがポップアップする  [it-notes.stylemap.co](https://it-notes.stylemap.co.jp/programs/what-is-ime-all-about-japanese-input-method-and-types/) | **一切表示されない** |
| **カーソル位置** | 候補ウィンドウに奪われ、ズレが発生する  [stackoverflow](https://stackoverflow.com/questions/48093581/strange-behaviour-with-japanese-ime-in-windows-10-1709) | **アプリの制御通りに維持される** |
| **日本語入力** | 再変換が発生し、意図しない文字になる | **確定済みテキストがそのまま入る** |

### 補足
この方法は、Windows OS自体が持っている「日本語入力支援」というフィルターを一時的にバイパスするものです。音声認識エンジンによって既に「正しい日本語」として完成したデータを扱うアプリにおいては、この「フィルターを通さない入力」が最も確実な通信手段となります。 [choge-blog](https://www.choge-blog.com/programming/microsoft-ime/)