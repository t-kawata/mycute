# 実験ログ: M1.76-KW4-CAL: Kind World 較正継続（外側ループ 2-24）

## 実験カウンタ

| 実験 ID | 日付 | 親 | 変更点 | J_kw | 収束 | 備考 |
|---------|------|-----|--------|------|------|------|
| kw4-CAL-001 | 2026-05-27 | kw4-1779783441-001 | evaluate_single SimulationContext 移行 | 0.001522 | 1 iter | 全 20 指標計算（6 指標非ゼロ） |

| kw4-CAL-002 | 2026-05-27 | kw4-CAL-001 | epsilon 1e-6→1e-10, perturbation 0.05→0.10 | 0.002095 | 30 iter (NC) | 30 iter max reached, not converged |
