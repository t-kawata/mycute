# 計画: ReputationProfileの永続化と再読込 (チケット#138)

## 要件の再確認

6つの Acceptance Criteria を満たす:
1. GraphStore トレイトに `store_reputation` / `load_reputation` が追加されている
2. `store_memoized_graph` が reputation を保存する
3. `load_memoized_graph` が保存済み reputation を復元する
4. 保存データがない場合、`cold_start()` にフォールバックする
5. 保存/読込の失敗が non-fatal である
6. 既存全テストが通過する

## RFC 既存実装状態検証

### 該当セクション
- RFC §8 (WorkflowCache と MemoizedGraph)
- RFC §15.10.3 (ReputationProfile)

### RFC §8 MemoizedGraph 型定義 vs 現行コード (trust.rs:32-68)

| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| id | WorkflowGraphId (=String) | String | ✅ 一致 |
| graph | WorkflowGraph | WorkflowGraph | ✅ 一致 (Serialize未実装) |
| task_embedding | Vec\<f32\> | Vec\<f32\> | ✅ 一致 |
| trust | TrustProfile | TrustProfile | ✅ 一致 |
| version | u64 | u64 | ✅ 一致 |
| last_virtual_seen | u64 | u64 | ✅ 一致 |
| experience_count | u32 | u64 | ❌ 型不一致（既知のシミュレーション拡張） |
| reputation | ReputationProfile | ReputationProfile | ✅ 一致 |
| gc_state | GcState | GcEvent | ✅ 一致 |
| 上記以外のRFCフィールド | - | (未実装) | ⚠️ 縮約実装のため既知 |

**評価サマリ**: reputation フィールドは RFC と完全一致。Serialize/Deserialize は ReputationProfile に既に実装済み。

### RFC §15.10.3 ReputationProfile: ✅ 全16フィールド、Serialize + Deserialize 済み

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/store/graph_store.rs | 変更 | GraphStore トレイトに store_reputation/load_reputation 追加。InMemoryGraphStore に実装 |
| src/store/coordinator.rs | 変更 | store_memoized_graph 拡張。load_memoized_graph 拡張 (cold_start フォールバック) |
| src/spaceposition.rs | 変更 | SpacePositionEmbedding に Serialize/Deserialize 追加 |

## 技術的判断: MemoizedGraph 全体の Serialize/Deserialize は不要

WorkflowNode/EdgeMeta (types.rs:57,78) に Serialize/Deserialize がなく、DiGraph 全体の serialization は現実的でない。reputation のみ serde_json::to_vec で個別保存する（Scope B の設計をそのまま採用）。SpacePositionEmbedding には Serialize/Deserialize を追加（影響範囲狭小）。

## 計装・観測の実装計画

### 実装するテストコード
- coordinator.rs mod tests 内に T1-T3 を追加

### 観測すべき統計量
- T1: ラウンドトリップ前後の reputation 16フィールド比較（誤差 < 1e-6）
- T2: cold_start フォールバック確認
- T3: 非 fatal エラーハンドリング確認
- T4: cargo test 全テスト PASS/FAIL カウント

### 較正ループ
- 本チケットでは較正は行わない（spec の Non-scope に明記）

## Boy Scout 改善

- store_memoized_graph のエラーハンドリングに reputation 保存失敗の warn! ログを追加
- 既存の store_embedding エラーハンドリングとパターンを統一

## 実装手順

1. spaceposition.rs: SpacePositionEmbedding に Serialize, Deserialize 追加
2. graph_store.rs: GraphStore トレイト + InMemoryGraphStore 実装追加
3. coordinator.rs: store_memoized_graph / load_memoized_graph 拡張
4. coordinator.rs: テストコード T1-T3 追加
5. cargo test 全テスト通過確認

## 物理的レビュー方法

```
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/store/coordinator.rs src/store/graph_store.rs src/spaceposition.rs | node "$_R/scripts/tickets/review/generate-report.js"
```

翻訳可能性チェック:
- 新規メソッド名が動詞句であること (store_reputation / load_reputation) ✅
- エラー握りつぶしがないこと
- マジックナンバーの直接使用がないこと

## リスク

- SpacePositionEmbedding の Serialize/Deserialize: Option<[f32; 3]> の newtype — serde の derive で自動対応可能。リスク低
- InMemoryGraphStore の内部状態: reputations HashMap 追加のみ。リスク極低
- load_memoized_graph のフォールバック条件: NotFound (保存データなし) と Storage (読込失敗) を区別。前者は cold_start、後者は warn! + cold_start
