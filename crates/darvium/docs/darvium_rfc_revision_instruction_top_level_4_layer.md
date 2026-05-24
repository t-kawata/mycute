# Darvium RFC-0001 改訂指示書　for v2.3-h

## 目的

本指示書は、Darvium RFC-0001 Unified Edition v2.3-f に含まれる GMR Retrieval Core / WorkflowDesignEmbedding / QueryDesignEmbedding / GED reranking 周辺の設計を、**最上階（最高抽象度）WorkflowGraph に対する 4 層検索方式**へ一貫して改訂するための、部下向けの完全な改訂指示書である。[cite:35]

今回の改訂の主要目的は次の 4 点である。[cite:35]

- WorkflowDesignEmbedding / QueryDesignEmbedding を「構造類似検索の主チャネル」として扱う設計を終了し、構造類似の第一級の source of truth を **最上階 WorkflowGraph に対する GED 系検索**へ移すこと。[cite:35]
- ただし semantic channel としての `taskembedding` は維持し、SearchWorkflow の mission-oriented な検索意図を失わないこと。[cite:35]
- 入れ子 DAG を持つ実装現実に合わせ、検索パイプラインの対象を **常に最上階 DAG のみ**に限定し、下位階層 DAG は retrieval front-channel には乗せないこと。[cite:35]
- 検索・ランキング・較正・replay・property-based test の各責務を分離し、RFC 全体の deterministic replay / auditability / calibration discipline と整合するように仕様を組み直すこと。[cite:35]

この改訂は、RFC の既存の source-of-truth 境界を壊してはならない。WorkflowGraph, GraphVersion, TrustProfile, Lifecycle state, SearchTrace の ownership は既存の Workflow Orchestration Plane 側に残り、Knowledge Plane / Training Plane / Conversational Path は strictly additive のままとすること。[cite:35]

## 背景理解

現行 RFC v2.3-f では、`MemoizedGraph` に `taskembedding`, `workflowdesigntext`, `workflowdesignembedding` が保持され、Stage 2a で task embedding ANN、Stage 2b で workflow design embedding ANN、Stage 2c で union / dedupe、Stage 3 で GED reranking、Stage 4 で ApplicabilityScore を計算する構成になっている。[cite:35]

現行 RFC は、WorkflowDesignText を canonical schema として保持しつつ、その embedding を「graph embedding の structural proxy」として扱っている。また QueryRepresentation は `missiontext`, `taskembedding`, `querydesigntext`, `querydesignembedding` を持ち、dual retrieval / bi-vector retrieval を行う前提になっている。[cite:35]

しかし本 RFC 自身が、GED を structural validation・structural match・abstraction trigger の中心機構として扱っており、また graphembedding / GNN encoder は将来マイルストーン寄りの位置づけである。したがって、構造類似の主要責務を design embedding から GED 系へ移すことは、RFC の基本精神に反しないどころか、WorkflowGraph を source of truth とする方向により整合的である。[cite:35]

さらに、実際の対象グラフはしばしば「DAG 自体を抽象化したノードを持つ多階層の入れ子 DAG」であるが、検索対象として重要なのは mission-completion 上の抽象的計画であるため、検索 front-channel は **最上階 DAG** のみを対象とするのが合理的である。[cite:35]

## 改訂の基本方針

改訂後の RFC は、構造検索に関して次の規範を採用すること。[cite:35]

1. 構造検索は **最上階 WorkflowGraph** のみを対象とする。下位 SubWorkflow 展開グラフや内部 DAG は retrieval front-channel では比較対象にしない。[cite:35]
2. 構造類似検索の主チャネルは **GED 系**であり、WorkflowDesignEmbedding / QueryDesignEmbedding は必須チャネルから外す。[cite:35]
3. Semantic channel としての `taskembedding` は残し、semantic coarse retrieval の責務を持たせる。[cite:35]
4. Retrieval pipeline は次の 4 層とする。[cite:35]
   - Layer S: semantic mission retrieval
   - Layer M: SQLite metadata filter
   - Layer G1: cheap GED filter
   - Layer G2: full GED rerank
5. 4 層は常時すべて必須ではなく、候補数が十分小さい場合には Layer G1 を skip してよい。ただし仕様としては 4 層構造を正式な reference architecture として記述すること。[cite:35]
6. `WorkflowDesignText` / `QueryDesignText` は canonical schema として残すが、構造検索の primary index としては扱わないこと。[cite:35]
7. ApplicabilityScore の structural 成分は、design embedding cosine ではなく、**GED 由来の正規化類似度**を用いること。[cite:35]

## 新アーキテクチャ概要

改訂後の GMR Retrieval Core の規範的パイプラインは次の通りである。[cite:35]

### Layer S: Semantic Mission Retrieval

入力 missiontext から `taskembedding` を生成し、最上階 WorkflowGraph に対応する candidate workflow 集合に対して semantic retrieval を行う。ここでの目的は、ミッション意味が大きく異なる workflow を除外することである。[cite:35]

### Layer M: SQLite Metadata Filter

