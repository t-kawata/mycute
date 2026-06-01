// 首長レジストリ (Phase 3.9, RFC §TBD)
//
// ChiefRegistry は各村の首長情報を一元管理するスレッドセーフシングルトン。
// Phase 3.8（首長選出）完了後に sync_from_chiefs で再構築され、
// Phase 3.9（空間移動）で主首長の特定・最寄り首長の検索に使用される。

use std::collections::HashMap;

use crate::spaceposition::l2_distance;
use crate::trust::MemoizedGraph;
use crate::types::{PersonId, VillageId};

/// 首長エントリ: 一人の首長の情報を保持する。
#[derive(Debug, Clone)]
pub struct ChiefEntry {
    pub person_id: PersonId,
    pub position: [f32; 3],
    pub chiefdom_score: f32,
    pub village_id: VillageId,
}

/// 首長レジストリ: 全首長を管理し、主首長特定・距離検索を提供する。
///
/// Arc<RwLock<ChiefRegistry>> として SimulationContext に保持され、
/// Phase 3.8→3.9 の合間に sync_from_chiefs で再構築される。
#[derive(Debug, Clone)]
pub struct ChiefRegistry {
    pub chiefs: HashMap<PersonId, ChiefEntry>,
}

impl ChiefRegistry {
    /// 空のレジストリを生成する。
    pub fn new() -> Self {
        Self { chiefs: HashMap::new() }
    }

    /// Phase 3.8 の village_chiefs マップからレジストリを再構築する。
    ///
    /// village_chiefs は HashMap<VillageId, PersonId> の形式。
    /// population から該当個体の現在位置・首長性スコアを取得して登録する。
    pub fn sync_from_chiefs(
        &mut self,
        village_chiefs: &HashMap<VillageId, PersonId>,
        population: &[MemoizedGraph],
    ) {
        self.chiefs.clear();
        for (&village_id, &person_id) in village_chiefs {
            if let Some(person) = population.get(person_id) {
                if person.alive {
                    if let Some(pos) = person.position.inner() {
                        self.chiefs.insert(person_id, ChiefEntry {
                            person_id,
                            position: *pos,
                            chiefdom_score: person.reputation.chiefdom_score,
                            village_id,
                        });
                    }
                }
            }
        }
    }

