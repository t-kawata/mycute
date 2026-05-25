# Darvium RFC 改訂指示書 — `WorkflowRepository` を `WorkflowCache` へ改名

## 改訂目的
本改訂は、Darvium RFC-0001 Unified Edition v2.3-i における `WorkflowRepository` という名称が、実装上の責務を過大に示唆している問題を解消するために実施する。[cite:1]

現行 RFC では、`WorkflowRepository` は Rust レベルで `Arc<RwLock<Vec<MemoizedGraph>>>` として表現される一方、MemoizedGraph の正本および検索可能性は SQLite と LadybugDB の dual-store consistency、startup repair scan、commit intent、ConsistencyState によって支えられているため、当該構造は source-of-truth ではなく runtime cache / in-memory index と解釈するのが設計意図に整合する。[cite:1]

このため、`WorkflowRepository` は今後 `WorkflowCache` に改名し、SQLite + LadybugDB から成る Repository Pair を正本、`WorkflowCache` をその runtime cache として明文化する。[cite:1]

## 改訂方針
改訂は **strictly additive clarification ではなく、用語是正を伴う編集改訂**として扱う。[cite:1]

ただし、既存の core invariant、dual-store recovery invariant、SearchWorkflow semantics、Training / Production separation、StructMem / Corpus2Skill の責務境界は変更してはならない。[cite:1]

名称変更の対象は、少なくとも以下を含む。[cite:1]

- `WorkflowRepository` → `WorkflowCache`
- `repository.rs` の責務説明 → 「cache facade over Repository Pair」へ修正
- 「repository」「repo」「workflow repository」等の自然言語表現 → 文脈に応じて `WorkflowCache` または `Repository Pair` に置換
- `RepositoryError` のうち dual-store 永続化責務に関する説明 → `Repository Pair` 側の責務として再配置
- `WorkflowRepository` を source-of-truth と誤読しうる記述 → 全て修正

## 規範的再定義
以下の文言を RFC 本文に追加し、既存の曖昧な読解可能性を除去すること。[cite:1]

> `WorkflowCache` は、SQLite + LadybugDB から構成される Repository Pair 上に永続化された MemoizedGraph 群の runtime cache である。`WorkflowCache` は source-of-truth ではなく、検索高速化・局所再利用・compile-time / retrieval-time 参照のための in-memory working set を提供する。MemoizedGraph の canonical persistence, consistency, repair, quarantine, and availability は Repository Pair により担保されなければならない (MUST)。[cite:1]

あわせて、以下の文言を明示すること。[cite:1]

> mission を受けた SearchWorkflow / RetrievalPrimitive は、論理的には Repository Pair 上の MemoizedGraph 全体を検索対象とする。`WorkflowCache` はその部分集合を保持する加速機構であり、cache miss は Repository Pair からの lazy load により解決されなければならない (MUST)。[cite:1]

## セクション別修正指示

### §2 用語集
`WorkflowRepository` の定義を削除し、以下へ差し替えること。[cite:1]

- **WorkflowCache**: Repository Pair 上に永続化された MemoizedGraph の runtime cache / in-memory index。[cite:1]
- **Repository Pair**: SQLite と LadybugDB により MemoizedGraph・WorkflowGraph・lineage・trust・consistency state を保持する永続化ペア。[cite:1]

必要に応じて、`WorkflowRegistry` と `ResolvedWorkflowRegistry` との区別も脚注または補注で明確化すること。[cite:1]

### §5 アーキテクチャ概観
Layer 3a の図中表記を以下へ変更すること。[cite:1]

- 旧: `WorkflowRepository, MemoizedGraph, 4-Layer Retrieval`
- 新: `WorkflowCache, MemoizedGraph, 4-Layer Retrieval`

さらに本文では、Layer 3a の検索対象が `WorkflowCache` 単独ではなく Repository Pair 上の全 MemoizedGraph であることを明文化すること。[cite:1]

### §8 WorkflowRepository と MemoizedGraph
セクション名を次へ変更すること。[cite:1]

- 旧: `WorkflowRepository と MemoizedGraph`
- 新: `WorkflowCache と MemoizedGraph`

本節では次を明記すること。[cite:1]

- `WorkflowCache` は runtime materialization / cache layer である。  
- MemoizedGraph の正本は SQLite + LadybugDB の Repository Pair に存在する。  
- cache eviction, preload, lazy load, warmup, pinning は実装ポリシーだが、cache hit / miss にかかわらず検索可能性は Repository Pair に依存する。  
- `ConsistencyState` は cache ではなく永続層整合性の状態である。  
- startup repair scan は cache 初期化ではなく Repository Pair の recovery procedure である。[cite:1]

