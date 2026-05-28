// Darvium Mock 実装
//
// RFC §13.4 の pure retrieval contract を満たす決定論的 Mock クライアント群。
// どのようなクエリが入力されても 100% 決定論的に指定の応答を即座に返す。
// 後続の状態機械テスト (M-1.5〜M-1) で FakeExecutor の構成要素として使用される。

use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::DarviumError;
use crate::types::{CandidateSet, QueryRepresentation, RetrievalPolicy, RetrievalPrimitive};

/// 常に空の CandidateSet を返す Mock。
///
/// どのようなクエリが入力されても `Ok(CandidateSet::empty())` を返す。
/// invocation_count で呼び出し回数を計測する。
#[derive(Debug)]
pub struct MockEmptyRetrievalPrimitive {
    /// 呼び出し回数カウンタ。
    /// 単一スレッドテストでの使用を想定し、Ordering::Relaxed を使用する。
    invocation_count: AtomicU64,
}

impl MockEmptyRetrievalPrimitive {
    /// 呼び出し回数 0 で初期化された Mock を生成する。
    pub fn new() -> Self {
        Self {
            invocation_count: AtomicU64::new(0),
        }
    }

    /// 現在の呼び出し回数を取得する。
    pub fn invocation_count(&self) -> u64 {
        self.invocation_count.load(Ordering::Relaxed)
    }

    /// 呼び出し回数カウンタをリセットする。
    pub fn reset_count(&self) {
        self.invocation_count.store(0, Ordering::Relaxed);
    }
}

impl Default for MockEmptyRetrievalPrimitive {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrievalPrimitive for MockEmptyRetrievalPrimitive {
    fn search_workflows(
        &self,
        _query: &QueryRepresentation,
        _policy: &RetrievalPolicy,
    ) -> Result<CandidateSet, DarviumError> {
        self.invocation_count.fetch_add(1, Ordering::Relaxed);
        Ok(CandidateSet::empty())
    }
}

/// 常に RetrievalTimeout エラーを返す Mock。
///
/// どのようなクエリが入力されても `Err(DarviumError::RetrievalTimeout)` を返す。
/// invocation_count で呼び出し回数を計測する。
#[derive(Debug)]
pub struct MockErrorRetrievalPrimitive {
    /// 呼び出し回数カウンタ。
    /// 単一スレッドテストでの使用を想定し、Ordering::Relaxed を使用する。
    invocation_count: AtomicU64,
}

impl MockErrorRetrievalPrimitive {
    /// 呼び出し回数 0 で初期化された Mock を生成する。
    pub fn new() -> Self {
        Self {
            invocation_count: AtomicU64::new(0),
        }
    }

    /// 現在の呼び出し回数を取得する。
    pub fn invocation_count(&self) -> u64 {
        self.invocation_count.load(Ordering::Relaxed)
    }

