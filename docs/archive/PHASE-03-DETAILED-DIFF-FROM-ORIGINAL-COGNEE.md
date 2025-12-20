# Phase 03 Detailed Diff from Original Cognee

本ドキュメントは、Phase-03完了時点での、Go言語版Cogneeコアクローン実装（`src`内）と、オリジナルのPython版Cognee実装（`cognee`内）の `add`, `cognify`, `search` 機能における詳細な比較分析レポートです。

目的：
1.  オリジナルPython版の実装ロジックとGo版の実装ロジックの差異を網羅的に洗い出す。
2.  Phase-01〜Phase-03の開発指示書に基づき、実装漏れや不整合がないか確認する。
3.  "完全クローン" および "日本語最適化" の観点から、現在の到達度を評価する。


## 1. Add Functionality (Ingestion)

### 1.1 Python 実装 (`cognee/api/v1/add/add.py`)

Python版の `add` は、多様なデータソースとフォーマットに対応する高機能なエントリポイントです。

**処理フロー:**
1.  **入力の正規化**: テキスト、ファイルパス、URL、S3パス、バイナリストリームなど多様な入力を `preferred_loaders` と共に処理可能な形式（`data`）に変換します。
2.  **セットアップ**: `setup()` でDB接続などを初期化します。
3.  **権限解決 (`resolve_authorized_user_dataset`)**: 指定された `dataset_name` と `user` に基づき、データセットの存在確認とユーザーのアクセス権限を確認・作成します。
4.  **パイプライン実行 (`run_pipeline`)**:
    - `resolve_data_directories`: ディレクトリが指定された場合の再帰的なファイル探索。
    - `ingest_data`: データの実質的な取り込み。

**コードスニペット (Python):**
```python
# cognee/api/v1/add/add.py
async def add(data, dataset_name="main_dataset", user=None, ...):
    # ... (Tasks definition)
    tasks = [
        Task(resolve_data_directories, include_subdirectories=True),
        Task(ingest_data, dataset_name, user, ...),
    ]
    # ... (Permission check)
    user, authorized_dataset = await resolve_authorized_user_dataset(...)
    # ... (Pipeline execution)
    async for run_info in run_pipeline(tasks=tasks, ...):
        pipeline_run_info = run_info
    return pipeline_run_info
```

### 1.2 Go 実装 (`src/pkg/cognee/cognee.go`, `src/pkg/cognee/tasks/ingestion/ingest_task.go`)

Go版の `Add` は、ローカルファイルの取り込みに特化した実装になっています。

**処理フロー:**
1.  **タスク生成**: `ingestion.NewIngestTask` を作成します。
2.  **パイプライン生成**: `pipeline.NewPipeline` でラップします。
3.  **パイプライン実行**:
    - **ハッシュ計算**: `calculateFileHash` (SHA256) でファイルのコンテンツハッシュを計算。
    - **重複チェック**: `t.vectorStorage.Exists(hash)` でDuckDB内の存在を確認。存在すればスキップ。
    - **Dataオブジェクト作成**: `uuid.New()` でIDを生成し、メタデータ（パス、拡張子など）を設定。
    - **保存**: `t.vectorStorage.SaveData` でDuckDBの `data` テーブルに保存。

**コードスニペット (Go):**
```go
// src/pkg/cognee/cognee.go
func (s *CogneeService) Add(ctx context.Context, filePaths []string, dataset string, user string) error {
    ingestTask := ingestion.NewIngestTask(s.VectorStorage)
    p := pipeline.NewPipeline([]pipeline.Task{ingestTask})
    _, err := p.Run(ctx, filePaths)
    return err
}

// src/pkg/cognee/tasks/ingestion/ingest_task.go
func (t *IngestTask) Run(ctx context.Context, input any) (any, error) {
    // ...
    for _, path := range filePaths {
        hash, err := calculateFileHash(path)
        if t.vectorStorage.Exists(ctx, hash) {
            continue // 重複スキップ
        }
        data := &storage.Data{
            ID:          uuid.New().String(), // ランダムUUID
            ContentHash: hash,
            // ...
        }
        if err := t.vectorStorage.SaveData(ctx, data); err != nil {
             return nil, err
        }
        dataList = append(dataList, data)
    }
    return dataList, nil
}
```