semantic 上で残った候補に対して、SQLite に保存された最上階 DAG の cheap な metadata を使ってフィルタリングする。ここではグラフ本体を Rust 側へロードせず、保存済みのメタ特徴だけで候補数を削減する。[cite:35]

### Layer G1: Cheap GED Filter

Layer M を通過した候補について、最上階 WorkflowGraph をロードし、full node alignment を行わない cheap lower-bound / approximate GED を計算する。ここでは「構造的に明らかに遠い候補」を早期除外する。[cite:35]

### Layer G2: Full GED Rerank

Layer G1 通過候補に対して full GED を計算し、top-k を決定する。ここで得られる GED は構造順位付けと structural validation の双方に使われる。[cite:35]

### Stage 4: Applicability Evaluation

候補 workflow ごとに、semantic 類似、GED 類似、DeterminismScore、TrustProfile、および必要時には Knowledge Applicability を統合し、最終的な REUSE / PATCH / COMPOSE / NEW / ABORT 判断へ接続する。[cite:35]

## 規範的改訂要求

以下は RFC 文書上で **MUST** として反映させるべき改訂要求である。[cite:35]

### 1. WorkflowDesignEmbedding / QueryDesignEmbedding の地位変更

現行 RFC における以下の思想を削除または非主系化すること。[cite:35]

- WorkflowDesignEmbedding を structural proxy retrieval の主チャネルとみなす記述。[cite:35]
- QueryDesignEmbedding を structural retrieval の query 側主表現とみなす記述。[cite:35]
- Dual ANN / bi-vector retrieval のうち、design embedding を semantic channel と同格の first-class retrieval channel とみなす記述。[cite:35]

改訂後は次のように書き換えること。[cite:35]

- `WorkflowDesignText` は canonical, replayable, auditable な **構造記述**として保持される。[cite:35]
- `QueryDesignText` は query sketch / policy hint / optional knowledge-aware hints の canonical 表現として保持される。[cite:35]
- `WorkflowDesignEmbedding` と `QueryDesignEmbedding` は **optional compatibility field** とし、旧実装や移行期間中の実験互換のために残してもよいが、normative retrieval path からは外す。[cite:35]
- 構造検索の主手段は top-level WorkflowGraph に対する GED 系検索である、と明記すること。[cite:35]

### 2. 最上階 DAG 制約の明文化

新しい subsection を追加し、次を明記すること。[cite:35]

- 多階層 WorkflowGraph における retrieval target は **highest abstraction layer graph** のみである。[cite:35]
- `WorkflowNode::SubWorkflow` の内部展開 DAG は retrieval front-channel の similarity 計算対象ではない。[cite:35]
- 下位 DAG は compile, execution, self-refinement, abstraction, patch proposal, lineage explanation, and post-retrieval structural audit のためにのみ利用される。[cite:35]
- すべての metadata / cheap GED / full GED は、最上階 WorkflowGraph に対して定義されること。[cite:35]

### 3. 4 層 retrieval の normative 化

現行 Stage 0–4 記述を改訂し、retrieval path を次の規範テーブルで置き換えること。[cite:35]

| Stage | 名称 | 主対象 | 目的 | 規範 |
|---|---|---|---|---|
| Stage 0 | hard gates | side effects / trust / version | 明白な非適格候補の除外 | MUST [cite:35] |
| Stage 1 | semantic retrieval | taskembedding | ミッション意味での coarse retrieval | MUST [cite:35] |
| Stage 2 | metadata filter | SQLite top-level metadata | cheap metadata による候補削減 | MUST [cite:35] |
| Stage 3 | cheap GED filter | top-level WorkflowGraph | lower-bound / approximate structural pruning | SHOULD, candidate count exceeds threshold のとき MUST [cite:35] |
| Stage 4 | full GED rerank | top-level WorkflowGraph | exact / bounded structural ranking | MUST [cite:35] |
| Stage 5 | applicability evaluation | Aworkflow / K / trust / determinism | action decision | MUST [cite:35] |

### 4. SQLite metadata layer の formalization

SQLite metadata filter は RFC 上で明示的 first-class layer として扱うこと。[cite:35]

最低限次の top-level metadata を normative candidate として列挙すること。[cite:35]

- `top_node_count`
- `top_edge_count`
- `top_source_count`
- `top_sink_count`
- `top_longest_path_len`
- `top_max_width`
- `top_label_histogram`
- `top_edge_type_histogram`
- `top_determinism_summary`
- `top_sideeffect_summary`
- `top_agentsethash` または top-level agent family summary
- `top_layer_signature`

これらは SQLite に materialize され、semantic retrieval の後、cheap SQL predicate または scored filter で適用されることを明記すること。[cite:35]

### 5. Cheap GED と Full GED の区別を明文化

現行 RFC は Stage 3 GED rerank を持つが、cheap lower-bound 層は formal に分離されていないため、次を必ず追加すること。[cite:35]