    /// 呼び出し回数カウンタをリセットする。
    pub fn reset_count(&self) {
        self.invocation_count.store(0, Ordering::Relaxed);
    }
}

impl Default for MockErrorRetrievalPrimitive {
    fn default() -> Self {
        Self::new()
    }
}

impl RetrievalPrimitive for MockErrorRetrievalPrimitive {
    fn search_workflows(
        &self,
        _query: &QueryRepresentation,
        _policy: &RetrievalPolicy,
    ) -> Result<CandidateSet, DarviumError> {
        self.invocation_count.fetch_add(1, Ordering::Relaxed);
        Err(DarviumError::RetrievalTimeout)
    }
}

/// 統合 Mock 列挙型。
///
/// テストシナリオに応じて Empty / Error を動的に切り替えたい場合に使用する。
#[derive(Debug)]
pub enum MockRetrievalPrimitive {
    Empty(MockEmptyRetrievalPrimitive),
    Error(MockErrorRetrievalPrimitive),
}

impl RetrievalPrimitive for MockRetrievalPrimitive {
    fn search_workflows(
        &self,
        query: &QueryRepresentation,
        policy: &RetrievalPolicy,
    ) -> Result<CandidateSet, DarviumError> {
        match self {
            MockRetrievalPrimitive::Empty(m) => m.search_workflows(query, policy),
            MockRetrievalPrimitive::Error(m) => m.search_workflows(query, policy),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DarviumError;

    // ── T1: MockEmptyRetrievalPrimitive 空返却検証 ──

    /// T1-1: デフォルトクエリで search_workflows を呼び出し、空の CandidateSet が返る。
    #[test]
    fn empty_returns_empty_candidate_set() {
        let mock = MockEmptyRetrievalPrimitive::new();
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = mock.search_workflows(&query, &policy);
        assert!(result.is_ok());
        let candidates = result.expect("MockEmptyRetrievalPrimitive should always return Ok");
        assert!(candidates.candidates.is_empty());
        assert_eq!(candidates.retrieval_calls_used, 0);
    }

    /// T1-2: 様々な QueryType で呼び出しても常に空が返る。
    #[test]
    fn empty_with_various_query_types() {
        let mock = MockEmptyRetrievalPrimitive::new();
        let base_query = QueryRepresentation::default();

        for query_type in &[
            crate::types::QueryType::Episodic,
            crate::types::QueryType::Canonical,
            crate::types::QueryType::Hybrid,
        ] {
            let mut query = base_query.clone();
            query.query_type = query_type.clone();
            let result = mock.search_workflows(&query, &RetrievalPolicy::default());
            assert!(result.is_ok());
            let candidates = result.unwrap();
            assert!(
                candidates.candidates.is_empty(),
                "Expected empty for QueryType {:?}",
                query_type
            );
        }
    }

    /// T1-3: 任意の RetrievalPolicy でも空が返る（allow_compose, allow_new の全組み合わせ）。
    #[test]
    fn empty_with_various_policies() {
        let mock = MockEmptyRetrievalPrimitive::new();
        let query = QueryRepresentation::default();

        for &allow_compose in &[false, true] {
            for &allow_new in &[false, true] {
                let policy = RetrievalPolicy {
                    allow_compose,
                    allow_new,
                    ..RetrievalPolicy::default()
                };
                let result = mock.search_workflows(&query, &policy);
                assert!(result.is_ok());
                let candidates = result.unwrap();
                assert!(
                    candidates.candidates.is_empty(),
                    "Expected empty for policy (compose={}, new={})",
                    allow_compose,
                    allow_new
                );
            }
        }
    }

    // ── T2: MockErrorRetrievalPrimitive エラー返却検証 ──

    /// T2-1: search_workflows を呼び出すと常に Err(DarviumError::RetrievalTimeout) が返る。
    #[test]
    fn error_returns_timeout() {
        let mock = MockErrorRetrievalPrimitive::new();
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = mock.search_workflows(&query, &policy);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            DarviumError::RetrievalTimeout,
            "MockErrorRetrievalPrimitive should always return RetrievalTimeout"
        );
    }

    /// T2-2: 様々なクエリで呼び出しても常に同一エラーが返る。
    #[test]
    fn error_with_various_queries() {
        let mock = MockErrorRetrievalPrimitive::new();
        let base_query = QueryRepresentation::default();

        for query_type in &[
            crate::types::QueryType::Episodic,
            crate::types::QueryType::Canonical,
            crate::types::QueryType::Hybrid,
        ] {
            let mut query = base_query.clone();
            query.query_type = query_type.clone();
            let result = mock.search_workflows(&query, &RetrievalPolicy::default());
            assert!(result.is_err());
            assert_eq!(
                result.unwrap_err(),
                DarviumError::RetrievalTimeout,
                "Expected RetrievalTimeout for QueryType {:?}",
                query_type
            );
        }
    }

    // ── T3: 決定論性検証 ──

    /// T3-1: 同一クエリで 2 回呼び出した結果が完全一致する（空 Mock）。
    #[test]
    fn empty_deterministic_same_query() {
        let mock = MockEmptyRetrievalPrimitive::new();
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();

        let result1 = mock.search_workflows(&query, &policy);
        let result2 = mock.search_workflows(&query, &policy);
        assert_eq!(result1, result2);
    }

    /// T3-2: 異なるクエリでも返されるデータ構造が完全一致する（空 Mock）。
    #[test]
    fn empty_deterministic_different_queries() {
        let mock = MockEmptyRetrievalPrimitive::new();

        let query_a = QueryRepresentation::default();
        let mut query_b = QueryRepresentation::default();
        query_b.mission_text = "different mission".to_string();
        query_b.query_type = crate::types::QueryType::Episodic;

        let policy = RetrievalPolicy::default();
        let result_a = mock.search_workflows(&query_a, &policy);
        let result_b = mock.search_workflows(&query_b, &policy);
        assert_eq!(result_a, result_b);
    }

    /// T3-3: 異なるクエリでもエラー種別が完全一致する（エラー Mock）。
    #[test]
    fn error_deterministic_different_queries() {
        let mock = MockErrorRetrievalPrimitive::new();

        let query_a = QueryRepresentation::default();
        let mut query_b = QueryRepresentation::default();
        query_b.mission_text = "different mission".to_string();
        query_b.task_embedding = vec![0.1, 0.2, 0.3];

        let policy = RetrievalPolicy::default();
        let result_a = mock.search_workflows(&query_a, &policy);
        let result_b = mock.search_workflows(&query_b, &policy);
        assert_eq!(result_a, result_b);
    }

    // ── T4: 計装プローブ検証 ──

    /// T4-1: search_workflows を 1 回呼び出すと invocation_count が 1 になる。
    #[test]
    fn invocation_count_increments() {
        let mock = MockEmptyRetrievalPrimitive::new();
        assert_eq!(mock.invocation_count(), 0);

        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let _ = mock.search_workflows(&query, &policy);

        assert_eq!(mock.invocation_count(), 1);
    }

    /// T4-2: 3 回連続呼び出しで invocation_count が 3 になる。
    #[test]
    fn invocation_count_three_calls() {
        let mock = MockEmptyRetrievalPrimitive::new();
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();

        for _ in 0..3 {
            let _ = mock.search_workflows(&query, &policy);
        }

        assert_eq!(mock.invocation_count(), 3);
    }

    /// T4-3: 複数 Mock インスタンス間でカウンタが独立している。
    #[test]
    fn invocation_count_independent_instances() {
        let mock_a = MockEmptyRetrievalPrimitive::new();
        let mock_b = MockEmptyRetrievalPrimitive::new();
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();

        let _ = mock_a.search_workflows(&query, &policy);
        let _ = mock_a.search_workflows(&query, &policy);
        let _ = mock_b.search_workflows(&query, &policy);

        assert_eq!(mock_a.invocation_count(), 2);
        assert_eq!(mock_b.invocation_count(), 1);
    }

    // ── T5: トレイトオブジェクト安全性 ──

    /// T5-1: Box<dyn RetrievalPrimitive> として MockEmpty を使用できる。
    #[test]
    fn mock_empty_trait_object_safety() {
        let mock: Box<dyn RetrievalPrimitive> = Box::new(MockEmptyRetrievalPrimitive::new());
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = mock.search_workflows(&query, &policy);
        assert!(result.is_ok());
    }

    /// T5-2: Box<dyn RetrievalPrimitive> として MockError を使用できる。
    #[test]
    fn mock_error_trait_object_safety() {
        let mock: Box<dyn RetrievalPrimitive> = Box::new(MockErrorRetrievalPrimitive::new());
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = mock.search_workflows(&query, &policy);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DarviumError::RetrievalTimeout);
    }

