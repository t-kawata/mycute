# Phase-02B: Pipeline & Task Infrastructure

## 0. 前提条件 (Prerequisites)

### Phase-02Aで実装済みであるべき項目
- [ ] DuckDB `data`, `documents`, `chunks`, `vectors` テーブルが存在する
- [ ] CozoDB `nodes`, `edges` リレーションが存在する
- [ ] `VectorStorage` および `GraphStorage` インターフェースが定義され、実装されていること。

## 1. 目的
Phase-02Aで構築したDB基盤の上に、**Pipeline/Taskアーキテクチャ**を実装し、最初の実用的なパイプラインである **Ingestion (Add)** 機能を完成させます。
本ドキュメントでは、Python版Cogneeのアーキテクチャ設計の意図を解説し、Goでの最適な実装方針を示します。

## 2. 実装詳細とPython解析

### 2.1. Pipeline基盤 (Core Layer)

**ファイル**: `src/pkg/cognee/pipeline/pipeline.go`

#### Python実装の解析 (`cognee/modules/pipelines/run_pipeline.py`)
Cogneeの処理は、独立した `Task` の連鎖として定義されています。

```python
# cognee/modules/pipelines/run_pipeline.py
async def run_pipeline(tasks, input_data):
    for task in tasks:
        input_data = await task(input_data)
    return input_data
```

**Why (設計根拠)**:
これにより、`cognify` のような複雑な処理を「分類 -> 抽出 -> 保存」という小さな単位に分割し、テストや入替えを容易にしています。
Goでもこのパターンを採用しないと、巨大な一枚岩の関数（Monolith）になり、保守性が著しく低下します。

#### Go実装方針 (Interface & Runner)
ジェネリクスを活用し、型安全かつ柔軟なパイプラインを構築します。

```go
type Task interface {
    Run(ctx context.Context, input any) (any, error)
}

type Pipeline struct {
    Tasks []Task
}

func (p *Pipeline) Run(ctx context.Context, initialInput any) (any, error) {
    currentInput := initialInput
    for _, task := range p.Tasks {
        var err error
        currentInput, err = task.Run(ctx, currentInput)
        if err != nil {
            return nil, fmt.Errorf("task failed: %w", err)
        }
    }
    return currentInput, nil
}
```

### 2.2. Ingestion Task (Add Functionality)

**ファイル**: `src/pkg/cognee/tasks/ingestion/ingest_task.go`

#### Python実装の解析 (`cognee/tasks/ingestion/ingest_data.py`)
Ingestion処理は単なるファイルコピーではありません。メタデータの抽出と重複排除が重要です。

```python
# cognee/tasks/ingestion/ingest_data.py
# 1. Metadata Extraction
original_file_metadata = classified_data.get_metadata()
# 2. Deduplication (Content Hash)
data_id = ingestion.identify(classified_data, user)
# 3. Persistence
data_point = Data(id=data_id, content_hash=original_file_metadata["content_hash"], ...)
```

**Why (設計根拠)**:
同じファイルを何度アップロードしても、システム内では単一のデータポイントとして扱われるべきです（冪等性）。
コンテンツハッシュ（SHA256等）をIDのシードとして使用することで、これを保証しています。

#### Go実装方針 (IngestTask)
1.  **Metadata Extraction**: ファイルサイズ、ハッシュ、拡張子を取得。
2.  **Deduplication**: コンテンツハッシュを用いて、既にDBに存在するデータはスキップする。
3.  **Persistence**: `VectorStorage` (DuckDB) にメタデータを保存する。

```go
func (t *IngestTask) Run(ctx context.Context, input any) (any, error) {
    filePaths := input.([]string)
    var dataList []*Data
    
    for _, path := range filePaths {
        // 1. Calculate Hash & Metadata
        fileInfo, _ := os.Stat(path)
        hash := calculateFileHash(path)
        
        // 2. Check Duplication (DuckDB)
        if t.vectorStorage.Exists(ctx, hash) {
            continue
        }
        
        // 3. Create Data Object
        data := &Data{
            ID: uuid.New(), // またはハッシュから生成
            Name: filepath.Base(path),
            Extension: filepath.Ext(path),
            ContentHash: hash,
            // ...
        }
        
        // 4. Save to DuckDB
        t.vectorStorage.SaveData(ctx, data)
        dataList = append(dataList, data)
    }
    return dataList, nil
}
```

### 2.3. CogneeService (Orchestration)

**ファイル**: `src/pkg/cognee/cognee.go`

`CogneeService` を実装し、`Add` メソッド内でパイプラインを構築・実行します。
依存性注入（DI）を行うことで、テスト時にモックストレージを使用できるようにします。

```go
func (s *CogneeService) Add(ctx context.Context, filePaths []string, dataset string) error {
    // Run Ingestion Pipeline
    pipeline := NewPipeline([]Task{
        NewIngestTask(s.vectorStorage),
    })
    _, err := pipeline.Run(ctx, filePaths)
    return err
}
```

## 3. 開発ステップ & Checkpoints

### Checkpoint 3: Pipeline基盤の実装
- [ ] `Task` インターフェースと `Pipeline` ランナーが実装されていること。
- [ ] 単体テストで、複数のタスクを連結して実行できることを確認する。

### Checkpoint 4: Add機能 (Ingestion) の実装
- [ ] `IngestTask` が実装され、ファイルのハッシュ計算とDB保存が行えること。
- [ ] `CogneeService.Add` が実装されていること。
- [ ] **検証コマンド**: `make run ARGS="add -f test_data/sample.txt"`
    - [ ] 成功し、DuckDBの `data` テーブルにレコードが追加されること。
    - [ ] 同じコマンドを再実行しても、重複して登録されないこと（重複排除の確認）。

## 4. 実行コマンド

```bash
# Add機能の検証
make run ARGS="add -f test_data/sample.txt"
```