Rust 擬似コード例は以下の方向に更新すること。[cite:1]

```rust
struct WorkflowCache {
    working_set: Arc<RwLock<Vec<MemoizedGraph>>>,
    ann_hint: Arc<RwLock<AnnHotIndex>>,
    policy: CachePolicy,
}

struct RepositoryPair {
    sqlite: SqliteStore,
    ladybug: LadybugStore,
}
```

## 実装責務の再配置
dual-store commit, repair, quarantine, startup scan, consistency state 遷移は `WorkflowCache` ではなく `Repository Pair` または `RepositoryPairFacade` の責務として再整理すること。[cite:1]

`WorkflowCache` の責務は次に限定すること。[cite:1]

- 最近参照された MemoizedGraph の保持
- preset / protected workflow の常駐
- lazy load 後の一時保持
- retrieval 時の hot-path 最適化
- eviction / warming / pinning policy の実装

逆に、以下を `WorkflowCache` の責務として記述してはならない。[cite:1]

- canonical persistence
- dual-store commit ordering
- repair orchestration
- quarantine decision の最終権限
- source-of-truth ownership

## 検索フロー修正指示
GMR / SearchWorkflow / RetrievalPrimitive の説明中で、検索開始時のフローを次のように書き換えること。[cite:1]

1. QueryDesignText / TopLevelQueryMetadata を生成する。  
2. SQLite 主導で semantic retrieval と metadata filter を実施する。  
3. 候補 ID に対応する MemoizedGraph が `WorkflowCache` に存在する場合はそれを利用する。  
4. cache miss の場合は Repository Pair から MemoizedGraph を lazy load する。  
5. 必要に応じて cheap GED / full GED 用に LadybugDB 上の WorkflowGraph 本体を参照する。  
6. hot candidate は `WorkflowCache` に昇格させてもよい (MAY)。[cite:1]

これにより、「WorkflowCache 単独が検索空間である」という誤解と、「全件インメモリ保持が前提である」という誤解を除去すること。[cite:1]

## 命名置換ルール
機械的な一括置換では不十分であり、以下のルールで精査すること。[cite:1]

| 現行表現 | 修正先 | 備考 |
|---|---|---|
| `WorkflowRepository` | `WorkflowCache` | 型名・節名・図表ラベルで使用[cite:1] |
| `repository.rs` | `workflow_cache.rs` または `cache.rs` | 実装構成に応じて選択[cite:1] |
| `RepositoryError` | `RepositoryPairError` または `PersistenceError` | dual-store 永続化責務を反映[cite:1] |
| `repository.get(...)` | `cache.get(...)` | lazy load facade を伴うなら `cache.get_or_load(...)` も可[cite:1] |
| `WorkflowRepository source-of-truth` | 削除 | 誤りとして除去[cite:1] |

## 後方互換ポリシー
コード移行期間中は type alias による一時互換を許可してよい。[cite:1]

```rust
#[deprecated(note = "Renamed to WorkflowCache; WorkflowRepository implied incorrect ownership semantics")]
type WorkflowRepository = WorkflowCache;
```

ただし RFC 本文では旧名を規範用語として残してはならない。[cite:1]

## 受け入れ基準
本改訂の完了条件は以下とする。[cite:1]

- RFC 全文中で `WorkflowRepository` が規範用語として残存しない。  
- source-of-truth が Repository Pair であることが §2, §5, §8, §18, §25 に一貫して明記される。  
- `WorkflowCache` が cache beyond ownership に読めない。  
- SearchWorkflow / GMR の検索開始パスが「DB 主導 + cache 加速」として読める。  
- dual-store consistency / startup repair / quarantine の責務主体が Repository Pair 側へ整理される。  
- PresetRegistry, ResolvedWorkflowRegistry, WorkflowRegistry との用語衝突が解消される。[cite:1]

## 編集者メモ
本改訂は単なる好みの問題ではなく、責務境界と source-of-truth を名前で誤誘導しないための重要な明確化である。[cite:1]

`WorkflowRepository` という名称は、実装者に対して「ここが保存庫である」「ここが正本である」という誤った設計圧力を与えうるため、`WorkflowCache` への改名は意味論的にも妥当である。[cite:1]
