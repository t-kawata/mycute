// Darvium 時間抽象化レイヤ
//
// Clock トレイトと3つの具象実装を提供する。
// SystemTime への依存を Clock トレイトで抽象化し、
// テスト時は ManualClock に差し替えることで
// 決定論的実行 (deterministic replay) を保証する。
//
// 関連RFC: §v1.7（Human Time / Virtual Time 二軸モデル）
// 関連チケット: M-2-1.8（Clock / ManualClock 抽象トレイトの定義）

use std::time::{SystemTime, UNIX_EPOCH};

/// 時間抽象化トレイト。
///
/// SystemTime に依存するコードを抽象化し、テスト時に ManualClock で
/// 差し替えることで決定論的実行を可能にする。
/// 全ての時刻は UTC (UNIX epoch) 起点のミリ秒で表現される。
pub trait Clock: Send + Sync {
    /// 現在時刻を UTC ミリ秒で返す。
    fn now_ms(&self) -> u64;

    /// 時間を delta_ms だけ進める。
    ///
    /// ManualClock でのみ意味を持つ。SystemClock / FrozenClock では
    /// 何も行わない (no-op)。
    fn advance(&mut self, delta_ms: u64);
}

// ── ManualClock ──

/// 手動操作で進行する決定論的クロック（旧称: VirtualClock）。
///
/// 内部カウンタを持ち、`advance()` でのみ時間が進行する。
/// SystemTime とは完全に独立しており、テストでの
/// deterministic replay を保証する。
///
/// # 注意
///
/// このクロックの `advance()` は時間計測用の操作である。
/// EventBus commit clock の advance は EventBus 実装内部に制限される
/// (RFC §12C.6 MUST #4)。時間計測用途以外での advance 呼び出しは禁止。
pub struct ManualClock {
    counter: u64,
}

impl ManualClock {
    /// デフォルト開始時刻 (0) から開始する ManualClock を生成する。
    pub fn new() -> Self {
        Self {
            counter: crate::constants::CLOCK_DEFAULT_START_MS,
        }
    }

    /// 任意の開始時刻 (UTC ミリ秒) から開始する ManualClock を生成する。
    pub fn with_start(ms: u64) -> Self {
        Self { counter: ms }
    }

    /// 現在の内部カウンタ値を取得する。
    pub fn current(&self) -> u64 {
        self.counter
    }
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.counter
    }

    fn advance(&mut self, delta_ms: u64) {
        // 飽和加算により u64 オーバーフローを防止する
        self.counter = self.counter.saturating_add(delta_ms);
    }
}

// ── SystemClock ──

/// 実時間クロック。
///
/// `SystemTime::now()` をラップし、UTC ミリ秒を返す。
/// `advance()` は no-op (実時間は外部操作で進められない)。
pub struct SystemClock;

impl SystemClock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn advance(&mut self, _delta_ms: u64) {
        // no-op: SystemClock は実時間に依存するため advance は無効
    }
}

// ── FrozenClock ──

/// 固定値クロック（テスト用）。
///
/// コンストラクタで指定された時刻 (UTC ミリ秒) を常に返す。
/// `advance()` は no-op。
pub struct FrozenClock {
    frozen_ms: u64,
}

impl FrozenClock {
    pub fn new(ms: u64) -> Self {
        Self { frozen_ms: ms }
    }
}

impl Clock for FrozenClock {
    fn now_ms(&self) -> u64 {
        self.frozen_ms
    }

    fn advance(&mut self, _delta_ms: u64) {
        // no-op: FrozenClock は固定値のため advance は無効
    }
}

