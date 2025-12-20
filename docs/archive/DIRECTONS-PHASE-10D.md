# Cognee Go Implementation: Phase-10D Detailed Development Directives
# VectorStorage Implementation (ベクトルストレージ実装)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-10D: VectorStorage Implementation** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。

> [!IMPORTANT]
> **Phase-10Dのゴール**
> `KuzuDBStorage` に `VectorStorage` インターフェースの全メソッドを完全実装する。
> DuckDBStorageと同等の機能をKuzuDBで再現する。

> [!CAUTION]
> **前提条件**
> Phase-10A（ビルド基盤）、Phase-10B（インターフェース設計）、Phase-10C（スキーマ検証）が完了していること

---

## 1. 実装対象メソッド一覧

| メソッド | DuckDBでの実装 | KuzuDBでの実装 | 優先度 |
|---------|---------------|-------------|--------|
| SaveData | INSERT ... ON CONFLICT | MERGE (d:Data {...}) | 高 |
| Exists | SELECT WHERE content_hash | MATCH ... WHERE content_hash | 高 |
| GetDataByID | SELECT WHERE id | MATCH ... WHERE id | 高 |
| GetDataList | SELECT WHERE group_id | MATCH ... WHERE group_id | 高 |
| SaveDocument | INSERT ... ON CONFLICT | MERGE (d:Document {...}) | 高 |
| SaveChunk | INSERT ... ON CONFLICT | MERGE (c:Chunk {...}) + SaveEmbedding | 高 |
| SaveEmbedding | INSERT ... ON CONFLICT | MERGE (v:Vector {...}) | 高 |
| Search | array_cosine_similarity | cosine_similarity ORDER BY | 高 |
| GetEmbeddingByID | SELECT embedding WHERE id | MATCH ... RETURN embedding | 中 |
| GetEmbeddingsByIDs | SELECT WHERE id IN (...) | MATCH WHERE id IN [...] | 中 |

---

## Step 1: SaveData実装

### 1.1 DuckDB参照実装

