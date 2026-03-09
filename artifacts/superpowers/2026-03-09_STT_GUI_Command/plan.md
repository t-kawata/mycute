# [STT Engine Instant Switch] 包括的実装計画書

## ゴール
音声認識エンジン（OpenAI / OS）をGUIから即座に切り替えられるようにする。
設定値は言語設定（locale）の流儀に倣い、WebView側のローカルストレージ（ldb）で一元管理する。
アプリ起動時およびGUIでの変更時に、フロントエンドからバックエンドへ最新のエンジン設定を強制同期することで、常に一貫した動作を保証する。

## Proposed Changes

### 1. フロントエンド連携用の専用定数定義 (`src/constants.rs`)
- [MODIFY] `src/constants.rs` に、エンジンの識別子として以下の定数を追加する。
  ```rust
  pub const ENGINE_OPENAI: &str = "openai";
  pub const ENGINE_OS: &str = "os";
  ```
- これらの定数は `scripts/gen-ts-constants.mjs` を介して `web/src/consts/generated_constants.ts` に自動エクスポートされ、フロントエンドでのハードコーディング（生文字列の使用）を完全に排除する。

### 2. バックエンド設定構造の調整とエンジン永続化の分離 (`src/stt_config.rs` / `settings.json.example`)
- [MODIFY] `src/stt_config.rs` の `Settings` 構造体の `stt_engine` フィールドに `#[serde(skip)]` を付与する。
  - 理由: ユーザーによるエンジン選択状態はフロントエンドの物理的なローカルストレージ（ldb）を正味の保存先（Source of Truth）とするため、バックエンドの `settings.json` への二重保存を避ける。
- [MODIFY] `settings.json.example` から `stt_engine` キーを削除する。
  - バックエンドの設定ファイルからは管理対象外となるため、初期設定サンプルからも除外して混乱を防ぐ。

### 3. Tauriスイッチコマンドの追加 (`src/tauri_cmd/settings.rs` / `src/mode/cl/main_of_cl.rs`)
- [NEW] フロントエンドからエンジン切り替えを通知するための `switch_stt_engine` コマンドを実装し、`main_of_cl.rs` の `invoke_handler!` に登録する。
- 引数: `engine: crate::stt_config::SttEngine` (Enum)
- 処理:
  1. 引数の Enum は、フロントエンドからの定数文字列（"openai" / "os"）を受け取り、Tauri/Serde によって型安全に自動デシリアライズされる。
  2. `state.manager` を通じて `SpeechRecognizer::update_config(engine, ...)` を呼び出す。
  3. 前工程で実装した「軽量な `update_config`」により、バックエンドは既存のインスタンスを即座に再稼働（.stop()/.start()）させ、ミリ秒単位での切り替えを実現する。

### 4. フロントエンド状態管理と永続化レイヤー (`web/src/stores/main-store.ts` / `src/utils/ldb.ts`)
- [MODIFY] `src/utils/ldb.ts` の `KEYS` に `SE: 'SE'` (STT Engine) を追記し、キー名のマジックストリングを排除する。
- [MODIFY] `main-store.ts` の `state` に `sttEngine` を追加。初期値は `get<string>(KEYS.SE) || ENGINE_OS` （デフォルトはOS）形式で `ldb` から復元する。
- [NEW] アクション `setSttEngine` を実装。
  1. `ldb.set(KEYS.SE, engine)` により WebView 側に永続化。
  2. `invoke('switch_stt_engine', { engine })` を呼び出し、バックエンドへ即座に反映。

### 5. 多言語化辞書 (i18n) の更新 (`web/src/i18n/ja-JP/index.ts` , `en-US/index.ts`)
- [MODIFY] 言語リソースに以下のキーを追加し、UIのあらゆる文言をハードコーディングから解放する。
  - `page.index.settings.sttEngine`: "音声認識エンジン" / "STT Engine"
  - `page.index.settings.sttEngineDescription`: "使用する音声認識エンジンを選択します。" / "Select the speech recognition engine to use."
  - `page.index.settings.sttEngineOpenAI`: "OpenAI (Cloud)"
  - `page.index.settings.sttEngineOs`: "OS Native"

### 6. 初期化時のバックエンド同期フロー (`web/src/App.vue` / `utils/some.ts`)
- [MODIFY] `App.vue` の `initApp()` シーケンスを拡張する。
- **言語設定の初期化（`useLangSetter`）と全く同一の重要度**として、起動直後に `mainStore.sttEngine` （ldbから復元された値）を `invoke('switch_stt_engine', ...)` でバックエンドに明示的に通知・適用するフローを差し込む。
- これにより、アプリ起動後の最初の音声認識から、ユーザーの意図したエンジンが確実に動作することを保証する。

### 7. GUI 実装 (`web/src/apps/SettingsApp.vue`)
- [MODIFY] 既存の「英語モード」トグルの直後に、エンジンの選択用セクションを追加する。
- **コンポーネント**: ご指定の `q-select` を使用。
- **実装内容**:
  - `v-model` で `mainStore.sttEngine` と算出プロパティ（computed）を介してパッチ。
  - `options` には i18n のラベルを載せたオブジェクト配列（定数 `ENGINE_OPENAI` / `ENGINE_OS` を値に持つ）を渡す。
  - Quasar の `filled` や `map-options`, `emit-value` 属性を使い、既存の美観を損なわない premium なデザインとする。

## Verification Plan

### Automated Tests
- `make check`: バックエンド側の型整合性とコンパイルの成功を確認。
- `make build-sdk-ts`: 定数抽出スクリプトが走り、`generated_constants.ts` に新規定数が正しく反映されることを確認。

### Manual Verification
1. **初期化同期の確認**: アプリ起動時のログにより、`ldb` の設定値に従ってバックエンドのエンジンが正しく指示されているか（例：`Switching engine to os`）を確認。
2. **GUIの表示とi18n**: `SettingsApp.vue` で `q-select` が適切な多言語ラベルで表示されるか確認。
3. **即時切替の検証**: 選択肢を変更した瞬間、認識動作が OpenAI ⇔ OS で切り替わり、それぞれのエンジン特有のログが出力されることを目視確認。
4. **永続性の確認**: エンジン変更後にアプリを再起動し、GUIの選択状態およびバックエンドの動作エンジンが、変更後の状態を正しく復元していることを確認。

## User Review Required
> [!IMPORTANT]
> - `Settings` 構造体の `stt_engine` フィールドに `#[serde(skip)]` を設定するため、既存の `settings.json` 内に保存されていたエンジンの値は今後無視されるようになります。

以上、詳細まで網羅いたしました。承認いただけましたら、`/superpowers-execute-plan` にてお知らせください。
