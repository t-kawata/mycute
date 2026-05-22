# 実装サマリ: M-2-1.6 LLMClient 抽象トレイトの定義

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/llm/mod.rs` | 新規作成 | LLMClient トレイト、LlmSchema 列挙型、FakeLlmClient 実装、T1-T16 + call_count テスト |
| `src/error.rs` | 修正 | DarviumError に Llm(String) および LlmMalformedJson(String) を追加 |
| `src/lib.rs` | 修正 | `pub mod llm;` モジュール登録を追加 |
| `src/constants.rs` | 修正 | `FAKE_LLM_DEFAULT_MALFORMED_PROB: f64 = 0.0` を追加 |

## 実装内容

### LLMClient トレイト
- `fn generate_structured(&self, prompt: &str, schema: &LlmSchema) -> Result<String, DarviumError>`
- `Send + Sync` 境界、Box<dyn LLMClient> のオブジェクト安全性確認済み

### LlmSchema 列挙型
- 4 バリアント: QueryDesignText, PatchOperations, SelfScore, FreeText
- Debug, Clone, PartialEq を derive

### FakeLlmClient
- 固定文字列モード: コンストラクタで指定された文字列を常に返す
- 乱数モード: LCG (乗算ハッシュ) ベースの決定論的乱数で確率的に不正フォーマットを注入
- 3種類の不正フォーマット: 空文字列, 不正JSON, 予期外文字列
- `call_count: Arc<AtomicUsize>` で呼び出し回数計測

### エラー型拡張
- DarviumError::Llm(String) — LLM 呼び出し一般エラー
- DarviumError::LlmMalformedJson(String) — JSON パース失敗

### テスト結果
- 全49テスト通過 (LLM: 17, Store: 23, Types: 8, Doc: 1)
- T1-T16 の全テストケースを実装・通過確認

### 特記事項
- Rust 2021 edition の prefix identifier 予約構文対策として文字列末尾に非識別子文字を付与
- Rust 1.92.0 の lexer bug (raw string in match guard) 対策として `r##` ダブルハッシュ区切りを使用
- T8 統計的検証は同一インスタンスの n_trials 回呼び出しで検証 (単一呼び出しインスタンスでは乗算ハッシュが prev=0 で常に同一値を返すため)
