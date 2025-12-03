# はじめに
この DIRECTONS-PHASE-01.md は、本件開発プロジェクトの Phase-01 についてだけ書かれている指示書である。

# 開発の目的
Cogneeという、Python製の超高性能記憶システムのコア部分をGo言語のパッケージに実装し直す。
Cogneeは超高性能であるものの、Python製であるため、より速く、より安定して、より安全に、よりポータブルで、より信頼性の高い記憶システムを提供するためにGo言語によるパッケージ化の意義が大きい。また、Cogneeというシステム全体をGo言語に移植するのではなく、「覚えさせる（add と cognify）」「答えさせる（search）」の二つのコアだけをGoパッケージとして実装することで、多数のプロジェクトを跨いだ再利用性を高く確保しようとするものである。

# 開発環境の要件
- Go言語 v1.25.3
- データベース: SurrealDB v2.4.0（ベクトル・グラフ・ドキュメント・リレーショナルの全て）

# Go言語での実装の際に注意しなければならない点
- CogneeのPython版の実際のロジックを徹底的にGo言語で模倣する実装方法とすること
- 日本語でのコメントアウトを各所に丁寧に入れることにより、処理単位での理解が容易になるよう親切に書くこと
- コメントアウトは簡潔であることよりも、説明が丁寧であることを優先し、冗長になったとしてもわかりやすく書くこと
- Python版では async/await を用いて非同期処理を多用しているが、Go言語実装ではその部分を goroutine と channel や WaitGroup などを用いて同じ意味で且つPythonよりもより効率的な実装を行うことで再現すること
- ベクトル計算部分等、Python版では様々なサードパーティライブラリを使用して効率的に実装を行なっている箇所が複数あると思うので、そのような箇所は、Go言語実装でもまずは同様のライブラリやパッケージが配布されていないかを確認し、安全に使用できると判断できるライブラリやパッケージがない場合にのみ、同じロジックを自前実装関数等で実装すること
- つまり、外部ライブラリやパッケージを積極的に利用して良いという意味であり、`go get` コマンドは `src` ディレクトリ内で実行するということ
- CogneeのGo言語再現版パッケージの実装は src/pkg/cognee ディレクトリ内に実装し、src/main.go からはコールするだけという方法で実装すること
- src/pkg/cognee というパッケージは、モジュールとして配布しない想定で良く、src/go.work に `use .` を既に記述済みであるため src/pkg/cognee 内に go.mod などを宣言する必要はありません。.go ファイルが実装されていれば十分です。
- ファイル保管（S3やローカルストレージ）を扱う際は、`src/pkg/s3client/s3client.go` を利用すること。
- Phase-01では、`s3client.NewS3Client` の引数 `useLocal` を `true` に設定し、ローカル保存モードで動作させること。

# Phase-01 における SurrealDB の扱いについて
- Phase-01 の実装においては、データベースアクセスの部分をインメモリでの実装とし、SurrealDBは使用しない
- ベクトル、グラフ、ドキュメント、リレーショナルのアクセスの全てを interface として実装し、その interface の実装であるインメモリ版のデータベースを SurrealDB の代わりに実装すること
- Phase-02 においては、インメモリ版のデータベースを SurrealDB にスイッチできるモードを実装する予定とする

# Phase-01 のために、私が行った前準備
- Cogneeの実際のPython版の最新ソースコードを `cognee` ディレクトリに配置した
- ファイル類の保管をローカルとAWS S3を抽象化して扱うための `src/pkg/s3client/s3client.go` を予め完全に実装した上で配置した
- その上で、「覚えさせる（add と cognify）」「答えさせる（search）」の二つのコア実装に関わる部分だけを残し、それ以外を削除した
- `src/main.go` を以下の内容で作成
```go
package main

func main() {}
```
- `src/go.mod` を以下の内容で作成
```
module github.com/t-kawata/mycute

go 1.25.3
```
- `src/go.work` を以下の内容で作成
```
go 1.25.3

use .
```
- cogneeのコアのGo言語版実装を格納するためのディレクトリとして `src/pkg/cognee` を空で作成
- 事前に、Cogneeの実際のPython版の最新のソースコードを私自ら解析し、下記「Cogneeの実際のPython版の最新のソースコードの解析結果」に記載した

# Cogneeの実際のPython版の最新のソースコードの解析結果

## 1. 全体構造
「覚えさせる（add と cognify）」「答えさせる（search）」のコア機能は、以下のディレクトリに分散して実装されています。

- **API層**: `cognee/api/v1/` (エントリーポイント)
- **タスク層**: `cognee/tasks/` (個別の処理単位)
- **モジュール層**: `cognee/modules/` (コアロジック、モデル、メソッド)

## 2. 「覚えさせる」機能: `add` の詳細解析

### 2.1. エントリーポイント: `cognee/api/v1/add/add.py`
`add` 関数は、データの取り込みプロセスをオーケストレーション（統合管理）する役割を担います。ユーザーからの入力を受け取り、適切なタスクをスケジュールして実行します。

**関数シグネチャと引数:**
```python
async def add(
    data: Union[BinaryIO, list[BinaryIO], str, list[str]],
    dataset_name: str = "main_dataset",
    user: User = None,
    # ... (省略)
):
```

**詳細な処理フロー:**

1.  **タスクの定義**:
    まず、データのパスを解決する `resolve_data_directories` タスクと、実際にデータを読み込んで保存する `ingest_data` タスクを定義します。これにより、処理の順序と依存関係が明確になります。
    ```python
    tasks = [
        Task(resolve_data_directories, include_subdirectories=True),
        Task(
            ingest_data,
            dataset_name,
            user,
            node_set,
            dataset_id,
            preferred_loaders,
        ),
    ]
    ```

2.  **セットアップと権限確認**:
    `setup()` を呼び出してデータベース等の初期化を行った後、`resolve_authorized_user_dataset` を使用して、操作を行うユーザーと対象のデータセットを特定・検証します。

3.  **パイプライン実行**:
    定義したタスクを `run_pipeline` 関数に渡して実行します。この関数が各タスクを順次（あるいは並列に）処理し、結果を返します。
    ```python
    async for run_info in run_pipeline(
        tasks=tasks,
        datasets=[authorized_dataset.id],
        data=data,
        user=user,
        pipeline_name="add_pipeline",
        # ...
    ):
        pipeline_run_info = run_info
    ```

