# 実装サマリ（改訂版）: 段階的人口制御（比例制御 + 指数平滑化）

## 変更点（初版からの差分）

### 段階的比例制御に変更
- **削除フィールド**: target_hysteresis, pressure_lambda_high/low, pressure_gamma_child_low/high
- **追加フィールド**: pressure_ramp_range(50), pressure_ramp_up_ticks(10), pressure_ramp_down_ticks(20), pressure_lambda_min(1.0), pressure_lambda_max(3.0), pressure_gamma_child_min(2.0), pressure_gamma_child_max(8.0)

### 指数平滑化
- `SimulationContext.current_pressure: f64` を追加
- 上昇/下降で異なる時定数（ramp_up_ticks / ramp_down_ticks）
- 平滑化式: `pressure += (target - pressure) * (1 - exp(-1/ramp_ticks))`
- ramp_ticks=0 で即時応答（平滑化なし）

### テスト TC1〜TC6（書き換え）
- TC1: None → pressure リセット確認
- TC2: below target → pressure=0 → lambda=L_min
- TC3: max overshoot → pressure≈1.0 → lambda≈L_max
- TC4: 中間 overshoot → pressure≈0.5 → lambda≈2.0（比例動作確認）
- TC5: 平滑化の収束確認（複数回呼出で漸近）
- TC6: ramp_ticks=0 → 即時応答確認

### server.rs / フロントエンド
- 変更なし（target_population の配線は初版から継続）
- index.html のスライダーデフォルト値は 1000 にユーザー変更済み

## 検証結果
- cargo test --features server → 1390 passed, 0 failed, 73 ignored
- TC1-TC6 → 6/6 PASS
