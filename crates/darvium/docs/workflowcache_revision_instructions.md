# WorkflowCache 安全解放のための改訂指示書

本指示書は、添付された 3 ファイルに対して、WorkflowCache を「Preset は絶対に捨てない」「それ以外は周期的・TTL 的・ResourcePressure ベースで安全に解放できる」ように仕様補完するための改訂指示をまとめたものです。[file:2][file:3][file:4]

## 改訂方針

今回の補完方針は次の 4 点です。[file:2]

- WorkflowCache は source-of-truth ではなく、Repository Pair 上の MemoizedGraph 群に対する runtime cache / in-memory working set であることを明示したまま、eviction を first-class に規範化する。[file:2]
- Preset 由来の資産、特に `GcState::Protected` および root-pinned preset は WorkflowCache から eviction してはならない、というガードを明文化する。[file:2][file:3]
- Preset 以外は、周期タスク、TTL、ResourcePressure、GcState 遷移イベントに応じて cache から除去してよいようにする。[file:2]
- eviction により WorkflowCache から消えても、`ConsistencyState::Committed` で Repository Pair に存在する限り `get_or_load` により再ロード可能でなければならない、という再ロード不変条件を規範化する。[file:2]

## 1. RFC 本文の改訂指示

### 対象ファイル

- `Darvium-RFC-0001-Unified-v2.3-final.md` [file:2]

### 1-1. §2 用語集に追加する項目

`WorkflowCache` と `Repository Pair` の定義が既にあるため、その直後または近傍に次の用語を追加してください。[file:2]

- `Cache Residency`: WorkflowCache に保持されている状態。永続化状態ではなく runtime residency を指す。[file:2]
- `Cache Eviction`: WorkflowCache から MemoizedGraph を in-memory で除去する操作。Repository Pair 上の canonical persistence を削除する意味を持たない。[file:2]
- `Cache TTL Policy`: `Provenance.last_used_at` と `last_virtual_seen` に基づいて eviction 候補化するポリシー。[file:2]
- `Pinned Cache Entry`: `GcState::Protected` または preset root policy により eviction 禁止となる cache entry。[file:2][file:3]
- `Cache Pressure State`: `ResourcePressure` と `EnvironmentPolicy.pressuremode` から導出される cache-side eviction aggressiveness 状態。[file:2]

### 1-2. §3 スコープに追加する項目

#### §3.1 In-Scope に追加

以下を追加してください。[file:2]

- WorkflowCache eviction policy, residency control, TTL, periodic cleanup, and preset-safe retention rules.[file:2]
- Event-driven cache invalidation / eviction on GcState transitions and repository repair state changes.[file:2]

#### §3.2 Out-of-Scope に追加

以下を追加してください。[file:2]

- OS-level memory reclamation strategy itself, allocator tuning, and kernel-specific page cache behavior are out of scope; this RFC only specifies application-level WorkflowCache eviction semantics.[file:2]

### 1-3. §4 設計上の前提と制約に追加する規範

P-16 の後に、少なくとも以下の制約を新設してください。[file:2]

- `P-17`: WorkflowCache は source-of-truth ではなく揮発 cache であり、WorkflowCache からの eviction は Repository Pair 上の canonical persistence を変更してはならない (MUST NOT)。[file:2]
- `P-18`: `GcState::Protected` の MemoizedGraph、および `ArtifactOriginKind::PresetSystem` または `PresetRootPolicy::RootPinned | RootAncestorPinned` に該当する preset-derived graph は WorkflowCache eviction 対象にしてはならない (MUST NOT)。[file:2][file:3]
- `P-19`: `GcState::Tombstoned` の graph は WorkflowCache に残存してはならない (MUST NOT)。[file:2]
- `P-20`: 実装は、WorkflowCache に対して periodic eviction もしくは capacity-bound eviction の少なくとも一方を実装しなければならない (MUST)。[file:2]
- `P-21`: `ConsistencyState != Committed` の graph は normal retrieval hot set から除外しなければならず、eviction 候補選定においては保守的に扱わなければならない (MUST)。[file:2][file:4]

### 1-4. §5 4 層アーキテクチャ概観の説明文修正