### 2.2. パス解決タスク: `cognee/tasks/ingestion/resolve_data_directories.py`
`resolve_data_directories` 関数は、入力されたデータパス（ローカルファイル、ディレクトリ、S3パスなど）を解析し、処理可能なフラットなファイルリストに変換します。

-   **入力**: ファイルパス、ディレクトリパス、S3パス、バイナリデータ、またはそれらのリスト。
-   **処理ロジック**:
    -   **S3パス**: `s3fs` ライブラリを使用してS3バケットにアクセスし、指定されたパス以下のファイルをリストアップします。
    -   **ローカルディレクトリ**: `os.walk` (再帰的) または `os.listdir` (非再帰的) を使用して、ディレクトリ内のファイルを探索します。
    -   **単一ファイル**: そのままリストに追加します。

**コードスニペット:**
```python
async def resolve_data_directories(data, include_subdirectories=True):
    # ...
    for item in data:
        if isinstance(item, str):
            if urlparse(item).scheme == "s3":
                # S3パスの処理 (s3fsを使用)
                # ...
            elif os.path.isdir(item):
                # ローカルディレクトリの再帰的探索
                for root, _, files in os.walk(item):
                    resolved_data.extend([os.path.join(root, f) for f in files])
            else:
                # 単一ファイル
                resolved_data.append(item)
    return resolved_data
```

### 2.3. データ取り込みタスク: `cognee/tasks/ingestion/ingest_data.py`
`ingest_data` 関数は、`add` プロセスの核心部分であり、データを物理的に保存し、そのメタデータをデータベースに記録します。

**詳細な処理ステップ:**

1.  **データセットの準備**:
    指定された `dataset_id` または名前を用いて、データの保存先となるデータセットを取得または作成します (`load_or_create_datasets`)。

2.  **データ処理ループ**:
    解決された各データ項目（ファイル）に対して、以下の処理を順次行います。

    -   **物理保存**: `save_data_item_to_storage` を呼び出し、データを物理ストレージ（デフォルトはローカルファイルシステム）に保存します。
    -   **テキスト変換**: `data_item_to_text_file` を使用して、PDFなどのバイナリデータをテキスト形式に変換し、`.txt` ファイルとして別途保存します。これは後のベクトル化やグラフ抽出のために必要です。
    ```python
    # 物理ストレージへの保存
    original_file_path = await save_data_item_to_storage(data_item)
    
    # テキスト形式への変換と保存
    cognee_storage_file_path, loader_engine = await data_item_to_text_file(
        actual_file_path,
        preferred_loaders,
    )
    ```

    -   **分類と識別**: `ingestion.classify` でファイルのメタデータ（拡張子、MIMEタイプなど）を取得し、`ingestion.identify` でファイル内容のハッシュに基づいた一意なデータIDを生成します。
    ```python
    async with open_data_file(original_file_path) as file:
        classified_data = ingestion.classify(file)
        data_id = ingestion.identify(classified_data, user) # ハッシュID生成
        original_file_metadata = classified_data.get_metadata()
    ```

3.  **データベースへの保存 (Dataモデル)**:
    取得したメタデータを用いて `Data` モデルのインスタンスを作成し、SQLデータベース（`Data` テーブル）に保存します。これにより、ファイルの実体とメタデータが紐付けられます。
    ```python
    data_point = Data(
        id=data_id,
        name=original_file_metadata["name"],
        raw_data_location=cognee_storage_file_path, # 変換後テキストのパス
        original_data_location=original_file_metadata["file_path"], # 元ファイルのパス
        extension=storage_file_metadata["extension"],
        mime_type=storage_file_metadata["mime_type"],
        # ...
    )
    # DBへの保存 (session.add / session.merge)
    ```

4.  **データセットへの紐付け**:
    最後に、保存したデータポイントを対象のデータセットに関連付け (`dataset.data.append(data_point)`)、変更をコミットします。

## 3. 「覚えさせる」機能: `cognify` の詳細解析

### 3.1. エントリーポイント: `cognee/api/v1/cognify/cognify.py`
`cognify` 関数は、取り込まれたデータ（`Data`）を処理し、ナレッジグラフを構築するためのパイプラインを実行します。このプロセスにより、非構造化データが構造化された知識に変換されます。

**関数シグネチャ:**
```python
async def cognify(
    datasets: Union[str, list[str], list[UUID]] = None,
    user: User = None,
    graph_model: BaseModel = KnowledgeGraph,
    # ...
):
```

**処理フロー:**

1.  **設定の解決**:
    オントロジー設定やLLMの設定を含む `Config` オブジェクトをロードします。

2.  **タスク生成**:
    `get_default_tasks` 関数を呼び出し、グラフ構築に必要な一連のタスクリストを生成します。
    ```python
    default_tasks = [
        Task(classify_documents),
        Task(check_permissions_on_dataset, user=user, permissions=["write"]),
        Task(
            extract_chunks_from_documents,
            max_chunk_size=chunk_size or get_max_chunk_tokens(),
            chunker=chunker,
        ),
        Task(
            extract_graph_from_data,
            graph_model=graph_model,
            # ...
        ),
        Task(summarize_text, ...),
        Task(add_data_points, ...),
    ]
    ```

3.  **パイプライン実行**:
    `run_pipeline` を実行し、上記のタスクを順次処理します。

### 3.2. パイプラインタスクの詳細構成

#### 3.2.1. ドキュメント分類: `classify_documents`
`cognee/tasks/documents/classify_documents.py` に実装されています。
`Data` オブジェクトの拡張子（pdf, txtなど）に基づいて、適切なドキュメントクラス（`PdfDocument`, `TextDocument` 等）にマッピングします。これにより、後続の処理で適切な読み込み方法が選択されます。

```python
EXTENSION_TO_DOCUMENT_CLASS = {
    "pdf": PdfDocument,
    "txt": TextDocument,
    # ...
}

async def classify_documents(data_documents: list[Data]) -> list[Document]:
    # ...
    document = EXTENSION_TO_DOCUMENT_CLASS[data_item.extension](...)
    # ...
```

#### 3.2.2. チャンク分割: `extract_chunks_from_documents`
`cognee/tasks/documents/extract_chunks/from_documents.py` に実装されています。
ドキュメントをLLMが処理可能なサイズ（トークン数）に分割（チャンク化）します。

```python
async def extract_chunks_from_documents(documents, max_chunk_size, chunker):
    for document in documents:
        async for document_chunk in document.read(
            max_chunk_size=max_chunk_size, chunker_cls=chunker
        ):
            yield document_chunk
```

