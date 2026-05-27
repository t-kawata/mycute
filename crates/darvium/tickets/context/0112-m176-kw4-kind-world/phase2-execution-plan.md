# Phase 2: 94因子パレート最適フロンティア探索 実行計画

## 0. 参照関係

- **本計画書を参照するファイル**: `experiments.md`（同ディレクトリ）が各サイクルの実験結果を記録。本計画書が運用手順書となる。
- **既存の類似ファイル**: `plan.md` は初期 M1.76-KW4 較正（7パラメーター、基本セットアップ）用であり、本計画（次元拡張戦略）とは目的が異なる。競合なし。
- **本計画書の位置づけ**: AI が自律的に実行する運用手順書。ユーザーへの報告は各グループ完了時および合流点でのみ行う。

---

## 1. 基本戦略

### 1.1 アプローチ: 観測駆動型次元拡張

94因子全てを一度に最適化することは、次元の呪いにより事実上不可能。そこで7つのグループに分割し、影響の大きい順に逐次追加する。

```text
G1探索（探索打ち切り）→ G1+G2探索 → G1+G2+G3探索 → ... → 全7グループ
```

各段階でパレート改善が頭打ちになった時点で次のグループを追加する。

### 1.2 グループ定義（J_kw_social への推定影響順）

| グループ | 因子数 | 内容 | 推定影響 |
|----------|--------|------|---------|
| G1 | 14 | 検索・探索系（s_search） | 最大 |
| G2 | 12 | GC・生存系 | 大 |
| G3 | 9 | ライフサイクル系 | 大 |
| G4 | 16 | 互恵性・評判系 | 中 |
| G5 | 12 | 成熟・成長系 | 中 |
| G6 | 19 | ネットワーク構造系 | 小〜中 |
| G7 | 12 | システム枠組み系 | 小 |

**合計: 94因子**

### 1.3 進行ルール

- 各グループ内では全因子を同時解放（一度に追加）
- パレート改善率が3サイクル連続で5%未満 = 頭打ち判定 → 次グループ追加
- 最大24サイクル that 全グループ完了 or 頭打ち判定

### 1.4 各サイクルの流れ

```text
[状態分析] → [パラメーター調整] → [cargo test -- --nocapture] → [experiments.md に記録] → [次サイクル判断]
```

### 1.5 停止条件

- **成功完了**: 全7グループが追加され、最終サイクルで3サイクル連続5%未満改善
- **早期打ち切り**: いずれかの段階で10サイクル連続で有意改善なし（→ そのグループは低影響と判断）
- **上限到達**: 8サイクル到達時点で最終報告

---

## 2. 7グループの詳細定義

### G1: 検索・探索系（14因子）— 第1優先

| # | 因子名 | 現在値 | 現状 | 備考 |
|---|--------|--------|------|------|
| 47 | REMOTE_EXPLORE_INTERVAL | 30 | (U) | kind_world.rs 未実装の remote_exploration_interval 待ち |
| 48 | REMOTE_EXPLORE_DECAY | 0.5 | (U) | 同上 |
| 49 | REMOTE_EXPLORE_STEPS | 3 | (U) | 同上 |
| 50 | REMOTE_EXPLORE_REWARD | 0.1 | (U) | 同上 |
| 84 | search_tick_fraction | 0.5 | (S) | ReciprocitySimulatorConfig |
| 85 | evaluate_fraction | 0.3 | (S) | 同上 |
| 86 | KW4_EVALUATION_POPULATION_SIZE | 400 | (U) | 実験枠組定数 |
| 87 | KW4_SIMULATION_TICKS | 200 | (U) | 実験枠組定数 |
| 88 | RECIPROCITY_ALPHA_HELP | 0.50 | (S) | α 互恵性 |
| 89 | RECIPROCITY_ALPHA_RECEIVE | 0.30 | (S) | α 互恵性 |
| 90 | RECIPROCITY_ALPHA_RANK | 0.20 | (S) | α 互恵性 |
| 91 | RECIPROCITY_ALPHA_OFFER | 0.40 | (S) | α 互恵性 |
| 92 | compute_search_radius_inverse return | 0.5 (stub) | (H) | 永久スタブ — 実装必須 |
| 94 | REMOTE_EXPLORE_SETTING.human_weight | 0.0 | (U) | kind_world.rs 未実装 |

