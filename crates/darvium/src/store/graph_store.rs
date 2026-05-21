// GraphStore トレイトおよび InMemoryGraphStore 実装
//
// LadybugDB 責務の抽象化: ワークフローグラフ、埋め込みベクトル、
// 知識オブジェクト、リレーション、OriginTrace の格納・検索。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::error::DarviumError;
use crate::types::{GraphId, KnowledgeObject, OriginTrace, RelationRecord, WorkflowGraph};

/// LadybugDB 責務を抽象化するトレイト。
///
/// 全13フェーズはこのトレイトに対するプログラミングで実装され、
/// 実DB接続フェーズでは別実装 (LadybugGraphStore) を追加するだけで差し替えが完了する。
pub trait GraphStore {
    /// ワークフローグラフを格納し、発行された GraphId を返す。
    fn store_workflow_graph(&self, graph: &WorkflowGraph) -> Result<GraphId, DarviumError>;

    /// GraphId に対応するワークフローグラフを読み出す。
    fn load_workflow_graph(&self, graph_id: &GraphId) -> Result<WorkflowGraph, DarviumError>;

    /// キーと埋め込みベクトルのペアを登録する。
    fn store_embedding(&self, key: &str, vector: &[f32]) -> Result<(), DarviumError>;

    /// クエリベクトルに最も類似した上位 top_k 件の (key, similarity) を返す。
    /// 線形探索 (O(n)) で全登録ベクトルとのコサイン類似度を計算する。
    fn semantic_search(
        &self,
        query: &[f32],
        top_k: u32,
    ) -> Result<Vec<(String, f64)>, DarviumError>;

    /// 知識オブジェクトを格納する。
    fn store_knowledge_object(&self, obj: &KnowledgeObject) -> Result<(), DarviumError>;

    /// object_id に対応する知識オブジェクトを読み出す。
    fn load_knowledge_object(&self, object_id: &str) -> Result<KnowledgeObject, DarviumError>;

    /// リレーションを登録する。
    fn store_relation(&self, relation: &RelationRecord) -> Result<(), DarviumError>;

    /// 指定された object_id に関連する全リレーションを取得する。
    fn load_relations(&self, object_id: &str) -> Result<Vec<RelationRecord>, DarviumError>;

    /// OriginTrace を記録する。
    fn record_origin_trace(&self, trace: &OriginTrace) -> Result<(), DarviumError>;

    /// 指定された object_id に関する全 OriginTrace を取得する。
    fn load_origin_traces(&self, object_id: &str) -> Result<Vec<OriginTrace>, DarviumError>;
}

/// メモリ内 GraphStore 実装。
///
/// HashMap / Vec による全操作のメモリ内実装。
/// 高速・決定論的であり、全13フェーズのテスト基盤として使用される。
///
/// # 内部可変性
///
/// トレイトメソッドが &self を取るため、内部状態は RefCell でラップする。
/// シングルスレッド環境でのみ使用すること。並行アクセスが必要な段階で
/// Mutex への置き換えを検討する。
pub struct InMemoryGraphStore {
    graphs: RefCell<HashMap<GraphId, WorkflowGraph>>,
    graph_id_counter: Cell<u64>,
    embeddings: RefCell<HashMap<String, Vec<f32>>>,
    knowledge_objects: RefCell<HashMap<String, KnowledgeObject>>,
    relations: RefCell<Vec<RelationRecord>>,
    origin_traces: RefCell<Vec<OriginTrace>>,
}

impl InMemoryGraphStore {
    /// 空の InMemoryGraphStore を生成する。
    pub fn new() -> Self {
        Self {
            graphs: RefCell::new(HashMap::new()),
            graph_id_counter: Cell::new(0),
            embeddings: RefCell::new(HashMap::new()),
            knowledge_objects: RefCell::new(HashMap::new()),
            relations: RefCell::new(Vec::new()),
            origin_traces: RefCell::new(Vec::new()),
        }
    }

    /// 次に発行するグラフ ID を生成する。
    fn next_graph_id(&self) -> GraphId {
        let id = self.graph_id_counter.get();
        self.graph_id_counter.set(id + 1);
        format!("graph-{}", id)
    }
}

