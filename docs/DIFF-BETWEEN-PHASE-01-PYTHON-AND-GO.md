# Phase-01 Go実装とオリジナルPython実装の差異分析

本ドキュメントでは、Phase-01完了時点のGo実装 (`src/pkg/cognee/cognee.go`) と、オリジナルPython実装 (`cognee/`) のロジックおよび処理シーケンスの差異を詳細に分析し、Phase-02以降で「完全なクローン」に近づけるための課題を洗い出します。

## 対象ファイル
- **Go**: `src/pkg/cognee/cognee.go`
- **Python**:
    - `cognee/api/v1/add/add.py`
    - `cognee/api/v1/cognify/cognify.py`
    - `cognee/api/v1/search/search.py`
    - `cognee/modules/search/methods/search.py`

---

## 1. `add` 機能の差異分析

### オリジナルPython実装 (`cognee/api/v1/add/add.py`)
- **アーキテクチャ**: `Task` ベースのパイプライン処理 (`run_pipeline`)。
- **処理シーケンス**:
    1. `setup()`: DB等の初期化。
    2. `resolve_authorized_user_dataset`: ユーザーとデータセットの権限確認・解決。
    3. `reset_dataset_pipeline_run_status`: パイプライン実行状態のリセット。
    4. `run_pipeline`: 以下のタスクを順次実行。
        - `resolve_data_directories`: 入力パス（ローカル, S3, URL）をフラットなファイルリストに解決。
        - `ingest_data`: データの物理保存、テキスト変換、メタデータ保存、データセットへの紐付け。
- **機能**:
    - 多様な入力対応: テキスト文字列, ファイルパス, S3パス (`s3://`), URL, バイナリio。
    - ローダー指定: `preferred_loaders` でファイルタイプごとの処理を指定可能。
    - ユーザー管理: 自動ユーザー作成 (`get_default_user`)。

### Phase-01 Go実装 (`src/pkg/cognee/cognee.go` - `Add`)
- **アーキテクチャ**: 単一関数による手続き的処理（モノリシック）。
- **処理シーケンス**:
    1. `s3client` 初期化 (ダミー認証情報)。
    2. 入力パス (`filePaths`) のループ処理。
    3. `s3client.Up`: ファイルアップロード（ローカル保存）。
    4. `os.ReadFile`: テキストとして読み込み（ローダーによる変換なし）。
    5. ID生成 (SHA256ハッシュ)。
    6. `MetadataStorage.SaveData`: メタデータ保存。
- **主な差異・不足点**:
    1. **パイプライン欠如**: タスク管理システムがなく、拡張性が低い。
    2. **入力解決の弱さ**: S3パスやURL、ディレクトリの再帰的探索（一部実装済みだが簡易）が不十分。
    3. **ローダー欠如**: PDFや画像などの非テキストファイルを扱えない。全てテキストとして読み込んでいる。
    4. **権限・データセット管理**: ユーザーやデータセットの存在確認・権限チェックがない。
    5. **セットアップ処理**: `setup()` に相当する初期化プロセスが関数内にハードコードされている。

---

## 2. `cognify` 機能の差異分析

### オリジナルPython実装 (`cognee/api/v1/cognify/cognify.py`)
- **アーキテクチャ**: `Task` ベースのパイプライン処理。
- **処理シーケンス**:
    1. 設定解決 (`config`, `ontology`).
    2. タスク生成 (`get_default_tasks` または `get_temporal_tasks`).
    3. `run_pipeline`: 以下のタスクを実行。
        - `classify_documents`: ドキュメント種類の判別。
        - `check_permissions_on_dataset`: 書き込み権限確認。
        - `extract_chunks_from_documents`: テキスト分割（Chunking）。
        - `extract_graph_from_data`: LLMによるグラフ抽出（並列処理）。
        - `summarize_text`: 要約生成。
        - `add_data_points`: グラフ・ベクトルDBへの保存。
- **機能**:
    - 柔軟な構成: `graph_model` (Pydantic), `chunker` を差し替え可能。
    - 時間的グラフ: `temporal_cognify` モードでイベント・時系列データを抽出可能。
    - バックグラウンド実行: `run_in_background` フラグ。
    - オントロジー: 外部オントロジー定義の読み込み。

### Phase-01 Go実装 (`src/pkg/cognee/cognee.go` - `Cognify`)
- **アーキテクチャ**: 単一関数。
- **処理シーケンス**:
    1. クライアント初期化 (LLM, Embedder, S3)。
    2. データ取得: インメモリメタデータから全データを取得（フィルタリングなし）。
    3. ファイル読み込み: `s3client.Down` -> `os.ReadFile`。
    4. チャンク分割: `langchaingo` (RecursiveCharacter) 固定。
    5. 埋め込み生成: OpenAI API。
    6. グラフ抽出: `processChunksInParallel` (goroutine + errgroup) で並列実行。
    7. 保存: `GraphStorage`, `VectorStorage` へ直接保存。