```go
// duckdb_storage.go (55-70行目)
func (s *DuckDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
    // UPSERT（INSERT or UPDATE）クエリ
    // ON CONFLICT句により、既存データがあれば更新、なければ挿入
    query := `
        INSERT INTO data (id, group_id, name, raw_data_location, extension, content_hash, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT (group_id, id) DO UPDATE SET
            name = excluded.name,
            raw_data_location = excluded.raw_data_location,
            extension = excluded.extension,
            content_hash = excluded.content_hash,
            created_at = excluded.created_at
    `
    _, err := s.db.ExecContext(ctx, query, data.ID, data.GroupID, data.Name, data.RawDataLocation, data.Extension, data.ContentHash, data.CreatedAt)
    return err
}
```

**根拠説明**: DuckDBでは `INSERT ... ON CONFLICT DO UPDATE` によるUPSERT操作を使用している。KuzuDBでは、Cypherの `MERGE` 句を使用して同等のアトミックなUPSERT操作を実現する。`MERGE` は存在しなければ作成、存在すれば更新を1つのステートメントで安全に実行できる。

### 1.2 KuzuDB実装

```go
// SaveData は、ファイルのメタデータをDataノードとして保存します。
//
// DuckDB参照実装:
//   INSERT INTO data (id, group_id, name, ...) VALUES (?, ?, ?, ...)
//   ON CONFLICT (group_id, id) DO UPDATE SET ...
//
// KuzuDB実装:
//   MERGE (d:Data {id: $id, group_id: $gid})
//   ON CREATE SET d.name = $name, ...
//   ON MATCH SET d.name = $name, ...
//
// MERGEを使用する理由:
//   - エラーメッセージによる判定に依存しない（安全）
//   - アトミックな操作（競合状態がない）
//   - DuckDBのON CONFLICTと完全に同等の動作
func (s *KuzuDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
    // KuzuDBで日時を扱う場合はISO8601形式の文字列を使用
    createdAt := data.CreatedAt.Format(time.RFC3339)

    // DuckDB: INSERT ... ON CONFLICT DO UPDATE
    // KuzuDB: MERGE ... ON CREATE SET / ON MATCH SET
    query := fmt.Sprintf(`
        MERGE (d:Data {id: '%s', group_id: '%s'})
        ON CREATE SET 
            d.name = '%s',
            d.raw_data_location = '%s',
            d.original_data_location = '%s',
            d.extension = '%s',
            d.mime_type = '%s',
            d.content_hash = '%s',
            d.owner_id = '%s',
            d.created_at = datetime('%s')
        ON MATCH SET 
            d.name = '%s',
            d.raw_data_location = '%s',
            d.original_data_location = '%s',
            d.extension = '%s',
            d.mime_type = '%s',
            d.content_hash = '%s',
            d.owner_id = '%s',
            d.created_at = datetime('%s')
    `, 
        escapeString(data.ID),
        escapeString(data.GroupID),
        // ON CREATE SET
        escapeString(data.Name),
        escapeString(data.RawDataLocation),
        escapeString(data.OriginalDataLocation),
        escapeString(data.Extension),
        escapeString(data.MimeType),
        escapeString(data.ContentHash),
        escapeString(data.OwnerID),
        createdAt,
        // ON MATCH SET
        escapeString(data.Name),
        escapeString(data.RawDataLocation),
        escapeString(data.OriginalDataLocation),
        escapeString(data.Extension),
        escapeString(data.MimeType),
        escapeString(data.ContentHash),
        escapeString(data.OwnerID),
        createdAt,
    )

    result, err := s.conn.Execute(query)
    if err != nil {
        return fmt.Errorf("failed to save data: %w", err)
    }
    result.Close()
    return nil
}

// escapeString は、Cypher文字列のエスケープを行います
func escapeString(s string) string {
    s = strings.ReplaceAll(s, "\\", "\\\\")
    s = strings.ReplaceAll(s, "'", "\\'")
    return s
}
```

---

## Step 2: Exists実装

### 2.1 DuckDB参照実装

```go
// duckdb_storage.go (87-97行目)
func (s *DuckDBStorage) Exists(ctx context.Context, contentHash string, groupID string) bool {
    var count int
    // content_hash と group_id の両方で検索
    query := `SELECT COUNT(*) FROM data WHERE content_hash = ? AND group_id = ?`
    err := s.db.QueryRowContext(ctx, query, contentHash, groupID).Scan(&count)
    if err != nil {
        return false
    }
    return count > 0
}
```

**根拠説明**: DuckDBでは `SELECT COUNT(*)` で存在チェックを行っている。KuzuDBでも同様に `MATCH ... RETURN count()` でノード数をカウントして存在確認する。

### 2.2 KuzuDB実装

```go
// Exists は、指定されたコンテンツハッシュを持つデータが存在するかをチェックします。
//
// DuckDB参照実装:
//   SELECT COUNT(*) FROM data WHERE content_hash = ? AND group_id = ?
//
// KuzuDB実装:
//   MATCH (d:Data) WHERE d.content_hash = $hash AND d.group_id = $gid RETURN count(d)
func (s *KuzuDBStorage) Exists(ctx context.Context, contentHash string, groupID string) bool {
    query := fmt.Sprintf(`
        MATCH (d:Data)
        WHERE d.content_hash = '%s' AND d.group_id = '%s'
        RETURN count(d) as cnt
    `, escapeString(contentHash), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return false
    }
    defer result.Close()

    if result.Next() {
        cnt, err := result.GetValue(0)
        if err != nil {
            return false
        }
        if count, ok := cnt.(int64); ok {
            return count > 0
        }
    }
    return false
}
```

---

## Step 3: GetDataByID実装

### 3.1 DuckDB参照実装

```go
// duckdb_storage.go (115-127行目)
func (s *DuckDBStorage) GetDataByID(ctx context.Context, id string, groupID string) (*storage.Data, error) {
    // id と group_id の両方で検索
    query := `SELECT id::VARCHAR, group_id, name, raw_data_location, extension, content_hash, created_at FROM data WHERE id = ? AND group_id = ?`
    row := s.db.QueryRowContext(ctx, query, id, groupID)

    var data storage.Data
    if err := row.Scan(&data.ID, &data.GroupID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
        return nil, err
    }
    return &data, nil
}
```

**根拠説明**: DuckDBでは `SELECT ... WHERE id = ? AND group_id = ?` でIDとグループIDの両方でフィルタリングしている。KuzuDBでも同様に `MATCH ... WHERE ... RETURN` で取得する。

### 3.2 KuzuDB実装

```go
// GetDataByID は、IDとグループIDでデータを取得します。
//
// DuckDB参照実装:
//   SELECT id::VARCHAR, group_id, name, ... FROM data WHERE id = ? AND group_id = ?
//
// KuzuDB実装:
//   MATCH (d:Data) WHERE d.id = $id AND d.group_id = $gid RETURN d.id, d.group_id, ...
func (s *KuzuDBStorage) GetDataByID(ctx context.Context, id string, groupID string) (*storage.Data, error) {
    query := fmt.Sprintf(`
        MATCH (d:Data)
        WHERE d.id = '%s' AND d.group_id = '%s'
        RETURN 
            d.id,
            d.group_id,
            d.name,
            d.raw_data_location,
            d.original_data_location,
            d.extension,
            d.mime_type,
            d.content_hash,
            d.owner_id,
            d.created_at
    `, escapeString(id), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get data by id: %w", err)
    }
    defer result.Close()

    if !result.Next() {
        return nil, nil // DuckDBと同様: 見つからない場合はnilを返す
    }

    // 各フィールドを取得
    dataID, _ := result.GetValue(0)
    dataGroupID, _ := result.GetValue(1)
    name, _ := result.GetValue(2)
    rawLoc, _ := result.GetValue(3)
    origLoc, _ := result.GetValue(4)
    ext, _ := result.GetValue(5)
    mimeType, _ := result.GetValue(6)
    contentHash, _ := result.GetValue(7)
    ownerID, _ := result.GetValue(8)
    createdAtVal, _ := result.GetValue(9)

    // 日時の変換
    var createdAt time.Time
    if ts, ok := createdAtVal.(string); ok {
        createdAt, _ = time.Parse(time.RFC3339, ts)
    }

    return &storage.Data{
        ID:                   getString(dataID),
        GroupID:              getString(dataGroupID),
        Name:                 getString(name),
        RawDataLocation:      getString(rawLoc),
        OriginalDataLocation: getString(origLoc),
        Extension:            getString(ext),
        MimeType:             getString(mimeType),
        ContentHash:          getString(contentHash),
        OwnerID:              getString(ownerID),
        CreatedAt:            createdAt,
    }, nil
}

// getString は任意の型を文字列に変換するヘルパー関数
func getString(v any) string {
    if v == nil {
        return ""
    }
    if s, ok := v.(string); ok {
        return s
    }
    return fmt.Sprintf("%v", v)
}
```

---

## Step 4: GetDataList実装

### 4.1 DuckDB参照実装

```go
// duckdb_storage.go (139-158行目)
func (s *DuckDBStorage) GetDataList(ctx context.Context, groupID string) ([]*storage.Data, error) {
    query := `SELECT id::VARCHAR, group_id, name, raw_data_location, extension, content_hash, created_at FROM data WHERE group_id = ?`
    rows, err := s.db.QueryContext(ctx, query, groupID)
    if err != nil {
        return nil, err
    }
    defer rows.Close()

    var dataList []*storage.Data
    for rows.Next() {
        var data storage.Data
        if err := rows.Scan(&data.ID, &data.GroupID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
            return nil, err
        }
        dataList = append(dataList, &data)
    }
    return dataList, nil
}
```

**根拠説明**: DuckDBでは `rows.Next()` でイテレートし、各行を構造体にマッピングしている。KuzuDBでも `result.Next()` で同様のイテレーションを行う。

### 4.2 KuzuDB実装

```go
// GetDataList は、指定されたグループIDに属するすべてのデータを取得します。
//
// DuckDB参照実装:
//   SELECT ... FROM data WHERE group_id = ?
//   rows.Next() でイテレート
//
// KuzuDB実装:
//   MATCH (d:Data) WHERE d.group_id = $gid RETURN ...
//   result.Next() でイテレート
func (s *KuzuDBStorage) GetDataList(ctx context.Context, groupID string) ([]*storage.Data, error) {
    query := fmt.Sprintf(`
        MATCH (d:Data)
        WHERE d.group_id = '%s'
        RETURN 
            d.id,
            d.group_id,
            d.name,
            d.raw_data_location,
            d.original_data_location,
            d.extension,
            d.mime_type,
            d.content_hash,
            d.owner_id,
            d.created_at
    `, escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get data list: %w", err)
    }
    defer result.Close()

    var dataList []*storage.Data

    // DuckDBと同様: result.Next() でイテレート
    for result.Next() {
        dataID, _ := result.GetValue(0)
        dataGroupID, _ := result.GetValue(1)
        name, _ := result.GetValue(2)
        rawLoc, _ := result.GetValue(3)
        origLoc, _ := result.GetValue(4)
        ext, _ := result.GetValue(5)
        mimeType, _ := result.GetValue(6)
        contentHash, _ := result.GetValue(7)
        ownerID, _ := result.GetValue(8)
        createdAtVal, _ := result.GetValue(9)

        var createdAt time.Time
        if ts, ok := createdAtVal.(string); ok {
            createdAt, _ = time.Parse(time.RFC3339, ts)
        }

        dataList = append(dataList, &storage.Data{
            ID:                   getString(dataID),
            GroupID:              getString(dataGroupID),
            Name:                 getString(name),
            RawDataLocation:      getString(rawLoc),
            OriginalDataLocation: getString(origLoc),
            Extension:            getString(ext),
            MimeType:             getString(mimeType),
            ContentHash:          getString(contentHash),
            OwnerID:              getString(ownerID),
            CreatedAt:            createdAt,
        })
    }

    return dataList, nil
}
```

---

## Step 5: SaveDocument実装

### 5.1 DuckDB参照実装

```go
// duckdb_storage.go (169-189行目)
func (s *DuckDBStorage) SaveDocument(ctx context.Context, doc *storage.Document) error {
    query := `
    INSERT INTO documents (id, group_id, data_id, text, metadata, created_at)
    VALUES (?, ?, ?, ?, ?, current_timestamp)
    ON CONFLICT (group_id, id) DO UPDATE SET
        text = excluded.text,
        metadata = excluded.metadata
    `
    metaJSON, err := json.Marshal(doc.MetaData)
    if err != nil {
        return fmt.Errorf("failed to marshal metadata: %w", err)
    }
    if _, err := s.db.ExecContext(ctx, query, doc.ID, doc.GroupID, doc.DataID, doc.Text, string(metaJSON)); err != nil {
        return fmt.Errorf("failed to save document: %w", err)
    }
    return nil
}
```

**根拠説明**: DuckDBでは MetaData を JSON文字列にマーシャルして保存している。KuzuDBでも同様にJSON文字列として保存する。

### 5.2 KuzuDB実装

```go
// SaveDocument は、ドキュメントをDocumentノードとして保存します。
//
// DuckDB参照実装:
//   INSERT INTO documents (id, group_id, data_id, text, metadata, created_at) VALUES (...)
//   ON CONFLICT DO UPDATE
//   metadata は json.Marshal で JSON文字列に変換
//
// KuzuDB実装:
//   MERGE (d:Document {id: ..., group_id: ...})
//   ON CREATE SET d.data_id = ..., d.text = ..., d.metadata = ...
//   ON MATCH SET d.text = ..., d.metadata = ...
func (s *KuzuDBStorage) SaveDocument(ctx context.Context, document *storage.Document) error {
    // DuckDB同様: メタデータをJSON文字列に変換
    metadataJSON := "{}"
    if document.MetaData != nil {
        if b, err := json.Marshal(document.MetaData); err == nil {
            metadataJSON = string(b)
        }
    }

    // DuckDB: INSERT ... ON CONFLICT DO UPDATE
    // KuzuDB: MERGE ... ON CREATE SET / ON MATCH SET
    query := fmt.Sprintf(`
        MERGE (d:Document {id: '%s', group_id: '%s'})
        ON CREATE SET 
            d.data_id = '%s',
            d.text = '%s',
            d.metadata = '%s'
        ON MATCH SET 
            d.text = '%s',
            d.metadata = '%s'
    `,
        escapeString(document.ID),
        escapeString(document.GroupID),
        escapeString(document.DataID),
        escapeString(document.Text),
        escapeString(metadataJSON),
        escapeString(document.Text),
        escapeString(metadataJSON),
    )

    result, err := s.conn.Execute(query)
    if err != nil {
        return fmt.Errorf("failed to save document: %w", err)
    }
    result.Close()
    return nil
}
```

---

## Step 6: SaveChunk実装

### 6.1 DuckDB参照実装

```go
// duckdb_storage.go (202-240行目)
func (s *DuckDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
    // 1. チャンクをchunksテーブルに保存
    chunkQuery := `
        INSERT INTO chunks (id, group_id, document_id, text, chunk_index, token_count, created_at)
        VALUES (?, ?, ?, ?, ?, ?, current_timestamp)
        ON CONFLICT (group_id, id) DO UPDATE SET
            text = excluded.text,
            chunk_index = excluded.chunk_index,
            token_count = excluded.token_count
    `
    _, err := s.db.ExecContext(ctx, chunkQuery, chunk.ID, chunk.GroupID, chunk.DocumentID, chunk.Text, chunk.ChunkIndex, chunk.TokenCount)
    if err != nil {
        return fmt.Errorf("failed to save chunk: %w", err)
    }

    // 2. ベクトル（embedding）が存在する場合、vectorsテーブルに保存
    if len(chunk.Embedding) > 0 {
        collectionName := "DocumentChunk_text"
        // SaveEmbedding を使用
        // ...
    }
    return nil
}
```

**根拠説明**: DuckDBでは chunks と vectors の2つのテーブルに分けて保存している。KuzuDBでもChunkノードとVectorノードを別々に作成する。

### 6.2 KuzuDB実装

```go
// SaveChunk は、チャンクとそのベクトル表現を保存します。
//
// DuckDB参照実装:
//   1. INSERT INTO chunks (...) ON CONFLICT DO UPDATE
//   2. if len(Embedding) > 0: INSERT INTO vectors (...)
//
// KuzuDB実装:
//   1. MERGE (c:Chunk {id: ..., group_id: ...}) ON CREATE SET / ON MATCH SET
//   2. if len(Embedding) > 0: SaveEmbedding() (これもMERGEを使用)
func (s *KuzuDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
    // 1. ChunkノードをMERGE（UPSERT）
    // DuckDB: INSERT INTO chunks ... ON CONFLICT DO UPDATE
    // KuzuDB: MERGE ... ON CREATE SET / ON MATCH SET
    chunkQuery := fmt.Sprintf(`
        MERGE (c:Chunk {id: '%s', group_id: '%s'})
        ON CREATE SET 
            c.document_id = '%s',
            c.text = '%s',
            c.token_count = %d,
            c.chunk_index = %d
        ON MATCH SET 
            c.text = '%s',
            c.token_count = %d,
            c.chunk_index = %d
    `,
        escapeString(chunk.ID),
        escapeString(chunk.GroupID),
        escapeString(chunk.DocumentID),
        escapeString(chunk.Text),
        chunk.TokenCount,
        chunk.ChunkIndex,
        escapeString(chunk.Text),
        chunk.TokenCount,
        chunk.ChunkIndex,
    )

    result, err := s.conn.Execute(chunkQuery)
    if err != nil {
        return fmt.Errorf("failed to save chunk: %w", err)
    }
    result.Close()

    // 2. Vectorノードを作成（Embeddingがある場合）
    // DuckDB同様: len(chunk.Embedding) > 0 の場合のみ
    if len(chunk.Embedding) > 0 {
        // DuckDB: collectionName := "DocumentChunk_text"
        // SaveEmbeddingも内部でMERGEを使用
        if err := s.SaveEmbedding(ctx, "DocumentChunk_text", chunk.ID, chunk.Text, chunk.Embedding, chunk.GroupID); err != nil {
            return fmt.Errorf("failed to save chunk embedding: %w", err)
        }
    }

    return nil
}
```

---

## Step 7: SaveEmbedding実装

### 7.1 DuckDB参照実装

```go
// duckdb_storage.go (255-269行目)
func (s *DuckDBStorage) SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32, groupID string) error {
    query := `
        INSERT INTO vectors (id, group_id, collection_name, text, embedding)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT (group_id, collection_name, id) DO UPDATE SET
            text = excluded.text,
            embedding = excluded.embedding
    `
    _, err := s.db.ExecContext(ctx, query, id, groupID, collectionName, text, vector)
    if err != nil {
        return fmt.Errorf("failed to save embedding: %w", err)
    }
    return nil
}
```

**根拠説明**: DuckDBでは `(group_id, collection_name, id)` の複合キーで UPSERT を行っている。KuzuDBでも同様のロジックが必要。

### 7.2 KuzuDB実装

```go
// SaveEmbedding は、任意のテキストのベクトル表現を保存します。
//
// DuckDB参照実装:
//   INSERT INTO vectors (id, group_id, collection_name, text, embedding) VALUES (?, ?, ?, ?, ?)
//   ON CONFLICT (group_id, collection_name, id) DO UPDATE SET text = excluded.text, embedding = excluded.embedding
//
// KuzuDB実装:
//   MERGE (v:Vector {id: ..., group_id: ..., collection_name: ...})
//   ON CREATE SET v.text = ..., v.embedding = ...
//   ON MATCH SET v.text = ..., v.embedding = ...
//
// MERGEを使用する理由:
//   - エラーメッセージによる判定に依存しない（バージョン非依存）
//   - アトミックな操作（レースコンディションを回避）
//   - DuckDBのON CONFLICT句と完全に同等の動作を保証
func (s *KuzuDBStorage) SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32, groupID string) error {
    // ベクトルを文字列形式に変換（KuzuDB FLOAT[]形式）
    vecStr := formatVectorForKuzuDB(vector)

    // DuckDB: INSERT ... ON CONFLICT (group_id, collection_name, id) DO UPDATE
    // KuzuDB: MERGE ... ON CREATE SET / ON MATCH SET
    // MERGEのキー: (id, group_id, collection_name) の複合キー
    query := fmt.Sprintf(`
        MERGE (v:Vector {id: '%s', group_id: '%s', collection_name: '%s'})
        ON CREATE SET 
            v.text = '%s',
            v.embedding = %s
        ON MATCH SET 
            v.text = '%s',
            v.embedding = %s
    `, 
        escapeString(id), 
        escapeString(groupID), 
        escapeString(collectionName), 
        escapeString(text), 
        vecStr,
        escapeString(text), 
        vecStr,
    )

    result, err := s.conn.Execute(query)
    if err != nil {
        return fmt.Errorf("failed to save embedding: %w", err)
    }
    result.Close()
    return nil
}

// formatVectorForKuzuDB は、float32スライスをKuzuDBのFLOAT[]形式に変換します
func formatVectorForKuzuDB(vec []float32) string {
    parts := make([]string, len(vec))
    for i, v := range vec {
        parts[i] = fmt.Sprintf("%f", v)
    }
    return "[" + strings.Join(parts, ", ") + "]"
}
```

---

## Step 8: Search実装

### 8.1 DuckDB参照実装

```go
// duckdb_storage.go (290-318行目)
func (s *DuckDBStorage) Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*storage.SearchResult, error) {
    // array_cosine_similarity: コサイン類似度を計算
    query := `
        SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) as score
        FROM vectors
        WHERE collection_name = ? AND group_id = ?
        ORDER BY score DESC
        LIMIT ?
    `
    rows, err := s.db.QueryContext(ctx, query, vector, collectionName, groupID, k)
    if err != nil {
        return nil, err
    }
    defer rows.Close()

    var results []*storage.SearchResult
    for rows.Next() {
        var res storage.SearchResult
        if err := rows.Scan(&res.ID, &res.Text, &res.Distance); err != nil {
            return nil, err
        }
        results = append(results, &res)
    }
    return results, nil
}
```

