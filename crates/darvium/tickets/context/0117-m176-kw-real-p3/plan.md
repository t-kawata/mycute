# 実装計画: M1.76-KW-REAL-P3: ワークフロー実行抽象化

## 要件の再確認
RFC §4A.4 の 3 機構のうち、SideEffectSet は既存流用、残る compile_to_steps と ErrorMode を abstract 実装。

## RFC 既存実装状態検証

### RFC §7.1 `compile_to_steps`
| 項目 | RFC の定義 | 現行コード | 状態 |
|------|-----------|-----------|------|
| 関数シグネチャ | fn(graph, registry, ctx) -> Result<Vec<OpenFangStep>, CompileError> | 未実装 | ❌ 型未定義 |
| SubWorkflow展開 | CompilerContext を使用 | — | — |
**評価**: P3 では Darvium-Tickets-v2.3.md の簡略版（fn(graph) -> Result<Vec<NodeId>, DarviumError>）を実装。

### RFC §6.1 / Darvium-Tickets-v2.3.md `ErrorMode`
| フィールド | RFC の型 | チケット定義 | 状態 |
|-----------|---------|-------------|------|
| Fail | Fail | FailOnAny | ⚠️ リネーム |
| Skip | Skip | SkipOnError | ⚠️ リネーム |
| Retry | Retry { max_attempts, backoff_secs } | RetryOnError(u32) | ⚠️ 簡略化 |
| Degrade | 未定義 | Degrade | ⚠️ 追加 |
**評価**: Darvium-Tickets-v2.3.md を絶対正本とし、チケット定義に従う。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/compiler.rs | CREATE | compile_to_steps 関数 + 全型 + テスト (~250行) |
| src/types.rs | MODIFY | ErrorMode / StepStatus / StepExecutionResult 追加 (~30行) |
| src/lib.rs | MODIFY | pub mod compiler; + pub use 追加 (~3行) |

## 計装・観測の実装計画
- 不変条件テスト: TC1-TC9 (compile_to_steps 各種グラフ形状 + 型構築)
- 観測テスト: TC10 (100ノード大規模グラフの実行時間)
- 較正: 不要（純粋関数）

## Boy Scout 改善
- types.rs の型追加（既存 SideEffectSet スタイルに合わせる）
- clippy 警告があれば修正

## 実装手順
1. src/types.rs: ErrorMode / StepStatus / StepExecutionResult 追加
2. src/compiler.rs: 新規作成（compile_to_steps + 全型 + テスト）
3. src/lib.rs: モジュール追加
4. cargo test 全 PASS + cargo clippy 警告ゼロ確認

## 物理的レビュー方法
run-quality-checks.js src/compiler.rs src/types.rs src/lib.rs | generate-report.js
翻訳可能性 grep

## リスク
- 低: 純粋関数 + 型定義のみ
- 注: petgraph の NodeIndex → usize 変換