**G1 の特徴**: s_search に直接影響。stub（#92）の実装が必須。

### G2: GC・生存系（12因子）— 第2優先

| # | 因子名 | 現在値 | 現状 | 備考 |
|---|--------|--------|------|------|
| 1 | GC_BASE_LAMBDA | 0.15 | (S) | 忘却曲線 |
| 2 | GC_HELP_DECAY | 0.10 | (S) | 忘却曲線 |
| 3 | GC_INTERVAL | 5-20 | (S) | GC 間隔 |
| 4 | GC_NOP_DECAY | 0.05 | (S) | 忘却曲線 |
| 5 | GC_SURVIVAL_THRESHOLD | 0.10 | (S) | GC 安全弁 |
| 6 | GC_OVERFLOW_RATIO | 3.0 | (S) | GC 容量 |
| 7 | GC_MAX_CYCLES_BETWEEN | 10 | (S) | GC 頻度 |
| 8 | GC_RESERVOIR_PROMOTE | 0.05 | (S) | GC 昇格 |
| 9 | GC_RESERVOID_DECAY | 0.25 | (S) | GC 忘却 |
| 10 | GC_SURVIVAL_BOOST | 10.0 | (S) | 生存促進 |
| 14 | lambda_gc_survival | 0.5 | (S) | GC 生存率 |
| 15 | human_weight (phase4) | 0.5 | (S) | 人間重み |

**G2 の特徴**: (S) が多く、既に tuning 探索済み。改善余地は limited。

### G3: ライフサイクル系（9因子）— 第3優先

| # | 因子名 | 現在値 | 現状 | 備考 |
|---|--------|--------|------|------|
| 10 | lifecycle_score compute | - | (S) | 計算式全体 |
| 11 | ALPHA_LIFECYCLE | 1.0 | (S) | ライフサイクル形状 |
| 12 | BETA_LIFECYCLE | 1.0 | (S) | ライフサイクル形状 |
| 13 | TRUST_DECAY | 0.10 | (S) | 信頼減衰 |
| 16 | compute_mean_freshness weight | 0.0 | (H) | メトリクス計算 |
| 65 | CHILD_MATURATION_AGE | 300 | (S) | 子供成熟 |
| 66 | INHERIT_TRUST_FRACTION | 0.70 | (S) | 信頼継承 |
| 67 | INHERIT_SKILL_FRACTION | 0.50 | (S) | 技能継承 |
| 68 | CHILD_HELP_BONUS | 0.20 | (S) | 子供支援 |

**G3 の特徴**: lifecycle_score 計算式全体が対象。（H）#16 は定数化して調整可能に。

### G4: 互恵性・評判系（16因子）— 第4優先

| # | 因子名 | 現在値 | 現状 | 備考 |
|---|--------|--------|------|------|
| 39 | HELPER_QUALITY_DECAY | 0.10 | (S) | 品質減衰 |
| 40 | HELPER_QUALITY_INIT | 1.0 | (S) | 品質初期値 |
| 41 | HELPER_QUALITY_MU | 1.0 | (S) | 品質学習率 |
| 42 | HELPER_QUALITY_LOGISTIC_K | 1.0 | (S) | 品質シグモイド |
| 43 | HELPER_QUALITY_LOGISTIC_x0 | 0.0 | (S) | 品質シグモイド |
| 44 | HELPER_QUALITY_POWER | 2.0 | (S) | 品質指数 |
| 45 | HELPER_QUALITY_SIGMA | 0.5 | (S) | 品質ノイズ |
| 46 | HELPER_QUALITY_S_WINDOW | 5 | (S) | 品質窓幅 |
| 51 | OFFER_BASE_PROB | 0.3 | (H) | 提供確率ベース |
| 52 | OFFER_BV_MULTIPLIER | 0.4 | (H) | 提供確率乗数 |
| 53 | ACCEPT_PROB | 0.5 | (H) | 受入確率 |
| 54 | OFFER_HIGH_COMPETENCE_MULTI | 0.3 | (H) | 高能力倍率 |
| 55 | OFFER_LOW_COMPETENCE_MULTI | 0.6 | (H) | 低能力倍率 |
| 56 | ADVANCE_HARMFUL_PROB | 0.25 | (H) | 危害確率 |
| 57 | ADVANCE_FAIL_PROB | 0.15 | (H) | 失敗確率 |
| 58 | ADVANCE_SUCCESS_HELP_QUALITY | 0.1 | (H) | 成功時品質 |