#### 3.2.3. グラフ抽出: `extract_graph_from_data`
`cognee/tasks/graph/extract_graph_from_data.py` に実装されています。
ここが `cognify` の核心です。LLMを使用して、各テキストチャンクからノードとエッジを抽出します。

-   **抽出**: `extract_content_graph` を呼び出し、LLMにテキストを解析させ、定義された `graph_model` に基づいてグラフ構造を生成させます。これは並列実行されます。
-   **統合**: `integrate_chunk_graphs` を呼び出し、抽出されたグラフ要素を統合し、オントロジーに基づいて検証・拡張します。

```python
async def extract_graph_from_data(data_chunks, graph_model, ...):
    # LLMによる抽出（並列実行）
    chunk_graphs = await asyncio.gather(*[
        extract_content_graph(chunk.text, graph_model, ...)
        for chunk in data_chunks
    ])
    
    # グラフの統合と保存処理へ
    return await integrate_chunk_graphs(data_chunks, chunk_graphs, ...)
```

#### 3.2.4. その他のタスク
-   **`summarize_text`**: 各チャンクの要約をLLMで生成し、`TextSummary` オブジェクトとして保存します。
-   **`add_data_points`**: 生成されたすべてのノードとエッジをグラフデータベースに保存し、ベクトルインデックスを作成して検索可能にします。

## 4. 「答えさせる」機能: `search` の詳細解析

### 4.1. エントリーポイント: `cognee/api/v1/search/search.py`
`search` 関数は、構築されたナレッジグラフに対する検索リクエストを受け付けます。

**関数シグネチャ:**
```python
async def search(
    query_text: str,
    query_type: SearchType = SearchType.GRAPH_COMPLETION,
    # ...
):
```

**処理フロー:**
1.  **ユーザーとデータセットの解決**: リクエストを行ったユーザーと、検索対象となるデータセットを特定します。
2.  **検索実行**: アクセス制御の設定に応じて `authorized_search` または `no_access_control_search` を呼び出しますが、最終的には `cognee.modules.search.methods.search` が実行されます。
3.  **結果整形**: `prepare_search_result` で、クライアントに返すためのJSONシリアライズ可能な形式に結果を変換します。

### 4.2. 検索ロジック: `cognee/modules/search/methods/search.py`
ここが検索の実体です。データセットごとのコンテキストを設定し、指定された `query_type` に基づいて適切な検索アルゴリズムを選択・実行します。

**詳細な処理ステップ:**

1.  **コンテキスト設定**:
    `set_database_global_context_variables` を呼び出し、検索対象のデータセットIDをDBセッションに設定します。

2.  **検索ツールの取得**:
    `get_search_type_tools` 関数を使用して、リクエストされた `query_type` （例: `GRAPH_COMPLETION`, `RAG_COMPLETION`）に対応する検索関数を取得します。
    ```python
    specific_search_tools = await get_search_type_tools(
        query_type=query_type,
        # ...
    )
    # 例: GRAPH_COMPLETION の場合、[get_completion, get_context] が返る
    ```

3.  **検索実行 (例: GRAPH_COMPLETION)**:
    グラフ補完検索の場合、以下の2段階で処理が行われます。
    -   **コンテキスト検索 (`get_context`)**: クエリに関連するノードやエッジを、ベクトル検索とグラフ探索を組み合わせて取得します。
    -   **回答生成 (`get_completion`)**: 取得したコンテキストと元のクエリをLLMに渡し、最終的な回答を生成させます。
    ```python
    # 関連情報の検索（ベクトル+グラフ）
    search_context = await get_context(query_text)
    
    # LLMによる回答生成
    search_result = await get_completion(query_text, search_context, ...)
    ```
    -   **`CODE`**: コード検索。
    -   **`CYPHER`**: 直接Cypherクエリ実行。
4.  **検索実行**: 取得した検索関数を実行。
    -   例 (`GRAPH_COMPLETION`):
        1.  `get_context`: 関連するノードやエッジを検索（ベクトル検索 + グラフ探索）。
        2.  `get_completion`: 検索されたコンテキストとクエリをLLMに渡し、回答を生成。

# Cogneeの「覚えさせる（add と cognify）」「答えさせる（search）」の二つのコア実装（３つの関数）をGo言語版として実装するための詳細な設計と計画

本セクションでは、Python版Cogneeのロジックを忠実に再現しつつ、Go言語の特性（静的型付け、並行処理）を活かした実装計画を詳述する。各設計項目には、その根拠となるPythonコードの引用と理由を付記する。

## 1. パッケージ構成と依存ライブラリ

### 1.1. パッケージ構成
`src/pkg/cognee` パッケージにコアロジックを集約する。

```
src/pkg/cognee/
├── cognee.go       # Add, Cognify, Search の公開関数
├── types.go        # データ構造 (Data, Document, Node, Edge 等)
├── config.go       # 設定 (Config)
├── context.go      # コンテキスト (User, Session)
├── storage.go      # インメモリDBインターフェースと実装
├── llm.go          # LLMインターフェースと実装
└── utils.go        # ユーティリティ
```

### 1.2. 推奨外部ライブラリ
Python版では多くのライブラリが使われているため、Go版でも以下のライブラリを活用して効率的に実装する。

-   **UUID生成**: `github.com/google/uuid`
    -   *根拠*: Python版 `cognee/tasks/ingestion/ingest_data.py` 等で `uuid.uuid4()` が多用されているため。
-   **LLM/Chain/Chunking**: `github.com/tmc/langchaingo`
    -   *根拠*:
        1.  **TextSplitter**: `Cognify` プロセスで必須となる「ドキュメントのチャンク分割（RecursiveCharacterTextSplitter等）」機能が提供されており、これを自前実装するのはコストが高いため。
        2.  **OpenAI互換性**: `langchaingo` の OpenAI プロバイダーは `BaseURL` の変更に対応しており、Bifrost（OpenAI形式のプロキシ）経由での接続が容易であるため。
        3.  **抽象化**: 将来的にChainやAgent機能が必要になった際、Python版Cogneeのアーキテクチャ（LangChain/DSPyライク）と親和性が高いため。
    -   *方針*: `openai-go` を直接使うよりも、チャンク分割機能を持つ `langchaingo` を採用する方が総合的に優れている。ただし、プロバイダーは **OpenAI** に固定し、Bifrost で差異を吸収する構成とする。
-   **設定管理**: `github.com/joho/godotenv`
    -   *根拠*: `.env` ファイルから `OPENAI_BASE_URL` (Bifrost用) や `OPENAI_API_KEY` を読み込むために使用。