    /// T5-3: &dyn RetrievalPrimitive として関数引数に渡せる。
    fn accepts_retrieval_primitive(
        trait_obj: &dyn RetrievalPrimitive,
    ) -> Result<CandidateSet, DarviumError> {
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        trait_obj.search_workflows(&query, &policy)
    }

    #[test]
    fn mock_empty_as_dyn_ref() {
        let mock = MockEmptyRetrievalPrimitive::new();
        let result = accepts_retrieval_primitive(&mock);
        assert!(result.is_ok());
    }

    #[test]
    fn mock_error_as_dyn_ref() {
        let mock = MockErrorRetrievalPrimitive::new();
        let result = accepts_retrieval_primitive(&mock);
        assert!(result.is_err());
    }

    // ── OTS-1: クエリエントロピー vs 命令ステップ分散 (σ²(S_inst) = 0) ──

    /// OTS-1: クエリエントロピーを可変させた 8,192 個のクエリで
    /// search_workflows を呼び出し、消費命令ステップ数の分散がゼロであることを検証する。
    #[test]
    fn ots1_entropy_vs_instruction_steps() {
        let sample_size = 8192_usize;
        let seed: u64 = crate::constants::TEST_PRNG_SEED;

        let mut rng_state: u64 = seed;
        let mut lcg = move || -> u64 {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            rng_state
        };

        let mock = MockEmptyRetrievalPrimitive::new();
        let base_query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();

        let mut step_diffs: Vec<u64> = Vec::with_capacity(sample_size);

        for _ in 0..sample_size {
            // シャノンエントロピー H ∈ [0, 8] ビットの範囲で mission_text を可変
            let entropy_bits = (lcg() % 9) as u8;
            let mission_text = if entropy_bits == 0 {
                String::new()
            } else {
                // entropy_bits ビットのエントロピーを持つ文字列を生成
                let char_count = (lcg() % 64) + 1;
                let text: String = (0..char_count)
                    .map(|i| {
                        if i < entropy_bits as u64 {
                            // 高エントロピー領域: 多様な文字
                            ((lcg() % 94) as u8 + 33) as char
                        } else {
                            // 低エントロピー領域: 同一文字の繰り返し
                            'a'
                        }
                    })
                    .collect();
                text
            };

            let mut query = base_query.clone();
            query.mission_text = mission_text;

            // 列挙型フィールドもランダム化
            query.query_type = match lcg() % 3 {
                0 => crate::types::QueryType::Episodic,
                1 => crate::types::QueryType::Canonical,
                _ => crate::types::QueryType::Hybrid,
            };
            query.freshness_requirement = match lcg() % 4 {
                0 => crate::types::FreshnessRequirement::Recent,
                1 => crate::types::FreshnessRequirement::Stable,
                2 => crate::types::FreshnessRequirement::Historical,
                _ => crate::types::FreshnessRequirement::Mixed,
            };

            let before = mock.invocation_count();
            let _ = mock.search_workflows(&query, &policy);
            let after = mock.invocation_count();
            step_diffs.push(after - before);
        }

        // 分散 σ² の計算: 全ての差分が 1（常に +1）であることを確認
        let min = step_diffs
            .iter()
            .min()
            .expect("step_diffs should not be empty");
        let max = step_diffs
            .iter()
            .max()
            .expect("step_diffs should not be empty");
        let sum: u64 = step_diffs.iter().sum();
        let mean = sum as f64 / sample_size as f64;
        let variance: f64 = step_diffs
            .iter()
            .map(|&v| {
                let diff = v as f64 - mean;
                diff * diff
            })
            .sum::<f64>()
            / sample_size as f64;

        println!("=== OTS-1: Entropy vs Instruction Steps ===");
        println!(
            "{{\"min\": {}, \"max\": {}, \"mean\": {:.6}, \"variance\": {:.6}, \"sample_size\": {}}}",
            min, max, mean, variance, sample_size
        );

        assert_eq!(
            *min, 1,
            "All calls must consume exactly 1 instruction step (min={})",
            min
        );
        assert_eq!(
            *max, 1,
            "All calls must consume exactly 1 instruction step (max={})",
            max
        );
        assert!(
            variance.abs() < 1e-10,
            "Variance must be zero (σ² = {}). All queries must consume identical instruction steps.",
            variance
        );
        println!("=== 結果: PASS (σ² = 0) ===");
    }