/// BlendedFreshness F_time を計算する (RFC §4A.9 機構 50)。
///
/// 二軸時間減衰モデル:
/// - F_H: 人間時間減衰 (UTC 経過時間)。指数減衰。
/// - F_V: 仮想時刻減衰 (virtual tick)。指数減衰。
/// - F_time = w_H * F_H + (1 - w_H) * F_V
///
/// # 引数
/// - `human_time_ms`: 最終アクセスからの人間時間経過 (ミリ秒)
/// - `virtual_ticks`: 最終アクセスからの仮想 tick 経過数
/// - `human_weight`: 人間時間の重み [0, 1]。1.0 で人間時間のみ評価
///
/// # 戻り値
/// - [0, 1] 範囲の Freshness 値。アクセス直後で 1.0、経過で 0.0 に減衰。
pub fn compute_blended_freshness(human_time_ms: u64, virtual_ticks: u64, human_weight: f64) -> f64 {
    let f_h = (-(human_time_ms as f64) / crate::constants::HUMAN_FRESHNESS_HALFLIFE_MS).exp();
    let f_v = (-(virtual_ticks as f64) / crate::constants::VIRTUAL_FRESHNESS_HALFLIFE).exp();
    let w = human_weight.clamp(0.0, 1.0);
    w * f_h + (1.0 - w) * f_v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ── 全実装共通 (T1-T3) ──

    /// T1: 全実装が Clock トレイト境界を充足することをコンパイル時検証
    #[test]
    fn test_trait_bound_satisfied() {
        fn assert_trait(_: &impl Clock) {}
        assert_trait(&ManualClock::new());
        assert_trait(&SystemClock::new());
        assert_trait(&FrozenClock::new(0));
    }

    /// T2: Box<dyn Clock> のオブジェクト安全性
    #[test]
    fn test_object_safety() {
        let virtual_clock: Box<dyn Clock> = Box::new(ManualClock::new());
        let system_clock: Box<dyn Clock> = Box::new(SystemClock::new());
        let frozen_clock: Box<dyn Clock> = Box::new(FrozenClock::new(42));

        assert_eq!(virtual_clock.now_ms(), 0);
        assert_eq!(frozen_clock.now_ms(), 42);
        let _ = system_clock.now_ms();
    }

    /// T3: Box<dyn Clock + Send + Sync> がスレッド間移動可能
    #[test]
    fn test_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>(_t: &T) {}
        assert_send_sync(&ManualClock::new());
        assert_send_sync(&SystemClock::new());
        assert_send_sync(&FrozenClock::new(0));

        let boxed: Box<dyn Clock> = Box::new(ManualClock::new());
        assert_send_sync(&boxed);
    }

    // ── ManualClock (T4-T9) ──

    /// T4: 初期値が 0 であること
    #[test]
    fn test_manual_clock_initial_value() {
        let clock = ManualClock::new();
        assert_eq!(clock.now_ms(), 0);
    }

    /// T4b: with_start で指定した初期値になること
    #[test]
    fn test_manual_clock_with_start() {
        let clock = ManualClock::with_start(1000);
        assert_eq!(clock.now_ms(), 1000);
    }

    /// T5: advance(100) で now_ms() が正確に 100ms 進行すること
    #[test]
    fn test_manual_clock_advance_exact() {
        let mut clock = ManualClock::new();
        clock.advance(100);
        assert_eq!(clock.now_ms(), 100);
    }

    /// T6: 複数回の advance の累積性 (100+200=300)
    #[test]
    fn test_manual_clock_advance_cumulative() {
        let mut clock = ManualClock::new();
        clock.advance(100);
        clock.advance(200);
        assert_eq!(clock.now_ms(), 300);
    }

    /// T7: advance(0) で値が変化しないこと
    #[test]
    fn test_manual_clock_advance_zero() {
        let mut clock = ManualClock::new();
        clock.advance(0);
        assert_eq!(clock.now_ms(), 0);
    }

    /// T8: 最大値付近からの advance でオーバーフローしないこと（飽和加算）
    #[test]
    fn test_manual_clock_advance_saturation() {
        let mut clock = ManualClock::with_start(u64::MAX - 50);
        clock.advance(100);
        assert_eq!(clock.now_ms(), u64::MAX);
    }

    /// T9: 単調増加性のアサーション（巻き戻し禁止の不変条件）
    #[test]
    fn test_manual_clock_monotonic() {
        let mut clock = ManualClock::new();
        let mut prev = clock.now_ms();
        for _ in 0..100 {
            clock.advance(1);
            let current = clock.now_ms();
            assert!(current >= prev, "単調増加違反: {} -> {}", prev, current);
            prev = current;
        }
    }

    // ── SystemClock (T10-T12) ──

    /// T10: now_ms() が実時間と大きく乖離しないこと（誤差 < 1秒）
    #[test]
    fn test_system_clock_now_ms() {
        let clock = SystemClock::new();
        let before = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let clock_ms = clock.now_ms();
        let after = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        assert!(
            clock_ms >= before,
            "clock_ms {} < before {}",
            clock_ms,
            before
        );
        assert!(clock_ms <= after, "clock_ms {} > after {}", clock_ms, after);

        let elapsed = after.saturating_sub(before);
        assert!(elapsed < 1000, "計測誤差 {}ms が 1秒を超過", elapsed);

        println!("=== SystemClock 実時間検証 ===");
        println!("before: {}ms", before);
        println!("clock_ms: {}ms", clock_ms);
        println!("after: {}ms", after);
        println!("誤差: {}ms", elapsed);
    }

    /// T11: advance() が no-op (パニックせず値が変化しないこと)
    #[test]
    fn test_system_clock_advance_noop() {
        let mut clock = SystemClock::new();
        let before = clock.now_ms();
        clock.advance(10000);
        let after = clock.now_ms();
        let elapsed = after.saturating_sub(before);
        // advance 後も値が極端に大きくならないこと
        assert!(elapsed < 100_000, "SystemClock advance 後に 100秒以上経過");
    }

    /// T12: 連続呼び出しで値が単調増加すること
    #[test]
    fn test_system_clock_monotonic() {
        let clock = SystemClock::new();
        let t1 = clock.now_ms();
        std::thread::sleep(Duration::from_millis(1));
        let t2 = clock.now_ms();
        assert!(
            t2 >= t1,
            "SystemClock が単調増加していない: {} -> {}",
            t1,
            t2
        );
    }

    // ── FrozenClock (T13-T15) ──

    /// T13: コンストラクタで指定した値を常に返すこと
    #[test]
    fn test_frozen_clock_initial_value() {
        let clock = FrozenClock::new(12345);
        assert_eq!(clock.now_ms(), 12345);
    }

    /// T14: 複数回呼び出しで同一値が返ること
    #[test]
    fn test_frozen_clock_constant() {
        let clock = FrozenClock::new(999);
        for _ in 0..10 {
            assert_eq!(clock.now_ms(), 999);
        }
    }

    /// T15: advance() が no-op (値が変化しないこと)
    #[test]
    fn test_frozen_clock_advance_noop() {
        let mut clock = FrozenClock::new(500);
        clock.advance(1000);
        assert_eq!(clock.now_ms(), 500);
    }

    // ── 計装・観測 (T16) ──

    /// T16: ManualClock の経過時間分布観測
    ///
    /// advance 1..=100 を累積適用し、期待通りの総経過時間 (5050ms)
    /// が観測されることを検証する。
    #[test]
    fn test_manual_clock_observation() {
        let mut clock = ManualClock::new();
        let n_advances: u64 = 100;
        let mut expected_total: u64 = 0;

        for i in 1..=n_advances {
            clock.advance(i);
            expected_total += i;
        }

        let observed = clock.now_ms();

        println!("=== ManualClock 経過時間観測 ===");
        println!("advance 回数: {}", n_advances);
        println!("期待累積時間: {}ms", expected_total);
        println!("観測累積時間: {}ms", observed);
        println!("一致: {}", observed == expected_total);
        println!("=== 結果: PASS ===");

        assert_eq!(observed, expected_total);
    }

    // ================================================================
    // P5: compute_blended_freshness (TC9)
    // ================================================================

    /// TC9: BlendedFreshness の基本挙動。
    ///
    /// - 経過 0 → 1.0 (アクセス直後)
    /// - 大経過 → 0.0 に漸近
    /// - human_weight によるブレンド効果
    #[test]
    fn tc9_compute_blended_freshness() {
        // 経過 0 → 1.0
        let f = compute_blended_freshness(0, 0, 0.5);
        assert!(
            (f - 1.0).abs() < f64::EPSILON,
            "freshness at 0 must be 1.0 (got {})",
            f
        );

        // 大経過 → 0.0 に漸近
        let f = compute_blended_freshness(u64::MAX, u64::MAX, 0.5);
        assert!(f < 1e-10, "freshness at MAX must be near 0.0 (got {})", f);

        // human_weight=1.0 → 仮想時刻無視
        let f_h_only = compute_blended_freshness(0, 1000, 1.0);
        assert!(
            (f_h_only - 1.0).abs() < f64::EPSILON,
            "w=1.0 with virtual ticks=1000 must still be 1.0 (got {})",
            f_h_only
        );

        // human_weight=0.0 → 人間時間無視
        let f_v_only = compute_blended_freshness(86_400_000, 0, 0.0);
        assert!(
            (f_v_only - 1.0).abs() < f64::EPSILON,
            "w=0.0 with human_time=1day must still be 1.0 (got {})",
            f_v_only
        );

        // w=0.5, human_time=24h, virtual=100tick → 両方減衰
        let f_both = compute_blended_freshness(86_400_000, 100, 0.5);
        let expected_f_h = (-1.0_f64).exp(); // human_time / halflife = 1.0
        let expected_f_v = (-1.0_f64).exp(); // virtual_ticks / halflife = 1.0
        let expected = 0.5 * expected_f_h + 0.5 * expected_f_v;
        assert!(
            (f_both - expected).abs() < 1e-10,
            "blended freshness mismatch: expected {}, got {}",
            expected,
            f_both
        );
    }
}
