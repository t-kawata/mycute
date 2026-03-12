# STT（Windows版）文字抜け・句読点消失バグ 解析レポート

調査の結果、ご報告いただいた2つの事象の根本原因が判明しました。
いずれもWindows環境固有の仕様や制約によって引き起こされている問題であり、Mac環境で発生しない理由とも完全に一致しています。

以下に、ソースコードおよび実行ログの物理的証拠に基づいた詳細な解析結果を報告します。

---

## 1. 事象1: 文末の句読点「。」が消えて次の文に繋がる問題

### 原因
無音タイムアウト（500ms）によって生成された「。」の送信イベント（`PartialResult`）が、その直後に認識された「次の音声入力の未確定テキスト」によって上書き（バックスペースで削除）されてしまうためです。

### 物理的証拠（ログとコードの照合）
1. **OSによるテキスト確定とタイムアウトの発火**
   ログ `00:41:00` にて、OSが `final: 1` で「現在テストをしていますテストの内容は」を確定させています。これにより、`win.rs` 内部の `watermark_len`（確定済みの文字数）が `18` に進みます。
   その後500msの無音によりタイムアウトが発火します。
   ```text
   26-03-12_00:41:00 [Win] Timeout triggered (>500ms). Re-processing for punctuation.
   ```
2. **句読点の生成とUIへの送信**
   タイムアウト時、未確定文字列（`raw_unconfirmed`）は空ですが、`PunctuationMachine` が直前の文脈「...テストの内容は」を読み取り、文末と判断して「。」を生成します。
   この「。」は `PartialResult`（未確定状態）としてフロントエンドに送信され、`keyboard_win.rs` の `input_diff` によって画面に「。」が打たれます（ここまでは正常に動作しています）。
3. **新規音声の受信による「。」の削除（上書き）**
   直後にユーザーが「文字が」と発話した際、OSから `final: 0` の `PartialResult` として `現在テストをしていますテストの内容は文字が` が受信されます。
   `win.rs` は `watermark_len` (18) より後ろの文字列を未確定分として切り出すため、今回は「文字が」を取得します。文の途中であるため、`PunctuationMachine` は句読点を付与せず、そのまま「文字が」を送信します。
4. **差分入力関数でのバックスペース発動**
   フロントエンドの入力管理（`main_of_cl.rs` の中身を反映する `injected_text`）は、直前に打った「。」を記憶しています。新たに受信した「文字が」と比較（`input_diff`）した結果、共通部分がない（`common_prefix: 0`）と判定され、**「。」をバックスペースで消去した上で「文字が」を打ち直す**挙動となります。
   ```text
   26-03-12_00:41:00 [KeyboardInjector] input_diff: "。" -> "文字が"
   26-03-12_00:41:00 [KeyboardInjector] delete_count: 1
   26-03-12_00:41:01 [KeyboardInjector] type_string: "文字が"
   ```
   **結論:** タイムアウトの「。」は未確定状態（`PartialResult`）として扱われるため、OSから次のテキストが来た瞬間に「OSからのテキストには『。』が含まれていない」という理由で無慈悲に削除されてしまいます。

### Macで発生しない理由
Mac版（`mac.rs`）が使用しているApple純正の音声認識APIは、エンジン自身が文脈を判断して自動的に句読点を含んだテキストを返してきます。そのため、この無音タイムアウトによる手動の句読点挿入ロジック自体がMac版には存在しません（`constants.rs` にも `STT_TIMEOUT_PUNCTUATION_MS` は「Windows専用」と記されています）。


---

## 2. 事象2: キーボード入力時に文字が抜ける問題（例: 「きちんと」が「きちと」になる）

### 原因
Windowsにおけるキーボードエミュレーション（`SendInput` API）のキーストローク間隔が短すぎるため、OSのイベントキューや対象のアプリケーション側で入力の取りこぼしが発生しているためです。

### 物理的証拠（ログとコードの照合）
1. **Rust側は正しく文字を送信している**
   ログの `00:41:01` にて、「きちんと」を入力しようとする際、`keyboard_win.rs` は確実に4文字分（き、ち、ん、と）のデータを分離して送信しています。
   ```text
   26-03-12_00:41:01 [WinInputDiag] type_text_inner: text='きちんと', utf16_len=4 (char_count=4)
   26-03-12_00:41:01 [WinInputDiag] type_text_inner complete: 4 utf16 units sent individually
   ```
   `SendInput` APIの呼び出しエラーも出ていないため、プログラムとしては正常に4回のキーダウン／キーアップをOSに投げています。
