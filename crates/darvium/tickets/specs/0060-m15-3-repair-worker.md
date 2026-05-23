---
ticket_id: 60
title: M1.5-3: 起動時修復スキャン（Repair Worker）によるクラッシュリカバリの決定論的テスト
slug: m15-3-repair-worker
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0060-m15-3-repair-worker/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0060-m15-3-repair-worker/observation-20260523-201926.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0060-m15-3-repair-worker/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0060-m15-3-repair-worker/review.md
---
# M1.5-3: 起動時修復スキャン（Repair Worker）によるクラッシュリカバリの決定論的テスト

## Summary

`DualStoreCoordinator` に `startup_repair_scan()` メソッドを実装し、システム再起動時に `Pending` / `NeedsRepair` 状態の全資産を走査して修復（または隔離）する。修復途中状態が retrieval selection path に露出しない hard exclusion を維持しつつ、有限ステップ内で全資産を `Committed` / `Tombstone` / `Quarantined` の安全状態に収束させる。10,000件損傷アンサンブルによる観測テストで指数減衰軌道 ln ||E(t)|| ~ -Γt を確認する。

## Background

M1.5-2 で実装した `ConsistencyState` と `DualStoreCoordinator::commit_dual_store_update()` は、片側成功状態（一方のストアのみ書き込み成功）が発生した場合に資産を `NeedsRepair` 状態に遷移させる。しかし、システムがそのままクラッシュした場合、次回起動時にこれらの不整合状態を検出し修復するメカニズムが欠落している。

RFC §18.2 & §25.x は「起動時修復スキャンにより、片側成功状態の放置を避けること」を規範とし、dual-store の壊れた状態を selection path から隔離し安全状態へ収束させることを要求する。v2.3 では startup repair scan は中核規律であり、片側成功状態の黙過を許さない。

**参照観察レポート:**
- `tickets/context/0059-m15-2-consistencystatepending/observation-20260523-195202.md` — M1.5-2 観測結果。OTS-4 で全資産が Committed/Quarantined に吸収されることを確認済み。NeedsRepair 残留は原理的にゼロ。示唆: M1.5-3 では FailingStore エラー確率を較正パラメータとし、共有 RefCell → Mutex/Arc 対応は M1.5-3 以降に先送り。

## Scope

1. **`startup_repair_scan()` メソッド**を `DualStoreCoordinator` に追加する
   - `consistency_states` を全走査し、`Pending` / `NeedsRepair` 状態の資産を特定する
   - 各不整合資産に対して修復を試行（`apply_repair` の再利用）
   - 修復失敗時は `Quarantined` へ遷移（`ConvertToTombstone` は将来拡張として今は `MarkQuarantined` で統一）
   - 修復処理中も `is_eligible_for_retrieval()` による hard exclusion を維持する
   - 戻り値: `RepairScanSummary { total, repaired, quarantined, failed, duration_ms }` 構造体

2. **不変条件テスト** (T1-T10):
   - 正常系: 全資産 Committed → スキャン不要、空サマリを返す
   - 全資産 Pending → 全件修復成功 (Committed)
   - 全資産 NeedsRepair → 全件修復成功 (Committed)
   - 混合状態: Committed 5 + Pending 3 + NeedsRepair 2 → 全5件修復成功、残りスルー
   - 空ストア → 0件サマリ
   - Pending 全資産 + 修復不可能ストア → 全件 Quarantined

3. **修復除外ゲートテスト** (T11-T15):
   - スキャン中の全資産 `is_eligible_for_retrieval()` が `false` を返す（Pending/NeedsRepair は元々 false）
   - スキャン完了後、`Committed` に戻った資産は `true`、`Quarantined` は `false`
   - `filter_retrieval_eligible()` との統合確認
   - スキャン前後で eligible 資産が増加したこと（修復により）を確認（増加しないケース含む）
   - スキャン中に新たに追加された資産の状態不変性