**根拠説明**: DuckDBでは `array_cosine_similarity` 関数でベクトル類似度を計算している。KuzuDBでは `cosine_similarity` 関数を使用する。

### 8.2 KuzuDB実装

```go
// Search は、ベクトル類似度検索を実行します。
//
// DuckDB参照実装:
//   SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) as score
//   FROM vectors
//   WHERE collection_name = ? AND group_id = ?
//   ORDER BY score DESC
//   LIMIT ?
//
// KuzuDB実装:
//   MATCH (v:Vector)
//   WHERE v.group_id = $gid AND v.collection_name = $coll
//   RETURN v.id, v.text, cosine_similarity(v.embedding, [...]) as similarity
//   ORDER BY similarity DESC
//   LIMIT k
func (s *KuzuDBStorage) Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*storage.SearchResult, error) {
    vecStr := formatVectorForKuzuDB(vector)

    // DuckDB: ORDER BY score DESC LIMIT ?
    // KuzuDB: ORDER BY similarity DESC LIMIT k
    query := fmt.Sprintf(`
        MATCH (v:Vector)
        WHERE v.group_id = '%s' AND v.collection_name = '%s'
        RETURN v.id, v.text, cosine_similarity(v.embedding, %s) as similarity
        ORDER BY similarity DESC
        LIMIT %d
    `, escapeString(groupID), escapeString(collectionName), vecStr, k)

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to search: %w", err)
    }
    defer result.Close()

    var results []*storage.SearchResult

    // DuckDB同様: rows.Next() → result.Next()
    for result.Next() {
        id, _ := result.GetValue(0)
        text, _ := result.GetValue(1)
        similarity, _ := result.GetValue(2)

        var distance float64
        switch v := similarity.(type) {
        case float64:
            distance = v
        case float32:
            distance = float64(v)
        }

        results = append(results, &storage.SearchResult{
            ID:       getString(id),
            Text:     getString(text),
            Distance: distance,
        })
    }

    return results, nil
}
```