impl Default for InMemoryGraphStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphStore for InMemoryGraphStore {
    fn store_workflow_graph(&self, graph: &WorkflowGraph) -> Result<GraphId, DarviumError> {
        let graph_id = self.next_graph_id();
        self.graphs.borrow_mut().insert(graph_id.clone(), graph.clone());
        Ok(graph_id)
    }

    fn load_workflow_graph(&self, graph_id: &GraphId) -> Result<WorkflowGraph, DarviumError> {
        self.graphs
            .borrow()
            .get(graph_id)
            .cloned()
            .ok_or_else(|| DarviumError::NotFound(format!("Graph not found: {}", graph_id)))
    }

    fn store_embedding(&self, key: &str, vector: &[f32]) -> Result<(), DarviumError> {
        if vector.is_empty() {
            return Err(DarviumError::Storage(
                "Cannot store empty embedding vector".to_string(),
            ));
        }
        self.embeddings.borrow_mut().insert(key.to_string(), vector.to_vec());
        Ok(())
    }

    fn semantic_search(
        &self,
        query: &[f32],
        top_k: u32,
    ) -> Result<Vec<(String, f64)>, DarviumError> {
        if query.is_empty() {
            return Err(DarviumError::Storage(
                "Query vector is empty".to_string(),
            ));
        }

        let embeddings = self.embeddings.borrow();

        // 登録ベクトルが存在しない場合は空結果を返す
        let expected_dim = match embeddings.values().next() {
            Some(vec) => vec.len(),
            // 登録ベクトルが存在しない場合は空結果を返す
            None => return Ok(Vec::new()),
        };
        if query.len() != expected_dim {
            return Err(DarviumError::EmbeddingDimensionMismatch {
                expected: expected_dim,
                actual: query.len(),
            });
        }

        let mut results: Vec<(String, f64)> = embeddings
            .iter()
            .map(|(key, vec)| {
                let similarity = cosine_similarity(query, vec);
                (key.clone(), similarity)
            })
            .collect();

        // 類似度降順でソート
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = (top_k as usize).min(results.len());
        results.truncate(top_k);

        Ok(results)
    }

    fn store_knowledge_object(&self, obj: &KnowledgeObject) -> Result<(), DarviumError> {
        self.knowledge_objects
            .borrow_mut()
            .insert(obj.id.clone(), obj.clone());
        Ok(())
    }

    fn load_knowledge_object(&self, object_id: &str) -> Result<KnowledgeObject, DarviumError> {
        self.knowledge_objects
            .borrow()
            .get(object_id)
            .cloned()
            .ok_or_else(|| DarviumError::NotFound(format!("Knowledge object not found: {}", object_id)))
    }

    fn store_relation(&self, relation: &RelationRecord) -> Result<(), DarviumError> {
        self.relations.borrow_mut().push(relation.clone());
        Ok(())
    }

    fn load_relations(&self, object_id: &str) -> Result<Vec<RelationRecord>, DarviumError> {
        let relations = self.relations.borrow();
        let filtered: Vec<RelationRecord> = relations
            .iter()
            .filter(|r| r.object_id == object_id)
            .cloned()
            .collect();
        Ok(filtered)
    }

    fn record_origin_trace(&self, trace: &OriginTrace) -> Result<(), DarviumError> {
        self.origin_traces.borrow_mut().push(trace.clone());
        Ok(())
    }

    fn load_origin_traces(&self, object_id: &str) -> Result<Vec<OriginTrace>, DarviumError> {
        let traces = self.origin_traces.borrow();
        let filtered: Vec<OriginTrace> = traces
            .iter()
            .filter(|t| t.object_id == object_id)
            .cloned()
            .collect();
        Ok(filtered)
    }
}

