---
ticket_id: 6
title: Windows音声入力の設定不足を検出しユーザーに通知するヘルスチェック機能
slug: windows
status: reviewed
created_at: 2026-05-18
updated_at: 2026-05-18
---
# Windows音声入力の設定不足を検出しユーザーに通知するヘルスチェック機能

## Summary

Windows 11 の音声入力（Alt ダブルタップ）が動作するために必要な3つの設定（音声認識言語モデルのインストール、音声認識プライバシートグル、マイク権限）をアプリ起動時に自動チェックし、不足があればユーザーに具体的な対処方法をダイアログ表示する。

## Background

### 問題

Windows 11 のクリーンインストール直後は以下の設定がデフォルトで無効であり、Alt ダブルタップで起動音は鳴るが音声認識が一切動作しない：

1. **音声認識言語モデル未インストール** — 設定 → 時刻と言語 → 言語と地域 → 言語オプション → 「音声認識」モジュールのダウンロードが必要
2. **音声認識プライバシートグル OFF** — 設定 → プライバシーとセキュリティ → 音声認識 → ON が必要
3. **マイク権限なし** — 設定 → プライバシーとセキュリティ → マイク → 「デスクトップアプリのマイクアクセスを許可する」が必要

### 症状

- Alt ダブルタップで起動音は鳴るが音声が認識されない
- ログに `[Win/SpeechHelper] Starting audio capture...` や `[Win] Received native audio:` が一切出力されない
- WinRT AudioGraph の `QuantumStarted` イベントが一度も発火しない
- C# 側の `RequestAuthorization()` がスタブ（no-op）であり、権限チェックをしていない

### 診断経緯

- Mac では問題なく動作 → Windows 固有の問題
- ブラウザの音声入力は WebRTC(getUserMedia) という別経路のため動作 → マイクハードウェア自体は正常
- ログ分析の結果、AudioGraph の量子クロックが進まず、音声データが Rust 側に一切届いていないことを確認
- ユーザーが Windows 設定で音声認識モデルのインストールとトグルの有効化を行ったところ解決

## Scope

### やること

1. **C# SpeechHelper にヘルスチェック関数 `speech_helper_check_health()` を追加**
   - `SpeechRecognizer.SupportedTopicLanguages` に対象ロケール（ja-JP / en）が含まれているか確認 → モデル未インストール検出
   - `SpeechRecognizer` を試作して `CompileConstraintsAsync()` の結果が `PrivacyPolicyDeclined` か確認 → プライバシートグル OFF 検出
   - `AudioGraph.CreateDeviceInputNodeAsync()` が `AccessDenied` を返すか確認 → マイク権限なし検出
   - 結果をintビットマスクで Rust 側に返す（0=正常, 1=モデル欠如, 2=音声認識OFF, 4=マイク権限なし）

2. **Rust 側でヘルスチェック結果を受け取り、Tauri イベントとしてフロントエンドに通知**
   - `WinSpeechBackend::new()` 内でヘルスチェックを実行し結果を保持
   - アプリ起動完了後 / Alt録音開始時に結果に応じてイベント emit

3. **フロントエンド（Vue）に Windows 専用の警告ダイアログを追加**
   - 不足項目ごとに具体的な対処手順を表示
   - 「設定を開く」ボタンで該当の設定画面を直接開く（`ms-settings:` URI）
   - ダイアログは閉じられるようにする
   - **ダイアログ内の全メッセージは i18n 対応（`LocaleCode::Ja` / `LocaleCode::En`）とする**
   - Windows 設定画面の名称（日本語「時刻と言語」→ 英語 "Time & Language" 等）もロケールに応じて切り替わること

4. **起動時と録音開始時の2箇所でチェック**
   - Windows 起動時に一度チェックし問題があればダイアログ表示
   - Alt 録音開始時にもチェックし問題があればダイアログ表示（既確認ならスキップ）

### やらないこと

- Mac の音声入力設定チェック（影響範囲外）
- 初回のみのダイアログ表示の永続化（設定ファイルへの保存）
- 設定不足の自動修正（ユーザーに手動設定を促すのみ）
- 既存の音声認識開始処理の変更（チェックは事前に行い、開始処理自体は変更しない）
- スナックバーでの通知（ダイアログのみ）

## Investigation

### 関連ソースファイル

| ファイル | 役割 |
|----------|------|
| `native/cs/SpeechHelper/SpeechHelper.cs` | C# WinRT 音声認識・音声キャプチャ。`RequestAuthorization()` は Line 99-104 にスタブ実装 |
| `src/stt/win.rs` | Rust Windows STT バックエンド。`start_native_audio_capture()` の失敗を検知できない設計 |
| `src/stt/recognizer.rs` | 認識器トレイト。`WinSpeechBackend` を保持 |
| `src/mycute_manager.rs` | `start_recording()` / `stop_recording()` で認識器を制御 |
| `src/tauri_cmd/system.rs` | ホットキーハンドラ。`HotkeyAction::Start` / `BufferFlush` を処理 |
| `src/mode/cl/main_of_cl.rs` | メインイベントループ。`SttEvent` を処理し Tauri のイベントを emit |

### 検出ロジックの詳細

**1. 音声認識モデル未インストール（SpeechHelper.cs 内で検出）**

```csharp
// SpeechRecognizer.SupportedTopicLanguages に対象ロケールがあるか
var supported = SpeechRecognizer.SupportedTopicLanguages;
bool hasModel = supported.Any(l => l.LanguageTag.StartsWith("ja-") || l.LanguageTag == "ja");
```