---

## Step 9: GetEmbeddingByID実装

### 9.1 DuckDB参照実装

```go
// duckdb_storage.go (341-389行目)
func (s *DuckDBStorage) GetEmbeddingByID(ctx context.Context, collectionName, id, groupID string) ([]float32, error) {
    query := `
        SELECT embedding 
        FROM vectors 
        WHERE id = ? AND collection_name = ? AND group_id = ?
    `
    row := s.db.QueryRowContext(ctx, query, id, collectionName, groupID)

    var vectorData any
    if err := row.Scan(&vectorData); err != nil {
        if err == sql.ErrNoRows {
            return nil, nil // 見つからない場合はエラーではなくnilを返す
        }
        return nil, fmt.Errorf("failed to scan embedding: %w", err)
    }
    // 型変換処理...
}
```

**根拠説明**: DuckDBでは `sql.ErrNoRows` の場合は nil を返し、エラーとして扱わない。KuzuDBでも同様に `result.Next()` が false の場合は nil を返す。

### 9.2 KuzuDB実装

```go
// GetEmbeddingByID は、指定されたIDのEmbeddingを取得します。
//
// DuckDB参照実装:
//   SELECT embedding FROM vectors WHERE id = ? AND collection_name = ? AND group_id = ?
//   sql.ErrNoRows → nil, nil を返す（エラーではない）
//
// KuzuDB実装:
//   MATCH (v:Vector) WHERE v.id = ... RETURN v.embedding
//   result.Next() == false → nil, nil を返す
func (s *KuzuDBStorage) GetEmbeddingByID(ctx context.Context, collectionName, id, groupID string) ([]float32, error) {
    query := fmt.Sprintf(`
        MATCH (v:Vector)
        WHERE v.id = '%s' AND v.collection_name = '%s' AND v.group_id = '%s'
        RETURN v.embedding
    `, escapeString(id), escapeString(collectionName), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get embedding: %w", err)
    }
    defer result.Close()

    // DuckDB同様: 見つからない場合はnilを返す
    if !result.Next() {
        return nil, nil
    }

    embeddingVal, err := result.GetValue(0)
    if err != nil {
        return nil, fmt.Errorf("failed to get embedding value: %w", err)
    }

    return parseEmbedding(embeddingVal)
}

// parseEmbedding は、KuzuDBから返されたベクトルデータをfloat32スライスに変換します
func parseEmbedding(val any) ([]float32, error) {
    switch v := val.(type) {
    case []float32:
        return v, nil
    case []float64:
        result := make([]float32, len(v))
        for i, f := range v {
            result[i] = float32(f)
        }
        return result, nil
    case []any:
        result := make([]float32, len(v))
        for i, elem := range v {
            switch e := elem.(type) {
            case float32:
                result[i] = e
            case float64:
                result[i] = float32(e)
            }
        }
        return result, nil
    default:
        return nil, fmt.Errorf("unexpected embedding type: %T", val)
    }
}
```