4. **観測テスト (OTS-1〜OTS-3)**:
   - OTS-1: 10,000件損傷アンサンブル修復収束率測定（平均修復成功率 >= 99%、エラーなしストア前提）
   - OTS-2: 修復減衰曲線 ln ||E(t)|| ~ -Γt（FailingStore 30% エラー率、Γを推定し Γ > 0 を確認）
   - OTS-3: 吸収状態分布 - `Committed` + `Quarantined` の合計が初期損傷数と一致、残留不整合ゼロ

5. **`src/constants.rs` への定数追加:**
   - `REPAIR_SCAN_MAX_RETRY: u32 = 3` — 修復スキャン1資産あたりの最大再試行回数（Safety Invariant）
   - `REPAIR_SCAN_BATCH_SIZE: usize = 100` — スキャンのバッチサイズ（Calibration Candidate）
   - `TEST_PRNG_SEED: u64 = 12345` — 既存、テスト固定シード

## Non-scope

- **共有 RefCell → Mutex/Arc 変換**: 真の並行アクセス対応は将来チケットに委ねる（M1.5-2 観測レポート参照）。本チケットでは独立インスタンスベースの `DualStoreCoordinator` を前提とする
- **`ConvertToTombstone` パス**: 本チケットでは `MarkQuarantined` のみを使用し、`ConvertToTombstone` による完全削除は将来スコープとする
- **MetadataStore 永続層との統合**: 本チケットは `InMemoryMetadataStore` / `InMemoryGraphStore` 上のメモリ内動作を検証する。SQLite/lmdb 永続層との結合は M4 以降のスコープ
- **Repair Worker の独立スレッド化**: 起動時一回きりの同期的スキャンとして実装し、バックグラウンド常駐ワーカースレッドは実装しない
- **起動時修復スキャンの進捗レポート・ロギング**: `RepairScanSummary` のみ返し、詳細イベントログは本チケットのスコープ外（将来の観測パイプラインで対応）

## Investigation

### ソースコード解析結果（物理的証拠）

1. **`src/types.rs` (L496-605):** `ConsistencyState` enum は `Committed` / `Pending { op_id, phase }` / `NeedsRepair { op_id, reason }` / `Quarantined { op_id, since }` の4バリアント。`is_eligible_for_retrieval()` は `Committed` のみ `true` を返す。`ConsistencyStateTag` も同様の4バリアントでフィルタリング用。`RepairAction` は `RetryMetaCommit` / `RetryBlobCommit` / `MarkQuarantined` / `ConvertToTombstone` の4種。

2. **`src/store/coordinator.rs` (L23-512):** `DualStoreCoordinator` は `consistency_states: RefCell<HashMap<String, ConsistencyState>>` と `repair_queue: RefCell<Vec<RepairLog>>` を保持。`commit_dual_store_update()` で Pending → BlobPrepared → Committed のプロトコルを実装。`apply_repair()` で NeedsRepair → 再書き込み試行 → Committed または Quarantined への遷移を実装。`is_eligible_for_retrieval()` / `filter_retrieval_eligible()` で検索除外ゲートを提供。

3. **`startup_repair_scan()` は未実装（grep でヒットなし）：** `grep -r "startup_repair" src/` で該当関数なし。新規実装が必要。

4. **`src/recovery.rs` (L1-552):** HITL 起動時回復ループ `recover_pending_interactions()` は MetadataStore 上の Pending インタラクション（人間フィードバック）の回復を担当。dual-store 整合性とは無関係。パターンとしては参考になる（`RecoverySummary` 構造体、store 走査パターン）。

5. **`src/constants.rs`:** `REPAIR_SCAN_*` 系定数は存在しない。`DUAL_STORE_MAX_RETRY=3` は既存の Safety Invariant。

6. **`tests/` ディレクトリ:** M1.5-3 専用の統合テストファイルは未作成。既存の coordinator.rs `mod tests` 内に T1-T50 が存在。

### 過去観測レポートからの示唆（M1.5-2）

- OTS-4 で `NeedsRepair` 残留ゼロが確認済み（修復成功 or Quarantined への完全吸収）
- `FailingStore` エラー確率は本チケットで較正パラメータとして導入すべき
- 共有 RefCell の Mutex/Arc 化は本チケットでは不要（独立インスタンス設計）

### 実装方針