- Cheap GED は full node alignment を含まない。[cite:35]
- Cheap GED は lower bound もしくは replayable approximation として定義される。[cite:35]
- Full GED は node alignment / edit path search を含む正規 ranking 距離である。[cite:35]
- Cheap GED は pruning 専用であり、最終順位確定に単独使用してはならない。[cite:35]
- Full GED は top-k ranking と structural validation に使用する。[cite:35]

### 6. ApplicabilityScore の再定義

現行 RFC では `S_total = 0.35 S_sem + 0.65 S_struct` のような blend と、`S_struct = GEDnormalized` がすでに示唆されているが、design embedding cosine と混線しているため、改訂後は次のように一本化すること。[cite:35]

- `S_sem` は taskembedding ベースの semantic similarity。[cite:35]
- `S_struct` は **top-level full GED から導く正規化構造類似度**。[cite:35]
- Cheap GED は `S_struct` の直接入力ではなく pruning gate とする。[cite:35]
- `S_total` は `S_sem` と `S_struct` の convex combination で定義し、係数は calibration candidate としてバージョン管理する。[cite:35]

推奨形は以下である。

\[
S_{total}(q,G)=\alpha S_{sem}(q,G)+(1-\alpha)S_{struct}(q,G) \tag{1}
\]

ここで

\[
S_{struct}(q,G)=\exp\left(-\lambda \cdot \widetilde{GED}(q,G)\right) \tag{2}
\]

\(
\widetilde{GED}(q,G)
\) は top-level DAG の正規化 GED である。[cite:35]

正規化 GED の標準案は次とする。

\[
\widetilde{GED}(q,G)=\frac{GED(q,G)}{\max\{|V_q|+|E_q|,\ |V_G|+|E_G|,\ 1\}} \tag{3}
\]

このとき `S_struct ∈ (0,1]` となり、ApplicabilityScore に滑らかに接続できる。[cite:35]

既存の workflow applicability は次でよい。

\[
A_{workflow}=\max(S_{total}, f_S)^{\alpha_S}\cdot \max(D, f_D)^{\alpha_D}\cdot \max(T, f_T)^{\alpha_T} \tag{4}
\]

ここで `D` は DeterminismScore、`T` は TrustProfile composite、`f_S,f_D,f_T` は floor である。[cite:35]

knowledge-aware path が有効な場合は、既存 RFC の knowledge applicability を保持して次を用いること。[cite:35]

\[
A_{final}=A_{workflow}^{0.70}\cdot K^{0.30} \tag{5}
\]

もしくは現行 RFC 準拠で weighted blend を使う場合も、versioned applicability model として固定すること。[cite:35]

## 改訂対象セクション別指示

以下、部下に対する section-by-section の改訂指示を示す。[cite:35]

## 1. 概要・イントロダクションの改訂

### 修正目的

文書全体の読者が「Darvium の retrieval は design embedding ではなく top-level DAG GED 主体へ移行した」と最初に理解できるようにすること。[cite:35]

### 改訂指示

- 概要部の retrieval summary から「dual ANN with workflowdesignembedding」を主設計として述べる文を削除すること。[cite:35]
- 代わりに「semantic mission retrieval + top-level structural GED retrieval」の二系統であると書くこと。[cite:35]
- 多階層 workflow を扱うが retrieval target は top-level DAG だけであることを短く明記すること。[cite:35]

### 差し替え文面の骨子

- Darvium retrieval is mission-first and structure-grounded.[cite:35]
- Semantic retrieval narrows intent-compatible candidates.[cite:35]
- Structural retrieval over the highest-abstraction WorkflowGraph performs metadata pruning, cheap GED pruning, and full GED ranking.[cite:35]
- Lower-level subworkflow graphs remain execution and refinement assets, not first-pass retrieval targets.[cite:35]

## 2. Section 8 WorkflowRepository / MemoizedGraph の改訂

### 修正目的

データモデルを新 retrieval path に合わせること。[cite:35]

### 改訂指示

`MemoizedGraph` 定義を次の思想で更新すること。[cite:35]

1. `workflowdesigntext` は保持する。[cite:35]
2. `workflowdesignembedding` は optional compatibility field に格下げする。[cite:35]
3. top-level DAG metadata を永続的に持つフィールドを追加する。[cite:35]
4. cheap GED 用の replayable signature field を追加してよい。[cite:35]

推奨構造は次である。[cite:35]

```rust
struct TopLevelGraphMetadata {
    top_node_count: u16,
    top_edge_count: u16,
    top_source_count: u16,
    top_sink_count: u16,
    top_longest_path_len: u16,
    top_max_width: u16,
    top_label_histogram: Vec<(String, u16)>,
    top_edge_type_histogram: Vec<(String, u16)>,
    top_determinism_summary: f32,
    top_sideeffect_summary: SideEffectSummary,
    top_layer_signature: Vec<u64>,
}

struct CheapGedSignature {
    topo_rank_labels: Vec<u64>,
    indegree_histogram: Vec<u16>,
    outdegree_histogram: Vec<u16>,
    ancestor_bitset_sketch: Vec<u64>,
    descendant_bitset_sketch: Vec<u64>,
    path_hash_multiset: Vec<(u64, u16)>,
    signature_version: String,
}

struct MemoizedGraph {
    id: WorkflowGraphId,
    graph: WorkflowGraph,
    taskembedding: Vec<f32>,
    workflowdesigntext: String,
    workflowdesignembedding: Option<Vec<f32>>, // compatibility only
    top_metadata: TopLevelGraphMetadata,
    cheap_ged_signature: CheapGedSignature,
    ...
}
```