---

## Step 10: GetEmbeddingsByIDs実装

### 10.1 DuckDB参照実装

```go
// duckdb_storage.go (391-450行目)
func (s *DuckDBStorage) GetEmbeddingsByIDs(ctx context.Context, collectionName string, ids []string, groupID string) (map[string][]float32, error) {
    if len(ids) == 0 {
        return make(map[string][]float32), nil
    }
    // IN句を使用して複数IDを1回のクエリで取得
    placeholders := make([]string, len(ids))
    args := make([]any, len(ids)+2)
    for i, id := range ids {
        placeholders[i] = "?"
        args[i] = id
    }
    args[len(ids)] = collectionName
    args[len(ids)+1] = groupID

    query := fmt.Sprintf(`
        SELECT id, embedding 
        FROM vectors 
        WHERE id IN (%s) AND collection_name = ? AND group_id = ?
    `, strings.Join(placeholders, ", "))

    rows, err := s.db.QueryContext(ctx, query, args...)
    // ...
}
```

**根拠説明**: DuckDBでは `IN句` を使用して複数IDを一度にクエリしている。KuzuDBでも同様に `WHERE id IN [...]` を使用する。

### 10.2 KuzuDB実装

```go
// GetEmbeddingsByIDs は、複数IDのEmbeddingを一括取得します。
//
// DuckDB参照実装:
//   SELECT id, embedding FROM vectors WHERE id IN (?, ?, ...) AND collection_name = ? AND group_id = ?
//
// KuzuDB実装:
//   MATCH (v:Vector) WHERE v.id IN ['id1', 'id2', ...] RETURN v.id, v.embedding
func (s *KuzuDBStorage) GetEmbeddingsByIDs(ctx context.Context, collectionName string, ids []string, groupID string) (map[string][]float32, error) {
    // DuckDB同様: 空の場合は空のマップを返す
    if len(ids) == 0 {
        return make(map[string][]float32), nil
    }

    // IDリストを作成
    // DuckDB: IN (?, ?, ?)
    // KuzuDB: IN ['id1', 'id2', 'id3']
    quotedIDs := make([]string, len(ids))
    for i, id := range ids {
        quotedIDs[i] = fmt.Sprintf("'%s'", escapeString(id))
    }
    idList := "[" + strings.Join(quotedIDs, ", ") + "]"

    query := fmt.Sprintf(`
        MATCH (v:Vector)
        WHERE v.id IN %s AND v.collection_name = '%s' AND v.group_id = '%s'
        RETURN v.id, v.embedding
    `, idList, escapeString(collectionName), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get embeddings: %w", err)
    }
    defer result.Close()

    embeddings := make(map[string][]float32)

    for result.Next() {
        idVal, _ := result.GetValue(0)
        embeddingVal, _ := result.GetValue(1)

        id := getString(idVal)
        vec, err := parseEmbedding(embeddingVal)
        if err != nil {
            continue
        }

        embeddings[id] = vec
    }

    return embeddings, nil
}
```

