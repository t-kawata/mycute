# Phase-04 Post-Implementation Review: Detailed Architecture Comparison

## 0. はじめに

本ドキュメントは、Phase-04までの実装完了時点（2025-12-05）における、オリジナルPython版Cogneeと、現在のGo移植版Cogneeのアーキテクチャおよびロジックの完全な比較分析レポートです。
「完全なクローン」および「日本での使用への最適化」というプロジェクトのゴールに基づき、現状の差異を洗い出し、Phase-05以降の指針とします。

---

## 1. Add (Ingestion) Logic Analysis

データの取り込み (`add`) プロセスの詳細比較です。

### 1-1. Python Implementation (`cognee/api/v1/add/add.py`)

Python版は、非常に柔軟な入力対応と、厳密なメタデータ管理・権限管理を特徴としています。

**処理フロー:**

1.  **API Entry Point (`add`)**:
    *   入力データの正規化（List化）。
    *   `dataset_name` と `user` の解決。
    *   `setup()` によるDB初期化（必要に応じて）。
2.  **Dataset & User Resolution (`resolve_authorized_user_dataset`)**:
    *   ユーザーが存在しない場合はデフォルトユーザーを作成。
    *   指定されたデータセットへのアクセス権限（WRITE）を確認。
3.  **Pipeline Orchestration**:
    *   `Task` のリストを作成: `[resolve_data_directories, ingest_data]`。
    *   `run_pipeline` でパイプラインを実行。
4.  **Ingestion Logic (`cognee/tasks/ingestion/ingest_data.py`)**:
    *   **Input Iteration**: データ項目ごとにループ。
    *   **Storage**: `save_data_item_to_storage` で元ファイルを保存。
    *   **Conversion**: `data_item_to_text_file` でPDF/画像等をテキストに変換。
    *   **Metadata Extraction**: `ingestion.classify` でメタデータ（MIMEタイプ、拡張子、サイズなど）を抽出。
    *   **Identify**: `ingestion.identify` でコンテントハッシュと所有者IDから一意な `data_id` を生成（決定論的ID）。
    *   **Data Object Construction**: `Data` モデル（SQLAlchemy）を作成。
    *   **Duplicate Check**: SQLクエリで同一IDの存在を確認。
    *   **Persist**: SQLデータベースにメタデータを保存。

```python
# snippet: ingest_data.py
# 決定論的IDの生成 (Hash + OwnerID)
data_id = ingestion.identify(classified_data, user)

# DBへの保存 (SQLAlchemy)
async with db_engine.get_async_session() as session:
    if len(new_datapoints) > 0:
        dataset.data.extend(new_datapoints)
    await session.commit()
```

### 1-2. Go Implementation (`src/pkg/cognee/cognee.go` & `ingest_task.go`)

Go版は、Phase-02時点では「テキストファイルのパス」のみを扱い、簡略化されたロジックになっています。

**処理フロー:**

1.  **API Entry Point (`Add`)**:
    *   `NewIngestTask` を作成。
    *   `pipeline.NewPipeline` でパイプラインを構築。
2.  **Ingest Task Logic (`src/pkg/cognee/tasks/ingestion/ingest_task.go`)**:
    *   **Input Iteration**: ファイルパスのリストをループ。
    *   **Hash Calculation**: `calculateFileHash` (SHA-256) でファイルハッシュを計算。
    *   **Duplicate Check**: `VectorStorage.Exists` (DuckDB) でハッシュの重複を確認。
        *   ※ Python版と異なり、IDではなくハッシュ値で重複チェックしている。
    *   **Data Object Construction**: `storage.Data` 構造体を作成。
        *   **ID**: `uuid.New().String()` (ランダムUUID) を使用。**ここに差異あり**。
    *   **Persist**: `VectorStorage.SaveData` (DuckDB) に保存。

```go
// snippet: ingest_task.go
// ランダムIDの生成 (Pythonは決定論的)
data := &storage.Data{
    ID:          uuid.New().String(),
    Name:        filepath.Base(path),
    ContentHash: hash,
    // ...
}
// DuckDBへの保存
if err := t.vectorStorage.SaveData(ctx, data); err != nil {
    return nil, err
}
```

### 1-3. 差異と課題 (Gaps)