`EmbeddingVersions.design` は削除してもよいが、移行互換のために deprecated field として残すなら、その理由を明記すること。[cite:35]

## 3. Section 9 WorkflowDesignText / QueryDesignText の改訂

### 修正目的

text は残すが embedding 検索主系ではないことを明確化すること。[cite:35]

### 改訂指示

- `WorkflowDesignText` の canonical schema 記述は維持する。[cite:35]
- `QueryDesignText` の canonical schema 記述と knowledge-aware extension も維持する。[cite:35]
- ただし、両者の embedding が structural retrieval の normative path であるかのような記述を除去すること。[cite:35]
- 次の一文を必ず追加すること。

> WorkflowDesignText and QueryDesignText are canonical, replayable, and auditable textual descriptions of top-level workflow intent and structure. They SHALL NOT, by themselves, define the primary structural retrieval metric in this revision. Primary structural retrieval SHALL be computed over the top-level WorkflowGraph through metadata filtering, cheap GED filtering, and full GED ranking.[cite:35]

### QueryRepresentation の改訂

```rust
struct QueryRepresentation {
    missiontext: String,
    taskembedding: Vec<f32>,
    querydesigntext: String,
    querydesignembedding: Option<Vec<f32>>, // compatibility only
    designtemplateversion: String,
    querytype: QueryType,
    freshnessrequirement: FreshnessRequirement,
    evidencestrictness: EvidenceStrictness,
    origintracerequired: bool,
    driftsensitivity: DriftSensitivity,
    top_query_metadata: TopLevelQueryMetadata,
    cheap_ged_signature: CheapGedSignature,
}
```

ここで `TopLevelQueryMetadata` は querydesigntext から deterministic formatter で導く top-level graph sketch metadata とすること。[cite:35]

## 4. Section 11 Applicability Check の改訂

### 修正目的

構造類似スコアの定義を design embedding cosine から GED 正規化へ一本化すること。[cite:35]

### 改訂指示

- AG-06 / AG-07 の命名と定義を見直し、「semantic channel version mismatch」は task embedding に限定すること。[cite:35]
- structural channel gate は embedding model version ではなく、`cheap_ged_signature_version` と `ged_cost_model_version` に置き換えること。[cite:35]
- SimilarityScore の定義を、semantic similarity と full GED similarity の合成で書き直すこと。[cite:35]
- cheap GED は gate / pruning 理由として SearchTrace に残すが、Applicability の直接項にはしないこと。[cite:35]

### 追加必須数式

Semantic similarity の標準形:

\[
S_{sem}(q,G)=\max\left(0,\frac{\langle e_q,e_G\rangle}{\|e_q\|\|e_G\|}\right) \tag{6}
\]

Full GED similarity の標準形:

\[
S_{struct}(q,G)=\exp(-\lambda\widetilde{GED}(q,G)) \tag{7}
\]

総合類似度:

\[
S_{total}(q,G)=\alpha S_{sem}(q,G)+(1-\alpha)S_{struct}(q,G),\quad \alpha\in[0,1] \tag{8}
\]

Applicability:

\[
A_{workflow}(q,G)=\max(S_{total},f_S)^{\alpha_S}\max(D_G,f_D)^{\alpha_D}\max(T_G,f_T)^{\alpha_T} \tag{9}
\]

knowledge-aware なら:

\[
A_{final}(q,G)=A_{workflow}(q,G)^{\beta}\cdot K(q,G)^{1-\beta} \tag{10}
\]

推奨初期値は、\(\alpha=0.45\), \(\lambda=4.0\), \(\beta=0.70\) とし、いずれも calibration candidate と明記すること。[cite:35]

## 5. Section 12 Layer 3a GMR Retrieval Core の改訂

### 修正目的

最重要改訂箇所。現行 Stage 2a/2b/2c を全面的に再構成すること。[cite:35]

### 現行から削除・修正すべきもの

- `ANN HNSW workflowdesignembedding top-kstruct` を normative stage から削除。[cite:35]
- `Dual ANN Union Rerank` を normative flow から削除。[cite:35]
- `workflowdesignembedding structural proxy` を主構造チャネルとする表現を削除。[cite:35]

### 新ステージ定義

#### Stage 1: Semantic Retrieval

- 入力: `taskembedding(q)`
- index: semantic ANN または exact cosine over taskembedding
- 出力: `C_sem(q)`
- サイズ上限: `K_sem`

\[
C_{sem}(q)=\operatorname{TopK}_{G\in\mathcal{R}} S_{sem}(q,G;task) \tag{11}
\]

