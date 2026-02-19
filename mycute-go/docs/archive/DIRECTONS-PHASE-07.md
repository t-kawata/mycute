# Cognee Go Implementation: Phase-07 Detailed Development Directives

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-07: Metacognition & Self-Growth** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。
`docs/AFTER-PHASE-06.md` で策定された設計に基づき、具体的なコード、ファイルパス、手順を網羅しています。

> [!IMPORTANT]
> **実装のゴール**
> エージェントに「無知の知（Metacognition of Ignorance）」を持たせ、自律的に知識の欠落を埋め、成長するサイクル（Self-Growth Loop）を実現すること。

---

## 1. 実装ステップ一覧 (Implementation Steps)

以下の順序で実装を進めます。各ステップは依存関係に基づいています。

1.  **Storage Layer Extensions**: グラフ構造の拡張（重み、信頼度）とクエリ機能の強化。
2.  **Ignorance Manager**: 「知らないこと(Unknown)」と「できること(Capability)」の管理基盤。
3.  **Self-Reflection Task**: 自問自答による知識検証ループ。
4.  **Recursive Memify**: Unknown解決を優先した再帰的知識抽出。
5.  **Knowledge Crystallization**: 知識の統合と抽象化。
6.  **Graph Refinement**: 知識グラフの剪定と最適化。
7.  **CLI & Integration**: コマンドラインインターフェースへの統合。

---

## Step 1: Storage Layer Extensions

エッジに「重み」と「信頼度」を追加し、グラフの動的な調整を可能にします。また、特定のノードやエッジを効率的に取得するためのメソッドを追加します。

### 1.1 `src/pkg/cognee/storage/interfaces.go` の修正

**目的**: `Edge` 構造体へのフィールド追加と `GraphStorage` インターフェースの拡張。

```go
// [MODIFY] Edge struct
type Edge struct {
	SourceID   string                 `json:"source_id"`
	TargetID   string                 `json:"target_id"`
	GroupID    string                 `json:"group_id"`
	Type       string                 `json:"type"`
	Properties map[string]any `json:"properties"`
	Weight     float64                `json:"weight"`     // [NEW] エッジの重み（0.0〜1.0）
	Confidence float64                `json:"confidence"` // [NEW] 信頼度（0.0〜1.0）
}

// [MODIFY] GraphStorage interface
type GraphStorage interface {
	// ... (既存メソッド) ...

	// [NEW] 指定されたタイプのノードを取得
	GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*Node, error)

	// [NEW] 指定されたエッジタイプでターゲットに接続されたノードを取得
	GetNodesByEdge(ctx context.Context, targetID string, edgeType string, groupID string) ([]*Node, error)

	// [NEW] エッジの重みを更新
	UpdateEdgeWeight(ctx context.Context, sourceID, targetID, groupID string, weight float64) error

	// [NEW] エッジを削除
	DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error

	// [NEW] 指定されたノードに接続されたエッジを取得
	GetEdgesByNode(ctx context.Context, nodeID string, groupID string) ([]*Edge, error)
}
```

### 1.2 `src/pkg/cognee/db/cozodb/cozo_storage.go` の修正

**目的**: 新しいインターフェースメソッドの実装。

