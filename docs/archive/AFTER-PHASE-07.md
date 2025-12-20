# Phase-08 実装計画: 生きた知識グラフ (The Living Graph)

Phase-07では「メタ認知レイヤー」の基礎を構築しました。Phase-08の目的は、このグラフを「静的なデータベース」から「動的に成長・代謝する脳」へと進化させることです。
本ドキュメントは、実装時に迷いが生じないよう、設計思想、具体的な実装手順、数学的モデル、および注意点を網羅的に記述します。

---

## 1. 厳格なデータパーティショニング (Strict Data Partitioning)

**目的**: ユーザーやデータセット間でのデータ漏洩をデータベースレベルで物理的・論理的に完全に防ぐ。`GroupID` は単なるフィルタではなく、アクセス制御の絶対的な境界線として機能させます。

### 1.1 現状の課題と解決策
- **現状**: `GetTriplets` など一部のクエリで `GroupID` の指定が不十分、またはアプリケーション層でのフィルタリングに依存している可能性がある。
- **解決策**: 全ての Datalog クエリにおいて、`group_id` を必須の結合条件として強制する。

### 1.2 実装詳細 (CozoDB Datalog)
CozoDBのクエリは以下のように厳格化します。

**悪い例 (Bad):**
```datalog
?[id, type] := *nodes[id, group_id, type, _], group_id = $group_id
```
※ これは一見良さそうですが、インデックスの使用効率や、将来的なマルチテナント構成での安全性において不十分な場合があります。

**良い例 (Good - 複合キー/インデックス活用):**
スキーマ定義で `group_id` をプライマリキーの一部、または第一インデックスとして設計します（現状のスキーマ確認が必要）。
クエリ内では常に以下のように記述します。

```datalog
# $group_id はクエリパラメータとしてバインド
?[id, type, props] := *nodes[id, $group_id, type, props]
```

### 1.3 `CozoStorage` 改修計画
`src/pkg/cognee/db/cozodb/cozo_storage.go` の全メソッドを監査し、以下のルールを適用します。

1.  **メソッドシグネチャ**: 全ての Public メソッドは `groupID` を引数に取る。
2.  **Delete操作**: `DeleteEdge`, `DeleteNode` は `id` だけでなく `group_id` も一致する場合のみ削除を実行する（誤って他人の同IDデータを消さないため）。
3.  **Cross-Group Access**: 異なる `group_id` をまたぐクエリ（例: 全体統計など）は、明示的に「管理者モード」のようなフラグがない限り禁止する。

---

## 2. 本番レベルの知識結晶化 (Production-Ready Crystallization)

**目的**: `O(N^2)` の計算量を持つ現在のプロトタイプ実装を、数万・数百万ノード規模でも動作するスケーラブルな実装へ昇華させる。

### 2.1 アルゴリズムの刷新: ベクトル検索ベースのクラスタリング
全ノード対の比較を廃止し、近似近傍探索 (ANN) を活用します。

**手順:**
1.  **候補抽出 (Retrieval)**:
    *   対象となる各「ルール」ノードについて、`VectorStorage` を使用して Top-K (例: K=10) の類似ノードを検索する。
    *   *Complexity*: `O(N * log N)` (HNSWインデックス使用時)
2.  **グラフ構築 (Graph Construction)**:
    *   検索結果に基づき、一時的な「類似度グラフ」をメモリ上に構築する。
    *   エッジの重みは「類似度」。閾値（例: 0.8）以下のエッジは張らない。
3.  **コミュニティ検出 (Community Detection)**:
    *   構築したグラフから連結成分 (Connected Components) または ルーバン法 (Louvain Method) を用いてクラスタを検出する。
    *   これにより、「AとBが似ている」「BとCが似ている」→「A, B, Cは同じクラスタ」という推移的なグループ化が可能になる。

### 2.2 アトミックなグラフ更新 (Atomic Graph Updates)
結晶化（統合）プロセスは破壊的な変更を伴うため、整合性を保つ必要があります。

**トランザクションフロー:**
1.  **統合ノード作成**:
    *   LLMにより統合されたテキストを持つ新しいノード `C` を作成。
    *   `properties` に `source_node_ids: [A, B]` を記録（トレーサビリティ）。
