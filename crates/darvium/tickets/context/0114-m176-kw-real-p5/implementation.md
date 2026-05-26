# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/lifecycle.rs | 新規 | LifecycleScore 構造体 + compute_lifecycle_score + TC2/TC3 テスト |
| src/event.rs | 変更 | GcEvent に Protected/Active 追加 (5変種)、transition_gc_state 追加、全マッチ箇所更新、TC4/TC5 テスト |
| src/event_channel.rs | 変更 | ランダム GcEvent 生成を 3→5 変種に拡張 |
| src/trust.rs | 変更 | inherit_trust / inherit_reputation + TC6/TC7 テスト |
| src/reciprocity.rs | 変更 | compute_experience_normalization (F-5) + TC8 テスト |
| src/clock/mod.rs | 変更 | compute_blended_freshness (F_time) + TC9 テスト |
| src/constants.rs | 変更 | EXPERIENCE_NORMALIZATION_SCALE, HUMAN_FRESHNESS_HALFLIFE_MS, VIRTUAL_FRESHNESS_HALFLIFE 追加 |
| src/lib.rs | 変更 | lifecycle モジュール公開、新規関数の再エクスポート |
| src/simulation.rs | 変更 | test_p5_lifecycle_instrumentation 観測テスト追加 |

## 実装内容の概要

### LifecycleScore (RFC §15.3)
- 5成分 (freshness, success, trust, usage, reputation) の幾何平均として定義
- `compute_lifecycle_score()`: 幾何平均 L(G) = (prod)^(1/5)

### GC 5状態機械 (RFC §4A.7 機構 40)
- GcEvent: Protected, Active, SoftDeleted, HardDeleteCandidate, Tombstoned
- `transition_gc_state()`: hazard 閾値ベースの状態遷移
  - Protected → Active (hazard > 0.0)
  - Active → SoftDeleted (hazard > 0.0)
  - SoftDeleted → HardDeleteCandidate (hazard > 0.5)
  - HardDeleteCandidate → Tombstoned (hazard > 0.8)
  - Protected → Tombstoned 直接遷移禁止

### 信頼継承・評判継承
- `inherit_trust()`: operational/semantic/temporal に減衰係数乗算
- `inherit_reputation()`: parent.final_score を child.inherited_score に継承

### ExperienceNormalization F-5 (RFC §4A.5 機構 35)
- `compute_experience_normalization()`: 1 - exp(-exp/SCALE) の非線形正規化

### BlendedFreshness F_time (RFC §4A.9 機構 50)
- `compute_blended_freshness()`: F_time = w_H * F_H + (1-w_H) * F_V
- 人間時間 (UTC) と仮想時刻 (tick) の二軸指数減衰

## テスト件数
- 新規テスト: 8件 (TC2-TC9)
- 観測テスト: 1件 (test_p5_lifecycle_instrumentation)
- 全テスト PASS: 1225件