**G4 の特徴**: 多くの (H) を含む。定数化が前提。

### G5: 成熟・成長系（12因子）— 第5優先

| # | 因子名 | 現在値 | 現状 | 備考 |
|---|--------|--------|------|------|
| 59 | CHILD_TRUST_INIT | 0.3 | (H) | 子供初期信頼 |
| 60 | ADULT_TRUST_INIT | 0.5 | (H) | 成人初期信頼 |
| 61 | ADULT_TRUST_NOISE | 0.3 | (H) | 成人信頼ノイズ |
| 69 | CHILD_MAX_ENTRIES | 20 | (S) | 子供最大数 |
| 70 | ADULT_MIN_ENTRIES | 5 | (S) | 成人最小数 |
| 71 | CHILD_TRUST_DECAY | 0.10 | (H) | 子供信頼減衰 |
| 72 | ADULT_TRUST_DECAY | 0.10 | (H) | 成人信頼減衰 |
| 73 | CHILD_TOPIC_BIAS | 0.30 | (H) | 子供話題偏向 |
| 74 | ADULT_HELP_QUALITY_THRESHOLD | 0.5 | (H) | 成人品質閾値 |
| 75 | ADULT_TOLERANCE | 5 | (H) | 成人許容度 |
| 76 | ADULT_BENEVOLENT_THRESHOLD | 0.4 | (H) | 成人慈愛閾値 |
| 93 | KW4_MISSION_RATE | 1.0 | (U) | 初期設定値 |

**G5 の特徴**: 初期化パラメーター中心。（H）の定数化が前提。

### G6: ネットワーク構造系（19因子）— 第6優先

| # | 因子名 | 現在値 | 現状 | 備考 |
|---|--------|--------|------|------|
| 17 | GAMMA_BENEVOLENCE | 0.25 | (S) | 慈愛伝播減衰 |
| 18 | GAMMA_MATURITY | 0.15 | (S) | 成熟伝播減衰 |
| 19 | GAMMA_COMPETENCE | 0.20 | (S) | 能力伝播減衰 |
| 20 | LAMBDA_GC_BASE | 0.10 | (S) | GC 基底 |
| 21 | DIRECT_RECIPROCITY_WEIGHT | 0.40 | (S) | 直接互恵性重み |
| 22 | INDIRECT_RECIPROCITY_WEIGHT | 0.30 | (S) | 間接互恵性重み |
| 23 | SOFTMAX_TEMPERATURE | 0.50 | (S) | ソフトマックス温度 |
| 24 | HELP_EXPLORATION_EPSILON | 0.10 | (S) | ε 探索 |
| 25 | TRUST_INHERIT_DECAY | 0.70 | (S) | 信頼継承減衰 |
| 26 | CAPABILITY_DIFFUSION_RATE | 0.50 | (S) | 能力拡散率 |
| 27 | REPUTATION_BIAS_FACTOR | 0.2 | (S) | 評判偏向 |
| 28 | REPUTATION_RESET_AGE | 3000 | (S) | 評判リセット |
| 29 | REPUTATION_MATURATION | 0.01 | (S) | 評判成熟 |
| 30 | REPUTATION_WEIGHT_BENEVOLENCE | 0.40 | (S) | 評判重み |
| 31 | REPUTATION_WEIGHT_COMPETENCE | 0.35 | (S) | 評判重み |
| 32 | REPUTATION_WEIGHT_ACTIVITY | 0.25 | (S) | 評判重み |
| 33 | INDIRECT_BETA_BENEVOLENCE | 0.30 | (S) | β 間接 |
| 34 | INDIRECT_BETA_COMPETENCE | 0.25 | (S) | β 間接 |
| 35 | INDIRECT_BETA_SOCIAL | 0.20 | (S) | β 間接 |