---

## 11. VectorStorage実装時の共通エラーパターン

### 11.1 よくあるエラーと対処法

| エラーメッセージ | 原因 | 対処法 |
|-----------------|------|--------|
| `no table named 'Data'` | EnsureSchema未実行 | `kuzuDBStorage.EnsureSchema(ctx)` を先に実行 |
| `constraint violation` | 同一IDで再INSERT | MERGE句を使用（既に対応済み） |
| `syntax error` | escapeString未使用 | 全文字列フィールドで `escapeString()` 使用 |
| `type mismatch: expected FLOAT[]` | ベクトルフォーマット不正 | `formatVectorForKuzuDB()` 使用 |
| `NULL value not allowed` | 必須フィールドがnil | デフォルト値を設定 |

### 11.2 デバッグ時の確認ポイント

```go
// 1. クエリ文字列をログ出力して構文を確認
log.Printf("[DEBUG] Query: %s", query)

// 2. GetValue戻り値の型を確認
val, _ := result.GetValue(0)
log.Printf("[DEBUG] Type: %T, Value: %v", val, val)

// 3. ベクトルフォーマットを確認
vec := []float32{0.1, 0.2}
formatted := formatVectorForKuzuDB(vec)
log.Printf("[DEBUG] Formatted vector: %s", formatted)
// 期待: "[0.100000, 0.200000]"
```

