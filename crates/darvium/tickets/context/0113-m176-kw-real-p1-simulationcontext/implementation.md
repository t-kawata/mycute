# 実装成果: M1.76-KW-REAL-P1 SimulationContext 基盤

## 変更ファイル一覧

| ファイル | 種別 | 変更内容 |
|----------|------|----------|
| src/simulation.rs | 追加 | SimulationContext 構造体（7フィールド）、VillageAssignment 型エイリアス、5メソッド（new/population_count/generate_node_id/add_person/remove_node）、Test TC1-TC8 |
| src/spaceposition.rs | 追加 | decompose_position 関数（3成分分割）、3テスト（標準・ゼロ・負値） |

## 実装のポイント

1. **新旧共存**: SimWorkflowState（既存13フィールド構造体）はそのまま維持。kind_world.rs の60箇所以上の使用箇所を変更せず、SimulationContext を新規追加。
2. **実部品再利用**: MemoizedGraph（trust.rs）をグラフ格納に使用。HelpSession（help.rs）を保持。既存の Darvium 型と完全互換。
3. **VillageAssignment**: `Option<usize>` の型エイリアスとして後方互換性を確保。
4. **decompose_position**: RFC §41B-2 に基づく位置分解の骨格実装。現時点では単純な3成分分割。完全式 `p_t(G) = λ_q q_t(G) + λ_h h_t(G) + λ_k k_t(G)` は後続チケットで実装予定。
5. **remove_node**: petgraph の `Graph::remove_node` は指数コンパクションを行うため、戻り値（Option）で存在確認を実施。インデックス範囲チェックのみでは不十分であることを確認済み。
6. **Boy Scout 改善**: decompose_position 追加時に既存コードスタイルに合わせたドキュメントコメントを付与。

## テスト結果

- 全1197テスト PASS（0 failures）
- 不変条件テスト（TC1-TC8）: 全通過
- 観測テスト（test_simulation_context_instrumentation）: 全通過（CSV/JSON 出力確認済み）
- 観察レポート: tickets/context/0113-m176-kw-real-p1-simulationcontext/observation-20260526-191047.md
