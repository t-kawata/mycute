// ワークフロー資産管理レジストリ (RFC §8.7, V-05/V-06)
//
// 全登録 MemoizedGraph を ID マップで保持し、登録・解決・存在確認を提供する。
// V-05 (SubWorkflow 参照先存在確認) および V-06 (入出力マッピング整合性) の
// バリデーション基盤として使用される。
//
// 永続化は GraphStore (LadybugDB / InMemoryGraphStore) が責務を持つ。
// WorkflowRegistry はドメイン層の registry であり、高速 lookup と全件走査を目的とする。

use std::collections::HashMap;

use crate::error::DarviumError;
use crate::trust::MemoizedGraph;
use crate::types::WorkflowGraphId;

/// ワークフロー資産管理レジストリ (RFC §8.7)。
///
/// 全登録 MemoizedGraph を ID マップで保持する。V-05/V-06 バリデーションの
/// 参照解決基盤。登録グラフの線形探索によるセマンティック検索も提供する。
///
/// 本 registry はドメイン層の役割を持ち、永続化 (GraphStore) とは独立している。
/// register() と永続化の同期は呼び出し元 (SearchWorkflow 等) が責務を持つ。
#[derive(Debug, Clone)]
pub struct WorkflowRegistry {
    /// ID → MemoizedGraph のマップ。
    graphs: HashMap<WorkflowGraphId, MemoizedGraph>,
    /// 自動インクリメント ID カウンタ。
    id_counter: u64,
}

impl WorkflowRegistry {
    /// 空のレジストリを生成する。
    pub fn new() -> Self {
        Self {
            graphs: HashMap::new(),
            id_counter: 0,
        }
    }

    /// MemoizedGraph を自動 ID 発行で登録し、発行された ID を返す。
    ///
    /// 登録前に memoized の id フィールドが空の場合、新しい ID で上書きする。
    /// 既に ID が設定されている場合はそのまま登録する。
    pub fn register(&mut self, mut memoized: MemoizedGraph) -> WorkflowGraphId {
        if memoized.id.is_empty() {
            let id = format!("wf-{:016x}", self.id_counter);
            self.id_counter += 1;
            memoized.id = id.clone();
            self.graphs.insert(id.clone(), memoized);
            id
        } else {
            let id = memoized.id.clone();
            self.graphs.insert(id.clone(), memoized);
            id
        }
    }

    /// 明示的な ID で MemoizedGraph を登録する。
    ///
    /// ID が既存の場合は `Err(DarviumError::NotFound(...))`。
    pub fn register_with_id(
        &mut self,
        id: WorkflowGraphId,
        memoized: MemoizedGraph,
    ) -> Result<WorkflowGraphId, DarviumError> {
        if self.graphs.contains_key(&id) {
            return Err(DarviumError::NotFound(format!(
                "WorkflowRegistry: ID '{}' already exists",
                id
            )));
        }
        let id2 = id.clone();
        self.graphs.insert(id, memoized);
        Ok(id2)
    }

    /// ワークフローグラフのみを自動 ID 発行で登録する（内部用簡易版）。
    ///
    /// 最小限の MemoizedGraph を生成し registry に登録する。
    /// 他フィールドはデフォルト値で初期化される。
    pub fn register_graph_only(
        &mut self,
        graph: crate::types::WorkflowGraph,
        mission: &str,
    ) -> WorkflowGraphId {
        let id = format!("wf-graph-{:016x}", self.id_counter);
        self.id_counter += 1;
        let task_embedding = crate::workflow_generation::mission_to_embedding(mission);
        let memoized = MemoizedGraph {
            id: id.clone(),
            graph,
            task_embedding,
            ..MemoizedGraph::default()
        };
        self.graphs.insert(id.clone(), memoized);
        id
    }

    /// ID に対応する MemoizedGraph への不変参照を返す。
    pub fn resolve(&self, graph_id: &WorkflowGraphId) -> Result<&MemoizedGraph, DarviumError> {
        self.graphs.get(graph_id).ok_or_else(|| {
            DarviumError::NotFound(format!(
                "WorkflowRegistry: graph '{}' not found",
                graph_id
            ))
        })
    }

    /// ID に対応する MemoizedGraph への可変参照を返す。
    pub fn resolve_mut(
        &mut self,
        graph_id: &WorkflowGraphId,
    ) -> Result<&mut MemoizedGraph, DarviumError> {
        self.graphs.get_mut(graph_id).ok_or_else(|| {
            DarviumError::NotFound(format!(
                "WorkflowRegistry: graph '{}' not found",
                graph_id
            ))
        })
    }

    /// ID が存在するかを確認する (V-05 バリデーション用)。
    pub fn exists(&self, graph_id: &WorkflowGraphId) -> bool {
        self.graphs.contains_key(graph_id)
    }

    /// 全登録グラフの不変イテレータを返す（検索エンジンの全件走査用）。
    pub fn all_graphs(&self) -> impl Iterator<Item = &MemoizedGraph> {
        self.graphs.values()
    }

    /// 登録グラフ総数を返す。
    pub fn graph_count(&self) -> usize {
        self.graphs.len()
    }