**G6 の特徴**: MagnificentSeven の一部を含む。（S）主体で calibratable だが数が多い。

### G7: システム枠組み系（12因子）— 第7優先

| # | 因子名 | 現在値 | 現状 | 備考 |
|---|--------|--------|------|------|
| 36 | NETWORK_DENSITY_TARGET | 0.05 | (S) | 密度目標 |
| 37 | NETWORK_REWIRE_PROB | 0.01 | (S) | 再配線確率 |
| 38 | NETWORK_CONNECT_PROB | 0.10 | (S) | 接続確率 |
| 77 | VILLAGE_POPULATION_TARGET | 500 | (H) | 村目標人口 |
| 78 | VILLAGE_MIGRATION_RATE | 0.05 | (H) | 村移動率 |
| 79 | CHILD_RATIO | 0.30 | (S) | 子供比率 |
| 80 | GC_INTERVAL_TICKS | 10 | (H) | GC tick 間隔 |
| 81 | GMR_N_NEAREST_NEIGHBORS | 5 | (S) | GMR 近傍数 |
| 82 | GMR_SEARCH_WIDTH | 10 | (S) | GMR 探索幅 |
| 83 | GMR_ENTRY_REFRESH_TICKS | 50 | (S) | GMR 更新間隔 |
| 84 | search_tick_fraction | 0.5 | (S) | 重複あり |
| 85 | evaluate_fraction | 0.3 | (S) | 重複あり |

**G7 の特徴**: システム全体の枠組み。（H）の定数化が前提。

---

## 3. コード変更計画

### 3.0 基盤インフラ（Stage 0 事前準備）

**目的**: 2パラメーター群への拡張を可能にする基盤整備

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.0a | `src/kind_world.rs` | `MagnificentSevenParams` を `All94Params` に拡張（全94フィールド保持）。`get_param()`/`set_param()` を 94 対応に拡張 |
| 3.0b | `src/kind_world.rs` | `to_sim_config()` 拡張（G1-G7 全ての定数マッピング） |
| 3.0c | `src/kind_world.rs` | `evaluate_single()` に全てのパラメーター注入経路を追加 |
| 3.0d | `src/kind_world.rs` | `NelderMeadOptimizer` を可変次元に一般化（7固定→N可変） |
| 3.0e | `src/kind_world.rs` | `tc7_kw4_bayesian_pareto()` をパラメーター数可変に拡張 |
| 3.0f | `src/simulation.rs` | `ReciprocitySimulatorConfig` に G1-G7 該当フィールドを追加 |
| 3.0g | `src/event.rs` | `ReciprocityLifecyclePolicy::Default` に全定数マッピングを追加 |

### 3.1 G1: 検索・探索系

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.1a | `src/kind_world.rs` | `compute_search_radius_inverse()` のスタブを本実装（エージェントの検索半径に基づく計算） |
| 3.1b | `src/simulation.rs` | `ReciprocitySimulatorConfig` に search_tick_fraction, evaluate_fraction 追加 |
| 3.1c | `src/kind_world.rs` | remote_exploration 未実装部分の計装（現状スキップでよいがパラメーターは受け付ける） |

### 3.2 G2: GC・生存系

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.2a | `src/simulation.rs` | phase4_gc_survival の child_protection（0.5）を定数化して `ReciprocitySimulatorConfig` 経由で注入可能に |

### 3.3 G3: ライフサイクル系

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.3a | `src/kind_world.rs` | `compute_mean_freshness` の human_weight=0.0 をパラメーター化 |
| 3.3b | `src/simulation.rs` | phase5 の capability_diffusion 減衰（0.7）を定数化 |

