# 実装計画: M1.5-2 異種ストア論理一貫性コミット（ConsistencyState::Pending）プロトコルのシミュレーション

## RFC 既存実装状態検証

### RFC §18.2 `ConsistencyState`
| バリアント | RFC の型 | 現行コード | 状態 |
|---|---|---|---|
| Committed | unit variant | (未実装) | ❌ 型未定義 |
| Pending { op_id, phase } | struct variant | (未実装) | ❌ 型未定義 |
| NeedsRepair { op_id, reason } | struct variant | (未実装) | ❌ 型未定義 |
| Quarantined { op_id, since } | struct variant | (未実装) | ❌ 型未定義 |

### RFC §18.2 `CommitPhase`
| バリアント | 現行コード | 状態 |
|---|---|---|
| MetaPrepared | (未実装) | ❌ 型未定義 |
| BlobPrepared | (未実装) | ❌ 型未定義 |
| MetaCommitted | (未実装) | ❌ 型未定義 |
| BlobCommitted | (未実装) | ❌ 型未定義 |

### RFC §18.2 `RepairLog`
| フィールド | RFC の型 | 現行コード | 状態 |
|---|---|---|---|
| op_id | String | (未実装) | ❌ 型未定義 |
| graph_id | WorkflowGraphId | (未実装) | ❌ 型未定義 |
| detected_at | SystemTime | (未実装) | ❌ 型未定義 |
| reason | String | (未実装) | ❌ 型未定義 |
| action | RepairAction | (未実装) | ❌ 型未定義 |

### RFC §18.2 `RepairAction`
| バリアント | 現行コード | 状態 |
|---|---|---|
| RetryMetaCommit | (未実装) | ❌ 型未定義 |
| RetryBlobCommit | (未実装) | ❌ 型未定義 |
| MarkQuarantined | (未実装) | ❌ 型未定義 |
| ConvertToTombstone | (未実装) | ❌ 型未定義 |

### DarviumError (既存 — ✅ 一致)
| エラー型 | RFC 相当 | 現行コード | 状態 |
|---|---|---|---|
| DualStoreCommit(String) | CrossStoreInconsistency 相当 | src/error.rs:73 | ✅ 一致 |
| DualStoreInconsistency(String) | 同左 | src/error.rs:77 | ✅ 一致 |

**評価サマリ**: 5 つの型が完全に未実装。エラー型は既に定義済みで利用可能。

## 要件の再確認

1. **型定義**: `ConsistencyState`, `CommitPhase`, `RepairLog`, `RepairAction`, `ConsistencyStateTag` を `src/types.rs` に追加
2. **`DualStoreCoordinator`**: GraphStore + MetadataStore を保持し、`commit_dual_store_update` を提供する Facade
3. **Hard retrieval exclusion**: `ConsistencyState != Committed` のアセットを検索候補から除外するフィルタ関数
4. **Repair キューイング**: `NeedsRepair` 状態への遷移 + 修復キューへの追加
5. **Clean state 復帰**: Repair 後の Committed / Tombstone への復帰
6. **エラー注入**: FailingStore ラッパーでタイムアウト・I/O エラーをシミュレート
7. **テスト**: T1-T50 (50 テスト) + OTS-1〜OTS-4 (4 観測テスト)

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/types.rs` | 修正 | `ConsistencyState`, `CommitPhase`, `RepairLog`, `RepairAction` を public で追加 |
| `src/constants.rs` | 修正 | 追加: `DUAL_STORE_MAX_RETRY`, `DUAL_STORE_ERROR_INJECTION_SEED` |
| `src/store/coordinator.rs` | **新規** | `DualStoreCoordinator`, `commit_dual_store_update`, `FailingGraphStore`, `FailingMetadataStore`, 全テストコード |
| `src/store/mod.rs` | 修正 | `mod coordinator;` + `pub use` 追加 |
| `src/lib.rs` | 修正 | 新規公開型の再エクスポート追加 |

## 計装・観測の実装計画

### 実装するテストコード

全テストは `src/store/coordinator.rs` 内の `#[cfg(test)] mod tests` に実装する。

- **T1-T10** (10 tests): ConsistencyState / CommitPhase / RepairLog / RepairAction / ConsistencyStateTag の型構築・メモリサイズ・Debug・PartialEq・Clone 検証（`src/types.rs` の型が正しいことを確認）
- **T11-T20** (10 tests): `DualStoreCoordinator::commit_dual_store_update` の正常系・異常系（両成功、片側失敗、両失敗、prepare phase、repair log、冪等性）
- **T21-T30** (10 tests): `DualStoreCoordinator::filter_retrieval_eligible` による hard exclusion gate（Pending/NeedsRepair/Quarantined → 除外、Committed → 許可、audit mode）
- **T31-T40** (10 tests): `FailingGraphStore` / `FailingMetadataStore` を用いたエラー注入（timeout、I/O error、burst 5 failures、recovery after repair、quarantine route）
- **T41-T50** (10 tests): `std::thread::scope` による並行アクセス競合状態テスト（2/4/8/16/32/64 スレッド、同一/異種 op_id、repair + search 並行）

### 観測テスト

