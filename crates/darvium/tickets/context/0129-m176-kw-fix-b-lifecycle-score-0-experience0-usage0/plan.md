# 実装計画: M1.76-KW-FIX-B — 子供 lifecycle_score = 0 問題

## RFC 既存実装状態検証

### RFC §15.5 LifecycleScore

| 観点 | RFC §15.5 | 現行コード | 状態 |
|------|-----------|-----------|------|
| 入力形式 | 5 個別 f32 引数 | LifecycleScore struct (f64) | ⚠️ 機能的同等 |
| 型 | f32 | f64 | ❌ 型不一致 |
| 重み | LIFECYCLE_ALPHA_* で成分別指数 | 均等幾何平均 (^(1/5)) | ❌ 重み未実装 |
| 幾何平均 | 重み付き積 | 均等積^(1/5) | ⚠️ RFC §4A.7 は幾何平均と記述 |
| usage 定義 | run_count/reuse_count/contribution | experience 正規化 | ⚠️ 簡略化実装 |

**評価サマリ**: LifecycleScore の重み (LIFECYCLE_ALPHA_*) は未実装だが、均等幾何平均は RFC §4A.7 の記述と矛盾しない。FIX-B の usage=0 問題は重みの有無に関わらず発生する（0 の何乗も 0）。本チケットでは重み未実装はスコープ外（spec の Non-scope 参照）。

### RFC §15.10.4 GC Hazard (F-7)

| 観点 | RFC §15.10.4 | 現行コード | 状態 |
|------|-------------|-----------|------|
| 式 | softplus(λ₀ - γ_L·L_i - γ_B·B_i - γ_C·C_i^protect) | softplus(lambda_gc_base - gamma_lifecycle*L - gamma_benevolence*B - gamma_child_protect*C) | ✅ 一致 |
| softplus | softplus | softplus | ✅ 一致 |
| パラメータ | λ₀, γ_L, γ_B, γ_C | lambda_gc_base, gamma_lifecycle, gamma_benevolence, gamma_child_protect | ✅ 機能的同等 |
| child_protect | η₁·1[Child(i)] + η₂·H_received + η₃·G_growth | child_protection 定数 (0.5) | ⚠️ 簡略化（スコープ外） |

**評価サマリ**: F-7 は現行コードと完全一致。child_protect の詳細式 (F-10) は簡略化実装だが FIX-B のスコープ外。

### RFC §41B (F-10) Child Protection

| 観点 | RFC | 現行コード | 状態 |
|------|-----|-----------|------|
| 構造 | η₁·1[Child] + η₂·H_received + η₃·G_growth | 定数 child_protection=0.5 | ⚠️ 簡略化（スコープ外） |
| 子供保護の根拠 | Grace Period + 支援量 + 成長量 | gamma_child_protect=10.0 のみ | ❌ lifecycle 項が 0 のため過剰依存 |

### Investigation 更新（plan 策定時: 2026-05-27）

spec 作成時から以下の変更は確認されていない。全証拠は現行コードと一致：
- experience=0 → usage=0.0 は `compute_experience_normalization` (reciprocity.rs:298-300) で確認済み
- lifecycle_score=0 は `compute_lifecycle_score` (lifecycle.rs:40-43) で確認済み
- gamma_child_protect=10.0 は constants.rs:726 で確認済み
- TC8 テスト (reciprocity.rs:5322-5328) は experience=0 → 0.0 をアサート

## 要件の再確認

**問題**: 子供ノードは初期化時に experience=0 で開始され、`compute_experience_normalization(0) = 0.0` となる。これにより lifecycle_score の usage 成分が 0 になり、幾何平均全体が 0 に固定される。結果として GC hazard の lifecycle 項が全く効かず、gamma_child_protect=10.0 という非現実的に高い定数で強引に子供を保護している。

**修正方針**: **オプション c（offset 導入）** を採用する。

```
compute_experience_normalization(e) = 1.0 - exp(-(e + OFFSET) / SCALE)
```

- 少ないコード変更で根本原因を解決
- 全ノードに一律適用（子供限定の条件分岐不要）
- 経験値大のノードへの影響は微小（OFFSET=1.0 で 10 exp 時の誤差 < 4%）
- TC8 テストの期待値を更新する必要あり

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 追加 | `EXPERIENCE_NORMALIZATION_OFFSET: f64 = 1.0` を追加 |
| `src/constants.rs` | 変更 | `GC_HAZARD_GAMMA_CHILD_PROTECT: 10.0 → 5.0`（初期緩和値） |
| `src/reciprocity.rs` | 変更 | `compute_experience_normalization` に offset 追加 |
| `src/reciprocity.rs` | 変更 | TC8 テスト期待値更新 (0.0 → approx 0.095) |
| `src/reciprocity.rs` | 追加 | FIX-B1 テスト: experience=0 で usage > 0 の確認 |
| `src/lifecycle.rs` | 追加 | FIX-B3 テスト: lifecycle_score > 0（usage のみ 0.095 で他成分が 0.8 の場合） |
| `src/simulation.rs` | 追加 | 観測テスト: FIX-B5/B6/B7 — lifecycle_score 分布・GC hazard 比較の println! 出力 |

## 計装・観測の実装計画

### テスト一覧