#### Stage 2: Metadata Filter

- 入力: `C_sem(q)` と `TopLevelQueryMetadata(q)`
- 処理: SQLite predicate / scored filter
- 出力: `C_meta(q)`

標準 scored filter は次を推奨すること。[cite:35]

\[
M(q,G)=w_v\Delta_V(q,G)+w_e\Delta_E(q,G)+w_l\Delta_L(q,G)+w_p\Delta_P(q,G)+w_s\Delta_S(q,G) \tag{12}
\]

ここで

- \(\Delta_V\): node count difference normalized
- \(\Delta_E\): edge count difference normalized
- \(\Delta_L\): label histogram distance
- \(\Delta_P\): longest path / layer signature distance
- \(\Delta_S\): side effect summary mismatch penalty

`C_meta(q)` は最小 `M(q,G)` の top `K_meta` とすること。[cite:35]

#### Stage 3: Cheap GED Filter

cheap GED lower bound を \(LB(q,G)\) とし、以下を満たすこと。[cite:35]

\[
LB(q,G) \le GED(q,G) \tag{13}
\]

cheap GED 候補集合は:

\[
C_{cheap}(q)=\{G\in C_{meta}(q)\mid LB(q,G) \le \tau_{cheap}(q)\} \tag{14}
\]

または top `K_cheap` 方式:

\[
C_{cheap}(q)=\operatorname{TopK}_{G\in C_{meta}(q)} -LB(q,G) \tag{15}
\]

cheap GED の候補には次の構成要素を明示すること。[cite:35]

- node/edge count lower bound
- label multiset mismatch lower bound
- topological layer mismatch lower bound
- ancestor/descendant reachability sketch mismatch lower bound
- bounded path-hash multiset mismatch lower bound

cheap GED は replayable deterministic function とし、乱数や hidden ANN 由来の近似を使わないこと。[cite:35]

#### Stage 4: Full GED Rerank

full GED 候補集合について、

\[
G^*_1,\dots,G^*_k = \operatorname{TopK}_{G\in C_{cheap}(q)} -GED(q,G) \tag{16}
\]

を計算する。full GED は top-level DAG に対する node alignment + edge edit cost を含む deterministic cost search とすること。[cite:35]

推奨 edit cost モデルは次とする。[cite:35]

\[
GED(q,G)=\min_{\pi\in\Pi(q,G)} \Bigg(\sum_{u\in V_q} c_V(u,\pi(u)) + \sum_{e\in E_q} c_E(e,\pi(e)) + c_{ins/del}(\pi)\Bigg) \tag{17}
\]

ノード置換コストの推奨形:

\[
c_V(u,v)=\eta_k \mathbf{1}[kind(u)\ne kind(v)] + \eta_a(1-J_A(u,v)) + \eta_i(1-J_I(u,v)) + \eta_o(1-J_O(u,v)) + \eta_d|det(u)-det(v)| \tag{18}
\]

ここで

- \(J_A\): agent/tag set Jaccard
- \(J_I\): input type set Jaccard
- \(J_O\): output type set Jaccard

エッジ置換コストの推奨形:

\[
c_E(e,f)=\eta_t\mathbf{1}[type(e)\ne type(f)] + \eta_b\mathbf{1}[branch(e)\ne branch(f)] \tag{19}
\]

ノード削除・挿入は定数コスト、ただし side effect を持つノードは高コストにすること。[cite:35]

\[
c_{del}(u)=\delta_0 + \delta_{se}\cdot risk(u),\qquad c_{ins}(v)=\iota_0 + \iota_{se}\cdot risk(v) \tag{20}
\]

## 6. Section 12.3A / 12.3B / 12.3C 付近の GED 記述の再整理

### 修正目的

現行の「transport approximation, balanced-validate, abstraction-trigger」を、top-level 4 層 retrieval の後段 auxiliary mechanism として整理すること。[cite:35]

### 改訂指示

- transport-based approximation は cheap GED の optional implementation として位置づけ直すこと。[cite:35]
- beam edit path は full GED の acceleration / bounded search option として位置づけること。[cite:35]
- `GraphNeedsAbstraction` は top-level graph size が policy limit を超えたときの fallback とし、今回の target regime（top-level 55 nodes 程度）では exception path と明記すること。[cite:35]
- 重要なのは「top-level 55 node regime では abstraction trigger は通常系ではない」と書くこと。[cite:35]

## 7. Section 20 Calibration Candidates の改訂

### 修正目的

今回の改訂の成否は calibration に強く依存するため、ここを強化すること。[cite:35]

### 追加すべき calibration candidate

以下を新しい versioned calibration parameters として列挙すること。[cite:35]

