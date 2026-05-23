---
ticket_id: 59
title: M1.5-2: 異種ストア論理一貫性コミット（ConsistencyState::Pending）プロトコルのシミュレーション
slug: m15-2-consistencystatepending
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0059-m15-2-consistencystatepending/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0059-m15-2-consistencystatepending/observation-20260523-195202.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0059-m15-2-consistencystatepending/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0059-m15-2-consistencystatepending/review.md
---

# M1.5-2: 異種ストア論理一貫性コミット（ConsistencyState::Pending）プロトコルのシミュレーション

## Summary

Darvium の dual-store アーキテクチャ（GraphStore / MetadataStore）において、`ConsistencyState::Pending` を中核とする論理コミットプロトコルを実装し、不完全状態のアセットが通常検索経路に露出しないことを検証する。

本チケットは RFC §18.2（Dual-Store Consistency Refinement）で規定された commit intent protocol を忠実に実装し、§25.x 以降の v2.3 cross-store 整合性規約に基づく hard retrieval exclusion を強制する。計装では並行アクセス・エラー注入・動的破壊実験による `P_taint = 0.00000` の一貫性遮断確認と、repair convergence time の観測を行う。

## Background

Darvium は知識オブジェクトを2つの異種ストア（GraphStore = LadybugDB 責務、MetadataStore = SQLite 責務）に分散して保持する。両ストアの更新は単一 ACID トランザクションではなく**論理コミット単位 (op_id)** として扱われ、更新中に障害が発生した場合、アセットは不完全状態（`Pending` / `NeedsRepair` / `Quarantined`）に遷移する。

v2.3 では以下の規約が強化された：

1. `ConsistencyState::Pending` / `NeedsRepair` / `Quarantined` のいずれの状態にあるアセットも通常の retrieval selection path に露出してはならない (MUST NOT)
2. Repair 完了後にのみ安全な復帰可能性を評価しうる
3. Dual-store protocol は application-level commit intent protocol であり、分散 XA ではない

**RFC §18.2 参照実装**:
```rust
fn commit_dual_store_update(op_id: String, graph: &mut MemoizedGraph) -> Result<(), RepositoryError> {
    graph.consistency_state = ConsistencyState::Pending {
        op_id: op_id.clone(),
        phase: CommitPhase::MetaPrepared,
    };

    sqlite_prepare(op_id.clone())?;
    ladybug_prepare(op_id.clone())?;

    match (sqlite_commit(op_id.clone()), ladybug_commit(op_id.clone())) {
        (Ok(()), Ok(())) => {
            graph.consistency_state = ConsistencyState::Committed;
            Ok(())
        }
        (meta_res, blob_res) => {
            graph.consistency_state = ConsistencyState::NeedsRepair {
                op_id: op_id.clone(),
                reason: format!("meta={:?}, blob={:?}", meta_res.err(), blob_res.err()),
            };
            enqueue_repair(op_id, graph.id.clone());
            Err(RepositoryError::CrossStoreInconsistency)
        }
    }
}
```

### 既存コードの状態

- `DarviumError::DualStoreCommit(String)` / `DarviumError::DualStoreInconsistency(String)` — エラー型は既に定義済み
- `GraphStore` / `MetadataStore` トレイト — 双方とも in-memory 実装が存在
- `ConsistencyState` / `CommitPhase` / `RepairLog` / `RepairAction` — RFC 定義のみでコード未実装
- `commit_dual_store_update` — 参照実装のみでコード未実装
- M1.5-1 の MockHnswIndex (`src/vector_index.rs`) はテスト基盤として利用可能

### 参照観察レポート

- `tickets/context/0058-m15-1-1536-hnsw-stage-2a2bmock/observation-20260523-193052.md` — HNSW Mock 実装完了。三角不等式 0 違反、ソート不変条件 100% 維持を確認。MockHnswIndex は本チケットのテスト基盤として使用可能。

## Scope