    /// 全登録グラフの埋め込みベクトルを線形探索し、
    /// クエリベクトルとのコサイン類似度上位 top_k 件を返す。
    pub fn semantic_search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Vec<(WorkflowGraphId, f64)> {
        if query_embedding.is_empty() || top_k == 0 {
            return Vec::new();
        }

        let mut scored: Vec<(WorkflowGraphId, f64)> = self
            .graphs
            .iter()
            .filter(|(_, g)| !g.task_embedding.is_empty())
            .map(|(id, g)| {
                let similarity = cosine_similarity(query_embedding, &g.task_embedding);
                (id.clone(), similarity)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// コサイン類似度を計算する。
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::WorkflowGraph;

    /// 空のレジストリは登録数 0。
    #[test]
    fn empty_registry() {
        let reg = WorkflowRegistry::new();
        assert_eq!(reg.graph_count(), 0);
    }

    /// register() で ID が自動発行される。
    #[test]
    fn register_auto_id() {
        let mut reg = WorkflowRegistry::new();
        let id = reg.register(MemoizedGraph::default());
        assert!(!id.is_empty());
        assert_eq!(reg.graph_count(), 1);
    }

    /// register_with_id() で明示 ID 登録と解決ができる。
    #[test]
    fn register_with_id_and_resolve() {
        let mut reg = WorkflowRegistry::new();
        let id = "my-graph".to_string();
        let memoized = MemoizedGraph {
            id: id.clone(),
            ..MemoizedGraph::default()
        };
        let registered_id = reg.register_with_id(id.clone(), memoized).unwrap();
        assert_eq!(registered_id, id);

        let resolved = reg.resolve(&id).unwrap();
        assert_eq!(resolved.id, id);
    }

    /// 重複 ID での register_with_id はエラー。
    #[test]
    fn duplicate_id_returns_error() {
        let mut reg = WorkflowRegistry::new();
        let id = "dup".to_string();
        reg.register_with_id(id.clone(), MemoizedGraph::default()).unwrap();
        let result = reg.register_with_id(id.clone(), MemoizedGraph::default());
        assert!(result.is_err());
    }

    /// exists() が存在/不在を正しく返す。
    #[test]
    fn exists_returns_correctly() {
        let mut reg = WorkflowRegistry::new();
        let id = reg.register(MemoizedGraph::default());
        assert!(reg.exists(&id));
        assert!(!reg.exists(&"nonexistent".to_string()));
    }

    /// 存在しない ID の resolve はエラー。
    #[test]
    fn resolve_missing_returns_error() {
        let reg = WorkflowRegistry::new();
        let result = reg.resolve(&"missing".to_string());
        assert!(result.is_err());
    }

    /// register_graph_only でグラフのみの簡易登録。
    #[test]
    fn register_graph_only_creates_entry() {
        let mut reg = WorkflowRegistry::new();
        let graph = WorkflowGraph::new();
        let id = reg.register_graph_only(graph, "test mission");
        assert!(!id.is_empty());
        assert!(reg.exists(&id));
        assert_eq!(reg.graph_count(), 1);
    }

    /// all_graphs() が全登録グラフを返す。
    #[test]
    fn all_graphs_returns_all() {
        let mut reg = WorkflowRegistry::new();
        reg.register(MemoizedGraph::default());
        reg.register(MemoizedGraph::default());
        reg.register(MemoizedGraph::default());
        assert_eq!(reg.all_graphs().count(), 3);
    }

    /// 空のクエリで semantic_search は空リストを返す。
    #[test]
    fn semantic_search_empty_query() {
        let reg = WorkflowRegistry::new();
        let results = reg.semantic_search(&[], 5);
        assert!(results.is_empty());
    }

    /// 登録グラフがない場合の semantic_search は空リスト。
    #[test]
    fn semantic_search_empty_registry() {
        let reg = WorkflowRegistry::new();
        let results = reg.semantic_search(&[0.1, 0.2], 5);
        assert!(results.is_empty());
    }

    /// top_k が結果数を制限する。
    #[test]
    fn semantic_search_top_k_limits_results() {
        let mut reg = WorkflowRegistry::new();
        for i in 0..10 {
            let mut embedding = vec![0.0f32; 4];
            embedding[0] = i as f32 * 0.1;
            let memoized = MemoizedGraph {
                id: format!("g{}", i),
                task_embedding: embedding,
                ..MemoizedGraph::default()
            };
            reg.register_with_id(format!("g{}", i), memoized).unwrap();
        }
        let results = reg.semantic_search(&[0.0, 0.0, 0.0, 0.0], 3);
        assert_eq!(results.len(), 3);
    }

    /// resolve_mut で可変参照を取得し変更できる。
    #[test]
    fn resolve_mut_allows_modification() {
        let mut reg = WorkflowRegistry::new();
        let id = reg.register(MemoizedGraph::default());
        let memo = reg.resolve_mut(&id).unwrap();
        memo.alive = false;
        assert!(!reg.resolve(&id).unwrap().alive);
    }
}