- `TOPLEVELONLYRETRIEVAL = true`
- `K_SEM`
- `K_META`
- `K_CHEAP`
- `K_FULL`
- `METAFILTER_THRESHOLD` または `METAFILTER_TOPK`
- `CHEAPGED_ENABLE_THRESHOLD`
- `CHEAPGED_LB_VERSION`
- `FULLGED_COST_MODEL_VERSION`
- `FULLGED_TIMEOUT_MS`
- `SIMILARITY_ALPHA`
- `STRUCT_GED_LAMBDA`
- `APPLICABILITY_BETA`
- `GED_NODE_DELETE_COST`
- `GED_NODE_INSERT_COST`
- `GED_EDGE_DELETE_COST`
- `GED_EDGE_INSERT_COST`
- `GED_SIDEEFFECT_PENALTY`
- `GED_KIND_MISMATCH_PENALTY`
- `GED_AGENTSET_WEIGHT`
- `GED_IO_WEIGHT`
- `GED_DETERMINISM_WEIGHT`

deprecated candidate として次を移すこと。[cite:35]

- `ANNTOPKSTRUCT`
- `workflowdesignembedding modelversion`
- `GEDBLENDMARGIN`（embedding blending の意味で使っていたなら廃止）

## 数学的検証・観察・較正ループ

この節は部下が必ず新設すること。単なる実装メモではなく、**運用可能な calibration protocol** を RFC annex か engineering note annex に含めること。[cite:35]

## 1. 評価問題の定義

評価対象を次の 3 問に分けること。[cite:35]

1. 候補集合段階で relevant workflow を十分保持できているか。 
2. 最終 full GED ranking が reuse / patch / compose decision quality に寄与しているか。 
3. ranking が小さな構造摂動や replay に対して安定か。 

## 2. 監督信号データセット

以下の evaluation set を作ること。[cite:35]

- **Gold reuse set**: mission に対して正解 workflow が一意または少数あるサンプル集合。[cite:35]
- **Gold patch set**: 既存 workflow の patch が妥当で、完全新規生成より優れるケース。[cite:35]
- **Gold compose set**: 単一 reuse より composition が妥当なケース。[cite:35]
- **Hard negatives**: semantic は近いが構造が不適切な workflow。[cite:35]
- **Structural perturbation set**: ラベル変更、leaf node 追加、branch reorder、optional step insertion など small edit を加えたペア集合。[cite:35]

## 3. 主要評価指標

### Retrieval Recall

semantic 層、metadata 層、cheap GED 層、full GED 層ごとに recall を測ること。[cite:35]

\[
Recall@K = \frac{1}{|Q|}\sum_{q\in Q}\mathbf{1}[\exists G\in TopK(q): G\in Rel(q)] \tag{21}
\]

層別には `Recall@K_sem`, `Recall@K_meta`, `Recall@K_cheap`, `Recall@K_full` を取ること。[cite:35]

### Ranking Quality

最終 ranking には nDCG と MRR を使うこと。[cite:35]

\[
DCG@K(q)=\sum_{i=1}^{K}\frac{2^{rel_i(q)}-1}{\log_2(i+1)} \tag{22}
\]

\[
NDCG@K=\frac{1}{|Q|}\sum_{q\in Q}\frac{DCG@K(q)}{IDCG@K(q)} \tag{23}
\]

\[
MRR=\frac{1}{|Q|}\sum_{q\in Q}\frac{1}{rank_q} \tag{24}
\]

### Decision Quality

REUSE / PATCH / COMPOSE / NEW / ABORT の意思決定について F1 を測ること。[cite:35]

\[
F1_c=\frac{2\cdot Precision_c\cdot Recall_c}{Precision_c+Recall_c} \tag{25}
\]

macro-F1 を主要指標にすること。[cite:35]

### Efficiency

レイテンシ分解を測ること。[cite:35]

\[
T_{total}=T_{sem}+T_{meta}+T_{cheap}+T_{full}+T_{app} \tag{26}
\]

さらに候補数分解:

\[
N_{sem}\ge N_{meta}\ge N_{cheap}\ge N_{full} \tag{27}
\]

`T_full / N_full`、`T_cheap / N_meta` を監視し、cheap GED の存在価値を数値で確認すること。[cite:35]

### Ranking Stability

構造摂動前後で順位相関を測ること。[cite:35]

\[
\rho_{Spearman}(q)=\rho(rank(q), rank(\tilde q)) \tag{28}
\]

また top-k Jaccard stability を使うこと。[cite:35]

\[
J_k(q,\tilde q)=\frac{|TopK(q)\cap TopK(\tilde q)|}{|TopK(q)\cup TopK(\tilde q)|} \tag{29}
\]

## 4. cheap GED の有効性検証

cheap GED を導入する意味は、「full GED を減らしつつ recall を維持すること」にある。したがって、必ず次を測ること。[cite:35]

\[
PruneGain = 1 - \frac{N_{cheap}}{N_{meta}} \tag{30}
\]

\[
MissRate_{cheap} = \frac{1}{|Q|}\sum_{q\in Q}\mathbf{1}[Rel(q)\cap C_{meta}(q) \ne \varnothing \land Rel(q)\cap C_{cheap}(q)=\varnothing] \tag{31}
\]

cheap GED は、`PruneGain` が十分高く、かつ `MissRate_cheap` が許容閾値以下である場合にのみ採用すべきと明記すること。[cite:35]