| ID | 観測量 | サンプルサイズ | 実装場所 |
|---|---|---|---|
| OTS-1 | P_taint 一貫性遮断曲線 (1〜64 スレッド) | 10,000 クエリ/スレッド数 | coordinator.rs tests |
| OTS-2 | Δτ_unclean vs エラーパルス強度 (p=0.1-0.9) | 1,000/確率点 | coordinator.rs tests |
| OTS-3 | Repair convergence time (復帰率・tombstone 率) | 10,000 初期状態 | coordinator.rs tests |
| OTS-4 | 状態分布の定常性 (吸収マルコフ連鎖) | 10,000 初期状態 | coordinator.rs tests |

### 観測出力

- `println!` による構造化テキスト出力、`cargo test -- --nocapture` で取得
- 固定シード: `StdRng::seed_from_u64(TEST_PRNG_SEED)`

### 較正計画

本チケットでは新規較正パラメータを導入しない。定数追加は最小限（エラー注入関連のみ）。
較正ループは M1.5-3 以降で実施。

## Boy Scout 改善（スコープ外の翻訳可能性修正）

1. `src/types.rs`: `#[allow(dead_code)]` アトリビュートが小規模に分散 — 新規型追加のタイミングで、周辺のデッドコードアノテーションの必要性を確認し、真に不要なもののみ削除する
2. 特に問題のある箇所は見当たらない（既存コードは翻訳可能性が良好に保たれている）

## 実装手順

### Step 1: 型定義追加 (`src/types.rs`)

`ConsistencyState`, `CommitPhase`, `RepairLog`, `RepairAction` を RFC §18.2 の定義に従って追加。

- `ConsistencyState`: `#[derive(Debug, Clone, PartialEq)]`, serde 対応
- `CommitPhase`: `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`, serde 対応
- `RepairLog`: `#[derive(Debug, Clone, PartialEq)]`
- `RepairAction`: `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
- 配置場所: `src/types.rs` の末尾（既存コードを乱さない）

### Step 2: 定数追加 (`src/constants.rs`)

```rust
/// デュアルストア コミット最大再試行回数 (Safety Invariant)
pub const DUAL_STORE_MAX_RETRY: u32 = 3;

/// デュアルストア エラー注入テスト用シード (Calibration Candidate)
pub const DUAL_STORE_ERROR_INJECTION_SEED: u64 = 67890;
```

### Step 3: `DualStoreCoordinator` 実装 (`src/store/coordinator.rs` 新規)

```rust
pub struct DualStoreCoordinator {
    graph_store: Box<dyn GraphStore>,
    metadata_store: Box<dyn MetadataStore>,
    repair_queue: RefCell<Vec<RepairLog>>,
    consistency_states: RefCell<HashMap<String, ConsistencyState>>, // asset_id -> state
}

impl DualStoreCoordinator {
    pub fn new(graph_store: Box<dyn GraphStore>, metadata_store: Box<dyn MetadataStore>) -> Self;
    pub fn commit_dual_store_update(&self, asset_id: &str, op_id: &str) -> Result<(), DarviumError>;
    pub fn filter_retrieval_eligible(&self, candidates: Vec<RankedCandidate>) -> Vec<RankedCandidate>;
    pub fn is_eligible_for_retrieval(&self, asset_id: &str) -> bool;
    pub fn get_consistency_state(&self, asset_id: &str) -> Option<ConsistencyState>;
    pub fn apply_repair(&self, asset_id: &str) -> Result<(), DarviumError>;
    pub fn list_repair_queue(&self) -> Vec<RepairLog>;
    pub fn enqueue_repair(&self, op_id: &str, graph_id: &str, reason: &str) -> RepairLog;
}
```

### Step 4: モジュール公開 (`src/store/mod.rs`)

```rust
mod coordinator;
pub use coordinator::DualStoreCoordinator;
```

### Step 5: テスト実装

全 T1-T50 + OTS-1〜OTS-4 を `coordinator.rs` の `#[cfg(test)] mod tests` に実装。

### Step 6: lib.rs エクスポート (`src/lib.rs`)

```rust
pub use store::DualStoreCoordinator;
pub use types::{ConsistencyState, CommitPhase, RepairAction, RepairLog};
```

## 物理的レビュー方法

### 自動チェック

```bash
# コンパイル確認
cargo check

# 全テスト実行（観測出力含む）
cargo test -- --nocapture

# クリッピー
cargo clippy -- -D warnings

# フォーマット
cargo fmt --check
```

### 手動チェック項目

1. **翻訳可能性チェック**: 全ての新規関数が動詞句で始まっていること。変数名がドメイン概念を表していること。一関数一責務が守られていること。
2. **RFC 無矛盾確認**: 実装後に RFC §18.2 の該当コードブロックと実装を 1 フィールド単位で比較する。
3. **テスト完全性**: T1〜T50 の全テストと OTS-1〜OTS-4 の観測テストが実装されていること。
4. **P_taint = 0.00000**: OTS-1 の出力で全スレッド数の汚染確率がゼロであることを目視確認。
5. **既存テスト非破壊**: 既存の全テストが通過していること。

## リスク

| リスク | 影響 | 対策 |
|--------|------|------|
| 並行テストでの競合（`RefCell` 制限） | 中 | `std::thread::scope` + 独立インスタンスで回避。Mutex 化は今回のスコープ外 |
| RFC §18.2 の RepairAction が複数バリアント | 低 | 全4バリアントを網羅的に実装 |
| テスト数50件 + 4観測テストで実行時間増加 | 低 | OTS は n=10^4 でも計算量が軽量なため許容範囲内 |