| 機能 | Python (Original) | Go (Current) | 課題・アクション |
| :--- | :--- | :--- | :--- |
| **ID生成** | **決定論的** (Content Hash + User) | **ランダム** (UUID v4) | 冪等性が低い。Python同様の決定論的ID（UUID v5 or Hash）に変更すべき。 |
| **メタデータ** | **詳細** (MIME, Classification) | **簡易** (Extensionのみ) | ファイル種別判定ロジックの実装が必要（Phase 4以降）。 |
| **データ変換** | **あり** (PDF -> Text) | **なし** (Textのみ) | `Unstructured` 連携やGoネイティブなパーサーが必要。 |
| **保存先** | **SQL (Relational)** | **DuckDB (Relational)** | 設計思想としてはOKだが、メタデータ検索の柔軟性に注意。 |
| **権限管理** | **あり** (Dataset permission) | **なし** | マルチテナント対応時に必須実装。 |

---

## 2. Cognify Logic Analysis

知識グラフ構築 (`cognify`) プロセスの詳細比較です。

### 2-1. Python Implementation (`cognee/api/v1/cognify/cognify.py`)

Python版は、タスク構成が動的であり、文書分類や権限チェックを含んだ包括的なパイプラインです。

**処理フロー:**

1.  **API Entry Point (`cognify`)**:
    *   `get_pipeline_execution_mode` で同期/非同期を決定。
    *   `get_default_tasks` でタスクリストを生成。
2.  **Task Sequence (`get_default_tasks`)**:
    1.  `classify_documents`: 文書分類。
    2.  `check_permissions_on_dataset`: 権限確認。
    3.  `extract_chunks_from_documents`: チャンク分割。
    4.  `extract_graph_from_data`: LLMによるグラフ抽出。
    5.  `summarize_text`: **要約生成 (Text Summary)**。
    6.  `add_data_points`: **データ保存 (Nodes, Edges, Summaries)**。
3.  **Execution**: `run_pipeline` で各タスクを順次実行。

```python
# snippet: cognify.py
default_tasks = [
    Task(classify_documents),
    Task(extract_chunks_from_documents, ...),
    Task(extract_graph_from_data, ...),
    Task(summarize_text, ...), # 要約生成
    Task(add_data_points, ...), # ここでまとめて保存
]
```

### 2-2. Go Implementation (`src/pkg/cognee/cognee.go`)

Go版は、Phase-04で「要約」を追加しましたが、タスク順序と責務分担がPythonとは異なります。

**処理フロー:**

1.  **API Entry Point (`Cognify`)**:
    *   パイプラインを手動で構築 (`NewPipeline`)。
2.  **Task Sequence**:
    1.  `ChunkingTask`: チャンク分割＆埋め込み生成。
    2.  `GraphExtractionTask`: LLMによるグラフ抽出。
    3.  `StorageTask`: グラフデータ（Node/Edge）とチャンクの保存。
    4.  `SummarizationTask`: **要約生成＆保存**。
3.  **Execution**: `p.Run` で実行。

```go
// snippet: cognee.go
p := pipeline.NewPipeline([]pipeline.Task{
    chunkingTask,
    graphTask,
    storageTask,       // Main Storage (Chunks/Graph)
    summarizationTask, // Summary Generation & Storage
})
```

### 2-3. 差異と課題 (Gaps)

| 機能 | Python (Original) | Go (Current) | 課題・アクション |
| :--- | :--- | :--- | :--- |
| **タスク順序** | Summarize -> Storage | Storage -> Summarize | 致命的ではないが、Pythonは「生成」と「保存」を分離している。GoはTask内で完結させている。Goの設計の方が疎結合だが、Python準拠にするなら分離が必要。 |
| **保存責務** | `add_data_points` に集約 | 各Task (`Storage`, `Summarization`) が分散して保存 | 現状のGoの方がマイクロサービス的で保守しやすい可能性があるが、トランザクション管理が難しい。 |
| **文書分類** | **あり** (`classify_documents`) | **なし** | スコープ外（Phase 4）。将来的に実装が必要。 |
| **埋め込み** | 専用ロジックで生成 | `ChunkingTask`/`SummarizationTask` 内で生成 | Goの実装は効率的。 |

---

## 3. Search Logic Analysis

検索 (`search`) プロセスの詳細比較です。

### 3-1. Python Implementation (`cognee/api/v1/search/search.py`)

Python版は、データセット単位でのコンテキスト分離と、動的な検索ツール選択が特徴です。

**処理フロー:**

1.  **API Entry Point (`search`)**:
    *   `query_type` に基づき検索ツール群 (`get_search_type_tools`) を取得。
2.  **Context Isolation (`search_in_datasets_context`)**:
    *   ユーザーがアクセス可能な各データセットに対して、個別にコンテキスト（DB接続先など）を切り替えて検索を実行。
    *   `set_database_global_context_variables` でグローバル変数を書き換えてDB接続を切り替える（かなりハッキーな実装）。
