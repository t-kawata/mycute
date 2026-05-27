# Phase 2 実験記録

---

## Cycle 1: G1 Bayesian Pareto 初回探索

### 設定
- **日付**: 2026-05-27
- **有効グループ**: G1（検索・探索系 14 パラメーター）
- **パラメーター数**: 14（うち simulation 経路結合済み: 2、スタブ: 12）
- **最適化手法**: MotpeSampler（50 trials, 6 objectives maximize）
- **シード**: eval=12345, sampler=42
- **live パラメーター**: G1_EVALUATION_POPULATION_SIZE, G1_SIMULATION_TICKS
- **スタブ**: REMOTE_EXPLORE_INTERVAL/DECAY/STEPS/REWARD, SEARCH_TICK_FRACTION, EVALUATE_FRACTION, RECIPROCITY_ALPHA_HELP/RECEIVE/RANK/OFFER, SEARCH_RADIUS_INVERSE, REMOTE_EXPLORE_HUMAN_WEIGHT

### 結果
- **パレートフロント件数**: 45/50
- **最良トレードオフ範囲**:
  - s_growth: 0.217〜0.701
  - s_density: 0.682〜0.699
  - s_topology: 0.497〜0.531
  - s_search: 0.345〜0.390
  - s_fairness: 0.997〜1.000
  - s_speed: 0.500〜0.988

### 判断
- **改善率（初回のため基準なし）**: N/A
- **3サイクル連続5%未満改善?**: 初回につき No
- **次グループ追加?**: No（スタブ実装が先）

### 解釈

G1 の 14 パラメーター中、実際にシミュレーションに影響を与えているのは population_size と max_ticks の 2 つだけであった。この 2 つの間には明確なトレードオフがある：人口を増やしたり tick 数を増やすと s_growth は向上するが s_speed は低下する（収束に時間がかかる）。逆に人口を減らすと高速収束するが成長率が犠牲になる。

ただし s_search 自体は 0.345〜0.390 の狭い範囲に張り付いており、真の「検索性能」を最適化できていない。これは search_radius_inverse が常に 0.5 を返すスタブであること、RECIPROCITY_ALPHA_* が constants.rs 直参照でチューニング経路に乗っていないことによる。

**結論**: G1 の真の効果を発揮するには、まず `compute_search_radius_inverse()` の実装と RECIPROCITY_ALPHA_* の config 経路化が必要。これを次のサイクルで行う。

---

## Cycle 2: G1 Bayesian Pareto 再探索（ALPHA 配線 + search_radius 実装後）

### 設定
- **日付**: 2026-05-27
- **有効グループ**: G1（検索・探索系 14 パラメーター）
- **パラメーター数**: 14（うち simulation 経路結合済み: 6、スタブ: 8）
- **最適化手法**: MotpeSampler（50 trials, 6 objectives maximize）
- **シード**: eval=12345, sampler=42
- **新規生きたパラメーター**: RECIPROCITY_ALPHA_HELP/SUCCESS/REJECT/HARM（4値）
- **実装したスタブ**: compute_search_radius_inverse（help session の L2 距離ベース）
- **残スタブ**: REMOTE_EXPLORE_INTERVAL/DECAY/STEPS/REWARD, SEARCH_TICK_FRACTION, EVALUATE_FRACTION, SEARCH_RADIUS_INVERSE（help session 不在により実質 0.5 固定）, REMOTE_EXPLORE_HUMAN_WEIGHT

### 結果
- **パレートフロント件数**: 45/50
- **最良トレードオフ範囲**:
  - s_growth: 0.216〜0.701（Cycle 1 比: 変化なし）
  - s_density: 0.681〜0.699（Cycle 1 比: 変化なし）
  - s_topology: 0.497〜0.537（Cycle 1 比: わずかに拡大 +0.006）
  - s_search: 0.372〜0.422（Cycle 1 比: +0.027〜+0.032、**8%改善**）
  - s_fairness: 0.993〜1.000（Cycle 1 比: 変化なし）
  - s_speed: 0.500〜0.988（Cycle 1 比: 変化なし）
  - J_kw_social: 0.030〜0.067（Cycle 1 比: 変化なし）

### 判断
- **改善率（前サイクル比）**: s_search のみ 8% 改善。他 5 指標は <5%。全体として marginal improvement。
- **3サイクル連続5%未満改善?**: 1 サイクル目（今回が初の比較可能）
- **次グループ追加?**: まだ。G2 パラメーターを追加して G1+G2 探索を開始する。

### 分析