Layer 3a の説明に、WorkflowCache の責務として次の一文を追加してください。[file:2]

- WorkflowCache is a volatile in-memory acceleration layer with explicit residency and eviction policy; Repository Pair remains the sole canonical persistence authority.[file:2]

### 1-5. §8 WorkflowCache と MemoizedGraph に新設・追記すべき内容

ここが本改訂の中心です。[file:2]

#### A. §8.0 または §8 直下に「Cache Residency and Eviction Semantics」を新設

この新節では、次を明文化してください。[file:2]

- WorkflowCache は lazy load で増えるが、unbounded growth を許可するものではない。[file:2]
- WorkflowCache entry は residency object であり、Repository Pair 上の `Committed` graph から再構築可能である。[file:2]
- eviction は graph deletion ではなく in-memory dereference である。[file:2]
- cache miss 後の `get_or_load` は Repository Pair から再読込して再常駐化してよい。[file:2]

#### B. §8 の `WorkflowCache` struct 定義に追記

現状の `workingset`, `annhint`, `policy` に加え、以下のようなメタを追加するよう指示してください。[file:2]

- `max_entries: usize`
- `max_bytes: usize`
- `default_ttl_human: Duration`
- `default_ttl_virtual: u64`
- `eviction_interval: Duration`
- `pinned_ids: HashSet<WorkflowGraphId>` または pinned 判定関数
- `residency_meta: HashMap<WorkflowGraphId, CacheResidencyMeta>`

併せて `CacheResidencyMeta` を新設してください。[file:2]

- `loaded_at: SystemTime`
- `last_cache_hit_at: SystemTime`
- `last_cache_hit_vt: u64`
- `estimated_bytes: usize`
- `eviction_exempt: bool`
- `eviction_reason_last: Option<String>`

#### C. `CachePolicy` enum の拡張

既存の `Default / Pinned / Preload` だけでは eviction semantics が弱いため、次のいずれかの形に更新してください。[file:2]

- 既存 enum を維持しつつ別 struct `EvictionPolicy` を追加する、または
- `CachePolicy` を residency / preload / ttl / capacity を含む richer policy object に置き換える。[file:2]

最低限必要な policy 項目は次です。[file:2]

- `protect_presets: bool` (デフォルト true)
- `enable_periodic_eviction: bool`
- `enable_ttl_eviction: bool`
- `evict_on_pressure: bool`
- `max_entries`
- `max_bytes`
- `ttl_human`
- `ttl_virtual`
- `pressure_mode_overrides`

#### D. §8.1 Provenance / VirtualClock に追記

既存の `lastusedat` と `lastvirtualseen` があるので、次の規範を追加してください。[file:2]

- cache hit 時には `Provenance.last_used_at` に加えて cache residency metadata 側の `last_cache_hit_at` / `last_cache_hit_vt` を更新する。[file:2]
- TTL 判定には `last_used_at` と `last_virtual_seen` を使用してよいが、preset-protected entry には適用してはならない (MUST NOT)。[file:2]

#### E. §8.4 GraphVersion / get_or_load 周辺に追記

`get_or_load` 擬似コードの直後に、以下の挙動を追記してください。[file:2]

- `get_or_load` 前に capacity guard を評価し、必要なら eviction pass を実行する。[file:2]
- `get_or_load` 後に新規 entry を追加する際、preset-safe guard を壊さずに `max_entries` / `max_bytes` を超えないことを確認する。[file:2]
- 超過時に非 protected entry を十分に落とせない場合は `CacheError::CapacityExceeded` または同等エラーを返す。[file:2]

`CacheError` には少なくとも次を追加してください。[file:2][file:3]

- `CapacityExceeded { max_entries: usize, max_bytes: usize }`
- `ProtectedEvictionForbidden(WorkflowGraphId)`
- `EvictionInvariantViolation(String)`

#### F. §8 に eviction API を新設

次の API 群を新設するよう指示してください。[file:2]

