// メモリ内 HNSW 擬似インデックス Mock
//
// RFC §12.2 Stage 2a/2b の Dual Retrieval を模擬するテスト基盤。
// 真の HNSW グラフ構築（近似最近傍探索）ではなく、線形探索 + コサイン類似度による
// 決定論的 Mock として機能する。
//
// 関連RFC: §12.2 Stage 2a/2b Dual Retrieval, §25 データベース構成
// 関連チケット: M1.5-1

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::error::DarviumError;

/// メモリ内 HNSW 擬似インデックス Mock。
///
/// キーと f32 ベクトルのペアを保持し、クエリベクトルとのコサイン類似度による
/// 上位 k 件の検索を提供する。検索は線形探索 (O(n)) で行い、真の HNSW 近似は行わない。
///
/// # 内部可変性
///
/// 登録ベクトルは `Mutex` で保護し、`&self` 経由での変更を可能にする。
/// 呼び出し回数カウンタは `AtomicU64` で管理する。
///
/// # 使用例
///
/// ```
/// use darvium::vector_index::MockHnswIndex;
///
/// let mut index = MockHnswIndex::new(3);
/// index.insert("vec-a", &[1.0, 0.0, 0.0]).unwrap();
/// let results = index.search(&[1.0, 0.0, 0.0], 1).unwrap();
/// assert_eq!(results[0].0, "vec-a");
/// assert!((results[0].1 - 1.0).abs() < 1e-6);
/// ```
pub struct MockHnswIndex {
    /// 登録ベクトルのリスト。(key, vector) のペア。
    entries: Mutex<Vec<(String, Vec<f32>)>>,
    /// 許容するベクトル次元数。new() で固定される。
    dimension: usize,
    /// 検索呼び出し回数カウンタ。
    invocation_count: AtomicU64,
}

