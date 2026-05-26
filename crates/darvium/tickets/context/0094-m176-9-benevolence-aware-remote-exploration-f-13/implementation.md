# M1.76-9 Benevolence-aware remote exploration (F-13) 実装サマリ

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/constants.rs:780-793` | 修正 | REMOTE_EXPLORATION_NEED_COEFF (a₁=1.0) および REMOTE_EXPLORATION_BENEVOLENCE_COEFF (a₂=1.0) 定数追加 |
| `src/event.rs:520-523` | 修正 | ReciprocityLifecyclePolicy に epsilon_remote_need_coeff / epsilon_remote_benevolence_coeff フィールド追加 |
| `src/event.rs:554-555` | 修正 | Default 実装に新規フィールド初期化を追加 |
| `src/event.rs:5212-5213` | 修正 | JSON roundtrip テストの struct literal に新規フィールド追加 |
| `src/reciprocity.rs:485-507` | 追加 | compute_benevolence_aware_remote_exploration 純粋関数 (F-13) |
| `src/reciprocity.rs:1961-2146` | 追加 | 不変条件テスト 7件 + 観測テスト 1件 |

## 関数実装

```rust
pub fn compute_benevolence_aware_remote_exploration(
    child_need: f32,
    local_benevolence_mean: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let raw = policy.epsilon_remote_base
        + policy.epsilon_remote_need_coeff * child_need
        - policy.epsilon_remote_benevolence_coeff * local_benevolence_mean;
    raw.clamp(0.0, policy.epsilon_remote_max)
}
```

## 接続方式

Adapter パターン: 呼び出し元で F-13 関数を事前計算し HelperSelectionPolicy.epsilon に設定する。既存の select_helpers のシグネチャは変更不要。

## テスト結果

- 不変条件テスト 7件: 全 PASS
  - T-1: boundary_min → ε=0.0 (clip 下限)
  - T-2: boundary_max → ε=0.20 (ε_max)
  - T-3: benevolence monotonic → 単調非増加確認
  - T-4: need monotonic → 単調非減少確認
  - T-5: backward compat → a₂=0, need=0 で ε₀=0.05 と一致
  - T-6: bounded random (n=10⁴) → 全値域 [0, ε_max] 確認
  - T-7: neutral → need=0.5, B_local=0.5 で ε₀=0.05 と一致
- 観測テスト: 2D 応答曲面 (a₁/a₂ ratio 3水準 sweep) + 差分分布 (n=10⁴) 出力完了
- 全既存テスト (974 unit + 17 integration/doc): 全 PASS