### 1.3 差異分析 (Add)

| 項目 | Python (Original) | Go (Current Clone) | 評価 |
| :--- | :--- | :--- | :--- |
| **入力ソース** | File, URL, S3, Text, Binary | FilePath (`[]string`) のみ | **△** (スコープ限定) |
| **権限管理** | RBAC (`resolve_authorized_user_dataset`) | 引数 `user` はあるがロジック未実装 | **△** (Phase 2要件外だが将来必要) |
| **ID生成** | UUID5 (Deterministic based on Hash) | UUID4 (Random) | **△** (冪等性に課題あり) |
| **重複排除** | あり (Content Hash) | あり (Content Hash + DuckDB Check) | **◯** (実装済み) |
| **保存先** | Graph DB & Vector DB (Meta) | DuckDB (`data` Table) | **◯** (アーキテクチャ通りの実装) |

**結論 (Add)**:
Go版は「ローカルファイルを処理する」というPhase-02のスコープにおいては正常に機能しています。しかし、完全なクローンを目指す場合は、URLやテキスト直接入力への対応、およびID生成の決定論的ロジック（UUID5）への変更が推奨されます。

**※プロジェクト方針 (Input Source)**:
本プロジェクトでは入力ソースを「テキスト」に完全に絞ります。「テキストになるまで（PDF解析など）」は別レイヤーで実装する方針であるため、Go版での多様なローダー実装は不要です。


## 2. Cognify Functionality (Graph Construction)

### 2.1 Python 実装 (`cognee/api/v1/cognify/cognify.py`)

Python版の `cognify` は、動的かつ高度に構成可能なパイプライン構築関数です。

**処理フロー:**
1.  **タスク構成 (`get_default_tasks`)**: 設定に基づきタスクリストを動的に生成します。
    - `classify_documents`: ドキュメント分類。
    - `check_permissions_on_dataset`: 権限確認。
    - `extract_chunks_from_documents`: チャンキング。
    - `extract_graph_from_data`: グラフ抽出（Pydanticモデル使用）。
    - `summarize_text`: 要約生成。
    - `add_data_points`: データの保存とインデックス作成。
2.  **パイプライン実行**: バックグラウンド処理やバッチ処理（`chunks_per_batch`）をサポート。

**コードスニペット (Python):**
```python
# cognee/api/v1/cognify/cognify.py
async def cognify(..., graph_model=KnowledgeGraph, ...):
    tasks = await get_default_tasks(...)
    # default_tasks include:
    # Task(classify_documents)
    # Task(extract_chunks_from_documents, ...)
    # Task(extract_graph_from_data, ...)
    # Task(add_data_points, ...)
    pipeline_executor_func = get_pipeline_executor(...)
    return await pipeline_executor_func(pipeline=run_pipeline, tasks=tasks, ...)
```

### 2.2 Go 実装 (`src/pkg/cognee/cognee.go`, `src/pkg/cognee/tasks/...`)

Go版の `Cognify` は、固定化されたステップを実行するパイプラインです。

**処理フロー:**
1.  **タスク初期化**:
    - `ChunkingTask`: 日本語最適化済みのチャンキング。チャンク埋め込み生成もここで行う。
    - `GraphExtractionTask`: LLMを用いたプロンプトベースのグラフ抽出。
    - `StorageTask`: チャンクとグラフの保存、および**ノード埋め込みの生成と保存 (Phase 3追加)**。
2.  **パイプライン実行**: DataListを入力として順次実行。

**コードスニペット (Go):**
```go
// src/pkg/cognee/cognee.go
func (s *CogneeService) Cognify(...) error {
    chunkingTask, _ := chunking.NewChunkingTask(...)
    graphTask := graph.NewGraphExtractionTask(llm)
    storageTask := storageTaskPkg.NewStorageTask(s.VectorStorage, s.GraphStorage, s.Embedder)

    p := pipeline.NewPipeline([]pipeline.Task{
        chunkingTask,
        graphTask,
        storageTask,
    })
    _, err = p.Run(ctx, dataList)
    return err
}

// src/pkg/cognee/tasks/storage/storage_task.go (Phase 3 Logic)
func (t *StorageTask) Run(...) {
    // ... Save Chunks & Graph ...
    // Index Nodes (Embeddings)
    for _, node := range output.GraphData.Nodes {
        embedding, _ := t.Embedder.EmbedQuery(ctx, node.Name)
        t.VectorStorage.SaveEmbedding(ctx, "Entity_name", node.ID, node.Name, embedding)
    }
}
```

