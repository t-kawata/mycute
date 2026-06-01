# 計画: チケット#146 GraphStore一元化 + HELP/GMR/自己抽象化の個体登録完全化

## RFC 既存実装状態検証

### RFC §8.7 `WorkflowRegistry` (RFC 918行)

| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| `graphs` | `HashMap<WorkflowId, Arc<WorkflowGraph>>` | `HashMap<WorkflowGraphId, MemoizedGraph>` | ⚠️ 意図的拡張 (MemoizedGraph 保持) |
| `store` | (未定義) | (欠落) | ❌ 本チケットで追加 |
| `id_counter` | (未定義) | `u64` | ⚠️ 余剰 (実装上の内部管理) |

### RFC §12.4 `patch_and_register` (RFC 4365-4384行)

| 要素 | RFC の定義 | 現行コード | 状態 |
|---|---|---|---|
| `new_graph` | `apply_patch_atomic` の結果 | 結果を上書き代入 | ✅ 機能一致 |
| `new_id` | `WorkflowGraphId::new_v4()` で新規発行 | **上書き代入 (新規IDなし)** | ❌ 本チケットで修正 |
| 個体登録 | 暗黙的に独立エンティティ | **追加なし** | ❌ 本チケットで修正 |

**評価サマリ**: RFC との乖離は本チケットの修正範囲と一致。

## 要件

**問題A (アーキテクチャ違反)**: WorkflowRegistry が独自 HashMap で全グラフ保持、GraphStore を経由しない。→ compute_all_node_count が SubWorkflow 解決失敗。

**問題B (個体登録欠落)**: HELP/GMR/自己抽象化結果が新個体として population に追加されず、上書きまたは未登録。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/store/graph_store.rs` | 改修 | GraphStore に Send+Sync + store/load_memoized_graph。InMemoryGraphStore の RefCell→Mutex |
| `src/workflow_registry.rs` | 改修 | store: Option<Arc<dyn GraphStore>> 追加。register 群で store 委譲。コメント修正 |
| `src/simulation.rs` | 改修 | サーバーパス統合 (Step 4)。HELP/GMR 上書き→出生 (Step 5-6)。unwrap_or(0)改善 (Step 8) |
| `src/self_refinement.rs` | 改修 | run_self_refinement_round に on_new_individual コールバック追加 (Step 7) |

## 計装・観測の実装計画

- T-A1〜T-A4: graph_store.rs / workflow_registry.rs テストモジュール
- T-B1〜T-B6: simulation.rs テストモジュール
- O1〜O3: tests/ ディレクトリ統合観測テスト
- 固定シード StdRng::seed_from_u64(12345)、CSV 出力、--nocapture

## 実装手順

1. GraphStore トレイト拡張 (Send+Sync, store/load_memoized_graph)
2. InMemoryGraphStore の Sync 化 (RefCell→Mutex, Cell→AtomicU64)
3. WorkflowRegistry 再実装 (store 委譲)
4. サーバーパス統合 (InMemoryGraphStore 連携)
5. HELP 個体登録 (propose_subgraph_and_accept 修正)
6. GMR 個体登録 (try_gmr_diffusion 修正)
7. 自己抽象化個体登録 (on_new_individual コールバック)
8. unwrap_or(0) 改善

## Boy Scout 改善

- workflow_registry.rs:7-8 嘘コメント削除
- simulation.rs:3019-3021 / 3237-3239 関数抽出 (出生ブロック)
- self_refinement.rs:126 関数改名

## リスク

- Arc<dyn GraphStore> と Clone 導出の互換性 → Arc は Clone 可能、問題なし
- 自己抽象化コールバックでの borrow conflict → Vec に貯めてから後処理
- InMemoryGraphStore Mutex 競合 → シミュレーションはシングルスレッド、実質的競合なし