- **主な差異・不足点**:
    1. **タスク構造の欠如**: 分類、要約、保存などが分離されていない。
    2. **ドキュメント分類なし**: 全てテキストとして処理。
    3. **要約機能なし**: `summarize_text` タスクが未実装。
    4. **構成の固定化**: チャンカーやグラフモデルがハードコードされており、カスタマイズ不可。
    5. **オントロジー非対応**: 独自のグラフ構造定義ができない。
    6. **時間的グラフ非対応**: `temporal_cognify` 機能がない。

---

## 3. `search` 機能の差異分析

### オリジナルPython実装 (`cognee/api/v1/search/search.py`)
- **アーキテクチャ**: API層 -> 権限管理層 -> 実装層 (`methods/search.py`)。
- **処理シーケンス**:
    1. ユーザー・データセット解決。
    2. アクセス制御分岐 (`authorized_search` vs `no_access_control_search`).
    3. `search_in_datasets_context`: データセットごとにコンテキスト（DB接続）を切り替えて検索。
    4. `get_search_type_tools`: `SearchType` に応じたツール（`get_context`, `get_completion` 等）を取得。
    5. 検索実行: ベクトル検索 + グラフ探索 (`get_context`) -> LLM回答生成 (`get_completion`)。
    6. 結果統合: 複数データセットの結果を統合 (`CombinedSearchResult`)。
- **機能**:
    - 多様な検索タイプ: `GRAPH_COMPLETION`, `RAG_COMPLETION`, `CHUNKS`, `SUMMARIES`, `CODE`, `CYPHER`, `FEELING_LUCKY`.
    - テレメトリ: 実行ログ・統計情報の送信。
    - セッション管理: 会話履歴の保持。

### Phase-01 Go実装 (`src/pkg/cognee/cognee.go` - `Search`)
- **アーキテクチャ**: 単一関数。
- **処理シーケンス**:
    1. クライアント初期化。
    2. クエリ埋め込み生成。
    3. ベクトル検索 (`VectorStorage.Search`).
    4. コンテキスト構築: チャンクテキストを結合（グラフ探索はスキップ）。
    5. LLM回答生成。
- **主な差異・不足点**:
    1. **検索タイプの欠如**: `GRAPH_COMPLETION` (簡易版) しか実装されていない。
    2. **グラフ探索の省略**: `get_context` におけるグラフ探索ロジックが実装されていない（ベクトル検索のみ）。
    3. **アクセス制御なし**: ユーザーやデータセットごとの権限チェックがない。
    4. **複数データセット非対応**: 単一のインメモリDBに対する検索のみ。
    5. **抽象化不足**: 検索ロジックが関数内にベタ書きされており、ツールとして切り出されていない。
    6. **テレメトリ・セッションなし**: ログ収集や会話履歴管理がない。

---

## Phase-02以降「完全なクローン」を目指すにあたっての主要課題

Phase-02で「完全なクローン」を目指すにあたり、以下の点が主要な課題となります。

1.  **パイプライン/タスク構造の導入**: `add` や `cognify` を単一関数から、再利用可能なタスクの集合体（パイプライン）へとリファクタリングする。
2.  **抽象化と拡張性**: ローダー、チャンカー、グラフモデル、検索ストラテジーをインターフェース化し、構成可能にする。
3.  **データ処理の高度化**: ドキュメント分類、要約、グラフ探索（`GetContext`）の完全な実装。
4.  **権限・コンテキスト管理**: ユーザー、データセット、アクセス制御の仕組みを導入する。

## 以上の分析のうち、Phase-02で行う対応項目（以下以外は、Phase-03以降にて行う）

### Phase-01の add における 主な差異・不足点 から、Phase-02で行うべき対応項目
    1. **パイプライン欠如**: タスク管理システムがなく、拡張性が低い。
    5. **セットアップ処理**: `setup()` に相当する初期化プロセスが関数内にハードコードされている。
    ※ これら以外は Phase-02では対応せず、Phase-03以降にて行う。

### Phase-01の cognify における 主な差異・不足点 から、Phase-02で行うべき対応項目
    1. **タスク構造の欠如**: 分類、要約、保存などが分離されていない。
    3. **要約機能なし**: `summarize_text` タスクが未実装。
    4. **構成の固定化**: チャンカーやグラフモデルがハードコードされており、カスタマイズ不可。
    6. **時間的グラフ非対応**: `temporal_cognify` 機能がない。
    ※ これら以外は Phase-02では対応せず、Phase-03以降にて行う。

