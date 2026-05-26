# Plan: M1.76-KW-REAL-P1 SimulationContext 基盤

## 要件の再確認
現行の SimWorkflowState（flat struct, 13 fields）を SimulationContext（実際の Darvium 型を参照）で置き換え、KW-REAL シリーズの基盤とする。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/simulation.rs | 変更 | SimWorkflowState 削除、SimulationContext 追加、SimHelpSession → HelpSession 統一 |
| src/spaceposition.rs | 追加 | decompose_position 関数追加（RFC §41B-2） |
| src/kind_world.rs | 変更なし | 本チケットでは触れない |

## 実装手順

1. simulation.rs: SimWorkflowState を SimulationContext で置き換え。SimHelpSession 削除、HelpSession に統一。
2. spaceposition.rs: decompose_position 追加。
3. テストコード（TC1〜TC8）追加。
4. cargo test 全 PASS 確認。

## 計装・観測の実装計画
- 観測テスト: simulation.rs mod tests に test_simulation_context_instrumentation
- 出力: CSV（生成統計）+ JSON（位置分解値）
- 固定シード: StdRng::seed_from_u64(12345)

## Boy Scout 改善
- SimWorkflowState → SimulationContext で翻訳可能性向上
- SimHelpSession → HelpSession で型安全性向上
- VillageAssignment 型定義追加

## リスク
- &'a mut MemoizedGraph の借用規則: ライフタイム制約を徹底
- VillageAssignment 型互換性: 既存 kind_world.rs の &[Option<usize>] と整合

## 完了条件
- TC1〜TC8 全実装・全 PASS
- cargo test 全 PASS
- 観測テスト出力確認
- 翻訳可能性チェック通過