1. **型定義**: `ConsistencyState` 列挙型、`CommitPhase` 列挙型、`RepairLog` 構造体、`RepairAction` 列挙型を `src/types.rs` に追加
2. **`commit_dual_store_update` 関数**: RFC §18.2 の commit intent protocol を実装。第一段階（MetaPrepared）→ 第二段階（BlobPrepared）→ commit → 失敗時 NeedsRepair
3. **Hard retrieval exclusion ゲート**: `ConsistencyState != Committed` のアセットを検索候補から除外する判定関数
4. **`DualStoreCoordinator` 構造体**: GraphStore と MetadataStore を保持し、`commit_dual_store_update` をラップする Facade
5. **Repair キューイング機構**: `NeedsRepair` 状態のアセットをキューに追加する `enqueue_repair` 関数
6. **Clean state 復帰**: Repair 完了後の `Committed` / `Tombstone` / 安全状態への復帰関数
7. **エラー注入対応**: シミュレーション用の DualStore simulator にタイムアウト・I/O エラーパルス注入機能を追加

## Non-scope

- 実際の SQLite / LadybugDB への永続化 (本チケットはメモリ内完結)
- 起動時修復スキャン (`startup_repair_scan`) — M1.5-3 で実装
- 実 DB 接続の dual-store 実装 — M3/M4 以降で対応
- XA / 分散トランザクション管理 — 本チケットは application-level commit intent protocol を実装するのみ
- GraphStore / MetadataStore トレイト自体の拡張（トレイトは既存のまま、ラッパーで整合性を管理）

## Investigation

### 調査結果

#### 1. 既存エラー型の確認

`src/error.rs` (L73-L78) に dual-store 用エラー型が既に定義されている：

```rust
#[error("Dual-store commit failed: {0}")]
DualStoreCommit(String),

#[error("Dual-store inconsistency: {0}")]
DualStoreInconsistency(String),
```

これらのエラー型を `commit_dual_store_update` のエラー返却に直接利用できる。

#### 2. 既存ストアトレイトの確認

`GraphStore` (`src/store/graph_store.rs`) と `MetadataStore` (`src/store/metadata_store.rs`) は独立したトレイトであり、相互の整合性を意識していない。整合性管理は上位レイヤー（`DualStoreCoordinator`）の責務となる。

`InMemoryGraphStore` / `InMemoryMetadataStore` はともに `RefCell` ベースの内部可変性を持ち、シングルスレッドでのみ使用可能。M1.5-2 では new() ごとに独立したインスタンスを使うことでスレッド安全性を確保する。

#### 3. RFC §18.2 型定義の所在

`Darvium-RFC-0001-Unified-v2.3-final.md` の L667-L689 に以下の型定義が記述されている：

```rust
enum ConsistencyState {
    Committed,
    Pending { op_id: String, phase: CommitPhase },
    NeedsRepair { op_id: String, reason: String },
    Quarantined { op_id: String, since: SystemTime },
}

enum CommitPhase {
    MetaPrepared,
    BlobPrepared,
    MetaCommitted,
    BlobCommitted,
}

struct RepairLog {
    op_id: String,
    graph_id: WorkflowGraphId,
    detected_at: SystemTime,
    reason: String,
    action: RepairAction,
}

enum RepairAction {
    Retry,
    Tombstone,
    Quarantine,
}
```

これらは `src/types.rs` に追加する必要がある。

#### 4. 整合性タグ

`Darvium-v2.3-final-table-and-struct-definition-spec.md` の L940 に `ConsistencyStateTag` が定義されている：

```rust
pub enum ConsistencyStateTag { Committed, Pending, NeedsRepair, Quarantined }
```

これは retrieval selection path でのフィルタリングに使用される。

## Test Plan

### テスト対象モジュール

- `src/types.rs` — 新規型定義を含むファイル（`ConsistencyState`, `CommitPhase`, `RepairLog`, `RepairAction`, `ConsistencyStateTag`）
- `src/store/coordinator.rs`（新規）— `DualStoreCoordinator`, `commit_dual_store_update`, retrieval exclusion gate, `enqueue_repair`
- `src/store/mod.rs` — 必要に応じた再エクスポート

### 外部依存のモック

- `InMemoryGraphStore` / `InMemoryMetadataStore` — 既存のメモリ内実装をそのまま使用
- エラー注入用に `FailingGraphStore` / `FailingMetadataStore` ラッパーを作成（指定回数または確率で失敗する）

### テスト一覧

#### T1-T10: ConsistencyState 型と状態遷移

