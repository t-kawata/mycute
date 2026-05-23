# 実装サマリー: M0.5-3 パッチ適用における未解決変数（VarScopeViolation）の確率的検出テスト

## 変更したファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/patch.rs` | 追加 | `compute_validator_score(E_v: usize) -> f32` — RFC §14.3 c_v 計算関数。減算規則: c_v = clamp(1.0 - 0.15*min(E_v, 3), 0.0, 1.0) |
| `src/lib.rs` | 変更 | re-export 行に `compute_validator_score` を追加 |
| `tests/m0_5.rs` | 追加 | OTS-V1, OTS-V2, OTS-V3, OTS-VS の 4 テスト関数を追加 |

## 観測テスト結果

| テスト | n | 結果 | 観測内容 |
|---|---|---|---|
| OTS-V1 | 11 | PASS | E_v=0..10 全点で c_v が RFC 値と完全一致、単調非増加性確認 |
| OTS-V2 | 10,000 | PASS | 同一 (c_s, E_v) での分散 = 0（決定論性確認）。c_s 水準別の差分商を観測 |
| OTS-V3 | 1,100 | PASS | c_s=0.50 で -3.97% の不連続ジャンプ観測（重み切り替えの相転移） |
| OTS-VS | 500 | PASS | 499/499 の変数スコープ違反を完全検出、見逃し 0 |

## 検証結果
- `cargo test`: 全テスト通過（既存含む）
- 警告: 0
- 翻訳可能性: 関数名は動詞句（`compute_validator_score`）、テスト関数名は観測対象を明示
- RFC 無矛盾: `compute_validator_score` は RFC §14.3 減算規則と完全一致