### 2.3 差異分析 (Cognify)

| 項目 | Python (Original) | Go (Current Clone) | 評価 |
| :--- | :--- | :--- | :--- |
| **パイプライン構成** | 動的 (Configurable) | 固定 (Fixed) | **△** (柔軟性が低い) |
| **チャンキング** | TextChunker, Langchain | Custom (Tiktoken + Kagome) | **◯** (日本語最適化済み) |
| **グラフ抽出** | Pydantic Model + Ontology | Prompt + JSON Parse | **△** (スキーマ検証が弱い) |
| **要約機能** | あり (`summarize_text`) | なし | **✕** (要実装) |
| **ノード埋め込み** | `index_data_points` で自動処理 | `StorageTask` で明示的に処理 | **◯** (Phase 3で達成) |
| **並行処理** | AsyncIO | Goroutines (`errgroup`) | **◯** (Goの強みを活用) |

**結論 (Cognify)**:
本来のコア機能である「チャンキング」「グラフ抽出」「保存・インデックス」は実装されており、特にPhase-3でノード埋め込みが追加されたことでグラフ検索の準備が整いました。しかし、Python版にある「要約(Summarization)」や「分類(Classification)」、「Ontology対応」は未実装であり、これらは将来的なPhaseでの実装候補となります。

**※Phase-04 優先実装項目**:
分類 (Classification)、要約 (Summarization)、Ontology対応は、次期 **Phase-04 の最重要項目** として実装します。


## 6. プロジェクト方針と重要課題 (Project Policy & Major Issues)

### 6.1 入力ソースの方針
*   **Text Only Policy**: Cogneeのコア機能への入力は「テキスト」に限定します。PDFや画像などからのテキスト抽出はCognifyの責務とせず、前段の別レイヤーで処理するアーキテクチャを採用します。

### 6.2 Phase-04 ロードマップ
*   **Focus**: Phase-04 は **Summarization (要約)** の実装に完全にフォーカスします。
*   **Out of Scope**: "Document Classification" (ドキュメント分類) はスコープ外とします（入力はテキストのみに限定されるため不要）。
*   **実装要件**:
    1.  **Cognify**: グラフ構築後に `Summarization` タスクを実行する（Python版 `summarize_text` 相当）。
    2.  **Search**: Summarization の実装により、以下の新たな検索タイプを可能にする必要があります。
        *   `SUMMARIES`: 要約テキストの検索と返却。
        *   `GRAPH_SUMMARY_COMPLETION`: グラフ構造と要約を組み合わせた検索。
    3.  **Ontology Integration**: スキーマ定義に基づくグラフ抽出（引き続き実装対象）。


### 6.3 [CRITICAL] プロンプト実装の重大な欠陥 (Resolved)
*   **問題**: 現在のGo実装 (`src/pkg/cognee/prompts/prompts.go`) のプロンプトが、オリジナルPython実装と異なっています。プロンプトエンジニアリングは本プロジェクトの核であり、勝手な変更は許されません。
*   **是正措置 (完了)**:
    1.  オリジナルPython実装 (`cognee/infrastructure/llm/prompts/generate_graph_prompt.txt` 等) の内容を**そのまま**コピーして `prompts.go` に適用しました。
    2.  `prompts.go` に警告コメントを追加し、ロックしました。
    3.  `graph_extraction_task.go` および `graph_completion.go` を修正し、オリジナルプロンプト（システムプロンプト）とJSONスキーマ指示/コンテキスト（ユーザー入力）を適切に分離・結合してLLMに渡すロジックを実装しました。
*   **検証結果**:
    *   `add`, `cognify`, `search` の全パイプラインが正常に動作し、正しい日本語のグラフ抽出および回答生成が行われることを検証済みです。