| ID | テスト名 | 分類 | 内容 |
|----|---------|------|------|
| T1 | `consistency_state_all_variants` | 正常 | 全4バリアントが構築可能であることを確認 |
| T2 | `commit_phase_all_variants` | 正常 | 全4フェーズが構築可能であることを確認 |
| T3 | `repair_log_construction` | 正常 | RepairLog が全フィールドを指定して構築可能 |
| T4 | `repair_action_all_variants` | 正常 | 全3アクションが構築可能 |
| T5 | `consistency_state_tag_all_variants` | 正常 | 全4タグが構築可能 |
| T6 | `consistency_state_size` | 境界 | ConsistencyState のメモリサイズが妥当な範囲内 |
| T7 | `commit_phase_size` | 境界 | CommitPhase のメモリサイズが妥当な範囲内 |
| T8 | `consistency_state_debug_format` | 正常 | Debug フォーマットが全バリアントで動作 |
| T9 | `consistency_state_partial_eq` | 正常 | PartialEq が同一/異種バリアントで正しく動作 |
| T10 | `consistency_state_clone` | 正常 | Clone が正しく動作 |

#### T11-T20: commit_dual_store_update 正常系・異常系

| ID | テスト名 | 分類 | 内容 |
|----|---------|------|------|
| T11 | `commit_dual_store_both_succeed` | 正常 | 両ストア成功 → Committed 遷移確認 |
| T12 | `commit_dual_store_graph_fails` | 異常 | GraphStore commit 失敗 → NeedsRepair |
| T13 | `commit_dual_store_metadata_fails` | 異常 | MetadataStore commit 失敗 → NeedsRepair |
| T14 | `commit_dual_store_both_fail` | 異常 | 両ストア失敗 → NeedsRepair |
| T15 | `commit_dual_store_prepare_phase` | 正常 | 準備段階で Pending → MetaPrepared が設定される |
| T16 | `commit_dual_store_pending_overwrite` | 境界 | 同一 op_id での再コミット動作 |
| T17 | `commit_dual_store_empty_op_id` | 境界 | 空文字 op_id の動作 |
| T18 | `commit_dual_store_stores_repair_log` | 正常 | NeedsRepair 時に RepairLog が記録される |
| T19 | `commit_dual_store_repair_action_retry` | 正常 | RepairAction::Retry 後の再コミット |
| T20 | `commit_dual_store_idempotent_retry` | 正常 | 冪等性: 同一 op_id で再試行しても重複不整合を生じない |

#### T21-T30: Hard retrieval exclusion gate

| ID | テスト名 | 分類 | 内容 |
|----|---------|------|------|
| T21 | `retrieval_exclusion_pending` | 正常 | Pending 状態 → 検索候補から除外 |
| T22 | `retrieval_exclusion_needs_repair` | 正常 | NeedsRepair 状態 → 検索候補から除外 |
| T23 | `retrieval_exclusion_quarantined` | 正常 | Quarantined 状態 → 検索候補から除外 |
| T24 | `retrieval_allows_committed` | 正常 | Committed 状態 → 検索候補に含める |
| T25 | `retrieval_exclusion_filter` | 正常 | 混合リストから不整合状態だけを除外 |
| T26 | `retrieval_exclusion_filter_all_non_committed` | 正常 | Committed 以外を全件除外 |
| T27 | `retrieval_exclusion_empty_input` | 境界 | 空リスト入力 → 空出力 |
| T28 | `retrieval_exclusion_all_pending` | 境界 | 全件 Pending → 空出力 |
| T29 | `retrieval_exclusion_audit_mode` | 正常 | 監査モードでは Pending/NeedsRepair を参照可能 |
| T30 | `retrieval_exclusion_input_immutability` | 正常 | フィルタが入力を変更しない |

#### T31-T40: Error injection tests