## 2. データ構造 (Structs) の設計

### 2.1. `Data` モデル
Python版 `cognee/infrastructure/databases/relational/models/Data.py` (SQLAlchemyモデル) を再現。

```go
type Data struct {
    ID                   string `json:"id"`
    Name                 string `json:"name"`
    RawDataLocation      string `json:"raw_data_location"`      // 変換後テキストのパス
    OriginalDataLocation string `json:"original_data_location"` // 元ファイルのパス
    Extension            string `json:"extension"`
    MimeType             string `json:"mime_type"`
    ContentHash          string `json:"content_hash"`
    OwnerID              string `json:"owner_id"`
}
```
-   *根拠*: `ingest_data.py` で `Data(id=data_id, name=..., ...)` として保存されているフィールドを網羅。GoではJSONタグを付与してシリアライズに対応させる。

### 2.2. `Document` と `Chunk`
Python版 `cognee/modules/data/models/Document.py` 等を再現。

```go
type Document struct {
    ID       string
    DataID   string // Dataとの紐付け
    Text     string
    MetaData map[string]interface{}
}

type Chunk struct {
    ID         string
    DocumentID string
    Text       string
    Embedding  []float32 // ベクトル検索用
    TokenCount int
}
```
-   *根拠*: `extract_chunks_from_documents.py` で `DocumentChunk` が生成され、テキストとメタデータを持つ構造になっているため。

#### 2.3. SearchType （検索タイプ）

Python版 `cognee/shared/SearchType.py` を再現。

```go
type SearchType string

const (
    SearchTypeGraphCompletion SearchType = "GRAPH_COMPLETION"
    SearchTypeRAGCompletion   SearchType = "RAG_COMPLETION"
    SearchTypeCode            SearchType = "CODE"
    SearchTypeCypher          SearchType = "CYPHER"
)
```

## 3. インターフェース設計 (In-Memory DB)

Phase-01ではSurrealDBを使用しないため、以下のインターフェースを定義し、インメモリ（`map` や `slice`）で実装する。

```go
// メタデータ保存用 (Data, Document)
type MetadataStorage interface {
    SaveData(ctx context.Context, data *Data) error
    GetData(ctx context.Context, id string) (*Data, error)
    // ... 他のメソッド
}

// ベクトル保存・検索用 (Chunk)
type VectorStorage interface {
    SaveChunk(ctx context.Context, chunk *Chunk) error
    Search(ctx context.Context, queryVector []float32, topK int) ([]*Chunk, error)
}

// グラフ保存・検索用 (Node, Edge)
type GraphStorage interface {
    AddNode(ctx context.Context, node *Node) error
    AddEdge(ctx context.Context, edge *Edge) error
    GetContext(ctx context.Context, nodeIDs []string) ([]*Node, []*Edge, error) // 関連ノード・エッジ取得
}
```
-   *根拠*: Python版 `cognee/modules/storage` や `cognee/infrastructure/databases` が担っている役割を抽象化。`search.py` の `get_context` でベクトル検索とグラフ探索が行われるため、それに対応するメソッドが必要。

### 3.1. インメモリ VectorStorage の実装例

Phase-01では、単純なスライスベースの実装で十分です。

```go
// src/pkg/cognee/storage.go
import (
    "context"
    "math"
    "sort"
    "sync"
)

type inMemoryVectorStorage struct {
    chunks []*Chunk
    mu     sync.RWMutex // 並行アクセス対策
}

func (s *inMemoryVectorStorage) SaveChunk(ctx context.Context, chunk *Chunk) error {
    s.mu.Lock()
    defer s.mu.Unlock()
    s.chunks = append(s.chunks, chunk)
    return nil
}

func (s *inMemoryVectorStorage) Search(ctx context.Context, queryVector []float32, topK int) ([]*Chunk, error) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    // 全チャンクとの類似度を計算
    type scoredChunk struct {
        chunk *Chunk
        score float32
    }
    scores := make([]scoredChunk, len(s.chunks))
    
    for i, chunk := range s.chunks {
        scores[i] = scoredChunk{
            chunk: chunk,
            score: cosineSimilarity(queryVector, chunk.Embedding),
        }
    }
    
    // スコアで降順ソート
    sort.Slice(scores, func(i, j int) bool {
        return scores[i].score > scores[j].score
    })
    
    // 上位topK個を返す
    result := make([]*Chunk, 0, topK)
    for i := 0; i < topK && i < len(scores); i++ {
        result = append(result, scores[i].chunk)
    }
    return result, nil
}

// コサイン類似度の計算（utils.goに実装）
func cosineSimilarity(a, b []float32) float32 {
    if len(a) != len(b) {
        return 0
    }
    var dotProduct, normA, normB float32
    for i := range a {
        dotProduct += a[i] * b[i]
        normA += a[i] * a[i]
        normB += b[i] * b[i]
    }
    if normA == 0 || normB == 0 {
        return 0
    }
    return dotProduct / (float32(math.Sqrt(float64(normA))) * float32(math.Sqrt(float64(normB))))
}
```

### 3.2. インメモリ GraphStorage の実装例

Phase-01では、エッジから隣接ノードを取得する単純な実装で十分です。

```go
type inMemoryGraphStorage struct {
    nodes map[string]*Node // key: Node.ID
    edges []*Edge
    mu    sync.RWMutex
}

func (s *inMemoryGraphStorage) AddNode(ctx context.Context, node *Node) error {
    s.mu.Lock()
    defer s.mu.Unlock()
    if s.nodes == nil {
        s.nodes = make(map[string]*Node)
    }
    s.nodes[node.ID] = node
    return nil
}

func (s *inMemoryGraphStorage) AddEdge(ctx context.Context, edge *Edge) error {
    s.mu.Lock()
    defer s.mu.Unlock()
    s.edges = append(s.edges, edge)
    return nil
}

// GetContext: 指定されたノードIDに関連するノードとエッジを取得
func (s *inMemoryGraphStorage) GetContext(ctx context.Context, nodeIDs []string) ([]*Node, []*Edge, error) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    nodeSet := make(map[string]bool)
    for _, id := range nodeIDs {
        nodeSet[id] = true
    }
    
    // 関連するエッジを抽出（ソースまたはターゲットがnodeIDsに含まれる）
    relatedEdges := make([]*Edge, 0)
    expandedNodeSet := make(map[string]bool)
    
    for _, edge := range s.edges {
        if nodeSet[edge.SourceID] || nodeSet[edge.TargetID] {
            relatedEdges = append(relatedEdges, edge)
            // 1-hopで接続されているノードも追加
            expandedNodeSet[edge.SourceID] = true
            expandedNodeSet[edge.TargetID] = true
        }
    }
    
    // 関連するノードを取得
    relatedNodes := make([]*Node, 0)
    for nodeID := range expandedNodeSet {
        if node, ok := s.nodes[nodeID]; ok {
            relatedNodes = append(relatedNodes, node)
        }
    }
    
    return relatedNodes, relatedEdges, nil
}
```