/// 2つのベクトル間のコサイン類似度を計算する。
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    // ============================================================
    // テスト1: InMemoryGraphStore が GraphStore トレイトを充足する
    // ============================================================
    #[test]
    fn in_memory_graph_store_implements_graph_store() {
        // コンパイル時検証: InMemoryGraphStore が GraphStore を実装していること
        let store: Box<dyn GraphStore> = Box::new(InMemoryGraphStore::new());
        let _ = store;
    }

    // ============================================================
    // テスト3: Box<dyn GraphStore> のオブジェクト安全性確認
    // ============================================================
    #[test]
    fn graph_store_trait_object_safety() {
        let store: Box<dyn GraphStore> = Box::new(InMemoryGraphStore::new());
        let graph = WorkflowGraph::new();
        let result = store.store_workflow_graph(&graph);
        assert!(result.is_ok());
    }

    // ============================================================
    // テスト5: グラフの登録 → 読取 → 内容一致確認
    // ============================================================
    #[test]
    fn store_and_load_workflow_graph() {
        let store = InMemoryGraphStore::new();
        let mut graph = WorkflowGraph::new();
        let node_idx = graph.add_node(WorkflowNode);
        graph.add_edge(node_idx, node_idx, EdgeMeta);

        let graph_id = store.store_workflow_graph(&graph).expect("store should succeed");
        let loaded = store.load_workflow_graph(&graph_id).expect("load should succeed");

        assert_eq!(loaded.node_count(), graph.node_count());
        assert_eq!(loaded.edge_count(), graph.edge_count());
    }

    // ============================================================
    // テスト6: semantic_search で同一ベクトルが類似度 1.0 で最上位に返る
    // ============================================================
    #[test]
    fn semantic_search_identical_vector() {
        let store = InMemoryGraphStore::new();
        let query = vec![1.0, 0.0, 0.0];

        store.store_embedding("vec-a", &query).expect("store should succeed");
        store.store_embedding("vec-b", &[0.0, 1.0, 0.0]).expect("store should succeed");
        store.store_embedding("vec-c", &[0.0, 0.0, 1.0]).expect("store should succeed");

        let results = store.semantic_search(&query, 3).expect("search should succeed");

        assert!(!results.is_empty(), "should return at least one result");
        assert_eq!(results[0].0, "vec-a", "identical vector should be top result");
        assert!((results[0].1 - 1.0).abs() < 1e-6, "similarity should be 1.0");
    }

    // ============================================================
    // テスト7: 知識オブジェクトの登録 → 読取 → 内容一致確認
    // ============================================================
    #[test]
    fn store_and_load_knowledge_object() {
        let store = InMemoryGraphStore::new();
        let obj = KnowledgeObject {
            id: "ko-1".to_string(),
            object_type: "test".to_string(),
            data: "test data".to_string(),
        };

        store.store_knowledge_object(&obj).expect("store should succeed");
        let loaded = store.load_knowledge_object("ko-1").expect("load should succeed");

        assert_eq!(loaded, obj);
    }

    // ============================================================
    // テスト8: リレーションの登録 → object_id 検索
    // ============================================================
    #[test]
    fn store_and_load_relations() {
        let store = InMemoryGraphStore::new();
        let rel = RelationRecord {
            object_id: "obj-1".to_string(),
            related_object_id: "obj-2".to_string(),
            relation_type: "depends_on".to_string(),
        };

        store.store_relation(&rel).expect("store should succeed");

        let loaded = store.load_relations("obj-1").expect("load should succeed");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], rel);
    }

    // ============================================================
    // テスト9: OriginTrace の記録 → 読取
    // ============================================================
    #[test]
    fn record_and_load_origin_traces() {
        let store = InMemoryGraphStore::new();
        let trace = OriginTrace {
            object_id: "obj-1".to_string(),
            source: "test-source".to_string(),
            timestamp_ms: 1000,
        };

        store.record_origin_trace(&trace).expect("record should succeed");

        let loaded = store.load_origin_traces("obj-1").expect("load should succeed");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], trace);
    }

    // ============================================================
    // テスト15: 3件のベクトル登録後、クエリに最も近いベクトルが top-1 で正しく返る
    // ============================================================
    #[test]
    fn semantic_search_top_k_ordering() {
        let store = InMemoryGraphStore::new();

        store.store_embedding("vec-a", &[1.0, 0.0, 0.0]).unwrap();
        store.store_embedding("vec-b", &[0.0, 1.0, 0.0]).unwrap();
        store.store_embedding("vec-c", &[0.0, 0.0, 1.0]).unwrap();

        let query = vec![0.9, 0.1, 0.0];
        let results = store.semantic_search(&query, 3).expect("search should succeed");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "vec-a", "vec-a should be top-1");
    }

    // ============================================================
    // テスト16: 同一 object_id に複数の OriginTrace を追加 → 全件取得
    // ============================================================
    #[test]
    fn multiple_origin_traces_for_same_object() {
        let store = InMemoryGraphStore::new();

        store.record_origin_trace(&OriginTrace {
            object_id: "obj-1".to_string(),
            source: "source-a".to_string(),
            timestamp_ms: 100,
        }).unwrap();
        store.record_origin_trace(&OriginTrace {
            object_id: "obj-1".to_string(),
            source: "source-b".to_string(),
            timestamp_ms: 200,
        }).unwrap();
        store.record_origin_trace(&OriginTrace {
            object_id: "obj-2".to_string(),
            source: "source-c".to_string(),
            timestamp_ms: 300,
        }).unwrap();

        let traces = store.load_origin_traces("obj-1").expect("load should succeed");
        assert_eq!(traces.len(), 2);
    }

    // ============================================================
    // テスト17: 存在しない graph_id の load → NotFound
    // ============================================================
    #[test]
    fn load_non_existent_graph_returns_not_found() {
        let store = InMemoryGraphStore::new();
        let result = store.load_workflow_graph(&"non-existent".to_string());
        assert!(matches!(result, Err(DarviumError::NotFound(_))));
    }

    // ============================================================
    // テスト18: 存在しない knowledge object の load → NotFound
    // ============================================================
    #[test]
    fn load_non_existent_knowledge_object_returns_not_found() {
        let store = InMemoryGraphStore::new();
        let result = store.load_knowledge_object("non-existent");
        assert!(matches!(result, Err(DarviumError::NotFound(_))));
    }

    // ============================================================
    // テスト19: 異なる次元数のベクトルで semantic_search → DimensionMismatch
    // ============================================================
    #[test]
    fn semantic_search_dimension_mismatch() {
        let store = InMemoryGraphStore::new();
        store.store_embedding("vec-3d", &[1.0, 0.0, 0.0]).unwrap();

        let query_2d = vec![1.0, 0.0];
        let result = store.semantic_search(&query_2d, 1);
        assert!(matches!(
            result,
            Err(DarviumError::EmbeddingDimensionMismatch { expected: 3, actual: 2 })
        ));
    }

    // ============================================================
    // テスト20: 空のベクトルで semantic_search → Storage エラー
    // ============================================================
    #[test]
    fn semantic_search_empty_query_returns_storage_error() {
        let store = InMemoryGraphStore::new();
        let result = store.semantic_search(&[], 1);
        assert!(matches!(result, Err(DarviumError::Storage(_))));
    }

    // ============================================================
    // 追加: 空のベクトル store_embedding → Storage エラー
    // ============================================================
    #[test]
    fn store_empty_embedding_returns_storage_error() {
        let store = InMemoryGraphStore::new();
        let result = store.store_embedding("empty", &[]);
        assert!(matches!(result, Err(DarviumError::Storage(_))));
    }

    // ============================================================
    // 追加: リレーションフィルタリングが正しく object_id で絞り込める
    // ============================================================
    #[test]
    fn relation_filtering_by_object_id() {
        let store = InMemoryGraphStore::new();

        store.store_relation(&RelationRecord {
            object_id: "obj-a".to_string(),
            related_object_id: "obj-b".to_string(),
            relation_type: "edge".to_string(),
        }).unwrap();
        store.store_relation(&RelationRecord {
            object_id: "obj-b".to_string(),
            related_object_id: "obj-c".to_string(),
            relation_type: "edge".to_string(),
        }).unwrap();

        let rels_a = store.load_relations("obj-a").expect("load should succeed");
        assert_eq!(rels_a.len(), 1);
        assert_eq!(rels_a[0].related_object_id, "obj-b");

        let rels_b = store.load_relations("obj-b").expect("load should succeed");
        assert_eq!(rels_b.len(), 1);
        assert_eq!(rels_b[0].related_object_id, "obj-c");
    }
}
