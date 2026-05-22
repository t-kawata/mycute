# 実装サマリ — M-1.5-3 SearchPolicyOscillation

## 変更したファイル

### src/constants.rs
- `OSCILLATION_MAX_COUNT: u32 = 3` を追加 (Calibration Candidate)

### src/types.rs
- `TerminalTransitionReason::OscillationDetected` バリアント追加
- `OscillationDetector` 構造体実装:
  - `new(max_oscillation_count)` / `default()` — コンストラクタ
  - `record_transition(to)` — 位相ベース交互遷移カウンタ
  - `is_oscillating()` — 閾値超過判定
  - `reset()` — カウンタリセット
  - `oscillation_count()` / `max_oscillation_count()` — アクセサ
- `attempt_transition(state, detector, next)` ヘルパー関数
- テスト追加: T1(発振検出基本3ケース) + T2(リセット3ケース) + T3(統合3ケース) + T4(can_terminate_with) + T5(飽和安全性) + OTS-1(検出マトリクス) + OTS-2(レイテンシ)
- 既存 OTS-3 テスト更新 (OscillationDetected を列挙に追加)

### src/lib.rs
- `OscillationDetector` を公開 API として追加

## 発振検出アルゴリズム
位相ベース交互遷移カウンタ方式:
- `expected_next` に期待する遷移先を保持
- 期待と一致 → `oscillation_count` を saturated 加算
- 不一致 → リセット
- `oscillation_count >= max_oscillation_count` → `is_oscillating() = true`

## 検証結果
- `cargo test`: 192 tests PASS (0 failed)
- `cargo clippy`: 要確認 (既存コードの警告のみ)
- 新規テスト: 14 M-1.5-3 テスト + 2 更新テスト (OTS-3)