## 4. コア機能の実装計画

### 4.0. 共通基盤: LLMクライアント設定 (Bifrost対応)
`src/pkg/cognee/llm.go` にて、Bifrostを経由するように `langchaingo` を初期化する。

```go
func NewLLMClient(ctx context.Context) (llms.Model, error) {
    // 環境変数からBifrostのURLを取得
    baseURL := os.Getenv("OPENAI_BASE_URL") 
    if baseURL == "" {
        baseURL = "https://bifrost.example.com/v1" // デフォルトまたはエラー
    }
    
    // LangchaingoのOpenAIプロバイダーを使用
    llm, err := openai.New(
        openai.WithBaseURL(baseURL),
        openai.WithToken(os.Getenv("OPENAI_API_KEY")),
        openai.WithModel("gpt-4o"), // デフォルトモデル
    )
    return llm, err
}
```

### 4.1. `Add` 機能の実装

**Go関数シグネチャ:**
```go
func Add(ctx context.Context, filePaths []string, datasetName string, userID string) error
```

**実装ステップ:**

1.  **パス解決 (`resolve_data_directories` 再現)**
    -   `s3client` を使用して、指定されたパス（ローカル/S3）からファイルリストを取得する。
    -   *実装例*:
        ```go
        // Phase-1ではローカルモードで動作させるため、ダミーの認証情報を使用する
        client, err := s3client.NewS3Client(
            "dummy_access", "dummy_secret", "us-east-1", "dummy_bucket",
            ".cognee_data/storage", ".cognee_data/downloads", true,
        )
        if err != nil {
            return fmt.Errorf("failed to initialize s3client: %w", err)
        }
        ```
    -   *根拠*: Python版 `resolve_data_directories.py` が `s3fs` や `os.walk` を使ってフラットなリストを作っているため。

2.  **データ取り込み (`ingest_data` 再現)**
    -   ループで各ファイルを処理：
        1.  **保存**: `s3client.Up()` でファイルを所定の場所に保存。
        2.  **テキスト変換**: ファイル拡張子を見てテキストを取り出す（Phase-1では `.txt` はそのまま読み込み、他はダミーまたは簡易的な抽出）。
        3.  **ID生成**: ファイル内容のハッシュ (SHA256等) からUUIDを生成。
        4.  **DB保存**: `MetadataStorage.SaveData()` で `Data` 構造体を保存。
    -   *根拠*: Python版 `ingest_data.py` の `save_data_item_to_storage`, `data_item_to_text_file`, `ingestion.identify`, `session.add(data_point)` のフローを再現。

### 4.2. `Cognify` 機能の実装

**Go関数シグネチャ:**
```go
func Cognify(ctx context.Context, datasetName string, userID string) error
```

**実装ステップ:**

1.  **未処理データの取得**
    -   `MetadataStorage` から処理対象の `Data` を取得。
    -   *実装詳細*: Phase-1のインメモリ実装では、インターフェースに反復メソッドがないため、型アサーションを使用して内部マップにアクセスする。
        ```go
        memStore, ok := metadataStorage.(*inMemoryMetadataStorage)
        if !ok {
            return fmt.Errorf("storage is not in-memory")
        }
        // memStore.data をループ処理
        ```

2.  **チャンク分割 (`extract_chunks_from_documents` 再現)**
    -   `Data` のテキストを読み込み、トークン数または文字数で分割する。
    -   *実装例*: `langchaingo/textsplitter` を使用。
        ```go
        // langchaingoの正確なAPI（v0.1.12基準）
        splitter := textsplitter.NewRecursiveCharacter(
            textsplitter.WithChunkSize(4000),      // Python版のmax_chunk_sizeに相当
            textsplitter.WithChunkOverlap(200),    // 重複部分
            textsplitter.WithSeparators([]string{"\n\n", "\n", " ", ""}), // 日本語を考慮して調整可能
        )

        // テキスト分割の実行
        docs, err := textsplitter.CreateDocuments(splitter, []string{text}, []map[string]any{{}})
        if err != nil {
            return fmt.Errorf("チャンク分割失敗: %w", err)
        }

        // Chunkオブジェクトへの変換
        chunks := make([]*Chunk, len(docs))
        for i, doc := range docs {
            chunks[i] = &Chunk{
                ID:         uuid.New().String(),
                DocumentID: documentID,
                Text:       doc.PageContent,
                TokenCount: len(strings.Split(doc.PageContent, " ")), // 簡易的なトークン数推定
            }
        }
        ```
    -   *根拠*: Python版 `extract_chunks_from_documents.py` で `TextChunker` が使われているため。

3.  **グラフ抽出 (`extract_graph_from_data` 再現)**
    -   各チャンクに対してLLMを呼び出し、ノードとエッジのJSONを生成させる。
    -   **並行処理**: Goの `goroutine` と `errgroup` を使用して、Pythonの `asyncio.gather` を再現し、複数のチャンクを並列にLLM処理する。
    -   *根拠*: Python版 `extract_graph_from_data.py` で `asyncio.gather` を使って並列化しているため、Goではこれが必須要件。

4.  **保存 (`add_data_points` 再現)**
    -   抽出された `Node` と `Edge` を `GraphStorage` に保存。
    -   チャンクの埋め込みベクトルを `VectorStorage` に保存。