### 11.3 Search関数のスコア範囲

```
cosine_similarity の戻り値:
- 1.0  = 完全一致
- 0.0  = 直交（無関係）
- -1.0 = 完全に逆方向

実用的なしきい値:
- 0.9以上 = 非常に類似
- 0.7以上 = 類似
- 0.5以上 = やや類似
- 0.5未満 = 関連性低い
```

---

## 12. 成功条件チェックリスト

### Phase-10D 完了条件

- [ ] SaveData が正常に動作（DuckDB INSERT/UPDATE と同等）
- [ ] Exists が正常に動作（DuckDB COUNT(*) と同等）
- [ ] GetDataByID が正常に動作（DuckDB SELECT WHERE id と同等）
- [ ] GetDataList が正常に動作（DuckDB SELECT WHERE group_id と同等）
- [ ] SaveDocument が正常に動作（DuckDB INSERT documents と同等）
- [ ] SaveChunk が正常に動作（DuckDB INSERT chunks + vectors と同等）
- [ ] SaveEmbedding が正常に動作（DuckDB INSERT vectors と同等）
- [ ] Search が正常に動作（DuckDB array_cosine_similarity と同等）
- [ ] GetEmbeddingByID が正常に動作（DuckDB SELECT embedding と同等）
- [ ] GetEmbeddingsByIDs が正常に動作（DuckDB SELECT IN と同等）
- [ ] `make build` がエラーなしで成功
- [ ] `test-kuzudb-vector` テストがPASSED
- [ ] 既存のDuckDB+CozoDBテストが引き続き動作