- `evict_one(graphid, reason) -> Result<(), CacheError>`
- `evict_expired(now, current_vt) -> EvictionReport`
- `evict_for_pressure(pressure, env) -> EvictionReport`
- `evict_to_capacity() -> EvictionReport`
- `handle_gc_state_transition(graphid, old, new) -> Result<(), CacheError>`
- `is_eviction_protected(graph: &MemoizedGraph) -> bool`

### 1-6. §12C / §12E Event Architecture に追加

GcState 遷移と cache 解放をイベント駆動にしたいので、Event Architecture に次を追記してください。[file:2]

- `DarviumEventKind::GcEvent` の payload 例として `GraphGcStateChanged { graphid, old_state, new_state, reason }` を明記する。[file:2]
- WorkflowCache は `GcEvent` を subscribe し、`SoftDeleted`, `HardDeleteCandidate`, `Tombstoned` 遷移を受信した場合、preset-protected でない限り速やかに cache eviction を試みなければならない (MUST)。[file:2]
- `Tombstoned` 遷移時は cache から除去完了までを invariant として扱う。[file:2]

### 1-7. §15 Lifecycle / GC に追記

#### A. §15.1 または §15.6 に cache-side 連動規範を追加

以下を追加してください。[file:2]

- GcState は persistence lifecycle だけでなく cache residency eligibility にも影響する。[file:2]
- `Protected` は cache eviction complete exclusion とする。[file:2]
- `SoftDeleted` / `HardDeleteCandidate` / `Tombstoned` は cache residency を縮退方向にしか遷移させてはならない。[file:2]

#### B. §15.8 Resource Pressure に具体化を追加

既存の `ResourcePressure` に cache memory / resident entries / ANN hot index bytes を結びつけてください。[file:2]

- `ResourcePressure` 観測値に `workflowcache_resident_entries`, `workflowcache_estimated_bytes`, `ann_hot_index_bytes` を加える。[file:2]
- `PressureMode::Constrained` 以上では periodic cache eviction を推奨ではなく運用上の標準動作として記述する。[file:2]
- `PressureMode::Emergency` では protected 以外の TTL 失効 entry と low-value entry の eviction を即時実行する SHOULD を入れる。[file:2]

#### C. Grace Period との整合補足

`experiencecount < MIN_SURVIVAL_EXPERIENCE` は persistence GC 保護であって、cache residency 永久保証ではないことを明記してください。[file:2]

- ただし grace-period entry は `SoftDeleted` や `HardDeleteCandidate` へは遷移しない一方、cache memory pressure 時の eviction 候補から完全除外する必要はない、と明文化する。[file:2]

### 1-8. §18 エラーハンドリングに追記

eviction 関連エラーと挙動を追加してください。[file:2]

- protected graph の eviction 要求は hard error。[file:2]
- Tombstoned graph が cache に残っていた場合は invariant violation とし、警告ではなく repair / panic policy の対象にするかを明文化する。[file:2]
- cache eviction failure 自体は persistence corruption ではないが、capacity guard failure により search path を degrade / abort してよいことを明記する。[file:2]

### 1-9. §19 性能目標に追加

性能目標に cache residency 指標を追加してください。[file:2]

- cache hit rate
- median reload latency
- eviction count per hour
- protected-entry eviction attempts = 0
- tombstoned-entry residency duration p95 = 0 or near-zero
- pressure-triggered eviction completion latency

### 1-10. §20 マイルストーン / Tickets 参照補足

v2.3-j の次改訂または次マイルストーンに、WorkflowCache eviction semantics 実装を独立タスクとして追加するよう明記してください。[file:2][file:3]

### 1-11. §22 付録 A 定数一覧に追加

少なくとも次の定数群を追加するよう指示してください。[file:2]

- `WORKFLOWCACHE_MAX_ENTRIES`
- `WORKFLOWCACHE_MAX_BYTES`
- `WORKFLOWCACHE_TTL_HUMAN_MS`
- `WORKFLOWCACHE_TTL_VIRTUAL_TICKS`
- `WORKFLOWCACHE_EVICTION_INTERVAL_MS`
- `WORKFLOWCACHE_PRESSURE_HIGH_WATERMARK`
- `WORKFLOWCACHE_PRESSURE_EMERGENCY_WATERMARK`
- `WORKFLOWCACHE_PROTECTED_EVICTION_ALLOWED = false` (safety invariant)

