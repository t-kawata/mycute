# 実装計画: M1.76-8 Helper quality score with benevolence (F-11) + softmax selection (F-12)

## 要件の再確認

RFC §41B.20.1 (F-11) および §41B.20.2 (F-12) で定義される 2 つの純粋関数を実装する:

**F-11**: `Q(h,c,M) = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d`
- 既存の helper quality score (41B-8) に benevolence 項 `w_b·B(h)` を additive に追加
- 入力は 6 成分 (mission_suitability, trust, reputation, benevolence, child_need, distance)
- 出力: 任意の実数 f32

**F-12**: `π(h|c,M) = exp(τ_Q·Q(h,c,M)) / Σ_g exp(τ_Q·Q(g,c,M))`
- 温度パラメータ τ_Q 付き softmax 選択
- log-sum-exp trick で数値的安定性を確保
- 出力: 非負の確率分布 (総和 = 1.0 ± ε)

### 依存関係チェーン

本チケットで実装する F-11/F-12 は以下の後続チケットで使用される:
- M1.76-9 (F-13): Benevolence-aware remote exploration → 本関数の出力を入力として利用
- M1.76-11: ReciprocityEvent インジェスションパイプライン → 定期再計算で参照
- M1.76-16/19: 較正目的関数 F-16 → quality score 重みのチューニング対象

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 追加 | 5 つの新規 Calibration Candidate 定数 |
| `src/event.rs` | 追加 | `ReciprocityLifecyclePolicy` に 6 フィールド + `QualityScoreBreakdown` + `SoftmaxWeight` 型 |
| `src/reciprocity.rs` | 追加 | `compute_helper_quality_score` + `softmax_helper_selection` + テスト 10 件 |
| `tickets/specs/0093-*.md` | 更新 | Investigation に RFC 検証結果追記（済） |

## 計装・観測の実装計画

### 実装するテストコード

全テストは `src/reciprocity.rs` の `mod tests` 内に実装:

| # | テスト名 | 検証内容 | assert 有無 |
|---|---------|---------|------------|
| TC-1 | `test_f11_wb_zero_backward_compat` | w_b=0 で benevolence が Q に影響しない | ✅ assert_eq |
| TC-2 | `test_f11_benevolence_monotonic` | B↑で Q 単調非減少 (n=101 sweep) | ✅ assert |
| TC-3 | `test_f11_all_zero` | 全入力 0 → Q=0 | ✅ assert_eq |
| TC-4 | `test_f11_nan_inf_absent` | ランダム入力で NaN/Inf 不在 | ✅ assert! |
| TC-5 | `test_f12_softmax_sum_one` | softmax 総和 = 1.0 ± 1e-6 | ✅ assert |
| TC-6 | `test_f12_tau_high_argmax` | τ=100 で argmax 確率 > 0.999 | ✅ assert |
| TC-7 | `test_f12_tau_low_uniform` | τ=0.001 で全確率 ≈ 1/N (誤差 ±0.01) | ✅ assert |
| TC-8 | `test_f12_empty_list` | 空スライス → 空 Vec | ✅ assert! |
| TC-9 (計装) | `test_f12_numerical_stability` | n=10^5 ランダム [-100,100] で NaN/Inf/確率和=1 | assert + 観測 |
| TC-10 (計装) | `test_f12_tau_entropy_sweep` | τ_Q 7 水準のエントロピー応答曲線出力 | 観測のみ |

### 観測出力取得方法

```bash
cargo test --package darvium --lib reciprocity::tests::test_f11_ -- --nocapture
cargo test --package darvium --lib reciprocity::tests::test_f12_ -- --nocapture
```

観測出力は構造化 CSV として `println!` で標準出力に書き出す。

### 観測すべき統計量とサンプルサイズ

| 観測対象 | サンプルサイズ | 出力形式 |
|---------|--------------|---------|
| softmax 確率和 | n >= 10^5 | `softmax_sum_check, sum={:.10}, max_dev={:.10}` |
| エントロピー応答 | 7 水準 × 3 分布 | `entropy_sweep, tau={:.3}, entropy={:.6}, n_candidates={}` |
| w_b 感度 | 5 水準 | `wb_sensitivity, wb={:.1}, delta_prob={:.6}` |

### 較正対象の定数と目的関数

本チケットでは較正ループなし（純粋関数実装の検証のみ）。較正は M1.76-16/19 で実施。

