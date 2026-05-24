// JsonMetadataStore — 簡易ファイル永続化 MetadataStore
//
// HITL インタラクションのみ JSON ファイルに永続化し、
// 非 HITL データは InMemoryMetadataStore と同様にメモリ上で管理する。
// 書込操作のたびにファイルへ原子書き込み（一時ファイル + rename）を実行する。

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::DarviumError;
use crate::store::MetadataStore;
use crate::types::{
    FusionMetadata, HumanOutcome, InteractionStatus, PatchHistory, SearchTrace, StoredInteraction,
    TrainingMetadata, TrustAuditLog,
};

/// ファイルに永続化するデータ構造。
#[derive(Serialize, Deserialize)]
struct PersistentData {
    human_interactions: HashMap<String, StoredInteraction>,
}

/// 簡易ファイル永続化 MetadataStore 実装。
///
/// # ファイル形式
///
/// ```json
/// {
///   "human_interactions": {
///     "uuid-1": { ... StoredInteraction ... }
///   }
/// }
/// ```
///
/// # 原子書き込み
///
/// 変更操作のたびに一時ファイル (`{path}.tmp`) に書き込んでから
/// `fs::rename` で置き換える。書き込み途中のクラッシュ後も元ファイルは完全な状態で残る。
pub struct JsonMetadataStore {
    path: PathBuf,
    search_traces: RefCell<HashMap<String, Vec<SearchTrace>>>,
    trust_audit_logs: RefCell<HashMap<String, Vec<TrustAuditLog>>>,
    patch_histories: RefCell<HashMap<String, Vec<PatchHistory>>>,
    training_metadata: RefCell<HashMap<String, TrainingMetadata>>,
    fusion_metadata: RefCell<HashMap<String, FusionMetadata>>,
    human_interactions: RefCell<HashMap<String, StoredInteraction>>,
}

impl JsonMetadataStore {
    /// ファイルパスを指定して JsonMetadataStore を生成する。
    ///
    /// ファイルが存在しない場合は空状態で初期化する（初回起動時）。
    /// ファイルが存在する場合は読み込んで復元する。
    /// ファイルが破損している場合は `Err(Storage)` を返す。
    pub fn new(path: impl AsRef<Path>) -> Result<Self, DarviumError> {
        let path = path.as_ref().to_path_buf();
        let human_interactions = if path.exists() {
            let content = fs::read_to_string(&path)
                .map_err(|e| DarviumError::Storage(format!("cannot read store file: {}", e)))?;
            let data: PersistentData = serde_json::from_str(&content)
                .map_err(|e| DarviumError::Storage(format!("corrupted store file: {}", e)))?;
            data.human_interactions
        } else {
            HashMap::new()
        };

        Ok(Self {
            path,
            search_traces: RefCell::new(HashMap::new()),
            trust_audit_logs: RefCell::new(HashMap::new()),
            patch_histories: RefCell::new(HashMap::new()),
            training_metadata: RefCell::new(HashMap::new()),
            fusion_metadata: RefCell::new(HashMap::new()),
            human_interactions: RefCell::new(human_interactions),
        })
    }

    /// 現在の全 HITL インタラクションをファイルに原子書き込みする。
    fn flush(&self) -> Result<(), DarviumError> {
        let data = PersistentData {
            human_interactions: self.human_interactions.borrow().clone(),
        };
        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| DarviumError::Storage(format!("serialization error: {}", e)))?;

        // 一時ファイルに書き込んでから rename（原子置き換え）
        let tmp_path = self.path.with_extension("tmp");
        {
            let mut tmp_file = fs::File::create(&tmp_path)
                .map_err(|e| DarviumError::Storage(format!("cannot create temp file: {}", e)))?;
            tmp_file
                .write_all(json.as_bytes())
                .map_err(|e| DarviumError::Storage(format!("cannot write temp file: {}", e)))?;
            tmp_file
                .flush()
                .map_err(|e| DarviumError::Storage(format!("cannot flush temp file: {}", e)))?;
        }
        fs::rename(&tmp_path, &self.path)
            .map_err(|e| DarviumError::Storage(format!("cannot rename store file: {}", e)))?;

        Ok(())
    }
}

impl MetadataStore for JsonMetadataStore {
    fn store_search_trace(&self, trace: &SearchTrace) -> Result<(), DarviumError> {
        self.search_traces
            .borrow_mut()
            .entry("default".to_string())
            .or_default()
            .push(trace.clone());
        Ok(())
    }