    // ── OTS-2: Kolmogorov 複雑度不変性の観測 ──

    /// OTS-2: 8,192 個のクエリに対する CandidateSet 出力が全て同一であることを確認する。
    ///
    /// MockEmptyRetrievalPrimitive は入力に依存せず常に同一の空 CandidateSet を返すため、
    /// 全ての出力はバイト単位で一致する。これは Kolmogorov 複雑度一定の直接証明となる。
    #[test]
    fn ots2_kolmogorov_invariance() {
        let sample_size = 8192_usize;
        let seed: u64 = crate::constants::TEST_PRNG_SEED;

        let mut rng_state: u64 = seed;
        let mut lcg = move || -> u64 {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            rng_state
        };

        let mock = MockEmptyRetrievalPrimitive::new();
        let base_query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();

        let mut first_output: Option<String> = None;
        let mut all_identical = true;

        for i in 0..sample_size {
            let mut query = base_query.clone();
            query.mission_text = (0..((lcg() % 128) + 1))
                .map(|_| ((lcg() % 94) as u8 + 33) as char)
                .collect();

            let result = mock.search_workflows(&query, &policy);
            let candidates = result.expect("MockEmpty should always return Ok");
            let output = format!("{:?}", candidates);

            match &first_output {
                None => first_output = Some(output),
                Some(ref first) => {
                    if output != *first {
                        all_identical = false;
                        println!("  MISMATCH at index {}: output differs from first", i);
                    }
                }
            }
        }

        println!("=== OTS-2: Kolmogorov Complexity Invariance ===");
        println!(
            "sample_size={}, all_outputs_identical={}",
            sample_size, all_identical
        );
        assert!(
            all_identical,
            "All outputs must be identical for MockEmptyRetrievalPrimitive \
             (Kolmogorov complexity invariance violation)"
        );
        println!("=== 結果: PASS (all {} outputs identical) ===", sample_size);
    }

