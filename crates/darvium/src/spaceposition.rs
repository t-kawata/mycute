// 空間位置埋め込みと位置更新ダイナミクス (RFC §41B.2)
//
// 各 MemoizedGraph の生態学的位置 (ecological position) を表現する型と、
// 指数平滑化による位置更新純粋関数、EventBus への位置更新イベント publish を提供する。
//
// 本モジュールは M1.75 マイルストーン（Child Support Villages / HELP Consensus）の
// 基盤であり、後続の village・help モジュールは本モジュールの型と関数に依存する。

use crate::constants::{SPACE_POSITION_L2_EPSILON, SPACE_POSITION_UPDATE_MIN_INTERVAL};
use crate::error::DarviumError;
use crate::event::{DarviumEventBus, DarviumEventKind, SpacePositionUpdatedPayload, SystemEvent};
use crate::types::VillageObservation;

// ============================================================
// 型定義 (RFC §41B.2)
// ============================================================

/// ワークフローの空間位置埋め込み newtype (RFC §41B.2)。
///
/// `Option<[f32; 3]>` のラッパーで、3次元連続空間における生態学的位置を表現する。
/// `None` は局所性不明 (locality-unknown) を意味し、中立的動作にフォールバックする。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacePositionEmbedding(Option<[f32; 3]>);

impl SpacePositionEmbedding {
    /// 局所性不明 (locality-unknown) を表すインスタンスを返す。
    pub fn unknown() -> Self {
        Self(None)
    }

    /// 内部の `Option<[f32; 3]>` への参照を返す。
    pub fn inner(&self) -> &Option<[f32; 3]> {
        &self.0
    }
}

impl From<[f32; 3]> for SpacePositionEmbedding {
    fn from(pos: [f32; 3]) -> Self {
        Self(Some(pos))
    }
}

impl From<Option<[f32; 3]>> for SpacePositionEmbedding {
    fn from(pos: Option<[f32; 3]>) -> Self {
        Self(pos)
    }
}

/// ランタイムの位置情報 (RFC §41B.2)。
///
/// 位置ベクトルと、最後に更新された VirtualClock 時刻を保持する。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VillagePosition {
    /// 3次元連続空間における位置ベクトル。
    pub position: [f32; 3],
    /// 最後に位置が更新された VirtualClock の時刻。
    pub last_updated_vt: u64,
}

impl VillagePosition {
    /// 指定された位置と時刻で新しい VillagePosition を生成する。
    pub fn new(position: [f32; 3], last_updated_vt: u64) -> Self {
        Self {
            position,
            last_updated_vt,
        }
    }
}

/// 位置更新ポリシー (RFC §41B.2)。
///
/// VirtualClock に基づく位置更新の条件を規定する。
#[derive(Debug, Clone, PartialEq)]
pub enum PositionUpdatePolicy {
    /// 常に更新を許可する。
    Always,
    /// 指定された最小間隔 (ticks) 以上経過した場合のみ更新を許可する。
    MinInterval(u64),
    /// 現在の VirtualClock 値に関わらず、明示的なトリガーのみで更新する。
    /// 現在のところ update_space_position の直接呼び出しがトリガーとなる。
    Manual,
}

impl Default for PositionUpdatePolicy {
    fn default() -> Self {
        PositionUpdatePolicy::MinInterval(SPACE_POSITION_UPDATE_MIN_INTERVAL)
    }
}

// ============================================================
// 純粋関数
// ============================================================

/// 指数平滑化による位置更新 (RFC §41B.2 式 41B-1)。
///
/// x_{t+1} = (1 - α) · x_t + α · Δ_t
///
/// ここで x_t は現在位置、Δ_t は観測の delta、α は指数平滑化率。
///
/// # 引数
/// - `prev`: 更新前の位置ベクトル
/// - `obs`: 観測（delta が平滑化の入力値として使用される）
/// - `alpha`: 指数平滑化率 (0.0 ≤ alpha ≤ 1.0)
///
/// # 境界値
/// - `alpha = 0.0`: 更新後位置は `prev` と完全一致
/// - `alpha = 1.0`: 更新後位置は `obs.delta` と完全一致
pub fn update_space_position(prev: [f32; 3], obs: &VillageObservation, alpha: f64) -> [f32; 3] {
    let alpha_f32 = alpha as f32;
    let one_minus_alpha = 1.0 - alpha_f32;
    [
        one_minus_alpha * prev[0] + alpha_f32 * obs.delta[0],
        one_minus_alpha * prev[1] + alpha_f32 * obs.delta[1],
        one_minus_alpha * prev[2] + alpha_f32 * obs.delta[2],
    ]
}