| ID | テスト名 | ファイル | 種別 | 内容 |
|----|---------|---------|------|------|
| FIX-B1 | `test_fixb_experience_zero_usage_positive` | reciprocity.rs | 不変条件 | experience=0 で usage > 0.05 をアサート |
| FIX-B2 | 既存 TC8 更新 | reciprocity.rs | 不変条件 | 既存アサートを 0.0 から approx 0.095 に変更 |
| FIX-B3 | `test_fixb_lifecycle_score_positive` | lifecycle.rs | 不変条件 | usage=0.095、他成分=0.8 で lifecycle_score > 0.3 をアサート |
| FIX-B5 | `test_fixb_observe_lifecycle_distribution` | simulation.rs | 観測 | println! で子供/成人別 lifecycle_score 分布、usage、GC hazard を出力 |
| FIX-B6 | `test_fixb_observe_usage_by_experience` | simulation.rs | 観測 | 子供/成人別 usage 値の 5 数要約 (min/q1/med/q3/max) を出力 |
| FIX-B7 | `test_fixb_observe_gc_hazard` | simulation.rs | 観測 | 子供 vs 成人の GC hazard 比較、gamma_child_protect 変更前後の差を出力 |

### 観測テスト実行方法

```bash
# 不変条件テスト
cargo test fixb

# 観測テスト（--nocapture）
cargo test fixb_observe -- --nocapture
```

### 観測統計量

- 子供 lifecycle_score 最小値（期待: > 0.3）
- 子供/成人別 mean lifecycle_score
- usage 値の children / adults 分布（5 数要約）
- GC hazard 値の children / adults 平均
- 固定シード: `StdRng::seed_from_u64(12345)`

### 較正計画

| 対象定数 | 現行値 | 目標値 | 方法 |
|---------|--------|--------|------|
| `GC_HAZARD_GAMMA_CHILD_PROTECT` | 10.0 | 5.0（初期）→ 2.0-5.0 に調整 | 観測テスト実行後、子供生存率が成人と同等〜やや高めになる範囲に収束 |
| `EXPERIENCE_NORMALIZATION_OFFSET` | 新設 | 1.0 | 固定。0.5-2.0 の範囲調整余地あり |

**較正ループの停止条件**: 子供の GC hazard が成人より高くなく（子供保護が機能）、かつ gamma_child_protect が 5.0 未満で安定していること。

## Boy Scout 改善（スコープ外の翻訳可能性修正）

### 1. phase4_gc_survival の lifecycle_score 計算ブロック抽出

`simulation.rs:1914-1935` の lifecycle_score 5 成分組立と GC hazard 計算が同一関数内に混在している。以下の関数に抽出する：

```rust
fn compute_node_lifecycle_score(
    age: u64,
    freshness: f64,
    node_experiences: &HashMap<NodeId, u64>,
    id: NodeId,
) -> LifecycleScore { ... }
```

### 2. compute_mean_lifecycle_score プロキシ値問題の調査

`kind_world.rs:2155` の `compute_mean_lifecycle_score` は GcEvent 状態ベースのプロキシ値（Active=0.8, Protected=1.0, ...）を使用している。本チケットの影響範囲を確認し、乖離が大きい場合は修正を計画に含める。

## 実装手順

1. `src/constants.rs`: `EXPERIENCE_NORMALIZATION_OFFSET` 定数を追加
2. `src/reciprocity.rs`: `compute_experience_normalization` の計算式に offset を追加
3. `src/reciprocity.rs`: FIX-B1 テスト追加 + TC8 テスト更新
4. `src/lifecycle.rs`: FIX-B3 テスト追加
5. `cargo test` で全テスト PASS 確認
6. `src/constants.rs`: `GC_HAZARD_GAMMA_CHILD_PROTECT` を 10.0 → 5.0 に変更
7. `src/simulation.rs`: FIX-B5/B6/B7 観測テスト追加（println! + --nocapture）
8. `cargo test` + `cargo test fixb_observe -- --nocapture` で観測データ取得
9. 観測データに基づき gamma_child_protect を調整
10. Boy Scout 改善（lifecycle_score 計算抽出）
11. `cargo test` / `cargo clippy` 最終確認
12. 品質チェック実行（run-quality-checks.js）
13. 観察レポート作成・保存
14. 実装サマリ保存 → done 遷移

## 物理的レビュー方法

```bash
# 品質チェック
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" \
  src/constants.rs src/reciprocity.rs src/lifecycle.rs src/simulation.rs \
  | node "$_R/scripts/tickets/review/generate-report.js"
```

### 翻訳可能性チェック
- `compute_experience_normalization` に offset 導入後も関数名と動作が一致しているか確認
- 新規追加する定数名がドメイン概念を表現しているか確認
- 観測テストの println! が構造化されているか確認

## リスク

| リスク | 影響 | 対策 |
|--------|------|------|
| offset 導入で経験値大のノードの usage が影響を受ける | 低 (OFFSET=1.0: 10exp で ~4% 変化) | 感度分析で確認。必要なら OFFSET を 0.5 に低減 |
| gamma_child_protect 緩和後、子供生存率が過度に低下 | 中 | 段階的緩和（10→5→3→2）。各ステップで観測 |
| 幾何平均の性質上、usage 以外の成分も 0 に近い場合 lifecycle_score が依然低い | 低 | 他成分（freshness/success/trust/reputation）は出生時に非ゼロが保証される |
| compute_mean_lifecycle_score のプロキシ値が真値と乖離 | 低 | Boy Scout 項目として調査のみ。本チケットで修正するかは乖離度に依存 |