```go
// [NEW] GetNodesByType
func (s *CozoStorage) GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*storage.Node, error) {
	query := `
		?[id, group_id, type, properties] := 
			*nodes[id, group_id, type, properties],
			group_id = $group_id,
			type = $type
	`
	params := map[string]any{"group_id": groupID, "type": nodeType}
	// ... (Run query & parse results similar to GetTriplets) ...
    // ヒント: 結果のパースロジックは GetTriplets 内のノード取得部分を再利用・共通化できますが、
    // ここでは独立して実装しても構いません。
}

// [NEW] GetNodesByEdge
func (s *CozoStorage) GetNodesByEdge(ctx context.Context, targetID string, edgeType string, groupID string) ([]*storage.Node, error) {
	// targetIDに向かう edgeType のエッジを持つ sourceID を取得し、その sourceID のノード情報を取得
	query := `
		?[id, group_id, type, properties] := 
			*edges[source_id, target_id, group_id, edge_type, _],
			target_id = $target_id,
			edge_type = $edge_type,
			group_id = $group_id,
			*nodes[id, group_id, type, properties],
			id = source_id
	`
	params := map[string]any{
		"target_id": targetID,
		"edge_type": edgeType,
		"group_id":  groupID,
	}
	// ... (Run query & parse results) ...
}

// [NEW] UpdateEdgeWeight
func (s *CozoStorage) UpdateEdgeWeight(ctx context.Context, sourceID, targetID, groupID string, weight float64) error {
	// CozoDBでは既存レコードの特定フィールド更新は :update を使用
    // ただし、edgesは [source_id, target_id, group_id, type] がキーであることに注意
    // ここでは簡略化のため、既存のエッジを取得して weight を書き換えて :put (upsert) するか、
    // クエリで更新を行います。
    
    // クエリ例:
    // ?[source_id, target_id, group_id, type, properties] := *edges[source_id, target_id, group_id, type, props], ...
    // propsを更新して :put
    
    // 実装推奨:
    // 1. エッジを取得
    // 2. properties["weight"] = weight (またはカラムとして追加した場合はそのカラム)
    // 3. :put で上書き
    // ※ Edge構造体にWeightフィールドを追加しましたが、CozoDBスキーマ上は properties JSON内に含めるか、
    //   スキーマ変更してカラムを追加するか選択が必要です。
    //   Phase-07では **properties JSON内に "weight" キーとして保存** する方針とします。
    //   よって、Edge構造体のWeightフィールドは、読み書き時に properties["weight"] とマッピングします。
}

// [NEW] DeleteEdge
func (s *CozoStorage) DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error {
	// :rm edges {source_id, target_id, group_id, type}
    // typeが必要なため、削除対象のエッジタイプを特定する必要があります。
    // 引数に type がないため、sourceID, targetID, groupID に一致する全エッジを削除します。
    
    query := `
        ?[source_id, target_id, group_id, type] := 
            *edges[source_id, target_id, group_id, type, _],
            source_id = $source_id,
            target_id = $target_id,
            group_id = $group_id
        :rm edges {source_id, target_id, group_id, type}
    `
    // ...
}

// [NEW] GetEdgesByNode
func (s *CozoStorage) GetEdgesByNode(ctx context.Context, nodeID string, groupID string) ([]*storage.Edge, error) {
    // source_id または target_id が nodeID に一致するエッジを取得
    query := `
        ?[source_id, target_id, group_id, type, properties] := 
            *edges[source_id, target_id, group_id, type, properties],
            (source_id = $node_id or target_id = $node_id),
            group_id = $group_id
    `
    // ...
}
```

---

## Step 2: Ignorance Manager Implementation

「無知」と「能力」を管理する中核コンポーネントです。

### 2.1 `src/pkg/cognee/tasks/metacognition/ignorance_manager.go` の作成

**目的**: `Unknown` と `Capability` の登録・管理ロジックの実装。

**コード**: `docs/AFTER-PHASE-06.md` の **2.3 実装詳細** (lines 99-329) を完全にコピーして実装してください。
*   `RegisterUnknown`: `resolution_requirement` をプロパティに含める。
*   `RegisterCapability`: 複数の `learned_from` エッジを作成する。
*   `CheckAndResolveUnknowns`: ベクトル検索で類似Unknownを探し、解決判定を行う。

### 2.2 `src/pkg/cognee/prompts/prompts.go` の修正

**目的**: メタ認知用プロンプトの追加。

**コード**: `docs/AFTER-PHASE-06.md` の **2.5 プロンプト追加** (lines 380-415) を追加してください。
*   `UnknownDetectionSystemPrompt`
*   `CapabilityGenerationSystemPrompt`

---

## Step 3: Self-Reflection Task Implementation

自律的に問いを生成し、知識の検証を行うタスクです。

### 3.1 `src/pkg/cognee/tasks/metacognition/self_reflection_task.go` の作成

**目的**: 自問自答ループの実装。

**コード**: `docs/AFTER-PHASE-06.md` の **4.2 実装詳細** (lines 532-769) を完全にコピーして実装してください。
*   `Run`: ルールから問いを生成し、回答を試みるメインループ。
*   `tryAnswer`: 検索とLLM回答生成、不確実性判定 (`containsUncertainty`)。

### 3.2 `src/pkg/cognee/prompts/prompts.go` の修正

**目的**: 問い生成用プロンプトの追加。

**コード**: `docs/AFTER-PHASE-06.md` の **4.3 プロンプト追加** (lines 776-794) を追加してください。
*   `QuestionGenerationSystemPrompt`

---

## Step 4: Recursive Memify Implementation

Unknown解決を優先した再帰的知識抽出の実装。

### 4.1 `src/pkg/cognee/cognee.go` の修正

**目的**: `RecursiveMemify` メソッドの実装と `MemifyConfig` の拡張。

**コード**: `docs/AFTER-PHASE-06.md` の **3.2 実装詳細** (lines 429-487) を参考に実装してください。
*   `MemifyConfig` に `RecursiveDepth`, `PrioritizeUnknowns` を追加。
*   `RecursiveMemify` メソッドを追加。
    *   **Phase A**: `IgnoranceManager` を使って未解決Unknownを取得し、`attemptToResolveUnknown` (要実装) を呼び出す。
    *   **Phase B**: 既存の `Memify` ロジック（または再帰的な呼び出し）を実行。