    /// レジストリ内で chiefdom_score が最大の首長（主首長）を返す。
    ///
    /// 同点の場合は最初に見つかった方を採用する（暗黙の村ID順）。
    pub fn get_paramount(&self) -> Option<&ChiefEntry> {
        self.chiefs.values().max_by(|a, b| {
            a.chiefdom_score
                .partial_cmp(&b.chiefdom_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// 指定位置から最も近い首長を返す。
    ///
    /// レジストリが空の場合は None を返す。
    pub fn get_nearest(&self, pos: &[f32; 3]) -> Option<&ChiefEntry> {
        self.chiefs.values().min_by(|a, b| {
            let da = l2_distance(pos, &a.position);
            let db = l2_distance(pos, &b.position);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// 指定位置から2番目に近い首長を返す。
    ///
    /// 首長が1人しかいない場合は None を返す。
    pub fn get_second_nearest(&self, pos: &[f32; 3]) -> Option<&ChiefEntry> {
        let mut distances: Vec<(f64, &ChiefEntry)> = self
            .chiefs
            .values()
            .map(|entry| (l2_distance(pos, &entry.position), entry))
            .collect();
        distances.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        distances.get(1).map(|(_, entry)| *entry)
    }

    /// レジストリが空かどうかを返す。
    pub fn is_empty(&self) -> bool {
        self.chiefs.is_empty()
    }

    /// 登録されている首長の数を返す。
    pub fn len(&self) -> usize {
        self.chiefs.len()
    }
}

impl Default for ChiefRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spaceposition::SpacePositionEmbedding;
    use crate::trust::MemoizedGraph;
    use std::collections::HashMap;

    /// テスト用の MemoizedGraph を生成する。
    fn make_person(
        id: PersonId,
        position: [f32; 3],
        chiefdom_score: f32,
        village_id: Option<VillageId>,
        alive: bool,
    ) -> MemoizedGraph {
        let mut person = MemoizedGraph::new_with_position(
            id.to_string(),
            0.5,
            SpacePositionEmbedding::from(position),
            0u64,
        );
        person.alive = alive;
        person.reputation.chiefdom_score = chiefdom_score;
        person.village_assignment = village_id;
        person
    }

    // T1: 首長レジストリ基本操作
    #[test]
    fn t1_registry_basic_operations() {
        // 空レジストリ
        let reg = ChiefRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.get_paramount().is_none());
        assert!(reg.get_nearest(&[0.0, 0.0, 0.0]).is_none());
        assert!(reg.get_second_nearest(&[0.0, 0.0, 0.0]).is_none());

        // sync_from_chiefs で構築
        let mut reg = ChiefRegistry::new();
        let population = vec![
            make_person(0, [0.1, 0.1, 0.1], 0.8, Some(0), true),
            make_person(1, [0.5, 0.5, 0.5], 0.6, Some(0), true),
            make_person(2, [0.9, 0.9, 0.9], 0.9, Some(1), true),
        ];
        let village_chiefs: HashMap<VillageId, PersonId> =
            [(0, 0), (1usize, 2usize)].into_iter().collect();
        reg.sync_from_chiefs(&village_chiefs, &population);

        assert!(!reg.is_empty());
        assert_eq!(reg.len(), 2);

        // 主首長は chiefdom_score 最大の chief 2 (score=0.9)
        let paramount = reg.get_paramount().unwrap();
        assert_eq!(paramount.person_id, 2);
        assert!((paramount.chiefdom_score - 0.9).abs() < 1e-6);
    }

    // T2: 主首長が最大スコアの首長である
    #[test]
    fn t2_get_paramount_returns_highest_score() {
        let mut reg = ChiefRegistry::new();
        let population = vec![
            make_person(0, [0.0, 0.0, 0.0], 0.5, Some(0), true),
            make_person(1, [0.5, 0.5, 0.5], 0.8, Some(1), true),
            make_person(2, [1.0, 1.0, 1.0], 0.3, Some(0), true),
        ];
        let village_chiefs: HashMap<VillageId, PersonId> =
            [(0, 0), (1usize, 1usize)].into_iter().collect();
        reg.sync_from_chiefs(&village_chiefs, &population);

        let paramount = reg.get_paramount().unwrap();
        assert_eq!(paramount.person_id, 1);
        assert!((paramount.chiefdom_score - 0.8).abs() < 1e-6);
    }

    // T3: 距離順（nearest / second_nearest）
    #[test]
    fn t3_nearest_and_second_nearest() {
        let mut reg = ChiefRegistry::new();
        let population = vec![
            make_person(0, [0.1, 0.1, 0.1], 0.5, Some(0), true),
            make_person(1, [0.5, 0.5, 0.5], 0.6, Some(1), true),
            make_person(2, [0.9, 0.9, 0.9], 0.7, Some(0), true),
        ];
        let village_chiefs: HashMap<VillageId, PersonId> =
            [(0, 0), (1usize, 1usize), (2usize, 2usize)]
                .into_iter()
                .collect();
        reg.sync_from_chiefs(&village_chiefs, &population);

        // 位置 [0.0, 0.0, 0.0] から: 最寄り=0, 2番目=1 (距離順)
        let nearest = reg.get_nearest(&[0.0, 0.0, 0.0]).unwrap();
        assert_eq!(nearest.person_id, 0);
        let second = reg.get_second_nearest(&[0.0, 0.0, 0.0]).unwrap();
        assert_eq!(second.person_id, 1);

        // 1人のみ: get_second_nearest → None
        let mut reg1 = ChiefRegistry::new();
        let pop1 = vec![make_person(0, [0.0, 0.0, 0.0], 0.5, Some(0), true)];
        let chiefs1: HashMap<VillageId, PersonId> = [(0, 0)].into_iter().collect();
        reg1.sync_from_chiefs(&chiefs1, &pop1);
        assert!(reg1.get_second_nearest(&[0.5, 0.5, 0.5]).is_none());
    }

    // T9: 死亡個体がレジストリに含まれない
    #[test]
    fn t9_dead_person_not_in_registry() {
        let mut reg = ChiefRegistry::new();
        let population = vec![
            make_person(0, [0.1, 0.1, 0.1], 0.8, Some(0), true),
            make_person(1, [0.5, 0.5, 0.5], 0.9, Some(1), false),
        ];
        let village_chiefs: HashMap<VillageId, PersonId> =
            [(0, 0), (1usize, 1usize)].into_iter().collect();
        reg.sync_from_chiefs(&village_chiefs, &population);
        assert_eq!(reg.len(), 1);
        assert!(reg.chiefs.contains_key(&0));
        assert!(!reg.chiefs.contains_key(&1));
    }
}
