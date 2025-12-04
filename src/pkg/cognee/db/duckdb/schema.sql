-- ベクトル拡張のインストールとロードは main.go で行われるため、ここには記述しません。
-- INSTALL vss;
-- LOAD vss;

-- Files metadata
CREATE TABLE IF NOT EXISTS data (
    id UUID PRIMARY KEY,
    name VARCHAR,
    raw_data_location VARCHAR,
    extension VARCHAR,
    mime_type VARCHAR,
    content_hash VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Extracted text content
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY,
    data_id UUID REFERENCES data(id),
    text VARCHAR,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Text chunks
CREATE TABLE IF NOT EXISTS chunks (
    id UUID PRIMARY KEY,
    document_id UUID REFERENCES documents(id),
    text VARCHAR,
    chunk_index INTEGER,
    token_count INTEGER,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Vector storage (Universal)
-- collection_name = Python版の index_name (例: "Entity_name", "DocumentChunk_text")
CREATE TABLE IF NOT EXISTS vectors (
    id UUID, -- References chunks(id) or nodes(id)
    collection_name VARCHAR, 
    text VARCHAR, -- For keyword search / debug
    embedding FLOAT[1536], -- Adjust dimension based on model
    PRIMARY KEY (id, collection_name)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_vectors_collection ON vectors(collection_name);
-- HNSW Index for vector similarity (Cosine)
CREATE INDEX IF NOT EXISTS idx_vectors_embedding ON vectors USING HNSW (embedding) WITH (metric = 'cosine');