**2. 音声認識プライバシートグル OFF（SpeechHelper.cs 内で検出）**

```csharp
// SpeechRecognizer を作成して CompileConstraintsAsync を試行
var recognizer = new SpeechRecognizer(new Language("ja-JP"));
var result = await recognizer.CompileConstraintsAsync();
bool privacyOff = (result.Status == SpeechRecognitionResultStatus.PrivacyPolicyDeclined);
```

**3. マイク権限なし（SpeechHelper.cs 内で検出）**

```csharp
// AudioGraph の CreateDeviceInputNodeAsync を試行
var settings = new AudioGraphSettings(AudioRenderCategory.Speech);
var graphResult = await AudioGraph.CreateAsync(settings);
var inputResult = await graphResult.Graph.CreateDeviceInputNodeAsync(MediaCategory.Speech);
bool micDenied = (inputResult.Status == AudioDeviceNodeCreationStatus.AccessDenied);
```

### 既存の問題点

- `RequestAuthorization()`（SpeechHelper.cs:99-104）は常に成功を返すスタブ
- `StartCapture()`（SpeechHelper.cs:257）は `Task.Run(async ...)` の完了を待たず常に 0 を返すため、Rust 側が初期化失敗を検知できない
- `OnAudioQuantumStarted` のエラーログは `_debugFrameCounter % 100 == 0` で絞られており、最初の99回のエラーが握りつぶされる（Lines 501, 528, 551, 587, 606）

### 設定を直接開くための ms-settings: URI

| 設定画面 | URI |
|---------|-----|
| プライバシー → 音声認識 | `ms-settings:privacy-speech` |
| プライバシー → マイク | `ms-settings:privacy-microphone` |
| 時刻と言語 → 言語と地域 | `ms-settings:regionlanguage-languageoptions` |

## Test Plan

### C# 側テスト

- `speech_helper_check_health()` が正常に呼び出せること（FFI 結合テスト）
- 各エラー状態が正しいビットマスクで返されること
  - 正常: 0
  - モデル未インストール: bit 0
  - プライバシートグル OFF: bit 1
  - マイク権限なし: bit 2
  - 複合状態: ビットの論理和

### Rust 側テスト

- `health_check_result` のパースが正しく行われること
- 検出結果に応じて正しい Tauri イベントが emit されること
- Mac ビルドでは該当コードが存在しないこと（`#[cfg(windows)]` 確認）

### フロントエンドテスト

- Windows イベント受信時にダイアログが表示されること
- 不足項目に応じたメッセージが正しく表示されること
- ダイアログが閉じられること
- 非 Windows ではダイアログが表示されないこと
- **`ja` ロケールでダイアログメッセージが日本語で表示されること**
- **`en` ロケールでダイアログメッセージが英語で表示されること**
- **ロケール切り替え時に Windows 設定画面の名称（例：「時刻と言語」↔ "Time & Language"）が正しく切り替わること**

### 実機テスト（自動化不可）

- 音声認識モデル未インストール状態で正しく警告が出ること
- 音声認識プライバシー OFF で正しく警告が出ること
- マイク権限なしで正しく警告が出ること
- 3つとも設定済みの場合に警告が出ないこと
- 「設定を開く」ボタンで該当の設定画面が開くこと
- Alt録音開始時のチェックで、すでに確認済みならスキップされること

## Boy Scout Rule — 翻訳可能性計画

このチケットで触るコードに対して以下の改善を行う：

1. **`RequestAuthorization()` の名前と実態の不一致を解消**: 現在はスタブであり名前と動作が一致していない。削除するか、実態に合った名前に変更する。
2. **`StartCapture()` の常時 0 返却**: `Task.Run` の結果を無視している。結果を Rust 側に伝えられる設計に修正する。
3. **`_debugFrameCounter % 100 == 0` のエラー絞り込み**: 最初のエラーが握りつぶされる。初回のエラーは必ずログ出力するよう修正する。
4. **新しい FFI 関数の命名**: `speech_helper_check_health` は動詞句として明確。戻り値のビットマスクは名前付き定数で定義する。

## Acceptance Criteria

- [ ] C# の `speech_helper_check_health()` が音声認識モデル・プライバシートグル・マイク権限の3状態を正確に検出できる
- [ ] 不足がある場合、Rust → Tauri イベント経由でフロントエンドに通知される
- [ ] フロントエンドに Windows 専用の設定不足警告ダイアログが表示される
- [ ] ダイアログに対象設定画面へのリンク（`ms-settings:` URI）が含まれる
- [ ] 3つとも設定済みの場合はダイアログが表示されない
- [ ] ダイアログの全メッセージが i18n 対応（LocaleCode::Ja / En）していること
- [ ] Mac ビルドに一切影響しない
- [ ] 既存の音声認識開始・終了の流れが変更されない
- [ ] 既存テストがすべて通過する
- [ ] 翻訳可能性の検証が通っている

## Notes

- Windows 設定 URI (`ms-settings:`) は Tauri の `opener` プラグインまたは `shell.open` で開く
- ヘルスチェックは `WinSpeechBackend::new()` 内で一度実行し、結果を保持する。録音開始時にもチェックするが、セッション内で既に確認済みならスキップする
- ダイアログは閉じられるようにし、録音操作をブロックしない（ユーザーが設定を無視して録音を試行できる）
- この機能の実装後、`SETUP.md` の音声入力設定手順はそのままでよい（プログラムによる検出とドキュメントの二重体制）