**注意**: `attemptToResolveUnknown` の実装が必要です。
```go
func (s *CogneeService) attemptToResolveUnknown(ctx context.Context, unknown *metacognition.Unknown, groupID string) error {
    // 1. Unknown.Text をクエリとして Search を実行
    // 2. 関連情報が見つかれば、SelfReflectionTask.tryAnswer のようなロジックで回答生成を試みる
    // 3. 回答できれば RegisterCapability を呼び出し、Unknown を解決済みとする（削除またはフラグ更新）
    //    ※ Unknownノードに "resolved": true プロパティを追加する等の対応が必要
    return nil
}
```

---

## Step 5: Knowledge Crystallization Implementation

知識の統合と抽象化。

### 5.1 `src/pkg/cognee/tasks/metacognition/crystallization_task.go` の作成

**目的**: 類似ルールの統合タスク。

**コード**: `docs/AFTER-PHASE-06.md` の **5.2 実装詳細** (lines 808-960) をコピーして実装してください。
*   `CrystallizeRules`: 全ルール取得 -> クラスタリング -> 統合 -> 保存。

### 5.2 `src/pkg/cognee/prompts/prompts.go` の修正

**目的**: 知識統合用プロンプトの追加。

**コード**: `docs/AFTER-PHASE-06.md` の **5.3 プロンプト追加** (lines 965-975) を追加してください。
*   `KnowledgeCrystallizationSystemPrompt`

---

## Step 6: Graph Refinement Implementation

グラフの最適化。

### 6.1 `src/pkg/cognee/tasks/metacognition/graph_refinement_task.go` の作成

**目的**: エッジ再評価タスク。

**コード**: `docs/AFTER-PHASE-06.md` の **6.4 実装詳細** (lines 1024-1145) をコピーして実装してください。
*   `RefineEdges`: エッジ評価 -> 重み更新/削除。

### 6.2 `src/pkg/cognee/prompts/prompts.go` の修正

**目的**: エッジ評価用プロンプトの追加。

**コード**: `docs/AFTER-PHASE-06.md` の **6.5 プロンプト追加** (lines 1150-1166) を追加してください。
*   `EdgeEvaluationSystemPrompt`

---

## Step 7: CLI & Integration

### 7.1 `src/main.go` の修正

**目的**: `memify` コマンドへのフラグ追加。

```go
// memifyCmd の定義部分
var memifyCmd = &cobra.Command{
    Use:   "memify",
    Short: "Enhance knowledge graph using Memify",
    Run: func(cmd *cobra.Command, args []string) {
        // ...
        recursive, _ := cmd.Flags().GetBool("recursive")
        depth, _ := cmd.Flags().GetInt("depth")
        selfReflect, _ := cmd.Flags().GetBool("self-reflect")
        crystallize, _ := cmd.Flags().GetBool("crystallize")
        refine, _ := cmd.Flags().GetBool("refine")

        config := &cognee.MemifyConfig{
            // ...
            RecursiveDepth: depth,
            EnableRecursive: recursive,
            PrioritizeUnknowns: true, // デフォルト有効
        }

        // 実行ロジックの分岐
        if recursive {
            s.RecursiveMemify(ctx, dataset, user, config)
        } else {
            s.Memify(ctx, dataset, user, config)
        }
        
        // オプションタスクの実行
        if selfReflect {
            // SelfReflectionTask 実行
        }
        if crystallize {
            // CrystallizationTask 実行
        }
        if refine {
            // GraphRefinementTask 実行
        }
    },
}

func init() {
    // フラグ登録
    memifyCmd.Flags().Bool("recursive", false, "Enable recursive memify")
    memifyCmd.Flags().Int("depth", 1, "Recursive depth")
    memifyCmd.Flags().Bool("self-reflect", false, "Run self-reflection loop")
    memifyCmd.Flags().Bool("crystallize", false, "Run knowledge crystallization")
    memifyCmd.Flags().Bool("refine", false, "Run graph refinement")
}
```

---

## 8. 詳細な検証計画 (Detailed Verification Plan)

各機能の実装後、以下の手順で動作確認と成否判断を行います。

### 8.1 提案E: 無知の知 (Metacognition of Ignorance)

**検証対象**: `IgnoranceManager`, `Unknown/Capability` ノード生成

**テスト手順**:
1.  **データ投入**: `make run ARGS="add test_data/metacognition_test.txt"` (未知の概念を含むテキスト)
2.  **Cognify実行**: `make run ARGS="cognify"`
3.  **確認**: CozoDBでノードを確認。

