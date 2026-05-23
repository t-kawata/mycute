# Plan: M0.5-1 — スクリプト化された壊れたフォーマット出力 Fake LLM クライアントの実装

## 要件の再確認
ScriptedFakeLlmClient を実装し、既存の FakeLlmClient（ハッシュベース・3固定パターンのみ）を改変せずに、PRNG 駆動の8種類の不正フォーマットをスクリプト制御可能にする。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---|---|---|
| src/constants.rs | 修正 | SCRIPTED_FAKE_LLM_DEFAULT_MALFORMED_PROB 定数を追加 |
| src/llm/mod.rs | 追加 | ScriptedFakeLlmClient 構造体、MalformationType 列挙型、LLMClient 実装、全テスト |

## 計装・観測の実装計画
全テストで StdRng::seed_from_u64(TEST_PRNG_SEED) を使用し完全再現性を保証。
- 不変条件テスト: S1〜S16（assert! / assert_eq!）
- 統計的検証: S17（比率の95%信頼区間, n=10,000）, S18（serde_json エラー確認）
- 観測テスト: OTS-S1（p_m sweep, 11 points × n=1,000）, OTS-S2（エントロピー, n=10,000）, OTS-S3（相転移プロファイル）

## 実装手順
1. src/constants.rs に新規定数追加
2. MalformationType 列挙型定義（8バリアント）
3. ScriptedFakeLlmClient 構造体定義（StdRng, malformed_probability, script_template）
4. LLMClient トレイト実装
5. apply_malformation() メソッド実装
6. テスト実装（S1〜S18）
7. 観測テスト実装（OTS-S1〜OTS-S3）
8. cargo test + cargo clippy + cargo fmt 全検証
9. Boy Scout 改善（generate_malformed() リファクタリング）

## 物理的レビュー方法
1. cargo test 全通過確認
2. cargo clippy -- -D warnings 警告ゼロ
3. cargo fmt 通過確認
4. 翻訳可能性 grep
5. 観測テスト出力確認（--nocapture）

## リスク
- 低: StdRng API 互換性確認済み
- 低: 既存コード改変なしのため回帰リスクなし