`DualStoreCoordinator` に以下のメソッドを追加する:

```rust
pub fn startup_repair_scan(&self) -> RepairScanSummary;
```

内部ロジック:
1. `consistency_states` を走査
2. `Pending` または `NeedsRepair` の資産を収集
3. 各資産に対して `apply_repair()` を呼び出し (最大 REPAIR_SCAN_MAX_RETRY 回)
4. 成功 → Committed、失敗 → `set_consistency_state(asset_id, ConsistencyState::Quarantined {...})`
5. 修復中も `is_eligible_for_retrieval()` は false を返し続ける（不変条件を変更しない）

戻り値 `RepairScanSummary`:

```rust
pub struct RepairScanSummary {
    pub total_scanned: usize,       // 走査した全資産数
    pub found_inconsistent: usize,  // 不整合状態の資産数
    pub repaired: usize,            // 修復成功 (→Committed)
    pub quarantined: usize,         // 隔離 (→Quarantined)
    pub already_clean: usize,       // 最初から Committed
    pub duration_ms: u64,           // スキャン所要時間
}
```

## Test Plan

### 不変条件テスト（T1-T10、coordinator.rs mod tests 内）

| ID | 名称 | 入力 | 期待結果 |
|----|------|------|----------|
| T1 | 全資産 Committed → スキャン不要 | 5資産すべて Committed | total=5, found=0, repaired=0 |
| T2 | 全資産 Pending → 全件修復成功 | 5資産すべて Pending | total=5, found=5, repaired=5 |
| T3 | 全資産 NeedsRepair → 全件修復成功 | 5資産すべて NeedsRepair | total=5, found=5, repaired=5 |
| T4 | 混合状態スキャン | Committed 5 + Pending 3 + NeedsRepair 2 | total=10, found=5, repaired=5 |
| T5 | 空ストア | 資産なし | total=0, found=0 |
| T6 | 全件 Pending & 修復不可能ストア | 5資産 Pending + FailingStore 100% | total=5, found=5, repaired=0, quarantined=5 |
| T7 | 全件 NeedsRepair & 修復不可能ストア | 5資産 NeedsRepair + FailingStore 100% | total=5, found=5, repaired=0, quarantined=5 |
| T8 | 混合 & 部分修復不可能 | Committed 3 + Pending 3(内2修復失敗) | repaired=1, quarantined=2 |
| T9 | スキャン2回実行（冪等性） | 同じストアで2回連続スキャン | 2回目は repaired=0 |
| T10 | 大規模 N=1000 | 1000資産（Committed 700 + Pending 200 + NeedsRepair 100） | repaired=300 |

### 修復除外ゲートテスト（T11-T15）

| ID | 名称 | 期待結果 |
|----|------|----------|
| T11 | スキャン中の eligible 常時 false | Pending/NeedsRepair 資産がスキャン中も一貫して false |
| T12 | スキャン後 Committed は true に復帰 | 修復成功資産の eligible が true に戻る |
| T13 | スキャン後 Quarantined は false 維持 | 隔離資産の eligible が false のまま |
| T14 | filter_retrieval_eligible 統合 | スキャン後フィルタリングで eligible のみ抽出される |
| T15 | スキャン前後で eligible 数が非減少 | 修復により eligible 数が減少しない（維持または増加） |

### 観測テスト（OTS-1〜OTS-3、固定シード StdRng::seed_from_u64(12345)）

| ID | 名称 | サンプルサイズ | 測定内容 |
|----|------|---------------|----------|
| OTS-1 | 10,000件損傷アンサンブル修復成功率 | N=10,000（損傷率50%） | 修復成功率 >= 99%（エラーなきストア）、平均修復時間 |
| OTS-2 | 修復減衰曲線 ln||E(t)|| vs ステップ | N=10,000（FailingStore 30%） | 指数減衰 Γ > 0 の確認、減衰定数推定 |
| OTS-3 | 吸収状態分布 | N=10,000（損傷率50%、エラー率30%） | Committed + Quarantined = 初期損傷数、残留不整合 = 0 |

## 計装方法・観測対象

