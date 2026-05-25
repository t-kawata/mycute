# 実装サマリ: M1.76-3 直接互恵性スコア compute_direct_reciprocity (F-1) 純粋関数実装

## 変更したファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/reciprocity.rs | 新規 | 相互互恵性モジュール。event_kind_weights / logistic_sigmoid / time_decay 補助関数 + compute_direct_reciprocity 本体 + テスト6件 |
| src/lib.rs | 修正 | 22行目に `pub mod reciprocity;` 追加（recovery と replay の間） |

## 実装内容の概要

### 公開関数

```rust
pub fn compute_direct_reciprocity(
    events: &[ReciprocityEvent],
    now: u64,
    policy: &ReciprocityLifecyclePolicy,
) -> f32
```

- 式 F-1: σ( Σ ω (α_h H + α_hs HS - α_r RJ - α_d DMG) exp(-ρ_dir Δt) )
- α_h, α_hs, α_r, α_d は constants.rs から参照
- ρ_dir は policy.rho_direct_decay から取得
- ω (イベント重み) は ReciprocityEvent.weight フィールド
- Δt = now - event.virtual_clock（saturating_sub でアンダーフロー防止）
- logistic sigmoid σ(x) = 1/(1+exp(-x)) で [0,1] 正規化

### 内部補助関数（非公開）
- event_kind_weights: 8種の ReciprocityEventKind → (H, HS, RJ, DMG) マッピング
- logistic_sigmoid: σ(x) = 1/(1+exp(-x))
- time_decay: exp(-ρΔt)

### テスト結果
- M1.76-3: 6/6 PASS
- 全クレートテスト: 944/944 PASS（既存テストへの影響ゼロ）
- 品質チェック: 全通過（println! は観測計装として意図的）
OBSERVE_EOF