| ID | テスト名 | 分類 | 内容 |
|----|---------|------|------|
| T31 | `error_injection_graph_timeout` | 異常 | GraphStore タイムアウト注入 → NeedsRepair |
| T32 | `error_injection_metadata_timeout` | 異常 | MetadataStore タイムアウト注入 → NeedsRepair |
| T33 | `error_injection_graph_io_error` | 異常 | I/O エラーパルス注入 → NeedsRepair |
| T34 | `error_injection_metadata_io_error` | 異常 | I/O エラーパルス注入 → NeedsRepair |
| T35 | `error_injection_burst_5_failures` | 異常 | 5連続失敗 → 全件 NeedsRepair に収束 |
| T36 | `error_injection_recovery_after_repair` | 正常 | 修復後 → Committed に復帰 + 検索候補復活 |
| T37 | `error_injection_quarantine_route` | 正常 | 修復不能 → Quarantined への経路確認 |
| T38 | `error_injection_multiple_op_ids` | 正常 | 異なる op_id で独立した障害 → 独立した状態管理 |
| T39 | `error_injection_repair_log_audit_trail` | 正常 | 障害 → RepairLog 監査証跡の完全性 |
| T40 | `error_injection_clean_state_restoration` | 正常 | Clean state 復帰後の検索候補復活確認 |

#### T41-T50: 並行アクセス競合状態テスト

| ID | テスト名 | 分類 | 内容 |
|----|---------|------|------|
| T41 | `concurrent_read_during_pending_2_threads` | 競合 | 2スレッド: Pending 中の読取が P_taint=0 |
| T42 | `concurrent_read_during_pending_4_threads` | 競合 | 4スレッド |
| T43 | `concurrent_read_during_pending_8_threads` | 競合 | 8スレッド |
| T44 | `concurrent_read_during_pending_16_threads` | 競合 | 16スレッド |
| T45 | `concurrent_read_during_pending_32_threads` | 競合 | 32スレッド |
| T46 | `concurrent_read_during_pending_64_threads` | 競合 | 64スレッド (上限) |
| T47 | `concurrent_commit_same_op_id` | 競合 | 同一 op_id の並行コミット競合 |
| T48 | `concurrent_commit_different_op_ids` | 競合 | 異なる op_id の並行コミット |
| T49 | `concurrent_repair_and_search` | 競合 | 修復中と検索の並行実行 |
| T50 | `concurrent_error_injection_and_recovery` | 競合 | エラー注入とリカバリの並行実行 |

### 観測テスト (OTS)

#### OTS-1: P_taint 一貫性遮断曲線

`Pending { phase: CommitPhase::MetaPrepared }` に拘束されたアセットに対し、1〜64 スレッドの並行サーチ要求を 10^4 件注入。不完全アセットがセマンティック候補セットに混入した確率 `P_taint` をスレッド数ごとに計測し、全スレッド数で `P_taint = 0.00000` が維持されることを確認する。

- サンプルサイズ: スレッド数あたり 10,000 クエリ
- シード: `StdRng::seed_from_u64(TEST_PRNG_SEED)`
- `n = [1, 2, 4, 8, 16, 32, 64]`

#### OTS-2: エラー注入パルス強度に対する不整合生存時間窓 Δτ_unclean

エラーパルス強度（失敗確率 p = 0.1, 0.2, ..., 0.9）に対するストア間の不整合生存時間窓 `Δτ_unclean` の極値統計分布を計測する。

- 各 p に対する測定回数: n=1,000
- 統計量: 平均、中央値、P90、P99、最大値
- 打ち切り時間: 1,000 仮想命令ステップ

#### OTS-3: Repair convergence time

NeedsRepair 状態からの clean state 復帰率、tombstone 収束率、repair convergence time を補助メトリクスとして記録。

- 損傷アンサンブルサイズ: n=10,000
- 修復試行回数上限: 100
- 計測: 復帰率、tombstone 率、quarantine 率、convergence time の分布

#### OTS-4: 状態分布の定常性

ConsistencyState の [Pending → NeedsRepair → {Committed, NeedsRepair, Quarantined}] 遷移分布が吸収マルコフ連鎖として定常状態に収束することを観測する。

- サンプルサイズ: 10,000 初期状態
- 最大遷移ステップ: 1,000
- 観測: 各吸収状態への収束確率、平均吸収時間

## 計装方法・観測対象

### 計装方法

- 全テストは `src/store/coordinator.rs` 内の `#[cfg(test)] mod tests` に実装（DualStoreCoordinator と同じファイル）
- 観測テストは `println!` による構造化テキスト出力を `--nocapture` 経由で標準出力に書き出す
- 並行テストは `std::thread::scope`（スコープ付きスレッド）を使用
- 固定シード: `StdRng::seed_from_u64(TEST_PRNG_SEED)`（`constants.rs` 定義値）
- 時間計測: `std::time::Instant` による仮想時間（実際の wall clock）