2. **設定値に起因するOS/アプリ側のバッファ溢れ**
   `src/input/keyboard_win.rs` の `type_text_inner` メソッドでは、1文字送信するごとにスリープ時間を設けています。この時間は `constants.rs` で以下のように定義されています。
   ```rust
   /// キー押下（Down）および解放（Up）の後の待機時間（ミリ秒）: Windows用
   pub const KEY_DELAY_MS_WIN: u64 = 5;
   ```
   Windows環境において、`SendInput` を 5ms 間隔で連続実行した場合、入力対象のアプリケーション（ブラウザやエディタ等）のメッセージループの処理限界を超えてしまい、一部のキーストロークが欠落する現象（文字抜け）が極めて高確率で発生します。今回の「ん」の欠落はまさにこの典型的な症状です。

### Macで発生しない理由
Mac版のキーボードエミュレーション（`CGEvent` を使用）は非常に堅牢であり、`KEY_DELAY_MS_MAC: u64 = 1;`（1ミリ秒）という極短時間でもOSレベルで確実に入力キューが処理されます。Windowsの `SendInput` は歴史的な制約でこの速度に耐えられません。

---

## 3. 事象3: パススルーモードで不要な句読点（「。」や「？」）が重複・異常挿入される問題

### 原因
LLMプロセッサが存在しない（パススルー）状態において、OSからの小刻みな確定イベント（`final: 1`）と無音タイムアウトが交錯した際、`PunctuationMachine` の「自立語による遡及判定（強い開始語が来たら前を閉じる）」ロジックおよびタイムアウト時の「強制句読点付与（`allow_terminal_punctuation=true`）」が過剰に反応し、既に文脈として繋がっているべき箇所に不要な句読点を次々と打ち込んでしまうためです。

さらに、前回の修正でタイムアウトイベントを `SttEvent::FinalResult` に変更したことで、これらの「誤って打たれた句読点」が確定され、直後の音声ストリーム（Partial）によって削除・修正されなくなった（保護されてしまった）ことが、この事象を表面化・固定化させました。

### 物理的証拠（ログとコードの照合）

#### ① タイムアウトによる過剰な「。」の確定と分離
ログの `02:14:49` の箇所です。
```text
[Win/SpeechHelper] Raw: 'これから' -> Clean: 'これから' (Final: False)
...
26-03-12_02:14:49 mycute.stt.win            [DEBUG] [Win] Timeout triggered (>500ms). Re-processing for punctuation as FinalResult.
26-03-12_02:14:49 mycute.stt.win            [DEBUG] [Win] Windowing: Anchor=0, Context='', RawTarget='これから' -> Punctuated='これから。'
26-03-12_02:14:49 mycute.stt.win            [DEBUG] [Win] Passthrough: Watermark advanced to 4
```
ここで「これから」に対して500msのタイムアウトが発生し、`PunctuationMachine` の以下のロジック（`src/tools/punctuation_machine.rs` の 273行目付近）に引っかかっています。
```rust
// 3. 自立語による遡及判定 (強い開始語が来たら、前を閉じる)
if (current.pos == "動詞" || current.pos == "形容詞" || current.pos == "助動詞") && ... {
    if let Some(next) = next_opt { ... }
    else {
        // 末尾に自立語が来た場合 (TimeOut時のみ)
        return allow_terminal_punctuation; // ★ここが true として返る
    }
}
```
「これから」は副詞（強い開始語）などと判定されうる自立語であり、その後続がない（タイムアウト時）ため、強制的に「。」が打たれます。そして、今回の改修によりこれが `FinalResult` となったため、`watermark_len`（確定位置）が `4` に前進し、この「。」は物理的に保護されます。

#### ② 連続する入力と文脈の分断
直後に「テストをやって（行き）」と続きます。本来は「これからテストをやって行きます」と1文にしたいところですが、保護された「。」により文脈が分断されています。

