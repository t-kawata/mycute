-- ベクトル拡張のインストールとロードは main.go で行われるため、ここには記述しません。
-- INSTALL vss;
-- LOAD vss;

-- Files metadata
CREATE TABLE IF NOT EXISTS data (
    id VARCHAR,          -- Changed from UUID to VARCHAR for deterministic ID
    group_id VARCHAR,    -- [NEW] Partition Key
    name VARCHAR,
    raw_data_location VARCHAR,
    extension VARCHAR,
    mime_type VARCHAR,
    content_hash VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, id) -- Partition Key First
);

-- Extracted text content
CREATE TABLE IF NOT EXISTS documents (
    id UUID,
    group_id VARCHAR,    -- [NEW]
    data_id VARCHAR,     -- Referencing data(id) but needed matching group_id for FK
    text VARCHAR,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, id),
    FOREIGN KEY (group_id, data_id) REFERENCES data(group_id, id)
);

-- Text chunks
CREATE TABLE IF NOT EXISTS chunks (
    id UUID,
    group_id VARCHAR,    -- [NEW]
    document_id UUID,
    text VARCHAR,
    chunk_index INTEGER,
    token_count INTEGER,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, id),
    FOREIGN KEY (group_id, document_id) REFERENCES documents(group_id, id)
);

-- Vector storage (Universal)
-- collection_name = Python版の index_name (例: "Entity_name", "DocumentChunk_text")
CREATE TABLE IF NOT EXISTS vectors (
    id VARCHAR, -- References chunks(id) or nodes(id)
    group_id VARCHAR,    -- [NEW]
    collection_name VARCHAR, 
    text VARCHAR, -- For keyword search / debug
    embedding FLOAT[1536], -- Adjust dimension based on model
    PRIMARY KEY (group_id, collection_name, id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_vectors_collection ON vectors(group_id, collection_name);
-- HNSW Index for vector similarity (Cosine)
-- Note: DuckDB VSS indexes might be per-table or global. ensuring group_id is part of filtering is key.
CREATE INDEX IF NOT EXISTS idx_vectors_embedding ON vectors USING HNSW (embedding) WITH (metric = 'cosine');