/// VirtualClock ベースの更新実行判定。
///
/// 現在時刻と最終更新時刻の差がポリシーの条件を満たす場合に true を返す。
///
/// # 引数
/// - `last_updated_vt`: 最終更新時の VirtualClock 値
/// - `now_vt`: 現在の VirtualClock 値
/// - `policy`: 位置更新ポリシー
pub fn should_update_position(
    last_updated_vt: u64,
    now_vt: u64,
    policy: &PositionUpdatePolicy,
) -> bool {
    match policy {
        PositionUpdatePolicy::Always => true,
        PositionUpdatePolicy::MinInterval(interval) => {
            now_vt.saturating_sub(last_updated_vt) >= *interval
        }
        PositionUpdatePolicy::Manual => false,
    }
}

/// L2 距離（ユークリッド距離）を計算する。
///
/// RFC §41B.2 式 41B-5 の局所性距離計算に使用する補助関数。
/// 固定点収束テストや近傍距離計算で利用される。
pub fn l2_distance(a: &[f32; 3], b: &[f32; 3]) -> f64 {
    let dx = (a[0] - b[0]) as f64;
    let dy = (a[1] - b[1]) as f64;
    let dz = (a[2] - b[2]) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// 3 次元位置ベクトルを各成分に分解する (RFC §41B-2)。
///
/// 複合 WorkflowGraph の位置を 3 成分 (空間的・時間的・認知的) に分解する。
/// 本実装では簡略化として各次元をそのまま返す。
/// 本番では非線形射影や正規化が適用される可能性がある。
///
/// # 引数
/// - `pos`: 3 次元位置ベクトル
///
/// # 戻り値
/// `(f32, f32, f32)` — (空間成分, 時間成分, 認知成分)
pub fn decompose_position(pos: [f32; 3]) -> (f32, f32, f32) {
    (pos[0], pos[1], pos[2])
}

/// 2つの位置が SPACE_POSITION_L2_EPSILON 以内で同一とみなせるかを判定する。
pub fn is_position_equal(a: &[f32; 3], b: &[f32; 3]) -> bool {
    l2_distance(a, b) < SPACE_POSITION_L2_EPSILON
}

// ============================================================
// EventBus publish
// ============================================================

/// 位置更新イベントを EventBus に publish する。
///
/// 位置更新発生時に DarviumEventKind::System(SpacePositionUpdated) イベントを
/// EventBus へ OneWay モードで publish する。更新1件につきイベント1件の対応を保証する。
///
/// # 引数
/// - `bus`: DarviumEventBus の実装
/// - `prev`: 更新前の位置
/// - `current`: 更新後の位置
/// - `observation`: 更新に使用された観測
/// - `alpha`: 適用された指数平滑化率
///
/// # 戻り値
/// - `Ok(())`: イベントが正常に publish された
/// - `Err(DarviumError)`: publish に失敗した
pub fn publish_position_update(
    bus: &dyn DarviumEventBus,
    prev: [f32; 3],
    current: [f32; 3],
    observation: VillageObservation,
    alpha: f64,
) -> Result<(), DarviumError> {
    let event_kind = DarviumEventKind::System(SystemEvent::SpacePositionUpdated(
        SpacePositionUpdatedPayload {
            prev,
            current,
            observation,
            alpha,
        },
    ));

    // 最小構成の DarviumEvent を生成して publish (OneWay)
    let event = crate::event::DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: event_kind,
        interaction_mode: crate::event::InteractionMode::OneWay,
        payload: serde_json::Value::Null,
        causality: crate::event::EventCausality {
            parent_event_id: None,
            root_event_id: None,
            trace_ref: None,
            mission_id: None,
            workflow_id: None,
            run_id: None,
        },
        metadata: crate::event::EventMetadata {
            clock: bus.current_clock(),
            timestamp: std::time::SystemTime::now(),
            source: crate::event::EventSource::System,
        },
        transport_meta: None,
        visibility: crate::event::EventVisibility::Internal,
        retention: crate::event::EventRetention {
            persist: true,
            ttl_days: None,
        },
        privacy: crate::event::EventPrivacy {
            contains_pii: false,
            sandbox_only: false,
            pii_handling: crate::event::PiiHandlingPolicy::Reject,
        },
    };

    let _event_id = bus.publish(event)?;
    Ok(())
}