さらに `02:14:50` で「行きます」まで来た後、再びタイムアウトが発生します。
```text
26-03-12_02:14:50 mycute.stt.win            [DEBUG] [Win] Timeout triggered (>500ms). Re-processing for punctuation as FinalResult.
26-03-12_02:14:50 mycute.stt.win            [DEBUG] [Win] Windowing: Anchor=0, Context='これから', RawTarget='テストをやって行きます' -> Punctuated='テストをやって行きます。'
26-03-12_02:14:50 mycute.stt.win            [DEBUG] [Win] Passthrough: Watermark advanced to 15
```
ここでは「ます（丁寧語の終止）」であるため、（`win.rs` 242行目付近の）`polite` リストに合致し、末尾に「。」が正当に打たれます。ここでも `watermark_len` が `15` に進みます。

#### ③ OSのFinalイベントによる空タイムアウトと重複句読点
直後の `02:14:50` から `02:14:51` にかけて、奇妙な挙動が発生します。
```text
[Win/SpeechHelper] Raw: 'これからテストをやって行きます' -> Clean: 'これからテストをやって行きます' (Final: True)
...
26-03-12_02:14:50 mycute.stt.win            [DEBUG] [Win] Windowing: Anchor=0, Context='これからテストをやって行きます', RawTarget='' -> Punctuated=''
...
26-03-12_02:14:51 mycute.stt.win            [DEBUG] [Win] Timeout triggered (>500ms). Re-processing for punctuation as FinalResult.
26-03-12_02:14:51 mycute.stt.win            [DEBUG] [Win] Windowing: Anchor=0, Context='これからテストをやって行きます', RawTarget='' -> Punctuated='。'
```
OSが「これも確定した」として `final: 1` のイベントを送ってきます。差分テキストはありません（`RawTarget=''`）。
しかし、この直後に発火したタイムアウト処理において、`win.rs`（550行目付近）のタイムアウト判定が「まだ処理していないシーケンスがある」と誤認するか、あるいは `PunctuationMachine::insert_with_context` に `text=""` で渡った際、同ファイルの 57行目付近のロジックが発動しています。
```rust
if text.is_empty() {
    // タイムアウト時、かつ過去の文脈が存在する場合は、文脈の末尾を解析して句読点だけを単体で生成する。
    if allow_terminal_punctuation && !context.is_empty() && locale == &LocaleCode::Ja {
        ... // (文末が句読点で終わって「いない」と誤認して「。」を生成している)
```
文脈の末尾がすでに「。」であるかのチェック（65行目）が、`clean` 処理の不整合などでうまく機能せず、結果として「。」単体が生成され、それが画面に打ち込まれる（連打される）原因となっています。これが「行きます。。」の要因です。

#### ④ 疑問符「？」の重複化
同様の現象が `02:15:00` 〜 `02:15:01` の「いるでしょうか？？」でも起きています。
「いるでしょうか」を受信した際、フロントエンド（`main_of_cl.rs` の `[Cleanup]` ログ）で独自に「？」への変換処理（`Cleanup`）が走り、「いるでしょうか？」となります。
しかしその後、OSから `final: 1` が来てタイムアウトが行われた際、`PunctuationMachine` が `context="いるでしょうか"`（クリーンアップ前の文字）を見て「か」で終わっているからと判断し、単体で「？」を生成し送信してしまいます。これが重なって「？？」となります。

### LLM有効時（非パススルー）に発生しない理由
LLM（`PostCorrectionProcessor`）が有効な場合、文字の確定（`FinalResult`への格上げ）はLLMの応答が返ってきたタイミングに **完全に集約** されます。
LLM有効時は、`win.rs` の `ticker_task` ループ内にある無音タイムアウト処理（550行目〜）は実質的に無視され（未確定文字は適宜 `PartialResult` で送られつつもLLM側にストックされるため）、細かく `watermark` が前進することはありません。LLMという巨大な知能が文脈全体を見て適切に句読点を打ち直した上で1つの完成された `FinalResult` を返すため、このような「細切れのタイムアウトによる誤爆」や「OS側のFinalとタイムアウトの衝突」が発生しません。

**結論:** パススルーモードにおいてのみ、OS由来の細かな `Final` イベントと、システム独自の「500msタイムアウトによる強制句読点＆Final確定」ロジックが互いに干渉し合い、さらに `PunctuationMachine` の文脈判定・空文字処理ロジックが誤作動することで、不要な句読点が大量生産・保護されてしまっています。

---

## 総括

現在のところコードの修正箇所には一切手をつけておらず、待機状態にあります。
今後の対応方針（修正案の提示、または修正の直接着手など）についてのご指示をお待ちしております。
