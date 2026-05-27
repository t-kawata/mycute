# 変更したファイル一覧と実装内容の概要

## src/constants.rs
- 3 定数を追加: SIMULATION_CHILD_TRUST_MAX (0.3), SIMULATION_ADULT_TRUST_MIN (0.3), SIMULATION_BENEVOLENT_THRESHOLD (0.5)
- Calibration Candidates 分類で追加

## src/simulation.rs
### ReciprocitySimulatorConfig
- 3 フィールド追加: child_trust_max, adult_trust_min, benevolent_threshold (全て f64)
- Default impl 更新: 3 フィールドを constants.rs の値で初期化

### generate_population()
- 子供 trust: rng.random::<f32>() * 0.3 → rng.random::<f32>() * config.child_trust_max as f32
- 成人 trust: rng.random::<f32>() * 0.5 + 0.3 → rng.random::<f32>() * 0.5 + config.adult_trust_min as f32

### observe_tick()
- benevolent_threshold: f32 パラメーター追加
- ハードコード 0.5 → パラメーター benevolent_threshold を使用 (4 箇所)
- コールサイト: observe_tick(tick, &population) → observe_tick(tick, &population, config.benevolent_threshold as f32)

### テスト (C1-C5)
- C1: child_trust_max=0.0 で子供 trust 全員 0.0 を検証
- C2: adult_trust_min=1.0 で成人 trust 全員 1.0 を検証
- C3: benevolent_threshold=1.0 で benevolent_survival_rate=0.0 を検証
- C4: benevolent_threshold=0.0 で non_benevolent_survival_rate=0.0 を検証
- C5: 新規フィールド変更後も決定論的再現性を検証

## src/kind_world.rs
### AllParams G4 グループ
- G3_COUNT = 8 (WIRE-A 用予約、未実装)
- G4_COUNT = 3
- G4_CHILD_TRUST_MAX (= 25), G4_ADULT_TRUST_MIN (= 26), G4_BENEVOLENT_THRESHOLD (= 27)

### default_g1g2g4() 追加
- G1+G2+G4 デフォルト値で構築
- G3 は 0.0 (inactive) で埋める
- G4 値は constants.rs のデフォルトを参照

### to_sim_config_g1g2g4() 追加
- G1+G2+G4 の全値を config に伝播
- child_trust_max, adult_trust_min, benevolent_threshold を設定