5.  **ベクトル埋め込みの生成（重要）**
    Phase-01では、以下のいずれかの方法で埋め込みベクトルを生成してください：

    **方法1: OpenAI Embeddings API経由（推奨）**
    ```go
    // src/pkg/cognee/embedding.go （新規作成）
    import (
        "context"
        "os"

        "github.com/tmc/langchaingo/embeddings"
        "github.com/tmc/langchaingo/llms/openai"
    )

    // NewEmbedder initializes a new Embedder client using OpenAI API (via Bifrost).
    func NewEmbedder(ctx context.Context) (embeddings.Embedder, error) {
        llm, err := openai.New(
            openai.WithBaseURL(os.Getenv("OPENAI_BASE_URL")),
            openai.WithToken(os.Getenv("OPENAI_API_KEY")),
            openai.WithModel("text-embedding-3-small"),
        )
        if err != nil {
            return nil, err
        }

        return embeddings.NewEmbedder(llm)
    }

    // Cognify内でのチャンクごとの埋め込み生成
    for _, chunk := range chunks {
        embeddings, err := embedder.EmbedDocuments(ctx, []string{chunk.Text})
        if err != nil {
            return fmt.Errorf("埋め込み生成失敗: %w", err)
        }
        chunk.Embedding = embeddings[0] // []float32
        vectorStorage.SaveChunk(ctx, chunk)
    }
    ```

### 4.3. `Search` 機能の実装

**Go関数シグネチャ:**
```go
func Search(ctx context.Context, query string, searchType SearchType, userID string) (string, error)
```

**実装ステップ:**

1.  **検索タイプの分岐 (`get_search_type_tools` 再現)**
    -   `searchType` (例: `GRAPH_COMPLETION`) に応じて処理を分岐。

2.  **コンテキスト検索 (`get_context` 再現)**
    -   **ベクトル検索**: クエリのベクトルに近い `Chunk` を `VectorStorage.Search` で取得。
    -   **グラフ探索**: (Phase-1実装ではスキップ) 本来は `GraphStorage.GetContext` で関連ノードを取得するが、Phase-1ではベクトル検索で見つかったチャンクのテキストをコンテキストとして使用する。
    -   *根拠*: Python版 `search.py` -> `get_context` がベクトル検索とグラフ探索を組み合わせているため。Phase-1では簡易化。

3.  **回答生成 (`get_completion` 再現)**
    -   取得したコンテキスト（テキスト）とクエリをプロンプトに埋め込み、LLMに送信して回答を得る。
    -   *根拠*: Python版 `search.py` -> `get_completion` のフロー。

## 5. 実装の安全性と確実性を高めるための実装指針

### 5.1. 並行処理の安全性
`Cognify` におけるグラフ抽出は、大量のチャンクを並列処理するため、リソース枯渇を防ぐ制御が必要である。
-   **ErrGroup**: `golang.org/x/sync/errgroup` を使用し、並行数を制限（`SetLimit`）しつつ、最初のエラーが発生したら即座にキャンセルする仕組みを導入する。
-   **重要**: `SetLimit(n)` は「同時に実行される goroutine の最大数」を制限しますが、`g.Go()` の呼び出し自体は即座に返ります。内部でセマフォ的な制御が行われ、上限に達した場合は新しい goroutine の開始が自動的にブロックされます。
-   **Context Propagation**: 全ての関数で `context.Context` を第一引数に取り、タイムアウトやキャンセルが正しく伝播するように実装する。
    ```go
    g.Go(func() error {
        // ctxのキャンセルを定期的にチェック
        select {
        case <-ctx.Done():
            return ctx.Err()
        default:
            // 処理を継続
        }
        // LLM呼び出しにもctxを渡す
        response, err := llms.GenerateFromSinglePrompt(ctx, llm, prompt)
        // ...
        return nil
    })
    ```

### 5.2. エラーハンドリングとロギング戦略
-   **Wrap Errors**: エラーを返す際は `fmt.Errorf("failed to process chunk: %w", err)` のようにラップし、スタックトレース代わりのコンテキスト情報を付与する。
-   **Graceful Degredation**: 一部のチャンク処理が失敗した場合でも、全体を失敗させるか、ログを出して続行するかを `Config` で制御できるように設計する。

**推奨ログ出力:**
各関数の開始・終了時、および重要イベント発生時に構造化ログを出力する。
```go
// 各関数の冒頭
log.Printf("[DEBUG] %s: Start - Args: %+v", functionName, args)
// 処理の重要なポイント
log.Printf("[INFO] %s: File processing - %s", functionName, filePath)
// エラー時
log.Printf("[ERROR] %s: Failed - %v", functionName, err)
// 関数終了時
log.Printf("[DEBUG] %s: End - Result: %+v", functionName, result)
```
環境変数 `COGNEE_DEBUG=true` でデバッグログを有効化する実装とする。

### 5.3. 設定の検証
-   起動時（`NewS3Client` や `NewLLMClient` 呼び出し時）に必須の環境変数が設定されているか検証し、不足していれば即座にエラー終了させることで、実行時エラーを防ぐ。

## 6. 実装パターン例 (Concrete Code Patterns)

### 6.1. パターン1: 並行チャンク処理（Cognify のコア）
Python版 `extract_graph_from_data.py` の `asyncio.gather` をGoで安全に再現するパターン。

```go
func processChunksInParallel(ctx context.Context, chunks []*Chunk, llm llms.Model) ([]*Node, []*Edge, error) {
    // 並行数を制限（最大10並列）
    g, ctx := errgroup.WithContext(ctx)
    g.SetLimit(10)
    
    // 結果を格納するスライス（スレッドセーフにするため、インデックスでアクセスするかチャネルを使う）
    // ここではインデックスアクセス方式を採用
    nodes := make([][]*Node, len(chunks))
    edges := make([][]*Edge, len(chunks))
    
    for i, chunk := range chunks {
        i, chunk := i, chunk // ループ変数のキャプチャ
        g.Go(func() error {
            log.Printf("[INFO] Processing chunk %d/%d...", i+1, len(chunks))
            
            // LLMでグラフを抽出
            extractedNodes, extractedEdges, err := extractGraphFromChunk(ctx, chunk, llm)
            if err != nil {
                return fmt.Errorf("Failed to process chunk %d: %w", i, err)
            }
            
            nodes[i] = extractedNodes
            edges[i] = extractedEdges
            return nil
        })
    }
    
    // 全ての goroutine の完了を待つ
    if err := g.Wait(); err != nil {
        return nil, nil, fmt.Errorf("並行処理中にエラー: %w", err)
    }
    
    // 結果を統合
    allNodes := make([]*Node, 0)
    allEdges := make([]*Edge, 0)
    for i := range chunks {
        allNodes = append(allNodes, nodes[i]...)
        allEdges = append(allEdges, edges[i]...)
    }
    
    return allNodes, allEdges, nil
}
```

### 6.2. パターン2: LLMプロンプト構築（グラフ抽出）
Python版のプロンプトエンジニアリングを再現し、JSON出力を確実にパースするパターン。

**重要**: 下記「7. プロンプトエンジニアリングとLLM連携の重要指針」を遵守して実際のプロンプトは書くこと。

