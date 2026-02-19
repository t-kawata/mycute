# Phase 02 オリジナル Cognee からの差分詳細

このドキュメントは、Phase 02 の実装（Go）とオリジナルの Cognee（Python）を比較した詳細な回顧分析を提供します。`Add`、`Cognify`、および `Search` 操作におけるロジックの不一致を特定します。

## 1. Add 機能（取り込み）

### Python 実装 (`cognee/api/v1/add/add.py`, `cognee/tasks/ingestion/ingest_data.py`)
Python 実装は、高レベルのオーケストレーションであり、以下を実行します：
1.  **データの解決**: さまざまな入力タイプ（ファイル、URL、S3、バイナリストリーム）を処理します。
2.  **パーミッション**: データセットのユーザーパーミッションを設定します。
3.  **パイプライン**: 以下の `add_pipeline` を実行します：
    - `resolve_data_directories`
    - `ingest_data`
4.  **データ取り込みタスク**:
    - 重複排除のためにコンテンツハッシュ（SHA256）を計算します。
    - メタデータ（拡張子、サイズなど）を抽出します。
    - `Data` オブジェクト（DataPoint）を作成します。
    - データポイントをグラフ/ベクトルDBに保存します。

### Go 実装 (`src/pkg/cognee/cognee.go`, `src/pkg/cognee/tasks/ingestion/ingest_task.go`)
Go 実装はローカルファイルの取り込みに焦点を当てています：
1.  **CogneeService.Add**: パイプラインをオーケストレーションします。
2.  **IngestTask**:
    - ファイルの SHA256 ハッシュを計算します。
    - **重複排除**: `VectorStorage.Exists(hash)` をチェックします。存在する場合はスキップします。
    - **メタデータ**: 基本的なファイル情報（名前、拡張子）を抽出します。
    - **永続化**: `Data` オブジェクトを `VectorStorage` (DuckDB `data` テーブル) に保存します。

### 不一致と欠落している機能
- **入力の多様性**: Python は URL、S3、バイナリストリームをサポートしています。Go は現在、ローカルファイルパスのみをサポートしています。
- **パーミッション**: Python には堅牢なパーミッションシステム (`resolve_authorized_user_dataset`) があります。Go にはプレースホルダーの `user` 文字列がありますが、実際のパーミッションロジックはありません。
- **グラフストレージ**: Python は DataPoint をグラフに保存します。Go は DuckDB (`data` テーブル) に保存します。これは設計上の選択（メタデータを DuckDB に）ですが、グラフがソースファイルをノードとして「認識しない」ことを意味します。

## 2. Cognify 機能（グラフ生成）

### Python 実装 (`cognee/api/v1/cognify/cognify.py`)
Python パイプラインは複雑で設定可能です：
1.  **タスク**:
    - `classify_documents`
    - `check_permissions`
    - `extract_chunks_from_documents` (階層的: 単語 -> 文 -> 段落)
    - `extract_graph_from_data` (LLM 抽出)
    - `summarize_text`
    - `add_data_points` (ストレージ)
2.  **チャンキング**: 決定論的 ID (`uuid5`) を持つ `TextChunker` を使用します。
3.  **グラフ抽出**: `asyncio.gather` を使用してチャンクを並行処理します。
4.  **ストレージ**: ノード/エッジを重複排除し、グラフに保存します。

### Go 実装 (`src/pkg/cognee/cognee.go`, `src/pkg/cognee/tasks/chunking`, `src/pkg/cognee/tasks/graph`)
1.  **タスク**:
    - `ChunkingTask`:
        - **日本語最適化**: 単語分割には `kagome` を（必要な場合）、トークンカウントには `tiktoken-go` を使用します。
        - **階層的**: 文分割 -> トークン数によるグループ化。
        - **埋め込み**: チャンクの埋め込みを即座に生成します。
    - `GraphExtractionTask`:
        - 並列 LLM 呼び出しに `errgroup` を使用します。
        - `GenerateGraphPrompt` を使用してノード/エッジを抽出します。
    - `StorageTask`:
        - チャンクを DuckDB (`chunks`, `vectors`) に保存します。
        - ノード/エッジを CozoDB (`nodes`, `edges`) に保存します。

