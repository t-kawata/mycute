---
experiment_cycle: 0
experiment_count: 1
latest_experiment_id: kw4-1779783441-001
---

# 実験ログ: M1.76-KW4 Kind World 較正ループ

## 実験系列管理

| サイクル | 実験 ID | J_kw | 収束 | フラグ | 変更点 | 日時 |
|---------|---------|------|------|--------|--------|------|
| 0 | kw4-1779783441-001 | 0.2107 | ✅ | 4/8 | ticks=100, γ=0.30, cr=0.40, st=0.30 | 2026-05-26T17:18 |

## 実験詳細

### Experiment 1 (kw4-1779783441-001)

**仮説**: シミュレーション長を 20→100 tick に延長し、慈悲重みと子比率を高めることで、人口成長・能力カバレッジ・村間相互作用が発現する。

**変更定数**:
- `KW4_SIMULATION_TICKS`: 20 → 100
- `KW4_INITIAL_GAMMA_BENEVOLENCE`: 0.15 → 0.30
- `KW4_INITIAL_CHILD_RATIO`: 0.30 → 0.40
- `KW4_INITIAL_SOFTMAX_TEMPERATURE`: 0.50 → 0.30

**結果**:
- J_kw = 0.2107 (baseline 0.2112 とほぼ同じ)
- 収束 iteration: 3 (非常に高速)
- フラグ: 4/8 (変化なし)

**最良パラメータ**:
| パラメータ | 値 |
|-----------|-----|
| gamma_benevolence | 0.306 |
| lambda_gc_base | 1.061 |
| direct_reciprocity_weight | 0.405 |
| indirect_reciprocity_weight | 0.305 |
| softmax_temperature | 0.335 |
| gc_interval | 3 |
| child_ratio | 0.390 |

**解釈**: シミュレーション長を 5 倍にしても J_kw がまったく向上しなかった。これは、システムの根本的な制約が tick 数ではなく、シミュレーションモデルの構造にあることを示唆している。次の実験では別の仮説を検証する必要がある。