### 計装方法
- テストコード: `src/store/coordinator.rs` の既存 `mod tests` ブロックに追記
- 観測データ出力: `println!` + `--nocapture` 経由。JSON/CSV 形式で構造化出力
- 固定シード: `StdRng::seed_from_u64(TEST_PRNG_SEED)`（全テストで再現性保証）
- 修復スキャンのステップカウントは `for` ループ内でインクリメントし、各ステップ後の残存不整合数を計測

### 観測対象

**OTS-1: 損傷アンサンブル修復成功率**
- 総数 N=10,000、損傷確率 50%（5,000件 Pending + 5,000件 NeedsRepair）
- 測定: repaired 件数、quarantined 件数、修復成功率 = repaired / found_inconsistent
- 期待: 修復成功率 >= 99%（エラーなしストア前提）

**OTS-2: 修復減衰曲線 ln||E(t)||**
- N=10,000、FailingGraphStore 30% エラー率
- 測定: 各ステップ t での不整合残存数 ||E(t)||、ln||E(t)|| vs t の片対数プロット
- 期待: ln ||E(t)|| ~ -Γt（Γ > 0）、線形回帰により Γ を推定

**OTS-3: 吸収状態分布**
- N=10,000、損傷率 50%、FailingStore エラー率 30%
- 測定: 最終状態分布（Committed / Quarantined の件数）
- 期待: 「修復成功件数 + 隔離件数 = 初期損傷件数」、残留 NeedsRepair/Pending = 0

### 較正計画

- **較正対象定数**: `REPAIR_SCAN_MAX_RETRY`（初期値 3、範囲 1-10）
- **目的関数 J(θ)**: `J = w1 * (1 - repair_rate) + w2 * quarantine_rate + w3 * scan_time_norm` — 修復率最大化、クオランティン最小化、スキャン時間最小化のトレードオフ評価
- **停止条件**: 3回連続で J(θ) の改善が 1% 未満、またはパラメータ範囲を全探索完了

## Boy Scout Rule — 翻訳可能性計画

1. **`startup_repair_scan` 関数名は動詞句として適切**: 「起動時修復スキャンを実行する」と日本語訳可能。変更不要。

2. **`RepairScanSummary` 構造体**: フィールド名はすべてドメイン概念を表す（`total_scanned`, `found_inconsistent`, `repaired`, `quarantined`, `already_clean`, `duration_ms`）。変更不要。

3. **責務分割**: 修復スキャンは単一責務（不整合状態の検出と収束）。`apply_repair` や `set_consistency_state` は既存メソッドを再利用し、重複ロジックを作らない。

4. **定数抽出**: `REPAIR_SCAN_MAX_RETRY`（ハードコードを避けて定数化）、`REPAIR_SCAN_BATCH_SIZE`（同上）。

5. **既存コード改善**: coordinator.rs 内の `list_repair_queue` の戻り値が `Vec<RepairLog>` で実体コピーが発生しているが、本チケットでは変更しない（public API の破壊を避ける）。

## Acceptance Criteria

- [ ] `DualStoreCoordinator::startup_repair_scan()` が実装され、全不整合状態を走査・修復・隔離する
- [ ] T1-T10（不変条件テスト）が全てパスする
- [ ] T11-T15（修復除外ゲートテスト）が全てパスする
- [ ] OTS-1（10,000件アンサンブル修復成功率）が出力通りパスする
- [ ] OTS-2（修復減衰曲線 ln||E(t)|| vs ステップ、Γ > 0 確認）がパスする
- [ ] OTS-3（吸収状態分布、残留不整合ゼロ確認）がパスする
- [ ] `REPAIR_SCAN_MAX_RETRY` 定数が constants.rs に追加され、適切にコメントが付与されている
- [ ] 修復途中状態が retrieval selection path に露出しない（hard exclusion 維持）
- [ ] 既存テスト（M1.5-2 までの全テスト）が影響を受けずにパスする
- [ ] `RepairScanSummary` の戻り値構造体が定義され、全フィールドが測定されている

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0060-m15-3-repair-worker/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0060-m15-3-repair-worker/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0060-m15-3-repair-worker/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0060-m15-3-repair-worker/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