    fn load_search_traces(&self, _mission_id: &str) -> Result<Vec<SearchTrace>, DarviumError> {
        let traces = self.search_traces.borrow();
        let all: Vec<SearchTrace> = traces.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_trust_audit_log(&self, log: &TrustAuditLog) -> Result<(), DarviumError> {
        self.trust_audit_logs
            .borrow_mut()
            .entry("default".to_string())
            .or_default()
            .push(log.clone());
        Ok(())
    }

    fn load_trust_audit_logs(&self, _target_id: &str) -> Result<Vec<TrustAuditLog>, DarviumError> {
        let logs = self.trust_audit_logs.borrow();
        let all: Vec<TrustAuditLog> = logs.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_patch_history(&self, history: &PatchHistory) -> Result<(), DarviumError> {
        self.patch_histories
            .borrow_mut()
            .entry("default".to_string())
            .or_default()
            .push(history.clone());
        Ok(())
    }

    fn load_patch_histories(&self, _graph_id: &str) -> Result<Vec<PatchHistory>, DarviumError> {
        let histories = self.patch_histories.borrow();
        let all: Vec<PatchHistory> = histories.values().flat_map(|v| v.iter()).cloned().collect();
        Ok(all)
    }

    fn store_training_metadata(&self, metadata: &TrainingMetadata) -> Result<(), DarviumError> {
        self.training_metadata
            .borrow_mut()
            .insert(metadata.mission_id.clone(), metadata.clone());
        Ok(())
    }

    fn load_training_metadata(&self, mission_id: &str) -> Result<TrainingMetadata, DarviumError> {
        self.training_metadata
            .borrow()
            .get(mission_id)
            .cloned()
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Training metadata not found: {}", mission_id))
            })
    }

    fn store_fusion_metadata(&self, metadata: &FusionMetadata) -> Result<(), DarviumError> {
        self.fusion_metadata
            .borrow_mut()
            .insert(metadata.pair_id.clone(), metadata.clone());
        Ok(())
    }