## 2. Tickets ファイルの改訂指示

### 対象ファイル

- `Darvium-Tickets-v2.3-2.md` [file:3]

このファイルには既存の M-0.5 / M1.5 / M1.75 系が並んでいるため、WorkflowCache eviction を独立 ticket 群として追加するのが良いです。[file:3]

### 2-1. 新規 ticket セットを追加

`M-0.5-7` が WorkflowCache / RepositoryPair 周辺をすでに扱っているため、その直後または同章に派生 ticket を追加してください。[file:3]

推奨 ticket 名:

- `M-0.5-7-E1 WorkflowCache protected eviction guard`
- `M-0.5-7-E2 WorkflowCache periodic eviction worker`
- `M-0.5-7-E3 WorkflowCache TTL eviction semantics`
- `M-0.5-7-E4 WorkflowCache pressure-driven eviction`
- `M-0.5-7-E5 WorkflowCache GcEvent-driven eviction`
- `M-0.5-7-E6 WorkflowCache eviction invariants and tests`

### 2-2. 各 ticket に入れるべき要件

#### E1 Protected eviction guard

- `GcState::Protected`, `ArtifactOriginKind::PresetSystem`, `PresetRootPolicy::RootPinned | RootAncestorPinned` を eviction 対象から除外する判定関数を実装。[file:2][file:3]
- protected entry への eviction 要求が失敗することをテスト。[file:2]
- root preset (`StructMem`, `Corpus2Skill`) が cache eviction されないことを replay で確認。[file:2][file:3]

#### E2 Periodic eviction worker

- バックグラウンド periodic worker を追加。[file:2]
- `eviction_interval` ごとに expired / pressure / over-capacity を評価。[file:2]
- EventBus 非依存でも最小構成で動作する fake 実装テストを追加。[file:3]

#### E3 TTL eviction semantics

- Human Time と VirtualClock の二軸 TTL を実装。[file:2]
- `last_used_at`, `last_virtual_seen`, `loaded_at` を使って eviction eligibility を判定。[file:2]
- protected preset は TTL 対象外であることをテスト。[file:2]

#### E4 Pressure-driven eviction

- `ResourcePressure` と `PressureMode` に応じて eviction aggressiveness を切り替える。[file:2]
- `Constrained` / `Emergency` で candidate selection が強まることをテスト。[file:2]
- ANN hot index bytes を pressure signal に含める。[file:2]

#### E5 GcEvent-driven eviction

- `DarviumEventKind::GcEvent` を購読して cache eviction を実行。[file:2][file:3]
- `SoftDeleted`, `HardDeleteCandidate`, `Tombstoned` で residency が縮退することをテスト。[file:2]
- `Tombstoned` が cache に残存しないことを invariant test 化。[file:2]

#### E6 Invariants and tests

- property-based test: protected never evicted, tombstoned never resident, committed reloadable, non-committed excluded from normal hot path。[file:2][file:3][file:4]
- replay test: GC event stream を replay して cache residency が deterministic に変化する。[file:2][file:3]
- capacity test: over-capacity で非 protected のみが落ちる。[file:2]

## 3. Table / Struct 定義ファイルの改訂指示

### 対象ファイル

- `Darvium-v2.3-final-table-and-struct-definition-spec-3.md` [file:4]

### 3-1. Rust struct 定義に追加

WorkflowCache struct の定義節に、次のフィールドまたは同等設計を追加してください。[file:4][file:2]

```rust
pub struct WorkflowCache {
    pub workingset: Arc<RwLock<Vec<MemoizedGraph>>>,
    pub annhint: Arc<RwLock<AnnHotIndex>>,
    pub policy: CachePolicy,
    pub max_entries: usize,
    pub max_bytes: usize,
    pub eviction_interval: Duration,
    pub default_ttl_human: Duration,
    pub default_ttl_virtual: u64,
}
```

追加型として次を定義してください。[file:2][file:4]