2.  **エッジの付け替え (Re-wiring)**:
    *   **Inbound**: `X -> A`, `Y -> B` というエッジがある場合、`X -> C`, `Y -> C` を作成。
    *   **Outbound**: `A -> Z`, `B -> W` というエッジがある場合、`C -> Z`, `C -> W` を作成。
    *   *重複排除*: `X -> A` と `X -> B` が両方ある場合、`X -> C` は1本だけ作成し、Weightを強化（加算またはMAX）する。
3.  **旧ノードの処理**:
    *   ノード `A`, `B` を即座に削除するのではなく、`status: "crystallized"` または `merged_into: "C"` というタグを付けて「論理削除」状態にする。
    *   物理削除は後述の「ガベージコレクション」に任せる（安全性のため）。

---

## 3. グラフ・ハイジーン (Graph Hygiene: 衛生管理)

**目的**: グラフの「エントロピー増大」を防ぎ、常にノイズの少ない、シャープな状態を維持する。

### 3.1 孤立ノード (Orphan Nodes) の定義と削除
「孤立」とは何かを厳密に定義します。

*   **定義**: `InDegree == 0` AND `OutDegree == 0` であるノード。
*   **例外 (削除してはいけないもの)**:
    *   **新規ノード**: 作成されてから一定時間（例: 1時間）以内のノード。まだエッジが張られる前の可能性がある。
    *   **Source Document**: 元データ（DocumentChunkなど）を表すノードは、グラフの起点であるため孤立していても保持する（または別管理）。
    *   **Root Concepts**: オントロジーの頂点として定義された固定ノード。

### 3.2 実装計画: `PruningTask`
`src/pkg/cognee/tasks/metacognition/pruning_task.go` を新規作成します。

```go
type PruningTask struct {
    // ... dependencies
    GracePeriod time.Duration // 猶予期間 (Default: 1h)
}

func (t *PruningTask) PruneOrphans(ctx context.Context) error {
    // 1. 全ノードの次数を取得 (CozoDB aggregation query)
    // 2. 次数0 かつ CreatedAt < (Now - GracePeriod) のノードを特定
    // 3. 一括削除
}
```

---

## 4. グラフ・リファインメントの統合 (Graph Refinement Integration)

**目的**: `GraphRefinementTask` を `Memify` パイプラインの標準工程として組み込み、情報の「矛盾解消」と「強化」を自動化する。

### 4.1 パイプラインへの組み込み
現在の `RecursiveMemify` のフローを以下のように拡張します。

1.  **Rule Extraction**: ドキュメントから新しいルール（仮説）を抽出。
2.  **Graph Refinement (NEW)**:
    *   抽出された新ルールと、既存のグラフ知識を突き合わせる。
    *   矛盾があれば、既存のエッジを `Weaken` または `Delete`。
    *   一致すれば、既存のエッジを `Strengthen`。
3.  **Crystallization**: 類似したノードを統合。
4.  **Pruning**: 不要なノードを削除。

### 4.2 検索戦略 (Search Strategy)
「どのエッジを再評価すべきか」を特定するロジックが重要です。

*   **Embedding Search**: 新しいルールのEmbeddingを用いて、既存の `Edge`（の周辺ノードのテキスト）を検索する。
*   **Keyword Matching**: ルール内の固有名詞（Entity）が含まれるノードに接続されたエッジを抽出する。

---

## 5. グラフの代謝モデル (Graph Metabolism)

**目的**: 情報の「鮮度」と「確からしさ」を数学的に管理し、質の低い情報を自動的に淘汰する仕組み（代謝）を実装する。

### 5.1 数学的モデル定義

エッジ $e$ は以下の2つの動的パラメータを持つ。

1.  **Semantic Importance ($W \in [0, 1]$)**:
    *   その関係性がどれほど重要か。
    *   頻繁に参照される、または多くの推論パスに含まれるほど高くなる。
2.  **Confidence ($C \in [0, 1]$)**:
    *   その情報が真実である確率（確信度）。
    *   検証（Refinement）されるたびに更新される。

**生存スコア (Survival Score $S$):**
$$ S = W \times C $$

### 5.2 更新ロジック (Dynamics)

#### A. 初期化 (Initialization)
*   **Fact (事実)**: ドキュメントから直接抽出された明示的な関係。
    *   $C_0 = 0.9, W_0 = 0.5$