**ALPHA 値の動作確認**: CSV 出力により alpha_help=0.1〜4.8、alpha_success=0.1〜4.9 など、実際に最適化器が ALPHA 4 値を広範囲に変動させていることを確認。これは ReciprocityLifecyclePolicy → to_sim_config_g1 → compute_direct_reciprocity の経路が正しく結合された証拠である。

**s_search の改善**: Cycle 1 の 0.345〜0.390 から 0.372〜0.422 へと 8% 改善した。これは以下の要因が考えられる：
1. ALPHA 値の変化が間接的に agent 間の相互作用パターンを変え、結果的にエントロピーを変化させた
2. compute_search_radius_inverse が一部の help session で正しく距離計算を行った

  → しかし J_kw_social 全体への影響は限定的。J_kw が常に 0 である根本問題は解決していない。

**残課題**: 8 個のスタブは simulation コードに存在しないパラメーター（REMOTE_EXPLORE_* は遠隔探索そのものが未実装、SEARCH_TICK_FRACTION/EVALUATE_FRACTION は該当機能なし）であり、これらを完全に結合するには simulation.rs への大規模なコード追加が必要。

**次ステップ方針**: 既存の ReciprocityLifecyclePolicy に定義済みで reciprocity.rs が実際に参照している G2 パラメーター（gamma_lifecycle, gamma_child_protect, kappa_e）を AllParams に追加し、G1+G2 探索を開始する。

---

## Cycle 3: G1+G2 Bayesian Pareto 探索（G2 初回追加）

### 設定
- **日付**: 2026-05-27
- **有効グループ**: G1 + G2（GC・生存系）
- **パラメーター数**: 17（G1 14 + G2 3、うち live: 9、スタブ: 8）
- **最適化手法**: MotpeSampler（30 trials, 6 objectives maximize）
- **シード**: eval=12345, sampler=42
- **新規生きた G2 パラメーター**: gamma_lifecycle（デフォルト 1.0, 範囲 0.0-5.0）、gamma_child_protect（デフォルト 8.0, 範囲 0.0-20.0）、kappa_e（デフォルト 0.01, 範囲 0.001-1.0）

### 結果
- **パレートフロント件数**: 25/30
- **最良トレードオフ範囲**:
  - s_growth: 0.188〜0.793（Cycle 2 比: +13%上限向上、**有意改善**）
  - s_density: 0.675〜0.699（Cycle 2 比: やや下限低下）
  - s_topology: 0.450〜0.522（Cycle 2 比: 下限低下）
  - s_search: 0.317〜0.426（Cycle 2 比: やや拡大）
  - s_fairness: 0.973〜1.000（Cycle 2 比: 変化なし）
  - s_speed: 0.500〜0.990（Cycle 2 比: 変化なし）
  - J_kw_social: 0.018〜0.083（Cycle 2 比: 上限向上）

### 判断
- **改善率（前サイクル比）**: s_growth 上限が 13% 改善。**5% 閾値を超える有意改善。**
- **3サイクル連続5%未満改善?**: No（1回目の改善幅超過によりカウントリセット）
- **次グループ追加?**: No。G1+G2 をさらに探索（trial 数を増やして精密化）。

### 分析

**G2 パラメーターの効果確認**: CSV 出力により、追加した 3 パラメーター全てが最適化器によって実際に変動していることを確認：
- gamma_lifecycle: 0.000〜4.992
- gamma_child_protect: 0.516〜20.0（上限に到達する trial あり）
- kappa_e: 0.001〜0.874

**s_growth 改善のメカニズム**: 高成長 trial（s_growth > 0.75）に共通するパラメーターパターン：
- gamma_lifecycle ≈ 3.0（デフォルト 1.0 の 3 倍）→ ライフサイクルスコアへの依存度増大
- kappa_e ≈ 0.001（デフォルト 0.01 の 1/10）→ 経験値正規化の飽和を促進、エージェント間の経験値差を縮小
- gamma_child_protect ≈ 8.0〜9.0（デフォルト 8.0 と同程度）→ 子供保護は適正範囲

この組み合わせは GC hazard 計算（reciprocity.rs:310-313）のバランスを変え、経験値の低いエージェントでも生存しやすくなることで人口成長を促進していると推定される。

**s_topology の下限低下**: G2 追加により topology の下限が 0.497→0.450 に低下。これは gamma_child_protect の高値設定が子供ノードを過剰保護し、ネットワーク構造を歪めている可能性を示唆する。

**次ステップ**: 50 trials で再実行し、30 trials で見られた傾向が安定しているか確認する。