---

## 13. テストコマンドテンプレート

```go
case "test-kuzudb-vector":
    // Phase-10D: VectorStorage機能テスト
    log.Println("--- Phase 10D Test: VectorStorage Verification ---")
    
    // 1. KuzuDBStorage作成
    kuzuDBPath := cfg.COGNEE_DB_DIR + "/kuzudb_vector_test"
    storage, err := kuzudb.NewKuzuDBStorage(kuzuDBPath)
    if err != nil {
        log.Fatalf("❌ Failed to create storage: %v", err)
    }
    defer func() {
        storage.Close()
        os.RemoveAll(kuzuDBPath)
    }()
    
    // 2. スキーマ作成
    if err := storage.EnsureSchema(ctx); err != nil {
        log.Fatalf("❌ EnsureSchema failed: %v", err)
    }
    
    // 3. SaveData テスト
    testData := &storage.Data{
        ID:          "test_data_id",
        GroupID:     "test_group",
        Name:        "test.txt",
        ContentHash: "hash123",
        CreatedAt:   time.Now(),
    }
    if err := storage.SaveData(ctx, testData); err != nil {
        log.Fatalf("❌ SaveData failed: %v", err)
    }
    log.Println("✅ SaveData: OK")
    
    // 4. Exists テスト
    exists := storage.Exists(ctx, "hash123", "test_group")
    if !exists {
        log.Fatalf("❌ Exists returned false, expected true")
    }
    log.Println("✅ Exists: OK")
    
    // 5. SaveEmbedding + Search テスト
    vec1 := make([]float32, 10)
    vec2 := make([]float32, 10)
    for i := range vec1 {
        vec1[i] = float32(i) * 0.1
        vec2[i] = float32(i) * 0.15
    }
    
    storage.SaveEmbedding(ctx, "Chunk", "vec1", "First text", vec1, "test_group")
    storage.SaveEmbedding(ctx, "Chunk", "vec2", "Second text", vec2, "test_group")
    
    results, err := storage.Search(ctx, "Chunk", vec1, 2, "test_group")
    if err != nil {
        log.Fatalf("❌ Search failed: %v", err)
    }
    log.Printf("✅ Search returned %d results", len(results))
    
    log.Println("✅ test-kuzudb-vector PASSED")
```

---

## 14. 次のフェーズへの準備

Phase-10Dが完了したら、VectorStorageの全機能がKuzuDBで動作する状態となる。
Phase-10Eでは、GraphStorageの各メソッドを完全実装する。