**成否判断基準**:
*   [ ] **成功**: `Unknown` ノードが生成されていること。
    *   クエリ: `?[id, text] := *nodes[id, _, "Unknown", props], text = get(props, "text", "")`
    *   期待値: テキスト内の不明瞭な点や未定義語が `text` に含まれている。
*   [ ] **成功**: `Capability` ノードが生成されていること。
    *   クエリ: `?[id, text] := *nodes[id, _, "Capability", props], text = get(props, "text", "")`
    *   期待値: "〜について理解した" 等のテキストが含まれている。

### 8.2 提案A: 再帰的Memify (Recursive Memify)

**検証対象**: `RecursiveMemify`, `PrioritizeUnknowns` ロジック

**テスト手順**:
1.  **Unknown準備**: (8.1でUnknownがある状態にする)
2.  **コマンド実行**: `make run ARGS="memify --recursive --depth 2"`
3.  **ログ確認**: 標準出力で処理フェーズを確認。

**成否判断基準**:
*   [ ] **成功**: ログに `RecursiveMemify: Phase A - Prioritizing Unknown Resolution` が表示される。
*   [ ] **成功**: ログに `RecursiveMemify: Phase B - General Graph Expansion` が表示される。
*   [ ] **成功**: レベル別のルール抽出が行われている。
    *   ログに `Processing Level 1`, `Processing Level 2` 等の表示がある。

### 8.3 提案B: 自問自答ループ (Self-Reflection Loop)

**検証対象**: `SelfReflectionTask`, 問い生成と回答

**テスト手順**:
1.  **コマンド実行**: `make run ARGS="memify --self-reflect"`
2.  **ログ確認**: 生成された問いと回答のログを確認。
3.  **DB確認**: 新たな `Unknown` または `Capability` の生成を確認。

**成否判断基準**:
*   [ ] **成功**: ログに `SelfReflectionTask: Generated X questions` と表示され、具体的な質問文（日本語）が出力されている。
*   [ ] **成功**: 質問に対して回答できた場合、`Capability` ノードが増加している。
*   [ ] **成功**: 質問に対して回答できなかった場合、`Unknown` ノードが増加している。

### 8.4 提案C: 知識の結晶化 (Knowledge Crystallization)

**検証対象**: `CrystallizationTask`, ルール統合

**テスト手順**:
1.  **準備**: 類似した内容を含む複数のテキストを追加し、`memify` を数回実行して類似ルールを生成させておく。
2.  **コマンド実行**: `make run ARGS="memify --crystallize"`
3.  **DB確認**: `CrystallizedRule` ノードの確認。

**成否判断基準**:
*   [ ] **成功**: ログに `CrystallizationTask: Crystallized X rules into 1` が表示される。
*   [ ] **成功**: CozoDBに `CrystallizedRule` タイプのノードが存在する。
    *   クエリ: `?[id, text] := *nodes[id, _, "CrystallizedRule", props], text = get(props, "text", "")`
    *   期待値: 複数のルールを要約したような包括的なテキストになっている。

### 8.5 提案D: グラフプルーニングと重み付け (Graph Pruning & Reweighting)

**検証対象**: `GraphRefinementTask`, エッジ重み更新

**テスト手順**:
1.  **準備**: 矛盾する情報や補強する情報を追加し、`memify` を実行。
2.  **コマンド実行**: `make run ARGS="memify --refine"`
3.  **DB確認**: エッジの `weight` プロパティを確認。

**成否判断基準**:
*   [ ] **成功**: ログに `GraphRefinementTask: Updated weight for edge ...` または `Deleted edge ...` が表示される。
*   [ ] **成功**: CozoDBのエッジプロパティで `weight` が更新されている（デフォルトの1.0以外になっている、または削除されている）。
    *   クエリ: `?[source, target, weight] := *edges[source, target, _, _, props], weight = get(props, "weight", 1.0)`

---

## 9. 注意事項 (Caveats)

*   **JSONパース**: LLMからのJSON応答は不安定な場合があります。`extractJSON` ユーティリティを必ず使用してください。
*   **CozoDBクエリ**: Datalogクエリの文字列結合時は、シングルクォートのエスケープ (`strings.ReplaceAll(s, "'", "\\'")`) を忘れないでください。
*   **日本語出力**: プロンプトには必ず `Output in Japanese` を含め、システム全体で言語を統一してください。
*   **APIコスト**: 自問自答や再帰処理はLLM呼び出し回数が増えるため、開発中は `RecursiveDepth` を小さく（1程度）設定することを推奨します。