    fn load_fusion_metadata(&self, pair_id: &str) -> Result<FusionMetadata, DarviumError> {
        self.fusion_metadata
            .borrow()
            .get(pair_id)
            .cloned()
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Fusion metadata not found: {}", pair_id))
            })
    }

    fn store_human_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError> {
        self.human_interactions
            .borrow_mut()
            .insert(record.interaction_id.clone(), record.clone());
        self.flush()?;
        Ok(())
    }

    fn load_human_interaction(
        &self,
        interaction_id: &str,
    ) -> Result<StoredInteraction, DarviumError> {
        self.human_interactions
            .borrow()
            .get(interaction_id)
            .cloned()
            .ok_or_else(|| {
                DarviumError::NotFound(format!("Human interaction not found: {}", interaction_id))
            })
    }

    fn list_pending_human_interactions(&self) -> Result<Vec<StoredInteraction>, DarviumError> {
        let interactions = self.human_interactions.borrow();
        let pending: Vec<StoredInteraction> = interactions
            .values()
            .filter(|r| r.status == InteractionStatus::Pending)
            .cloned()
            .collect();
        Ok(pending)
    }

    fn resolve_human_interaction(
        &self,
        interaction_id: &str,
        outcome: &HumanOutcome,
    ) -> Result<(), DarviumError> {
        let mut interactions = self.human_interactions.borrow_mut();
        let record = interactions.get_mut(interaction_id).ok_or_else(|| {
            DarviumError::NotFound(format!("Human interaction not found: {}", interaction_id))
        })?;
        record.outcome = Some(outcome.clone());
        record.status = InteractionStatus::Resolved;
        drop(interactions);
        self.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::io::Write;

    // ============================================================
    // T-JM1: JsonMetadataStore 基本動作
    // store → ファイル書込 → 再読込で同一内容が復元されること
    // ============================================================
    #[test]
    fn json_metadata_store_basic_persistence() {
        let dir = std::env::temp_dir().join(format!("darvium-jm1-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.json");

        {
            let store = JsonMetadataStore::new(&path).unwrap();
            let record = StoredInteraction {
                interaction_id: "id-1".to_string(),
                payload: HitlPayload { request: HumanRequest {
                    subject: "persist".into(),
                    body: "test".into(),
                    context: serde_json::json!({"key": "value"}),
                    timeout: None,
                },},
                outcome: Some(HumanOutcome::Responded(HumanResponse {
                    decision: HumanDecision::Approved,
                    comment: Some("persisted".into()),
                    revised_body: None,
                })),
                status: InteractionStatus::Resolved,
                created_at: 1000,
                updated_at: 1000,
            };
            store.store_human_interaction(&record).unwrap();
        }

        // 新インスタンスで再読込
        let store2 = JsonMetadataStore::new(&path).unwrap();
        let loaded = store2.load_human_interaction("id-1").unwrap();
        assert_eq!(loaded.interaction_id, "id-1");
        assert_eq!(loaded.status, InteractionStatus::Resolved);
        assert!(matches!(loaded.outcome, Some(HumanOutcome::Responded(_))));

        let _ = fs::remove_dir_all(&dir);
    }

    // ============================================================
    // T-JM2: JsonMetadataStore 原子書き込み
    // 一時ファイル書込中のクラッシュを模擬しても、元ファイルが完全なこと
    // ============================================================
    #[test]
    fn json_metadata_store_atomic_write() {
        let dir = std::env::temp_dir().join(format!("darvium-jm2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.json");

        // 正常に 1 件保存
        {
            let store = JsonMetadataStore::new(&path).unwrap();
            let record = StoredInteraction {
                interaction_id: "atomic-1".to_string(),
                payload: HitlPayload { request: HumanRequest {
                    subject: "atomic".into(),
                    body: "original".into(),
                    context: serde_json::json!({}),
                    timeout: None,
                },},
                outcome: None,
                status: InteractionStatus::Pending,
                created_at: 100,
                updated_at: 100,
            };
            store.store_human_interaction(&record).unwrap();
        }

        // 一時ファイルだけを破損させる（rename 前にクラッシュした状態を模擬）
        let tmp_path = path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp_path).unwrap();
            write!(f, "corrupted data").unwrap();
        }

        // 元ファイルは完全なまま
        let store2 = JsonMetadataStore::new(&path).unwrap();
        let loaded = store2.load_human_interaction("atomic-1").unwrap();
        assert_eq!(loaded.request().subject, "atomic");

        let _ = fs::remove_dir_all(&dir);
    }

    // ============================================================
    // T9: 初回起動時ファイル不在
    // ファイルが存在しない → 空状態で正常初期化
    // ============================================================
    #[test]
    fn initial_start_with_no_file() {
        let dir = std::env::temp_dir().join(format!("darvium-t9-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("nonexistent.json");

        // ファイルが存在しない状態で初期化 → 正常に空状態で開始
        let store = JsonMetadataStore::new(&path).unwrap();
        let pending = store.list_pending_human_interactions().unwrap();
        assert!(pending.is_empty(), "空状態でなければならない");

        // 書き込み後、ファイルが作成されていること
        let record = StoredInteraction {
            interaction_id: "first".to_string(),
            payload: HitlPayload { request: HumanRequest {
                subject: "init".into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },},
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 0,
            updated_at: 0,
        };
        store.store_human_interaction(&record).unwrap();
        assert!(path.exists(), "store 後にファイルが作成されていること");

        let _ = fs::remove_dir_all(&dir);
    }

    // ============================================================
    // T10: ファイル破損（不正 JSON）
    // 破損ファイル → Err を返し、クラッシュしない
    // ============================================================
    #[test]
    fn corrupted_json_file() {
        let dir = std::env::temp_dir().join(format!("darvium-t10-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corrupted.json");

        // 破損 JSON を書き込む
        {
            let mut f = fs::File::create(&path).unwrap();
            write!(f, "corrupted json").unwrap();
        }

        // 初期化が Err を返すこと
        let result = JsonMetadataStore::new(&path);
        assert!(result.is_err(), "破損ファイルからの初期化は Err を返すこと");
        assert!(
            matches!(result, Err(DarviumError::Storage(_))),
            "Storage エラーを返すこと"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    // ============================================================
    // Pending のみ抽出
    // ============================================================
    #[test]
    fn list_pending_only() {
        let dir = std::env::temp_dir().join(format!("darvium-pending-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.json");
        let store = JsonMetadataStore::new(&path).unwrap();

        let make = |id: &str, status: InteractionStatus| StoredInteraction {
            interaction_id: id.to_string(),
            payload: HitlPayload { request: HumanRequest {
                subject: id.into(),
                body: "".into(),
                context: serde_json::json!({}),
                timeout: None,
            },},
            outcome: None,
            status,
            created_at: 0,
            updated_at: 0,
        };
        store
            .store_human_interaction(&make("p1", InteractionStatus::Pending))
            .unwrap();
        store
            .store_human_interaction(&make("p2", InteractionStatus::Pending))
            .unwrap();
        store
            .store_human_interaction(&make("r1", InteractionStatus::Resolved))
            .unwrap();

        let pending = store.list_pending_human_interactions().unwrap();
        assert_eq!(pending.len(), 2);
        assert!(pending
            .iter()
            .all(|r| r.status == InteractionStatus::Pending));

        let _ = fs::remove_dir_all(&dir);
    }

    // ============================================================
    // resolve で状態遷移 + ファイル永続化
    // ============================================================
    #[test]
    fn resolve_persists_to_file() {
        let dir = std::env::temp_dir().join(format!("darvium-resolve-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("store.json");

        {
            let store = JsonMetadataStore::new(&path).unwrap();
            let record = StoredInteraction {
                interaction_id: "resolve-id".to_string(),
                payload: HitlPayload { request: HumanRequest {
                    subject: "resolve-test".into(),
                    body: "".into(),
                    context: serde_json::json!({}),
                    timeout: None,
                },},
                outcome: None,
                status: InteractionStatus::Pending,
                created_at: 0,
                updated_at: 0,
            };
            store.store_human_interaction(&record).unwrap();
            let outcome = HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            });
            store
                .resolve_human_interaction("resolve-id", &outcome)
                .unwrap();
        }

        // 再読込して解決状態が維持されていること
        let store2 = JsonMetadataStore::new(&path).unwrap();
        let loaded = store2.load_human_interaction("resolve-id").unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);

        let _ = fs::remove_dir_all(&dir);
    }
}