3.  **Tool Execution**:
    *   `GRAPH_COMPLETION` や `SUMMARIES` などのロジックを実行。
4.  **Aggregation**:
    *   複数データセットの結果を結合 (`CombinedSearchResult`)。

### 3-2. Go Implementation (`graph_completion.go`)

Go版は、単一のデータストアに対する検索を実行します。

**処理フロー:**

1.  **API Entry Point (`Search`)**:
    *   `GraphCompletionTool.Search` を呼び出し。
2.  **Disparch Logic**:
    *   `switch searchType` でロジックを分岐（`case SearchTypeSummaries`, `case GraphCompletion`...）。
3.  **Execution**:
    *   単純なベクトル検索やグラフ探索を実行し、結果を返す。
    *   データセットの概念は（コード上は）希薄。全データからの検索となる。

### 3-3. 差異と課題 (Gaps)

| 機能 | Python (Original) | Go (Current) | 課題・アクション |
| :--- | :--- | :--- | :--- |
| **データセット分離** | **あり** (Dataset Context) | **なし** (Flat Storage) | マルチテナント/マルチデータセット対応には、DBスキーマレベルでの `tenant_id` / `dataset_id` フィルタリングが必要。 |
| **ツール選択** | 動的 (`get_search_type_tools`) | 静的 (`switch`) | Goは静的型付けなのでこれで正解。 |
| **検索ロジック** | グラフ探索アルゴリズムが豊富 | 基本的な探索のみ | Python側の高度なアルゴリズム（例: 次数中心性など）は未移植。 |

---

## 4. 結論と次のステップ (Conclusion)

### 実装の達成度
Phase-04の目標である「Summarization機能の実装」と「基本的なAdd/Cognify/Searchパイプラインの構築」は完了しています。
コアロジックのクローンとしては機能していますが、「データパーティション（知識の分離）」に関しては重大な欠落があり、現状ではシングルユーザーかつ全データ共有状態となっています。

### Phase-05 の開発目標 (Objectives)

Phase-05の主題は、**「保守性の高いデータパーティション設計と決定論的IDの実装」** です。
以下の3点を軸に、`CogneeService` および下層のアーキテクチャを再設計します。

#### 1. 物理的パーティション (Physical Partitioning)
*   **構造**: `CogneeService` 構造体をクラスのように扱い、インスタンスごとに**独立したDBファイルパス**（DuckDB/CozoDB）を保持させる。
*   **効果**: インスタンスAとインスタンスBは物理的に異なるDBファイルを参照するため、データ混在が物理レベルで発生しない。これは最強のセキュリティ境界となる。
*   **実装方針**: `NewCogneeService` 時にDBパスを注入し、そのインスタンス専用のDBコネクションを確立・保持する。

#### 2. 論理的パーティション (Logical Partitioning)
*   **構造**: 参照するDBファイル内部においても、フラットではなく **階層的なユニーク文字列ID**（例: `"12-yokohama-AB"`）を用いたデータ区分（論理パーティション）を持たせる。
*   **ID設計**: ハイフンやスラッシュで区切られた線型かつ階層的な文字列（例: `TenantID-GroupID-UserID`）を想定し、将来的な複雑な権限構造に対応できる柔軟性を持たせる。
*   **実装方針**: 全テーブル（Data, Chunks, Nodes, Edges）に `partition_id` (VARCHAR) カラムを追加し、全てのクエリでこのIDによるフィルタリングを強制する設計とする。

#### 3. 決定論的ID生成 (Deterministic ID Generation)
*   **課題**: 現在の `add` 処理はランダムUUIDを使用しており、同一ファイルを再登録しても別データとして扱われる（冪等性がない）。
*   **実装方針**: Python版と同様に、**コンテンツハッシュ + 論理パーティションID** に基づく決定論的なハッシュID生成ロジックを導入する。これにより、何度実行しても同一データは同一IDとなり、重複排除や更新処理が正しく機能するようにする。

---

## 5. プロンプトの日本語最適化計画 (Prompt Optimization)

Step 13の指示に基づき、`src/pkg/cognee/prompts/prompts.go` 内のプロンプトを更新します。
**原文（英語）は保持し**、出力形式に関する強力な追加指示（System Instruction）を付与します。

**（変更前例）**
```text
Summarize the following text...
```

**（変更後イメージ）**
```text
Summarize the following text...

IMPORTANT INSTRUCTION:
You must analyze the content in English to maintain accuracy, but your final OUTPUT MUST BE IN JAPANESE.
Translate your summary into natural, professional Japanese.
```

これにより、LLMの推論能力（英語が得意）を活かしたまま、ユーザー体験（日本語）を向上させます。