**固定初期値**:
- `HELP_QUALITY_SUITABILITY_WEIGHT` = 1.0 (w_s)
- `HELP_QUALITY_TRUST_WEIGHT` = 1.0 (w_t)
- `HELP_QUALITY_REPUTATION_WEIGHT` = 1.0 (w_r)
- `HELP_QUALITY_CHILD_NEED_WEIGHT` = 1.0 (w_n)
- `HELP_QUALITY_DISTANCE_PENALTY` = 1.0 (w_d)
- `HELP_WEIGHT_BENEVOLENCE` = 0.20 (w_b、既存)
- `HELP_SOFTMAX_TAU` = 1.0 (τ_Q、既存)

## Boy Scout 改善（スコープ外の翻訳可能性修正）

特になし。本チケットは新規関数の追加のみであり、既存コードの翻訳可能性は変更しない。

## 実装手順

### Step 1: constants.rs — 5 定数の追加

`HELP_QUALITY_SUITABILITY_WEIGHT` から `HELP_QUALITY_DISTANCE_PENALTY` までの 5 定数を `HELP_WEIGHT_BENEVOLENCE` の直後に追加。

### Step 2: event.rs — データ型 + ポリシー拡張

2a. `ReciprocityLifecyclePolicy` 構造体に `helper_quality_w_s`〜`helper_quality_w_d` の 6 f32 フィールドを `tau_helper_softmax` の直後に追加。

2b. `Default` インプリメントに各フィールドの定数参照初期化を追加。

2c. `QualityScoreBreakdown` 構造体定義追加 (Serialize, Deserialize derive)。

2d. `SoftmaxWeight` 構造体定義追加。

### Step 3: reciprocity.rs — 関数実装

3a. `compute_helper_quality_score` — F-11 線形結合、policy から重み読み取り。

3b. `softmax_helper_selection` — F-12 log-sum-exp trick、空リストガード。

### Step 4: テスト実装

TC-1 〜 TC-10 を `mod tests` に追加。

### Step 5: コンパイル + テスト実行

```bash
cargo test --package darvium --lib 2>&1 | tail -30
```

### Step 6: 既存テスト退行確認

```bash
cargo test --package darvium 2>&1 | tail -20
```

### Step 7: 定数ダンプテスト更新

`src/event.rs` の `test_calibration_candidates_constants_non_nan` に HELP_QUALITY_* 5 定数のダンプ + 非 NaN アサーションを追加。

## 物理的レビュー方法

### run-quality-checks.js による自動チェック

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" \
  src/constants.rs \
  src/event.rs \
  src/reciprocity.rs \
  tickets/specs/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12.md
```

### 翻訳可能性 grep

```bash
# 関数定義: 動詞始まりの確認
grep -n "^pub fn \|^fn " src/reciprocity.rs | grep -v "test_"

# 1文字変数
grep -n "let [a-z] " src/reciprocity.rs | grep -v "//\|test_\|///"

# ハードコードされた数値リテラル
grep -nE "[0-9]{4,}" src/constants.rs | grep -v "///\|pub const\|// "
```

### レビュー観点

1. 定数定義の命名・コメントが適切か
2. ポリシーフィールドの Default 値が正しい定数を参照しているか
3. F-11/F-12 の数式が RFC と一致するか
4. log-sum-exp trick が正しく実装されているか
5. エッジケース（空リスト、NaN 入力）の処理が適切か
6. softmax 確率和が浮動小数点誤差範囲内で 1 になるか
7. 既存テスト（F-1〜F-10）が全て PASS するか
8. RFC §41B.20.1 / §41B.20.2 との無矛盾確認

## リスク

| リスク | 確度 | 影響 | 対策 |
|-------|------|------|------|
| softmax の数値安定性 (log-sum-exp 未使用による overflow) | 低 | 中 | log-sum-exp trick の強制使用 |
| ポリシーフィールド追加によるシリアライズ非互換 | 中 | 高 | serde の default attribute で後方互換維持（既存 derive が自動対応） |
| w_b=0.20 が低すぎて benevolence 効果が観測ノイズに埋もれる | 中 | 低 | 較正フェーズ (M1.76-16/19) で調整。本チケットでは単調性のみ検証 |
| QualityScoreBreakdown の冗長性 | 低 | 低 | 現状設計で許容。パフォーマンス問題が確認された場合は遅延評価化を検討 |