推奨初期閾値:

- `PruneGain >= 0.50`
- `MissRate_cheap <= 0.01`

## 5. full GED cost model の較正

full GED の edit cost 重みは supervised ranking と pairwise preference の双方で較正すること。[cite:35]

### pairwise margin loss

relevant graph \(G^+\) と irrelevant graph \(G^-\) に対して、

\[
L_{pair}(q)=\max(0, m + GED(q,G^+) - GED(q,G^-)) \tag{32}
\]

を最小化する。ここで \(m>0\) は margin である。[cite:35]

### decision-aware objective

最終目的が ranking だけでなく decision quality であるため、

\[
L = \lambda_1 L_{pair} + \lambda_2 L_{action} + \lambda_3 L_{stability} \tag{33}
\]

とし、

- \(L_{action}\): REUSE / PATCH / COMPOSE / NEW 分類誤差
- \(L_{stability}\): small perturbation に対する順位変動ペナルティ

を含めること。[cite:35]

たとえば安定性ペナルティは次でよい。[cite:35]

\[
L_{stability}(q,\tilde q)=1-\rho_{Spearman}(q,\tilde q) \tag{34}
\]

## 6. 構造類似と semantic 類似の blend 係数較正

\(\alpha\) を固定値にせず、検証セットで探索すること。[cite:35]

\[
\alpha^* = \arg\max_{\alpha\in\mathcal{A}} \left( NDCG@K - \mu\, MissRate_{reuse} - \nu\, FalseNewRate \right) \tag{35}
\]

ここで

- `MissRate_reuse`: 再利用すべきケースを取りこぼす率
- `FalseNewRate`: 本来 REUSE/PATCH/COMPOSE 可能なのに NEW を選んでしまう率

を指す。[cite:35]

## 7. 4 層そのもののアブレーション実験

必ず次の ablation を実施し、4 層が本当に必要かを検証すること。[cite:35]

- A0: semantic only
- A1: semantic + full GED
- A2: semantic + metadata + full GED
- A3: semantic + metadata + cheap GED + full GED
- A4: semantic + metadata + cheap GED + full GED + applicability gates

比較指標:

- `Recall@K_full`
- `NDCG@10`
- `MacroF1_action`
- `P95 latency`
- `PruneGain`
- `J_k stability`

A3 が A2 より latency を十分下げ、品質劣化が軽微なら cheap GED 採用を正当化できる。[cite:35]

## 8. OOD と drift 監視

運用後は検索分布 drift も監視すること。[cite:35]

semantic embedding 分布 drift:

\[
D_{sem}=W_2(\mathcal{E}_{train}, \mathcal{E}_{prod}) \tag{36}
\]

top-level metadata drift:

\[
D_{meta}=\sum_j JS(H^{train}_j, H^{prod}_j) \tag{37}
\]

full GED difficulty drift:

\[
D_{ged}=\mathbb{E}_{q\sim prod}[\widetilde{GED}(q,G_1^*)]-\mathbb{E}_{q\sim train}[\widetilde{GED}(q,G_1^*)] \tag{38}
\]

これらの drift が閾値を超えた場合、cheap GED threshold と cost model の再較正を起動すること。[cite:35]

## 改訂作業手順（部下向け）

以下の順序を厳守して改訂させること。[cite:35]

## Step 1: 現行 RFC の retrieval 関連記述を全抽出

次の語で全文検索し、関連箇所を一覧化すること。[cite:35]

- `WorkflowDesignEmbedding`
- `QueryDesignEmbedding`
- `workflowdesignembedding`
- `querydesignembedding`
- `graph embedding`
- `structural proxy`
- `Dual Retrieval`
- `Bi-Vector Retrieval`
- `Stage 2a`
- `Stage 2b`
- `Stage 2c`
- `GED reranking`
- `ANNTOPKSTRUCT`
- `EmbeddingVersions.design`
- `workflowdesigntext`
- `querydesigntext`

抽出結果を表にし、各箇所に対して `KEEP / REWRITE / DELETE / DEPRECATE` を付けること。[cite:35]

## Step 2: 用語の再定義表を先に作る

次の terminology table を新設し、既存用語との対応を明文化すること。[cite:35]

| 旧用語 | 新扱い | 注記 |
|---|---|---|
| WorkflowDesignEmbedding | deprecated primary retrieval field | optional compatibility field [cite:35] |
| QueryDesignEmbedding | deprecated primary retrieval field | optional compatibility field [cite:35] |
| Structural proxy retrieval | top-level GED retrieval | embedding proxy ではない [cite:35] |
| workflowdesign ANN | removed from normative path | migration-only optional [cite:35] |
| Stage 2b structural ANN | replaced | metadata + cheap GED [cite:35] |

## Step 3: Section 8, 9, 11, 12, 20 を改訂

本文の改訂順は必ずこの順にすること。[cite:35]

1. Section 8 data model
2. Section 9 canonical text semantics
3. Section 11 applicability semantics
4. Section 12 retrieval flow
5. Section 20 calibration + testing discipline