### 3.4 G4: 互恵性・評判系

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.4a | `src/constants.rs` | (H) 因子を全て名前付き定数として追加 |
| 3.4b | `src/simulation.rs` | offer_help_probability, advance_help_sessions のハードコード値を定数参照に置換 |
| 3.4c | `src/event.rs` | 対応する Default 実装に各定数マッピングを追加 |

### 3.5 G5: 成熟・成長系

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.5a | `src/constants.rs` | (H) 因子（child/adult trust init 等）を定数化 |
| 3.5b | `src/simulation.rs` | generate_population のハードコード値を定数参照に置換 |

### 3.6 G6: ネットワーク構造系

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.6a | `src/kind_world.rs` | 全 (S) 因子を All94Params に追加（多くは既に 7 パラメーターに含まれる） |

### 3.7 G7: システム枠組み系

| 変更 | ファイル | 内容 |
|------|---------|------|
| 3.7a | `src/constants.rs` | 残り (H) 因子の定数化 |
| 3.7b | `src/simulation.rs` | 対応するハードコード値の置換 |

---

## 4. 進行予測

| グループ | 推定サイクル数 | 累計 | 備考 |
|----------|---------------|------|------|
| G1 | 2-4 | 2-4 | search_radius_inverse 実装が鍵 |
| G1+G2 | 2-3 | 4-7 | GC 系は既 tuning 済みで改善少ない可能性 |
| G1+G2+G3 | 2-3 | 6-10 | lifecycle 系も大半既 tuning |
| +G4 | 2-3 | 8-13 | (H) 定数化が初めて効く |
| +G5 | 1-2 | 9-15 | 初期化パラメーターは影響限定的 |
| +G6 | 2-3 | 11-18 | 数多いが影響は小 |
| +G7 | 1-2 | 12-20 | システム枠組み |

**最小: 8サイクル / 最大: 24サイクル**

---

## 5. 実験記録形式（experiments.md）

各サイクル完了時に以下を experiments.md に追記:

```markdown
## Cycle N: YYYY-MM-DD

### 設定
- 有効グループ: G1〜Gk
- パラメーター数: M
- 変更したパラメーター: [param名: 旧値→新値, ...]

### 結果
- パレートフロント件数: X
- 最良 J_kw_social: X.XXX
  - s_growth: X.XXX
  - s_density: X.XXX
  - s_topology: X.XXX
  - s_search: X.XXX
  - s_fairness: X.XXX
  - s_speed: X.XXX

### 判断
- 改善率（前サイクル比）: X.X%
- 3サイクル連続5%未満改善? Yes/No
- 次グループ追加? Yes（Gk+1）/ No

### 解釈
<平易な日本語での現象説明>
```

---

## 6. ユーザー報告計画

| タイミング | 内容 | 頻度 |
|-----------|------|------|
| 各グループ完了時（追加判断時） | グループの成果、改善率、次のグループ追加判断 | 最大7回 |
| 重要発見時 | 予想外の因子間相互作用、stub 発見など | 適宜 |
| 最終報告（全完了 or 上限到達） | 全94因子の影響度ランキング、パレートフロント、示唆 | 1回 |

**通常時は experiments.md への記録のみでユーザーへの報告は行わない。**

---

## 7. リスクと対策

| リスク | 影響 | 対策 |
|--------|------|------|
| 次元の呪い（20+ 次元で探索効率激減） | 収束不能 | 逐次追加戦略で軽減。20次元超えたら Bayesian Optimization の trial 数を増やす |
| 計算コスト（trial 数 × パラメーター数） | 1サイクルに数時間 | まずは trial=50 から開始、必要に応じて 100, 200 と増加 |
| G1 の search_radius_inverse stub | 最適化対象 not 実装 | G1 着手時に実装。実装できない場合は当該因子を除外して進行 |
| remote_exploration 未実装部分 | G1 の 4 因子が実質 tuning 不可 | パラメーターは受け付けるが、計算パスが通っていない場合は無視して進行 |
| (H) の定数化漏れ | G4/G5/G7 で探索不能 | 各グループ着手時にグローバル grep で全ハードコード値を洗い出す |