```rust
pub struct CacheResidencyMeta {
    pub graphid: WorkflowGraphId,
    pub loaded_at: SystemTime,
    pub last_cache_hit_at: SystemTime,
    pub last_cache_hit_vt: u64,
    pub estimated_bytes: usize,
    pub eviction_exempt: bool,
    pub last_eviction_reason: Option<String>,
}
```

```rust
pub struct EvictionReport {
    pub scanned: usize,
    pub evicted: usize,
    pub skipped_protected: usize,
    pub skipped_non_committed: usize,
    pub freed_estimated_bytes: usize,
}
```

### 3-2. enum / error 定義の追加

`CacheError` を次のように拡張するよう指示してください。[file:2][file:4]

```rust
pub enum CacheError {
    CasConflict { expected: u64, actual: u64 },
    NotFound(WorkflowGraphId),
    LoadFailed(String),
    CapacityExceeded { max_entries: usize, max_bytes: usize },
    ProtectedEvictionForbidden(WorkflowGraphId),
    EvictionInvariantViolation(String),
}
```

必要なら `EvictionReason` enum も追加してください。[file:2]

```rust
pub enum EvictionReason {
    TtlExpiredHuman,
    TtlExpiredVirtual,
    CapacityPressure,
    ResourcePressure,
    GcStateTransition,
    ManualCleanup,
}
```

### 3-3. MemoizedGraph / Preset 関連の説明補強

MemoizedGraph 行または対応 Rust 型の注記に、次を明記してください。[file:2][file:4]

- `artifactoriginkind`, `presetsourceinfo`, `rootpolicy`, `gcstate` は cache eviction protection 判定の入力となる。[file:2]
- `GcState::Protected` は persistence GC exclusion だけでなく WorkflowCache eviction exclusion でもある。[file:2][file:3]

### 3-4. SQLite スキーマの扱い

WorkflowCache は in-memory cache なので residency state を DB 永続化する必要は必須ではありません。[file:2][file:4]

そのため、改訂指示としては以下が安全です。[file:4]

- `memoizedgraphs` テーブル自体は原則変更不要。[file:4]
- ただし運用観測用に必要なら、新規テーブル `cacheevictionlog` を SQLite に追加してもよい (MAY)。[file:4]

追加するなら最低限の列は次です。[file:4]

- `evictionid`
- `graphid`
- `reason`
- `protected`
- `estimatedbytesfreed`
- `createdatms`

### 3-5. Event 型定義の追加

GcEvent payload の型定義を table/struct spec に明示追加してください。[file:2][file:4]

```rust
pub struct GraphGcStateChanged {
    pub graphid: WorkflowGraphId,
    pub old_state: GcState,
    pub new_state: GcState,
    pub reason: Option<String>,
}
```

また、Projection または observer として次のような型を追加できます。[file:2][file:3][file:4]

```rust
pub struct CacheEvictionEvent {
    pub graphid: WorkflowGraphId,
    pub reason: EvictionReason,
    pub freed_estimated_bytes: usize,
    pub created_at: SystemTime,
}
```

## 4. 編集優先順位

実際の改訂担当者には、次の順で編集させるのが安全です。[file:2][file:3][file:4]

1. RFC 本文 §8 / §15 / §4 / §22 を先に修正し、規範を固定する。[file:2]
2. その規範に合わせて table/struct spec の Rust 型・エラー型・イベント型を更新する。[file:4]
3. 最後に tickets に実装タスクとテストタスクを追加する。[file:3]

## 5. 最低限の規範セット

時間がなく、最小改訂だけ先に入れる場合は、次の 6 点だけでも先に本文へ反映してください。[file:2]

- WorkflowCache は volatile cache であり、eviction は persistence deletion ではない。[file:2]
- `GcState::Protected` と preset-root は cache eviction 対象外 (MUST NOT)。[file:2][file:3]
- `GcState::Tombstoned` は cache 残存禁止 (MUST NOT)。[file:2]
- 実装は periodic または capacity-bound eviction を必ず持つ (MUST)。[file:2]
- TTL / ResourcePressure / GcEvent に基づく eviction を許可する (MAY / SHOULD)。[file:2]
- `ConsistencyState::Committed` のみ normal hot path に載せ、再ロード可能であることを保証する。[file:2][file:4]
