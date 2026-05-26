# M1.76-13: 決定論的リプレイテスト（MUST replay test）

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/reciprocity.rs` | 新規コード追加 | ReciprocityReplayScenario, ReciprocityReplayTrace, compute_trace_hash, run_reciprocity_replay, ReplayTraceComparator, テスト6種 |

## 実装内容

### 新規公開型・関数

1. **`ReciprocityReplayScenario`** — リプレイシナリオ（events, metrics, policy, clock_schedule, lifecycle_scores, child_protections）
2. **`ReciprocityReplayTrace`** — 実行結果（profiles, hazards, snapshots, trace_hash）
3. **`compute_trace_hash`** (private) — 決定論的多項式ハッシュで trace を一意識別
4. **`run_reciprocity_replay`** — シナリオ逐次実行エンジン
5. **`ReplayTraceComparator::assert_bitwise_eq`** — 2 トレースのビットレベル一致検証

### 設計判断

- `ReciprocityReplayScenario` の event 型は `Vec<ReciprocityEvent>`（`DarviumEvent` の変換レイヤーを分離し、テスト構築を実用的に）
- trace_hash は HashMap の iteration 順序非依存性を排除するため、全マップをキーソート後に決定論的多項式ハッシュを計算
- `DefaultHasher` はプロセスごとにランダムな内部キーを使用するため不使用

### テスト結果（6/6 PASS）

- T1: 完全同一シナリオのビットレベル一致 ✅
- T2: policy version 変更による限定差分 ✅
- T3: clock_schedule 変更による限定差分 ✅
- T4: イベント順序維持の再実行で完全一致 ✅
- T5: n=100 独立実行で全 trace_hash 一致 ✅
- T6: golden trace 保存と回帰検出 ✅

### RFC 無矛盾確認

- RFC §41B.20.8 "Replay test (MUST)" — 同一 event stream / policy / VirtualClock → 再計算結果一致を確認
- RFC §41C.3 "M1.x" — ReciprocityEvent ingestion + policy-versioned recompute + snapshot comparison を満たす

### 全テスト通過

lib: 1015 passed, 0 failed（うち6新規）
合計: 1032 passed, 0 failed