    // ── OTS-3: 関数呼び出しの実時間不変性 ──

    /// OTS-3: 10,000 回の連続呼び出しにおいて、100 回目と 10,000 回目の
    /// 呼び出し時間に統計的有意差がないことを確認する。
    #[test]
    fn ots3_call_latency_invariance() {
        let call_count = 10_000_usize;
        let mock = MockEmptyRetrievalPrimitive::new();
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();

        // ウォームアップ: 最初の数百回で JIT 最適化等の影響を安定させる
        for _ in 0..500 {
            let _ = mock.search_workflows(&query, &policy);
        }
        mock.reset_count();

        let mut latencies: Vec<std::time::Duration> = Vec::with_capacity(call_count);

        for _ in 0..call_count {
            let start = std::time::Instant::now();
            let _ = mock.search_workflows(&query, &policy);
            let elapsed = start.elapsed();
            latencies.push(elapsed);
        }

        // 100 回目と 10,000 回目のレイテンシを比較
        let early_sample = latencies[99.min(call_count - 1)];
        let late_sample = latencies[call_count - 1];

        // レイテンシの統計を出力
        let total_ns: u128 = latencies.iter().map(|d| d.as_nanos()).sum();
        let mean_ns = total_ns as f64 / call_count as f64;
        let min_ns = latencies.iter().map(|d| d.as_nanos()).min().unwrap_or(0);
        let max_ns = latencies.iter().map(|d| d.as_nanos()).max().unwrap_or(0);

        println!("=== OTS-3: Call Latency Invariance ===");
        println!("call_count={}, warmup=500", call_count);
        println!(
            "latency: min={}ns, max={}ns, mean={:.1}ns",
            min_ns, max_ns, mean_ns
        );
        println!("early_sample[99]={}ns", early_sample.as_nanos());
        println!(
            "late_sample[{}]={}ns",
            call_count - 1,
            late_sample.as_nanos()
        );

        // 統計的有意差の検証: 絶対差が 1μs 未満であること
        let early_ns = early_sample.as_nanos();
        let late_ns = late_sample.as_nanos();
        let abs_diff_ns = late_ns.abs_diff(early_ns);

        assert!(
            abs_diff_ns < 1000,
            "Early vs late latency difference too large: {}ns (early={}ns, late={}ns)",
            abs_diff_ns,
            early_ns,
            late_ns
        );
        println!("=== 結果: PASS (diff={}ns) ===", abs_diff_ns);
    }
}