// ============================================================
// ユニットテスト (M1.75-1: T-1 .. T-8)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::SPACE_POSITION_UPDATE_ALPHA;
    use crate::event::FakeEventBus;

    // -------------------------------------------------------
    // T-1: 位置更新の境界値 (alpha = 0.0)
    // -------------------------------------------------------
    #[test]
    fn test_update_alpha_zero() {
        let prev = [1.0, 2.0, 3.0];
        let obs = VillageObservation::new([10.0, 20.0, 30.0]);
        let result = update_space_position(prev, &obs, 0.0);
        assert_eq!(
            result, prev,
            "alpha = 0.0 のとき更新後位置は prev と完全一致する必要があります"
        );
    }

    // -------------------------------------------------------
    // T-2: 位置更新の境界値 (alpha = 1.0)
    // -------------------------------------------------------
    #[test]
    fn test_update_alpha_one() {
        let prev = [1.0, 2.0, 3.0];
        let obs = VillageObservation::new([10.0, 20.0, 30.0]);
        let result = update_space_position(prev, &obs, 1.0);
        assert_eq!(
            result, obs.delta,
            "alpha = 1.0 のとき更新後位置は obs.delta と完全一致する必要があります"
        );
    }

    // -------------------------------------------------------
    // T-3: 指数固定点収束
    // -------------------------------------------------------
    #[test]
    fn test_exponential_convergence_to_fixed_point() {
        let alpha = 0.3;
        let obs = VillageObservation::new([5.0, 5.0, 5.0]);

        // 初期位置を観測とは異なる値に設定
        let mut pos = [0.0, 0.0, 0.0];
        let fixed_point = obs.delta;
        let mut prev_residual = l2_distance(&pos, &fixed_point);

        // 反復ごとに残差が単調減少することを確認
        for step in 1..=50 {
            pos = update_space_position(pos, &obs, alpha);
            let residual = l2_distance(&pos, &fixed_point);

            assert!(
                residual < prev_residual + 1e-10,
                "ステップ {} で残差が単調減少しません: 前={}, 現在={}",
                step,
                prev_residual,
                residual
            );
            prev_residual = residual;
        }

        // 十分な反復後、固定点に収束していることを確認
        // f32 の精度限界（約7桁）を考慮して 5e-6 で判定
        let convergence_eps = 5e-6;
        let residual = l2_distance(&pos, &fixed_point);
        assert!(
            residual < convergence_eps,
            "50 回の反復後も固定点に収束していません: pos={:?}, fixed_point={:?}, residual={}",
            pos,
            fixed_point,
            residual
        );
    }

    // -------------------------------------------------------
    // T-4: L2 距離の正値性・同一性・三角不等式
    // -------------------------------------------------------
    #[test]
    fn test_l2_distance_identity() {
        let a = [1.0, 2.0, 3.0];
        assert!(
            l2_distance(&a, &a) < SPACE_POSITION_L2_EPSILON,
            "同一位置間の L2 距離は 0 である必要があります"
        );
    }

    #[test]
    fn test_l2_distance_non_negative() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        assert!(
            l2_distance(&a, &b) >= 0.0,
            "L2 距離は非負である必要があります"
        );
        assert!(
            (l2_distance(&a, &b) - l2_distance(&b, &a)).abs() < 1e-10,
            "L2 距離は対称である必要があります"
        );
    }

    #[test]
    fn test_l2_distance_triangle_inequality() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 0.0, 0.0];
        let c = [0.0, 1.0, 0.0];
        let d_ab = l2_distance(&a, &b);
        let d_bc = l2_distance(&b, &c);
        let d_ac = l2_distance(&a, &c);
        assert!(
            d_ac <= d_ab + d_bc + 1e-10,
            "三角不等式が成立する必要があります: d_ac={} <= d_ab={} + d_bc={}",
            d_ac,
            d_ab,
            d_bc
        );
    }

    // -------------------------------------------------------
    // T-5: should_update_position の更新窓制御
    // -------------------------------------------------------
    #[test]
    fn test_should_update_same_tick() {
        assert!(
            !should_update_position(5, 5, &PositionUpdatePolicy::MinInterval(1)),
            "last_updated_vt == now_vt のとき false を返す必要があります"
        );
    }

    #[test]
    fn test_should_update_sufficient_interval() {
        assert!(
            should_update_position(5, 10, &PositionUpdatePolicy::MinInterval(5)),
            "now - last >= interval のとき true を返す必要があります"
        );
    }

    #[test]
    fn test_should_update_insufficient_interval() {
        assert!(
            !should_update_position(5, 9, &PositionUpdatePolicy::MinInterval(5)),
            "now - last < interval のとき false を返す必要があります"
        );
    }

    #[test]
    fn test_should_update_always() {
        assert!(
            should_update_position(0, 0, &PositionUpdatePolicy::Always),
            "Always ポリシーでは常に true を返す必要があります"
        );
        assert!(
            should_update_position(100, 200, &PositionUpdatePolicy::Always),
            "Always ポリシーでは常に true を返す必要があります"
        );
    }

    #[test]
    fn test_should_update_manual() {
        assert!(
            !should_update_position(0, 100, &PositionUpdatePolicy::Manual),
            "Manual ポリシーでは常に false を返す必要があります"
        );
    }

    // -------------------------------------------------------
    // T-6: SpacePositionUpdate イベント publish
    // -------------------------------------------------------
    #[test]
    fn test_publish_position_update_event() {
        let bus = FakeEventBus::new();
        let prev = [1.0, 2.0, 3.0];
        let current = [4.0, 5.0, 6.0];
        let obs = VillageObservation::new(current);
        let alpha = 0.3;

        publish_position_update(&bus, prev, current, obs, alpha)
            .expect("publish が成功する必要があります");

        let published = bus.published_events();
        assert_eq!(
            published.len(),
            1,
            "位置更新1件につきイベント1件が publish される必要があります"
        );

        let event = &published[0];
        match &event.kind {
            DarviumEventKind::System(SystemEvent::SpacePositionUpdated(payload)) => {
                assert_eq!(
                    payload.prev, prev,
                    "prev が正しく格納されている必要があります"
                );
                assert_eq!(
                    payload.current, current,
                    "current が正しく格納されている必要があります"
                );
                assert_eq!(
                    payload.observation, obs,
                    "obs が正しく格納されている必要があります"
                );
                assert!(
                    (payload.alpha - alpha).abs() < 1e-10,
                    "alpha が正しく格納されている必要があります"
                );
            }
            other => panic!(
                "SpacePositionUpdated イベントが publish される必要があります: {:?}",
                other
            ),
        }
    }

    // -------------------------------------------------------
    // T-7: SpacePositionEmbedding newtype
    // -------------------------------------------------------
    #[test]
    fn test_space_position_embedding_from_array() {
        let pos = SpacePositionEmbedding::from([1.0, 2.0, 3.0]);
        assert_eq!(
            pos.inner(),
            &Some([1.0, 2.0, 3.0]),
            "from([x, y, z]) で正しく生成できる必要があります"
        );
    }

    #[test]
    fn test_space_position_embedding_unknown() {
        let pos = SpacePositionEmbedding::unknown();
        assert_eq!(pos.inner(), &None, "unknown() は None を返す必要があります");
    }

    #[test]
    fn test_space_position_embedding_from_option_none() {
        let pos = SpacePositionEmbedding::from(None);
        assert_eq!(
            pos.inner(),
            &None,
            "from(None) は None を返す必要があります"
        );
    }

    // -------------------------------------------------------
    // T-8: VillagePosition の構築
    // -------------------------------------------------------
    #[test]
    fn test_village_position_new() {
        let vp = VillagePosition::new([1.0, 2.0, 3.0], 42);
        assert_eq!(vp.position, [1.0, 2.0, 3.0]);
        assert_eq!(vp.last_updated_vt, 42);
    }

    // -------------------------------------------------------
    // T-E1: 位置更新の線形補間特性（境界値の中間確認）
    // -------------------------------------------------------
    #[test]
    fn test_update_linear_interpolation() {
        // alpha = 0.5 のとき、結果が prev と obs.delta の中間になることを確認
        let prev = [0.0, 0.0, 0.0];
        let obs = VillageObservation::new([10.0, 10.0, 10.0]);
        let result = update_space_position(prev, &obs, 0.5);
        let expected = [5.0, 5.0, 5.0];
        for i in 0..3 {
            assert!(
                (result[i] - expected[i]).abs() < 1e-6,
                "alpha=0.5 で中間値になる必要があります: result={:?}, expected={:?}",
                result,
                expected
            );
        }
    }

    // -------------------------------------------------------
    // T-E2: SpacePositionEmbedding アクセサ
    // -------------------------------------------------------
    #[test]
    fn test_space_position_embedding_inner() {
        let pos = SpacePositionEmbedding::from([1.5, 2.5, 3.5]);
        let inner = pos.inner();
        if let Some(arr) = inner {
            assert_eq!(arr[0], 1.5);
            assert_eq!(arr[1], 2.5);
            assert_eq!(arr[2], 3.5);
        } else {
            panic!("Some が期待される位置で None が返されました");
        }
    }

    // -------------------------------------------------------
    // T-E3: 計装サマリ (observation test)
    // -------------------------------------------------------
    #[test]
    fn test_spaceposition_instrumentation_summary() {
        println!("\n=== M1.75-1 計装サマリ ===");
        println!("定数:");
        println!(
            "  SPACE_POSITION_UPDATE_ALPHA = {}",
            SPACE_POSITION_UPDATE_ALPHA
        );
        println!(
            "  SPACE_POSITION_UPDATE_MIN_INTERVAL = {}",
            SPACE_POSITION_UPDATE_MIN_INTERVAL
        );
        println!(
            "  SPACE_POSITION_L2_EPSILON = {}",
            SPACE_POSITION_L2_EPSILON
        );
        println!("型定義:");
        println!("  SpacePositionEmbedding: Option<[f32; 3]> の newtype");
        println!("  VillagePosition: position=[f32;3], last_updated_vt=u64");
        println!("  VillageObservation: 3成分 + delta");
        println!("  PositionUpdatePolicy: Always / MinInterval / Manual");
        println!("純粋関数:");
        println!("  update_space_position(prev, obs, alpha) -> [f32;3]");
        println!("  should_update_position(last_vt, now_vt, policy) -> bool");
        println!("  l2_distance(a, b) -> f64");
        println!("  is_position_equal(a, b) -> bool");
        println!("EventBus 結合:");
        println!("  SystemEvent::SpacePositionUpdated(...) -> publish");
        println!();
    }

    // -------------------------------------------------------
    // TC4: decompose_position
    // -------------------------------------------------------
    #[test]
    fn test_decompose_position_standard() {
        let pos = [1.0, 2.0, 3.0];
        let (s, t, c) = decompose_position(pos);
        assert!(
            (s - 1.0).abs() < 1e-6,
            "空間成分が正しく分解される必要があります"
        );
        assert!(
            (t - 2.0).abs() < 1e-6,
            "時間成分が正しく分解される必要があります"
        );
        assert!(
            (c - 3.0).abs() < 1e-6,
            "認知成分が正しく分解される必要があります"
        );
    }

    #[test]
    fn test_decompose_position_zero() {
        let pos = [0.0, 0.0, 0.0];
        let (s, t, c) = decompose_position(pos);
        assert!((s - 0.0).abs() < 1e-6, "ゼロベクトルの空間成分");
        assert!((t - 0.0).abs() < 1e-6, "ゼロベクトルの時間成分");
        assert!((c - 0.0).abs() < 1e-6, "ゼロベクトルの認知成分");
    }

    #[test]
    fn test_decompose_position_negative() {
        let pos = [-1.0, -2.0, -3.0];
        let (s, t, c) = decompose_position(pos);
        assert!((s - (-1.0)).abs() < 1e-6, "負値の空間成分");
        assert!((t - (-2.0)).abs() < 1e-6, "負値の時間成分");
        assert!((c - (-3.0)).abs() < 1e-6, "負値の認知成分");
    }
}