*   **Hypothesis (仮説)**: 推論によって導かれた関係。
    *   $C_0 = 0.6, W_0 = 0.5$

#### B. 強化 (Reinforcement)
新しい証拠が既存のエッジを支持した場合：
$$ C_{new} = C_{old} + (1 - C_{old}) \times \alpha $$
$$ W_{new} = W_{old} + (1 - W_{old}) \times \beta $$
*   $\alpha$: 学習率 (例: 0.2) - 確信度の上昇幅
*   $\beta$: 重要度ブースト (例: 0.1)

#### C. 減衰 (Decay / Weakening)
新しい証拠と矛盾した場合、または長期間アクセスがない場合：
$$ C_{new} = C_{old} \times (1 - \delta) $$
*   $\delta$: ペナルティ係数 (例: 0.3)

#### D. 淘汰 (Pruning)
$$ S < T_{prune} \implies \text{Delete Edge} $$
*   $T_{prune}$: 淘汰閾値 (例: 0.1)

### 5.3 実装への落とし込み
`src/pkg/cognee/storage/interfaces.go` の `Edge` 構造体は既に `Weight`, `Confidence` を持っています。
これらを操作するためのメソッド `UpdateEdgeMetrics` を `GraphStorage` に追加し、`GraphRefinementTask` 内で上記の数式を適用します。

```go
// 擬似コード
func (t *GraphRefinementTask) updateMetrics(edge *Edge, action string) {
    const alpha = 0.2
    const delta = 0.3
    
    switch action {
    case "strengthen":
        edge.Confidence += (1.0 - edge.Confidence) * alpha
    case "weaken":
        edge.Confidence *= (1.0 - delta)
    }
    
    survivalScore := edge.Weight * edge.Confidence
    if survivalScore < t.PruneThreshold {
        t.GraphStorage.DeleteEdge(...)
    } else {
        t.GraphStorage.UpdateEdgeMetrics(...)
    }
}
```

---

## 6. 設定値管理 (Configuration Management)

**重要**: Phase-08で導入する全てのパラメータ（閾値、係数、期間など）は、ハードコーディングせず、必ず環境変数として管理してください。

### ルール
1.  **環境変数化**: 設定値は全て環境変数から読み込む。
2.  **ドキュメント化**: `src/.env` および `src/.env.example` に、その変数の意味、デフォルト値、推奨設定などを**詳細な日本語コメント**と共に追記する。
3.  **構造体への追加**: `src/pkg/cognee/cognee.go` の `CogneeConfig` 構造体にフィールドを追加し、`main.go` で読み込む。

### Phase-08で追加が見込まれる設定値の例
*   `COGNEE_GRAPH_PRUNING_GRACE_PERIOD` (孤立ノード削除の猶予期間)
*   `COGNEE_GRAPH_METABOLISM_ALPHA` (強化学習率)
*   `COGNEE_GRAPH_METABOLISM_BETA` (重要度ブースト率)
*   `COGNEE_GRAPH_METABOLISM_DELTA` (減衰ペナルティ率)
*   `COGNEE_GRAPH_METABOLISM_PRUNE_THRESHOLD` (淘汰閾値)

---

## Phase-08 実装チェックリスト

### 1. Storage Layer
- [ ] `CozoStorage` の全メソッドに `group_id` フィルタを適用。
- [ ] `UpdateEdgeMetrics` (Weight/Confidence更新) の実装。
- [ ] `DeleteNode` (論理削除/物理削除) の実装。

### 2. Metacognition Layer
- [ ] **Crystallization**: ベクトル検索を用いたクラスタリングの実装。
- [ ] **Crystallization**: ノード統合時のエッジ付け替えロジックの実装。
- [ ] **GraphRefinement**: 新ルールに基づくエッジ評価と、上記の「代謝モデル」の適用。
- [ ] **Pruning**: `PruningTask` の新規作成と、孤立ノード削除ロジックの実装。

### 3. Pipeline Integration
- [ ] `RecursiveMemify` フローへの `Refinement` -> `Crystallization` -> `Pruning` の順序での統合。
- [ ] 各タスクの実行ログ（統計情報）の詳細化。

### 4. Verification
- [ ] テストデータを用いた「成長」と「淘汰」のシミュレーション。
- [ ] 大規模データ（数千ノード）でのパフォーマンス検証。
