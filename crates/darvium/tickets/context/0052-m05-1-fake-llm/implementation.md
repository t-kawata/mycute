# 実装サマリ: M0.5-1: スクリプト化された壊れたフォーマット出力 Fake LLM クライアントの実装

## 変更したファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/llm/mod.rs | 新規実装 | ScriptedFakeLlmClient 構造体、MalformationType 列挙型、LLMClient トレイト実装、18 テスト + 3 観測テスト |
| src/llm/mod.rs | Boy Scout 改善 | FakeLlmClient::generate_malformed() のハードコード定数を名前付き定数に抽出 |
| src/constants.rs | 新規定数 | SCRIPTED_FAKE_LLM_DEFAULT_MALFORMED_PROB を追加 |
| tickets/specs/0052-m05-1-fake-llm.md | 更新 | plan-phase Investigation 更新 (RFC §14.1 §14.2 型検証結果) |

## 実装内容の概要

### ScriptedFakeLlmClient
- PRNG 駆動（StdRng::seed_from_u64(TEST_PRNG_SEED)）の確率的制御
- p_m ∈ [0.0, 1.0] の連続制御パラメータ
- 内部 Mutex<StdRng> によるスレッドセーフな interior mutability
- 8 種類の MalformationType（MissingClosingBrace, WrongKeyName, BitFlip, EmptyObject, ExtraField, TypeMismatch, NestedBraceDestruction, RawError）

### テスト
- S1-S18: 全 18 ユニットテスト（trait bounds, 正常系, 異常系, 再現性, 分布検証等）
- OTS-S1: p_m sweep（0.0→1.0 を 0.1 刻み × 各 n=1,000）
- OTS-S2: エントロピー一致性（p_m=0.3, n=10,000）
- OTS-S3: デシリアライズ相転移プロファイル（n=1,000 × 11 points）

### Boy Scout 改善
- FakeLlmClient::generate_malformed() の count % 3 マジックナンバーを MALFORMED_PATTERNS 定数配列に抽出

### 品質
- cargo test: 450 lib + 5 doc + 1 integration = 全 456 テスト PASS
- cargo clippy: 警告ゼロ
- cargo fmt: 通過