```go
func extractGraphFromChunk(ctx context.Context, chunk *Chunk, llm llms.Model) ([]*Node, []*Edge, error) {
    // プロンプトテンプレート（Python版を忠実に再現）
    prompt := fmt.Sprintf(`以下のテキストから、エンティティ（ノード）と関係性（エッジ）を抽出してください。

テキスト:
%s

出力形式（JSON）:
{
  "nodes": [
    {"id": "uuid形式", "type": "エンティティタイプ", "properties": {"name": "名前", ...}}
  ],
  "edges": [
    {"source_id": "uuid", "target_id": "uuid", "type": "関係性", "properties": {...}}
  ]
}
JSON以外のテキストは出力しないでください。`, chunk.Text)

    // LLM呼び出し
    response, err := llms.GenerateFromSinglePrompt(ctx, llm, prompt)
    if err != nil {
        return nil, nil, err
    }

    // JSON抽出（堅牢化）
    // LLMが "Here is the JSON..." などの余計な文言をつける場合があるため、
    // 最初の '{' から最後の '}' までを抽出する。
    jsonStr := extractJSON(response)
    if jsonStr == "" {
        return nil, nil, fmt.Errorf("valid JSON not found in response")
    }

    // パース
    var result struct {
        Nodes []*Node `json:"nodes"`
        Edges []*Edge `json:"edges"`
    }
    if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
        return nil, nil, fmt.Errorf("JSON unmarshal failed: %w", err)
    }

    return result.Nodes, result.Edges, nil
}

// extractJSON は文字列から最初の '{' と最後の '}' の間の部分文字列を抽出します
func extractJSON(s string) string {
    start := strings.Index(s, "{")
    end := strings.LastIndex(s, "}")
    if start == -1 || end == -1 || start > end {
        return ""
    }
    return s[start : end+1]
}
```

## 7. プロンプトエンジニアリングとLLM連携の重要指針

**重要**: `Cognify` と `Search` の精度は、使用されるプロンプトに大きく依存します。Python版Cogneeの動作を再現するため、以下の指針を厳守してください。

### 7.1. プロンプトの移植方針
-   **原則**: `cognee/infrastructure/llm/prompts/` にあるオリジナルのプロンプトファイル（`.txt`）を**一言一句変更せずに**使用してください。
-   **例外**: Python版では `instructor` ライブラリ等が Pydantic モデルから自動的に JSON スキーマ制約を LLM に課していますが、Go版（`langchaingo`）ではこれを明示的にプロンプトに含める必要があります。したがって、**「オリジナルのプロンプト + JSON出力指示（スキーマ定義）」** という形式でプロンプトを構築します。

### 7.2. 具体的なプロンプト定義

#### 7.2.1. グラフ抽出 (`Cognify`)
-   **オリジナル**: `cognee/infrastructure/llm/prompts/generate_graph_prompt.txt`
-   **Goでの実装**:

```go
// src/pkg/cognee/prompts.go (新規作成推奨)

const GenerateGraphSystemPrompt = `You are a top-tier algorithm designed for extracting information in structured formats to build a knowledge graph.
**Nodes** represent entities and concepts. They're akin to Wikipedia nodes.
**Edges** represent relationships between concepts. They're akin to Wikipedia links.

The aim is to achieve simplicity and clarity in the knowledge graph.
# 1. Labeling Nodes
**Consistency**: Ensure you use basic or elementary types for node labels.
  - For example, when you identify an entity representing a person, always label it as **"Person"**.
  - Avoid using more specific terms like "Mathematician" or "Scientist", keep those as "profession" property.
  - Don't use too generic terms like "Entity".
**Node IDs**: Never utilize integers as node IDs.
  - Node IDs should be names or human-readable identifiers found in the text.
# 2. Handling Numerical Data and Dates
  - For example, when you identify an entity representing a date, make sure it has type **"Date"**.
  - Extract the date in the format "YYYY-MM-DD"
  - If not possible to extract the whole date, extract month or year, or both if available.
  - **Property Format**: Properties must be in a key-value format.
  - **Quotation Marks**: Never use escaped single or double quotes within property values.
  - **Naming Convention**: Use snake_case for relationship names, e.g., acted_in.
# 3. Coreference Resolution
  - **Maintain Entity Consistency**: When extracting entities, it's vital to ensure consistency.
  If an entity, such as "John Doe", is mentioned multiple times in the text but is referred to by different names or pronouns (e.g., "Joe", "he"),
  always use the most complete identifier for that entity throughout the knowledge graph. In this example, use "John Doe" as the Persons ID.
Remember, the knowledge graph should be coherent and easily understandable, so maintaining consistency in entity references is crucial.
# 4. Strict Compliance
Adhere to the rules strictly. Non-compliance will result in termination`

// Go版で追加するJSON出力指示
const GraphExtractionJSONInstruction = `
IMPORTANT: You must output the result in strict JSON format matching the following schema. Do not include any markdown formatting or explanations.

{
  "nodes": [
    {
      "id": "string (human readable identifier)",
      "type": "string (e.g. Person, Organization)",
      "properties": {
        "name": "string",
        "description": "string",
        ...other properties
      }
    }
  ],
  "edges": [
    {
      "source_node_id": "string (must match a node id)",
      "target_node_id": "string (must match a node id)",
      "type": "string (relationship verb, snake_case)",
      "properties": {
        "weight": "number (optional)"
      }
    }
  ]
}`
```

#### 7.2.2. 検索タイプ選択 (`Search`)
-   **オリジナル**: `cognee/infrastructure/llm/prompts/search_type_selector_prompt.txt`
-   **Goでの実装**: オリジナルの内容をそのまま定数として定義し、最後に `Your response MUST be a single word...` の指示があることを確認して使用します。

## 8. 依存ライブラリの詳細
```
require (
    github.com/google/uuid v1.6.0
    github.com/tmc/langchaingo v0.1.12
    github.com/joho/godotenv v1.5.1
    golang.org/x/sync v0.6.0
)
```

### 各ファイルでのインポート例
```go
// src/pkg/cognee/cognee.go
package cognee

import (
    "context"
    "fmt"
    "log"
    
    "github.com/google/uuid"
    "github.com/tmc/langchaingo/llms"
    "github.com/tmc/langchaingo/llms/openai"
    "github.com/tmc/langchaingo/textsplitter"
    "golang.org/x/sync/errgroup"
)
```

## 9. Phase-01 マイルストーン構成と検証手順

