# 実装サマリ: M0-1 CompositionPlan データ整合性及び変数スコープ静的バリデータ

## 変更したファイル

| ファイル | 変更種別 | 内容 |
|----------|---------|------|
| src/types.rs | 修正 | VarType enum / VarDecl struct / WorkflowNode enum / EdgeMeta enum / CompositionPlan struct を RFC §6.1/§6.2/§13.3 に準拠して拡張 |
| src/composition.rs | 新規 | validate_composition_plan() / detect_frontier_leakage() + 全テスト T1-T12, OTS-1-3 |
| src/lib.rs | 修正 | pub mod composition の追加 |
| src/store/graph_store.rs | 修正 | WorkflowNode / EdgeMeta のユニット構築を enum variant に変更 |

## 実装内容

1. VarType enum / VarDecl struct (RFC §6.1) — 変数宣言の型安全な表現
2. WorkflowNode enum (RFC §6.1) — AgentStep / SubWorkflow variant を定義
3. EdgeMeta enum (RFC §6.2) — DependsOn / DataFlow / Conditional / FanOut / Collect を定義
4. CompositionPlan struct (RFC §13.3) — 5 フィールドの完全構造体に拡張
5. validate_composition_plan() — V-03/V-04 の静的検証
6. detect_frontier_leakage() — 並列 frontier 上の scope leakage 検出

## テスト結果

- 全 387 テスト通過（既存 372 + 新規 15）
- OTS-1: 捕捉率 1.0000（10,000 trials, 3,264,531 injections）
- OTS-2: 線形スケーリング γ ≈ 1.0 確認
- OTS-3: max_steps=2,048 < 10,000（有界性確認）