### 不一致と欠落している機能
- **要約**: Python には `summarize_text` が含まれています。Go の実装ではこれはスキップされました（Phase 2 の指示にはありませんでした）。
- **ドキュメント分類**: Python はドキュメントを分類します。Go はテキストを前提としています。
- **ノード埋め込み**:
    - **重要**: Python は `DataPoint` (ノード) のプロパティ（例：`Entity.name`）をベクトル DB (`index_data_points.py`) にインデックスします。
    - **Go**: **チャンク** の埋め込みのみを生成および保存します。ノードは CozoDB に保存されますが、DuckDB (VectorStorage) にはインデックスされません。これは「グラフ Search」機能（下記参照）を破綻させます。

## 3. Search 機能（検索）

### Python 実装 (`cognee/api/v1/search/search.py`, `brute_force_triplet_search.py`)
1.  **Search タイプ**: `GRAPH_COMPLETION`、`RAG_COMPLETION`、`CHUNKS` などをサポートします。
2.  **グラフ補完ロジック**:
    - **ベクトル Search**: 複数のコレクション（チャンク、エンティティ名など）を横断して検索します。
    - **マッピング**: ベクトル結果をグラフノードにマッピングします (`map_vector_distances_to_graph_nodes`)。
    - **トラバーサル**: クエリに関連する「トリプレット」（ノード -> エッジ -> ノード）を見つけます。
    - **コンテキスト**: これらのトリプレットを LLM のコンテキストとして使用します。

### Go 実装 (`src/pkg/cognee/tools/search/graph_completion.go`)
1.  **ロジック**:
    - **ベクトル Search**: DuckDB の `DocumentChunk_text` コレクションを検索します。
    - **コンテキスト**: 見つかった**チャンクのテキスト**を使用します。
    - **LLM**: チャンクテキストに基づいて回答を生成します。
2.  **グラフの使用**:
    - **欠落**: コードは、ノードの埋め込みがないため、チャンクをグラフノードにマッピングできないことを明示的にコメントしています。
    - **結果**: 実質的に **RAG (Retrieval Augmented Generation)** を実行しており、**グラフ RAG** ではありません。CozoDB (グラフ) はデータが投入されていますが、**Search 中には使用されません**。

### 不一致と欠落している機能
- **グラフトラバーサル**: Go では完全に欠落しています。Search は純粋にベクトルベースの RAG です。
- **ノードインデックス作成**: Cognify で述べたように、ノードはベクトルインデックスされていないため、「エンティティ」を直接 Search することは不可能です。
- **Search タイプ**: Go は単一の「Search」メソッドのみを実装しており、実質的に `RAG_COMPLETION` です。

## 4. Phase 03 への結論と推奨事項

Phase 02 の実装は、インフラストラクチャ (DuckDB/CozoDB) とパイプラインアーキテクチャを構築することに成功しました。しかし、ベクトル Search とグラフノード間のリンクが欠落しているため、**Search** ドメインにおいて「完全なクローン」には及びませんでした。

**Phase 03 で必要となる重要な修正点:**
1.  **ノードのインデックス作成**: `StorageTask`（または新しいタスク）で、`Node.properties.name`（および潜在的に他のフィールド）の埋め込みを生成し、`VectorStorage`（例：コレクション `Entity_name`）に保存します。
2.  **グラフトラバーサルの実装**: `GraphCompletionTool` を更新して以下を実行します：
    - `DocumentChunk_text` に加えて `Entity_name` コレクションも Search します。
    - 見つかったエンティティを CozoDB ノード ID にマッピングします。
    - CozoDB クエリを実行して、1ホップ/2ホップの近隣（トリプレット）を取得します。
    - トリプレットとチャンクテキストをコンテキストとして使用します。
3.  **チャンキングの改善**: チャンクが、そこから抽出されたノードにリンクされていることを確認します（例：エッジまたはメタデータの `source_id` を介して）。これにより、双方向のトラバーサル（チャンク -> ノード、ノード -> チャンク）が可能になります。