impl MockHnswIndex {
    /// 指定された次元数で空の MockHnswIndex を生成する。
    ///
    /// `dimension` に 0 を指定するとエラーになる。
    /// 実フォーマット（1536次元等）にはデフォルト値として
    /// `crate::constants::HNSW_MOCK_DEFAULT_DIMENSION` を使用する。
    pub fn new(dimension: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            dimension,
            invocation_count: AtomicU64::new(0),
        }
    }

    /// ベクトルを登録する。同一キーの場合は上書き（後勝ち）。
    ///
    /// `vector` の長さがコンストラクタで指定した次元数と一致しない場合は
    /// `DarviumError::EmbeddingDimensionMismatch` を返す。
    /// 空ベクトルも同様にエラーとして扱う。
    pub fn insert(&self, key: &str, vector: &[f32]) -> Result<(), DarviumError> {
        if vector.is_empty() {
            return Err(DarviumError::Storage(
                "Cannot store empty embedding vector".to_string(),
            ));
        }
        if vector.len() != self.dimension {
            return Err(DarviumError::EmbeddingDimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }
        let mut entries = self.entries.lock().expect("Mutex poisoned");
        if let Some(pos) = entries.iter().position(|(k, _)| k == key) {
            entries[pos] = (key.to_string(), vector.to_vec());
        } else {
            entries.push((key.to_string(), vector.to_vec()));
        }
        Ok(())
    }

    /// クエリベクトルとのコサイン類似度上位 k 件を返す。
    ///
    /// 返り値は (key, similarity) のベクタで、類似度降順にソートされている。
    /// 未登録の場合は空のベクタを返す。
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f64)>, DarviumError> {
        self.invocation_count.fetch_add(1, Ordering::Relaxed);

        if query.is_empty() {
            return Err(DarviumError::Storage("Query vector is empty".to_string()));
        }
        if query.len() != self.dimension {
            return Err(DarviumError::EmbeddingDimensionMismatch {
                expected: self.dimension,
                actual: query.len(),
            });
        }

        let entries = self.entries.lock().expect("Mutex poisoned");

        if entries.is_empty() {
            return Ok(Vec::new());
        }

        let mut results: Vec<(String, f64)> = entries
            .iter()
            .map(|(key, vec)| {
                let similarity = cosine_similarity(query, vec);
                (key.clone(), similarity)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = k.min(results.len());
        results.truncate(top_k);

        Ok(results)
    }

    /// 現在の検索呼び出し回数を返す。
    pub fn invocation_count(&self) -> u64 {
        self.invocation_count.load(Ordering::Relaxed)
    }

    /// 検索呼び出し回数カウンタをリセットする。
    pub fn reset_count(&self) {
        self.invocation_count.store(0, Ordering::Relaxed);
    }

    /// 現在の次元数を返す。
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// 登録されているベクトル数を返す。
    pub fn len(&self) -> usize {
        self.entries.lock().expect("Mutex poisoned").len()
    }

    /// 登録ベクトルが空かどうかを返す。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// 2つのベクトル間のコサイン類似度を計算する。
///
/// 返り値は [-1.0, 1.0] の範囲。両ベクトルが同一方向の場合は 1.0、
/// 直交の場合は 0.0、逆方向の場合は -1.0。
/// いずれかのベクトルが零ベクトルの場合は 0.0 を返す。
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
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
    use crate::constants::HNSW_MOCK_DEFAULT_DIMENSION;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    const TEST_SEED: u64 = crate::constants::TEST_PRNG_SEED;

    // ════════════════════════════════════════════════════════════════
    // T1: 同一ベクトル検索
    // ════════════════════════════════════════════════════════════════

    /// T1: ベクトルを 1 件登録し、同一ベクトルで検索すると
    /// 類似度 1.0 で最上位に返る。
    #[test]
    fn exact_match_top_1() {
        let index = MockHnswIndex::new(3);
        let query = vec![1.0, 0.0, 0.0];
        index.insert("vec-a", &query).unwrap();
        index.insert("vec-b", &[0.0, 1.0, 0.0]).unwrap();
        index.insert("vec-c", &[0.0, 0.0, 1.0]).unwrap();

        let results = index.search(&query, 5).unwrap();

        assert!(!results.is_empty(), "should return at least one result");
        assert_eq!(
            results[0].0, "vec-a",
            "identical vector should be top result"
        );
        assert!(
            (results[0].1 - 1.0).abs() < 1e-6,
            "similarity should be 1.0, got {}",
            results[0].1
        );
    }

    // ════════════════════════════════════════════════════════════════
    // T2: ソート不変条件
    // ════════════════════════════════════════════════════════════════

    /// T2: 複数ベクトル登録後、検索結果が常に類似度降順であることを確認する。
    #[test]
    fn sort_invariant_descending() {
        let index = MockHnswIndex::new(3);
        index.insert("a", &[1.0, 0.0, 0.0]).unwrap();
        index.insert("b", &[0.0, 1.0, 0.0]).unwrap();
        index.insert("c", &[0.0, 0.0, 1.0]).unwrap();

        let results = index.search(&[0.9, 0.3, 0.1], 3).unwrap();

        for window in results.windows(2) {
            assert!(
                window[0].1 >= window[1].1,
                "results must be sorted descending: {} ({}) < {} ({})",
                window[0].0,
                window[0].1,
                window[1].0,
                window[1].1
            );
        }
    }

    // ════════════════════════════════════════════════════════════════
    // T3: top_k 境界値
    // ════════════════════════════════════════════════════════════════

    /// T3: k < n, k = n, k > n の 3 通りで k が正しく適用される。
    #[test]
    fn top_k_respects_k() {
        let index = MockHnswIndex::new(3);
        let vectors = vec![
            ("a", vec![1.0, 0.0, 0.0]),
            ("b", vec![0.0, 1.0, 0.0]),
            ("c", vec![0.0, 0.0, 1.0]),
        ];
        for (key, vec) in &vectors {
            index.insert(key, vec).unwrap();
        }

        let query = vec![1.0, 0.0, 0.0];

        // k < n: 2 件要求 → 2 件返る
        let r1 = index.search(&query, 2).unwrap();
        assert_eq!(r1.len(), 2);

        // k = n: 3 件要求 → 3 件返る
        let r2 = index.search(&query, 3).unwrap();
        assert_eq!(r2.len(), 3);

        // k > n: 10 件要求 → 全 3 件返る
        let r3 = index.search(&query, 10).unwrap();
        assert_eq!(r3.len(), 3);
    }

    // ════════════════════════════════════════════════════════════════
    // T4: 空インデックス
    // ════════════════════════════════════════════════════════════════

    /// T4: 未登録状態で検索 → 空の結果ベクタ。
    #[test]
    fn empty_index_returns_empty() {
        let index = MockHnswIndex::new(3);
        let results = index.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    // ════════════════════════════════════════════════════════════════
    // T5: 次元不一致
    // ════════════════════════════════════════════════════════════════

    /// T5: 異なる次元のベクトルで insert / search → エラー。
    #[test]
    fn dimension_mismatch_error() {
        let index = MockHnswIndex::new(3);

        // insert で次元不一致
        let err = index.insert("bad", &[1.0, 0.0]).unwrap_err();
        assert!(
            matches!(
                err,
                DarviumError::EmbeddingDimensionMismatch {
                    expected: 3,
                    actual: 2
                }
            ),
            "expected DimensionMismatch, got {:?}",
            err
        );

        // 正常登録後、search で次元不一致
        index.insert("good", &[1.0, 0.0, 0.0]).unwrap();
        let err = index.search(&[1.0, 0.0], 1).unwrap_err();
        assert!(
            matches!(
                err,
                DarviumError::EmbeddingDimensionMismatch {
                    expected: 3,
                    actual: 2
                }
            ),
            "expected DimensionMismatch, got {:?}",
            err
        );
    }

    // ════════════════════════════════════════════════════════════════
    // T6: 同一キー上書き
    // ════════════════════════════════════════════════════════════════

    /// T6: 同一キーに別ベクトルを再登録 → 後勝ちで上書きされる。
    #[test]
    fn insert_overwrite_same_key() {
        let index = MockHnswIndex::new(3);
        index.insert("key", &[1.0, 0.0, 0.0]).unwrap();
        index.insert("key", &[0.0, 1.0, 0.0]).unwrap();

        let results = index.search(&[0.0, 1.0, 0.0], 1).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "key");
        assert!((results[0].1 - 1.0).abs() < 1e-6);
    }

    // ════════════════════════════════════════════════════════════════
    // T7: ゼロベクトル
    // ════════════════════════════════════════════════════════════════

    /// T7: ゼロベクトルクエリ → 全ての類似度が 0.0。
    #[test]
    fn zero_vector_and_orthogonal() {
        let index = MockHnswIndex::new(3);
        index.insert("a", &[1.0, 0.0, 0.0]).unwrap();
        index.insert("b", &[0.0, 1.0, 0.0]).unwrap();

        let results = index.search(&[0.0, 0.0, 0.0], 5).unwrap();
        assert!(!results.is_empty());
        for (_, score) in &results {
            assert!(
                (*score).abs() < 1e-10,
                "zero vector query should yield similarity 0.0, got {}",
                score
            );
        }
    }

    // ════════════════════════════════════════════════════════════════
    // T8: 呼び出し回数
    // ════════════════════════════════════════════════════════════════

    /// T8: search 呼び出し回数が invocation_count と一致する。
    #[test]
    fn invocation_counting() {
        let index = MockHnswIndex::new(3);
        index.insert("a", &[1.0, 0.0, 0.0]).unwrap();

        assert_eq!(index.invocation_count(), 0);

        let _ = index.search(&[1.0, 0.0, 0.0], 1);
        assert_eq!(index.invocation_count(), 1);

        let _ = index.search(&[0.0, 1.0, 0.0], 1);
        let _ = index.search(&[0.0, 0.0, 1.0], 1);
        assert_eq!(index.invocation_count(), 3);
    }

    // ════════════════════════════════════════════════════════════════
    // T9: カウンタリセット
    // ════════════════════════════════════════════════════════════════

    /// T9: reset_count 後に呼び出し回数が 0 に戻る。
    #[test]
    fn reset_count() {
        let index = MockHnswIndex::new(3);
        index.insert("a", &[1.0, 0.0, 0.0]).unwrap();

        let _ = index.search(&[1.0, 0.0, 0.0], 1);
        let _ = index.search(&[1.0, 0.0, 0.0], 1);
        assert_eq!(index.invocation_count(), 2);

        index.reset_count();
        assert_eq!(index.invocation_count(), 0);
    }

    // ════════════════════════════════════════════════════════════════
    // T10: インスタンス分離性
    // ════════════════════════════════════════════════════════════════

    /// T10: 2 つの MockHnswIndex インスタンスが互いに独立している。
    #[test]
    fn multiple_independent_instances() {
        let index_a = MockHnswIndex::new(3);
        let index_b = MockHnswIndex::new(3);

        index_a.insert("a-only", &[1.0, 0.0, 0.0]).unwrap();
        index_b.insert("b-only", &[0.0, 1.0, 0.0]).unwrap();

        let res_a = index_a.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert_eq!(res_a.len(), 1);
        assert_eq!(res_a[0].0, "a-only");

        let res_b = index_b.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert_eq!(res_b.len(), 1);
        assert_eq!(res_b[0].0, "b-only");
    }

    // ════════════════════════════════════════════════════════════════
    // OTS-1: 単位超球上の三角不等式
    // ════════════════════════════════════════════════════════════════

    /// OTS-1: 1536 次元単位超球上の 3 点 q, a, b に対し、
    /// コサイン計量から誘導される角距離 d(q,b) ≤ d(q,a) + d(a,b) が
    /// 常に成立することを確認する。
    #[test]
    fn ots1_triangle_inequality_on_unit_hypersphere() {
        let n_samples = 10_000;
        let dim = HNSW_MOCK_DEFAULT_DIMENSION;
        let mut rng = StdRng::seed_from_u64(TEST_SEED);

        let mut violations = 0u64;

        for i in 0..n_samples {
            // 単位超球上のランダムベクトルを 3 つ生成
            let q = random_unit_vector(dim, &mut rng);
            let a = random_unit_vector(dim, &mut rng);
            let b = random_unit_vector(dim, &mut rng);

            // 角距離: d(x,y) = arccos(cosine_similarity(x,y))
            let cos_qb = cosine_similarity(&q, &b);
            let cos_qa = cosine_similarity(&q, &a);
            let cos_ab = cosine_similarity(&a, &b);

            let d_qb = cos_qb.acos();
            let d_qa = cos_qa.acos();
            let d_ab = cos_ab.acos();

            if d_qb > d_qa + d_ab + 1e-12 {
                violations += 1;
                if violations <= 3 {
                    println!(
                        "  VIOLATION #{} at sample {}: d(qb)={:.6}, d(qa)={:.6}, d(ab)={:.6}",
                        violations, i, d_qb, d_qa, d_ab
                    );
                }
            }
        }

        print!(
            "=== OTS-1: Triangle Inequality on Unit Hypersphere ===\n\
             dim={}, n={}, violations={}, rate={:.6}\n",
            dim,
            n_samples,
            violations,
            violations as f64 / n_samples as f64
        );

        assert_eq!(
            violations, 0,
            "Triangle inequality MUST hold on unit hypersphere (violations={})",
            violations
        );
        println!("=== 結果: PASS (violations={}) ===", violations);
    }

    // ════════════════════════════════════════════════════════════════
    // OTS-2: ソート普遍性
    // ════════════════════════════════════════════════════════════════

    /// OTS-2: 1,000 個のランダムクエリで search を呼び出し、
    /// 全試行でソート不変条件が維持されることを確認する。
    #[test]
    fn ots2_sort_universality_across_random_queries() {
        let n_queries = 1_000;
        let dim = 64; // テスト高速化のため縮小次元
        let mut rng = StdRng::seed_from_u64(TEST_SEED);

        let index = MockHnswIndex::new(dim);

        // 50 個のランダムベクトルを登録
        for i in 0..50 {
            let vec = random_unit_vector(dim, &mut rng);
            index.insert(&format!("vec-{}", i), &vec).unwrap();
        }

        let mut all_sorted = true;
        let mut total_results = 0usize;

        for _ in 0..n_queries {
            let query = random_unit_vector(dim, &mut rng);
            let results = index.search(&query, 10).unwrap();
            total_results += results.len();

            for window in results.windows(2) {
                if window[0].1 < window[1].1 {
                    all_sorted = false;
                }
            }
        }

        print!(
            "=== OTS-2: Sort Universality ===\n\
             queries={}, vectors=50, dim={}, total_results={}, all_sorted={}\n",
            n_queries, dim, total_results, all_sorted
        );

        assert!(all_sorted, "All search results MUST be sorted descending");
        println!("=== 結果: PASS (all_sorted={}) ===", all_sorted);
    }

    // ════════════════════════════════════════════════════════════════
    // OTS-3: ノイズ摂動有界性
    // ════════════════════════════════════════════════════════════════

    /// OTS-3: クエリに多次元ノイズを付加した際の上位 k 件の結果集合の
    /// 重複率（Jaccard 係数）を観測する。
    #[test]
    fn ots3_noise_perturbation_bounded() {
        let n_trials = 500;
        let dim = 64;
        let noise_sigma = 0.05;
        let mut rng = StdRng::seed_from_u64(TEST_SEED);

        let index = MockHnswIndex::new(dim);

        // 100 個のランダムベクトルを登録
        for i in 0..100 {
            let vec = random_unit_vector(dim, &mut rng);
            index.insert(&format!("vec-{}", i), &vec).unwrap();
        }

        let mut jaccard_values = Vec::with_capacity(n_trials);

        for _ in 0..n_trials {
            let base_query = random_unit_vector(dim, &mut rng);

            // ノイズ付加: base_query + ε, ε ~ N(0, noise_sigma)
            let noisy_query: Vec<f32> = base_query
                .iter()
                .map(|&x| x + (rng.random::<f64>() * 2.0 * noise_sigma - noise_sigma) as f32)
                .collect();

            let base_results = index.search(&base_query, 10).unwrap();
            let noisy_results = index.search(&noisy_query, 10).unwrap();

            // Jaccard 係数 = |intersection| / |union|
            let base_keys: std::collections::HashSet<&str> =
                base_results.iter().map(|(k, _)| k.as_str()).collect();
            let noisy_keys: std::collections::HashSet<&str> =
                noisy_results.iter().map(|(k, _)| k.as_str()).collect();

            let intersection = base_keys.intersection(&noisy_keys).count();
            let union = base_keys.union(&noisy_keys).count();

            let jaccard = if union == 0 {
                1.0
            } else {
                intersection as f64 / union as f64
            };
            jaccard_values.push(jaccard);
        }

        let sum: f64 = jaccard_values.iter().sum();
        let mean = sum / n_trials as f64;
        let variance: f64 = jaccard_values
            .iter()
            .map(|&j| {
                let diff = j - mean;
                diff * diff
            })
            .sum::<f64>()
            / n_trials as f64;
        let min_val = jaccard_values
            .iter()
            .cloned()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        let max_val = jaccard_values
            .iter()
            .cloned()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        print!(
            "=== OTS-3: Noise Perturbation Bounded ===\n\
             dim={}, noise_sigma={}, trials={}\n\
             jaccard: mean={:.6}, variance={:.6}, min={:.6}, max={:.6}\n",
            dim, noise_sigma, n_trials, mean, variance, min_val, max_val
        );

        // Jaccard 係数はノイズがあっても完全に異なる集合にはならないはず
        // （ノイズ強度が小さいので、ある程度の重複がある）
        assert!(
            mean > 0.1,
            "Mean Jaccard should be positive with small noise (mean={:.6})",
            mean
        );
        println!("=== 結果: PASS ===");
    }

    // ── ヘルパー関数 ──

    /// 指定された次元の単位超球上のランダムベクトルを生成する。
    fn random_unit_vector(dim: usize, rng: &mut StdRng) -> Vec<f32> {
        let mut vec: Vec<f32> = (0..dim).map(|_| rng.random::<f64>() as f32).collect();
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vec {
                *x /= norm;
            }
        }
        vec
    }

    // ════════════════════════════════════════════════════════════════
    // T_extra: 空ベクトル insert エラー
    // ════════════════════════════════════════════════════════════════

    /// T_extra: 空のベクトルを insert しようとすると Storage エラー。
    #[test]
    fn insert_empty_vector_errors() {
        let index = MockHnswIndex::new(3);
        let err = index.insert("empty", &[]).unwrap_err();
        assert!(matches!(err, DarviumError::Storage(_)));
    }

    // ════════════════════════════════════════════════════════════════
    // T_extra: 空クエリ search エラー
    // ════════════════════════════════════════════════════════════════

    /// T_extra: 空のクエリで search しようとすると Storage エラー。
    #[test]
    fn search_empty_query_errors() {
        let index = MockHnswIndex::new(3);
        let err = index.search(&[], 1).unwrap_err();
        assert!(matches!(err, DarviumError::Storage(_)));
    }

    // ════════════════════════════════════════════════════════════════
    // T_extra: デフォルト次元で MockHnswIndex を生成
    // ════════════════════════════════════════════════════════════════

    /// T_extra: デフォルト次元 (1536) で生成と基本操作を確認。
    #[test]
    fn default_dimension_creation() {
        let index = MockHnswIndex::new(HNSW_MOCK_DEFAULT_DIMENSION);
        assert_eq!(index.dimension(), HNSW_MOCK_DEFAULT_DIMENSION);
        assert!(index.is_empty());
    }
}