実装を3つのマイルストーンに分割し、段階的に検証を行う。

### 9.1.Milestone 1: 基盤とAdd機能（検証可能な最小実装）
**目標**: ファイルを取り込み、メタデータをインメモリDBに保存できることを確認
**実装範囲**:
- `types.go`, `storage.go`（インメモリ実装）
- `cognee.go` の `Add` 関数のみ
- `main.go` からの呼び出しと結果確認

**検証方法**: 
```bash
# 1. 依存関係のインストール
cd src && go mod tidy

# 2. テスト用ファイルの作成
echo "これはテスト用のサンプルテキストです。" > test_data/sample.txt

# 3. main.go の実行 (COGNEE_DEBUG=true)
COGNEE_DEBUG=true go run main.go
```
期待される出力:
```
[DEBUG] Add: 開始 - 引数: {filePaths:[test_data/sample.txt] ...}
[INFO] Add: ファイル処理中: test_data/sample.txt
[INFO] Add: Data保存完了 - ID: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
[DEBUG] Add: 完了
```

### 9.2. Milestone 2: Cognify機能（グラフ構築）
**前提条件**: Milestone 1が成功していること
**実装範囲**:
- `llm.go`（Bifrost経由のLLM接続）
- `cognee.go` の `Cognify` 関数
- チャンク分割とグラフ抽出

**検証方法**: 
```bash
# Bifrost接続のための環境変数設定
export OPENAI_BASE_URL=https://your-bifrost-endpoint.com/v1
export OPENAI_API_KEY=your-api-key
export COGNEE_DEBUG=true

go run main.go
```
期待される出力:
```
[INFO] Cognify: LLM接続成功
[INFO] Cognify: チャンク分割完了 - 3個
[INFO] チャンク 1/3 を処理中...
[INFO] Cognify: グラフ保存完了 - ノード: 15個、エッジ: 23個
```

### 9.3. Milestone 3: Search機能（検索と回答生成）
**前提条件**: Milestone 2が成功していること
**実装範囲**:
- `cognee.go` の `Search` 関数
- ベクトル検索とグラフ探索
- LLMによる回答生成

**検証方法**:
- `main.go` で `Search("テストについて教えて")` を呼び出し、回答が出力されることを確認。

### 9.4. main.go の実装例

各マイルストーンで使用する `src/main.go` の具体例を以下に示します。

#### Milestone 1 用
```
package main

import (
    "context"
    "log"
    "os"
    
    "github.com/joho/godotenv"
    "github.com/t-kawata/mycute/pkg/cognee"
)

func main() {
    // 環境変数の読み込み（.envがあれば）
    godotenv.Load()
    
    // DEBUGモードの設定
    if os.Getenv("COGNEE_DEBUG") == "true" {
        log.SetFlags(log.LstdFlags | log.Lshortfile)
    }
    
    ctx := context.Background()
    
    // Add機能のテスト
    err := cognee.Add(ctx, []string{"test_data/sample.txt"}, "test_dataset", "user1")
    if err != nil {
        log.Fatalf("❌ Add failed: %v", err)
    }
    
    log.Println("✅ Milestone 1: Add機能が正常に動作しました")
}
```

#### Milestone 2 用
```
package main

import (
    "context"
    "log"
    "os"
    
    "github.com/joho/godotenv"
    "github.com/t-kawata/mycute/pkg/cognee"
)

func main() {
    godotenv.Load()
    
    if os.Getenv("COGNEE_DEBUG") == "true" {
        log.SetFlags(log.LstdFlags | log.Lshortfile)
    }
    
    ctx := context.Background()
    
    // Add
    log.Println("📥 Step 1: データ取り込み...")
    if err := cognee.Add(ctx, []string{"test_data/sample.txt"}, "test_dataset", "user1"); err != nil {
        log.Fatalf("❌ Add failed: %v", err)
    }
    
    // Cognify
    log.Println("🧠 Step 2: グラフ構築...")
    if err := cognee.Cognify(ctx, "test_dataset", "user1"); err != nil {
        log.Fatalf("❌ Cognify failed: %v", err)
    }
    
    log.Println("✅ Milestone 2: Cognify機能が正常に動作しました")
}
```

#### Milestone 3 用（完全版）
```
package main

import (
    "context"
    "log"
    "os"
    
    "github.com/joho/godotenv"
    "github.com/t-kawata/mycute/pkg/cognee"
)

func main() {
    godotenv.Load()
    
    if os.Getenv("COGNEE_DEBUG") == "true" {
        log.SetFlags(log.LstdFlags | log.Lshortfile)
    }
    
    ctx := context.Background()
    
    // フルパイプライン実行
    log.Println("📥 Step 1: データ取り込み...")
    if err := cognee.Add(ctx, []string{"test_data/sample.txt"}, "test_dataset", "user1"); err != nil {
        log.Fatalf("❌ Add failed: %v", err)
    }
    
    log.Println("🧠 Step 2: グラフ構築...")
    if err := cognee.Cognify(ctx, "test_dataset", "user1"); err != nil {
        log.Fatalf("❌ Cognify failed: %v", err)
    }
    
    log.Println("🔍 Step 3: 検索実行...")
    result, err := cognee.Search(ctx, "サンプルテキストについて教えてください", cognee.SearchTypeGraphCompletion, "user1")
    if err != nil {
        log.Fatalf("❌ Search failed: %v", err)
    }
    
    log.Printf("✅ 検索結果:\n%s\n", result)
    log.Println("🎉 Milestone 3: 全機能が正常に動作しました！")
}
```

## 10. トラブルシューティング

### 問題1: LangChainGo の OpenAI クライアントが Bifrost に接続できない
**症状**: `connection refused` または `401 Unauthorized`
**対処法**:
1. 環境変数が正しく設定されているか確認 (`echo $OPENAI_BASE_URL`)
2. Bifrost エンドポイントが `/v1` で終わっているか確認
3. `llm.go` 内で `openai.WithBaseURL` が正しく渡されているか確認

### 問題2: 並行処理でメモリ不足
**症状**: `out of memory` エラー
**対処法**:
- `errgroup.SetLimit` の並行数を減らす（10 → 3）
- チャンクサイズを増やして総数を減らす

### 問題3: JSONパースエラーが頻発
**症状**: `invalid character` エラー
**対処法**:
- LLMプロンプトに「JSONのみを出力し、説明文を含めないこと」を追加
- レスポンスから `````` マーカーを除去する前処理（`strings.TrimPrefix`等）が正しく実装されているか確認