### Phase-01の search における 主な差異・不足点 から、Phase-02で行うべき対応項目
    2. **グラフ探索の省略**: `get_context` におけるグラフ探索ロジックが実装されていない（ベクトル検索のみ）。
    4. **複数データセット非対応**: 単一のインメモリDBに対する検索のみ。
    5. **抽象化不足**: 検索ロジックが関数内にベタ書きされており、ツールとして切り出されていない。
    6. **テレメトリ・セッションなし**: ログ収集や会話履歴管理がない。
    ※ これら以外は Phase-02では対応せず、Phase-03以降にて行う。

## 以上の、Phase-02で行う対応項目を実装成功させるために最重要であること
    1. inMemoryMetadataStorage を廃止し、正式なデータベースを用いること
    2. ベクトルデータベースとして DuckDB を採用し、CGO によってGo言語実装に完全に埋め込む
    3. グラフデータベースとして CozoDB を採用し、CGO によってGo言語実装に完全に埋め込む

## Phase-02で行う対応項目を実装成功させるための具体化

### 1. インフラストラクチャ層の刷新 (最重要)
Phase-01のインメモリ実装を、CGOを用いた組み込みデータベースに完全に置き換えます。

*   **DuckDB (ベクトル・リレーショナル)**:
    *   **役割**: `MetadataStorage` (Data, Document) および `VectorStorage` (Chunk, Embedding) の実装。
    *   **実装方針**: `database/sql` ドライバー (`github.com/marcboeker/go-duckdb`) を使用し、CGO経由でDuckDBを埋め込む。
    *   **スキーマ設計**:
        *   `data`: ファイルメタデータ。
        *   `documents`: テキストデータ。
        *   `chunks`: テキストチャンクとベクトル埋め込み（`ARRAY`型または専用型）。
    *   **ベクトル検索**: DuckDBのベクトル拡張機能（またはSQLベースの距離計算）を利用して `Search` メソッドを実装する。

*   **CozoDB (グラフ)**:
    *   **役割**: `GraphStorage` (Node, Edge) の実装。
    *   **実装方針**: CozoDBのGoバインディングを使用し、CGO経由でエンジンを埋め込む。
    *   **スキーマ設計**:
        *   `nodes`: ID, Type, Properties (JSON)。
        *   `edges`: SourceID, TargetID, Type, Properties (JSON)。
    *   **グラフ探索**: Datalog を用いて、`GetContext` における「関連ノードの取得（k-hop検索）」を実装する。

### 2. パイプライン・タスクアーキテクチャの導入 (`Add` / `Cognify`)
モノリシックな関数を、再利用可能なタスクとパイプライン構造にリファクタリングします。

*   **Task インターフェース**:
    ```go
    type Task interface {
        Run(ctx context.Context, input any) (any, error)
    }
    ```
*   **Pipeline 構造**: タスクの連鎖を管理し、データの受け渡しとエラーハンドリングを行う。
*   **Add パイプライン**:
    *   `ResolveDataTask`: パス解決。
    *   `IngestDataTask`: S3保存、ID生成、DuckDBへのメタデータ保存。
*   **Cognify パイプライン**:
    *   `ClassifyTask`: ドキュメント分類（拡張子等）。
    *   `ChunkTask`: テキスト分割（設定可能なChunker）。
    *   `GraphExtractionTask`: LLMによる抽出 -> CozoDBへの保存。
    *   `SummarizeTask`: 要約生成 -> DuckDBへの保存。
    *   `TemporalTask`: 時系列情報抽出 -> CozoDBへの保存（Time-aware graph）。

### 3. Search 機能の高度化
DuckDBとCozoDBの連携により、本来の「グラフ補完検索」を実現します。

*   **GetContext の実装**:
    1.  **ベクトル検索 (DuckDB)**: クエリに近い `Chunk` を検索。
    2.  **エンティティ特定**: チャンクに含まれる（または関連する） `Node` を特定。
    3.  **グラフ探索 (CozoDB)**: 特定されたノードを起点に、関連するノード・エッジを探索（1-hop, 2-hop等）。
    4.  **コンテキスト結合**: チャンクテキストとグラフ構造（トリプレット）を統合してLLMに渡す。
*   **複数データセット対応**:
    *   DuckDB/CozoDBのクエリに `dataset_id` フィルタを追加し、権限のあるデータセットのみを検索対象とする。
*   **テレメトリ・セッション**:
    *   検索履歴や会話ログを DuckDB の `interactions` テーブル等に保存する。

### 4. 設定と拡張性
*   **Config構造体**: `Chunker` 設定、`GraphModel` (抽出するエンティティタイプ等)、`Ontology` 設定を保持。
*   **Setup関数**: アプリケーション起動時に DuckDB と CozoDB のマイグレーション（テーブル作成）を自動実行する。