理由は、data model と score definition が固まる前に retrieval flow を書き換えると整合性が崩れやすいためである。[cite:35]

## Step 4: 一貫性チェック

以下を必ず確認すること。[cite:35]

- どこにも `workflowdesignembedding` が mandatory retrieval path として残っていないか。[cite:35]
- `QueryRepresentation` と `MemoizedGraph` のフィールド定義が本文説明と一致しているか。[cite:35]
- Stage numbering が全章で一致しているか。[cite:35]
- cheap GED と full GED の責務が混ざっていないか。[cite:35]
- top-level only 制約が retrieval, calibration, testing の全章で一貫しているか。[cite:35]
- ApplicabilityScore の数式と説明文が一致しているか。[cite:35]
- knowledge-aware extension が Aworkflow/Afinal と矛盾していないか。[cite:35]

## Step 5: 追加すべき疑似コード

部下には次の 2 つの pseudo-code を RFC annex または implementation note に追加させること。[cite:35]

### Retrieval pseudo-code

```rust
fn retrieve_top_level_candidates(q: QueryRepresentation, repo: WorkflowRepository, k: usize) -> Vec<Candidate> {
    let c_sem = semantic_topk(&q.taskembedding, repo, K_SEM);
    let c_meta = sqlite_metadata_filter(&q.top_query_metadata, c_sem, K_META);
    let c_cheap = if c_meta.len() > CHEAPGED_ENABLE_THRESHOLD {
        cheap_ged_filter(&q.cheap_ged_signature, c_meta, K_CHEAP)
    } else {
        c_meta
    };
    let ranked = full_ged_rerank(&q, c_cheap, K_FULL);
    ranked
}
```

### Applicability pseudo-code

```rust
fn evaluate_candidate(q: &QueryRepresentation, g: &MemoizedGraph, full_ged: f32) -> ApplicabilityOutcome {
    let s_sem = cosine(&q.taskembedding, &g.taskembedding).max(0.0);
    let s_struct = (-STRUCT_GED_LAMBDA * normalize_ged(full_ged, q, g)).exp();
    let s_total = SIMILARITY_ALPHA * s_sem + (1.0 - SIMILARITY_ALPHA) * s_struct;
    let d = g.graph.aggregate_determinism(SOFTMIN_BETA);
    let t = g.trust.composite(g.provenance.clone(), g.timedecay.clone(), current_virtual_clock(), g.lastvirtualseen);
    let a_workflow = applicability(s_total, d, t);
    finalize_with_knowledge_if_needed(q, g, a_workflow)
}
```

## Step 6: 実験計画 annex の作成

RFC 本文とは別に、次の章立てで annex を追加させること。[cite:35]

1. Datasets
2. Metrics
3. Ablation study
4. Stability tests
5. Cost-model tuning
6. Drift monitoring
7. Replay and property-based testing

## 実装・検証時の注意事項

- cheap GED と full GED の双方は deterministic でなければならない。[cite:35]
- tie-break は `WorkflowGraphId` の安定順序で固定すること。[cite:35]
- cost model version は SearchTrace, SearchRunLog, TrainingRunLog に残すこと。[cite:35]
- cheap GED skip が発生した場合も、その理由（candidate count below threshold）を trace に残すこと。[cite:35]
- 下位 DAG を retrieval に使わない代わりに、post-selection explanation と patch proposal では参照してよいことを明記すること。[cite:35]
- knowledge-aware query フィールドは構造意味を変えないという既存原則を維持すること。[cite:35]

## 最終的に部下へ要求する納品物

部下には次の 5 点を納品させること。[cite:35]

1. 改訂済み RFC 草案全文。[cite:35]
2. 変更箇所一覧（旧文→新文の diff summary）。[cite:35]
3. 用語変更表。[cite:35]
4. 数式付き calibration/testing annex。[cite:35]
5. Open Questions 更新版（残課題と将来拡張、例: lower-level DAG retrieval を将来別 RFC に切り出すか）。[cite:35]

## 最終判断

今回の改訂において採用すべき公式方針は次である。[cite:35]

- **Top-level only retrieval** を正式採用する。[cite:35]
- **4 層 retrieval** を reference architecture として採用する。[cite:35]
- WorkflowDesignEmbedding / QueryDesignEmbedding は canonical text の補助 field に格下げし、normative structural retrieval から外す。[cite:35]
- 構造類似の primary metric は **top-level DAG に対する full GED 正規化類似度**とする。[cite:35]
- cheap GED は pruning 専用の lower-bound / approximate structural gate とする。[cite:35]
- 本改訂の妥当性は、recall・ranking・action quality・latency・stability の 5 軸で数学的に観測し、versioned calibration loop により最適化する。[cite:35]

以上を満たす改訂であれば、今回の RFC 改訂は、Darvium の mission-oriented workflow retrieval を、より source-of-truth に忠実で、監査可能で、replay 可能で、かつ practical な top-level DAG retrieval system として再定義できる。[cite:35]