### 計測プローブ出力形式

```
=== OTS-1: P_taint Consistency Isolation Curve ===
threads=1, queries=10000, tainted=0, P_taint=0.000000
threads=2, queries=10000, tainted=0, P_taint=0.000000
...
threads=64, queries=10000, tainted=0, P_taint=0.000000
=== 結果: PASS ===

=== OTS-2: Δτ_unclean vs Error Pulse Strength ===
p=0.1, mean=5.23, median=3.00, p90=12.00, p99=45.00, max=128.00, censored=0
p=0.2, mean=8.45, median=5.00, p90=18.00, p99=67.00, max=201.00, censored=0
...
=== 結果: PASS ===

=== OTS-3: Repair Convergence ===
ensemble_size=10000, max_attempts=100
recovery_rate=0.9500, tombstone_rate=0.0300, quarantine_rate=0.0200
mean_convergence_time=4.23, p90=8.00, p99=15.00
=== 結果: PASS ===

=== OTS-4: Absorption Distribution ===
samples=10000, max_steps=1000
committed=0.6700, needs_repair=0.2000, quarantined=0.1300
mean_absorption_steps=7.45
=== 結果: PASS ===
```

### 観測対象

| 観測量 | シンボル | サンプルサイズ | 期待値 |
|--------|---------|---------------|--------|
| 汚染読取確率 | P_taint | 10,000/スレッド数 | 0.00000 |
| 不整合生存時間 | Δτ_unclean | 1,000/確率点 | 有界（最大 1,000 未満） |
| 修復収束時間 | - | 10,000 | 有限ステップ内収束 |
| 修復成功率 | - | 10,000 | >= 90% |
| Tombstone 収束率 | - | 10,000 | < 10% |
| Quarantine 収束率 | - | 10,000 | < 10% |
| Clean state 復帰率 | - | 10,000 | >= 90% |

### 較正計画

本チケットでは新規の較正パラメータを導入しない。較正ループは M1.5-3 以降で実施する。

## Boy Scout Rule — 翻訳可能性計画

本チケットで触る `src/types.rs`（新規型追加）および `src/store/coordinator.rs`（新規ファイル）では以下の翻訳可能性を確保する：

1. **関数名は動詞句**: `commit_dual_store_update`（デュアルストア更新をコミットする）、`exclude_non_committed`（非Committedを除外する）、`is_eligible_for_retrieval`（検索可能か判定する）
2. **変数名はドメイン概念**: `op_id`（操作ID）、`consistency_state`（整合性状態）、`repair_queue`（修復キュー）
3. **一関数一責務**: commit_dual_store_update は「コミット試行」のみ、retrieval_exclusion は「フィルタリング」のみ
4. **ハードコード値の定数化**: エラー注入確率、並行スレッド数上限、タイムアウト値は `constants.rs` に抽出
5. **エラー握りつぶし禁止**: dual-store commit のエラーは全て `DarviumError::DualStoreCommit` または `DarviumError::DualStoreInconsistency` で伝播

既存コードの修正範囲:
- `src/types.rs`: Boy Scout — `enum ConsistencyState` 追加時に既存コードとの整合性を確認。既存の `#[allow(dead_code)]` アトリビュートを削除できる箇所は積極的に削除する
- 既存の Store 関連ファイルは触らない（新しい `src/store/coordinator.rs` からのみ参照）

## Acceptance Criteria

- [ ] 実装要件を満たしている: `ConsistencyState` / `CommitPhase` / `RepairLog` / `RepairAction` 型定義が実装され、`commit_dual_store_update` が RFC §18.2 に従って動作する
- [ ] 全 T1-T50 テストが通過している
- [ ] 全 OTS-1〜OTS-4 観測テストが通過し、P_taint=0.00000 が全スレッド数で確認されている
- [ ] Hard retrieval exclusion: `Pending` / `NeedsRepair` / `Quarantined` の全状態で検索候補露出ゼロ
- [ ] 翻訳可能性の検証が通っている: 関数名・変数名が散文として読めること
- [ ] 既存テストが通過している（`cargo test` で既存失敗なし）

## Notes

### 成果物

- 計画: context/0059-m15-2-consistencystatepending/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0059-m15-2-consistencystatepending/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0059-m15-2-consistencystatepending/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0059-m15-2-consistencystatepending/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
