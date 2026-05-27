# 計画: #131 FIX-D help_successes 二重処理バグ修正

## 要件の再確認

- **修正内容**: `run_kw_real_simulation` と `run_evaluation_simulation` の両方で、`help_successes` 累積変数を削除し、`phase5_capability_diffusion` に `new_successes`（当 tick 分のみ）を直接渡す
- **修正方針**: Option A（変数分割）

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/simulation.rs` | 修正 | 2 箇所の help_successes 二重処理を修正 + FIX-D1〜D4 テスト追加 |

## RFC 既存実装状態検証

参照セクション: RFC §41C（シミュレーション実行）、§13（capability diffusion）

- RFC に型定義の不一致は存在しない（本チケットは実装バグ修正であり RFC 無矛盾）
- 修正後の挙動「各 tick の HELP 成功のみを該当 tick で処理」は RFC の設計意図と完全に一致
- ✅ 乖離なし

## 計装・観測の実装計画

| ID | テスト関数 | 種別 | 実装内容 |
|----|-----------|------|---------|
| D1 | `test_fixd_single_help_single_exp` | 不変条件 | `fixd_kw_config()` でシミュレーション実行後、全ノードの experience == 1 を `assert!` |
| D2 | `test_fixd_two_ticks_separate_exp` | 不変条件 | 2 tick 連続 HELP 後の各 experience == 1 を `assert!` |
| D3 | `test_fixd_avg_exp_decrease` | 観測 | `println!` で平均経験値を出力。`--nocapture` で確認 |
| D4 | `test_fixd_existing_tests_pass` | 不変条件 | `cargo test` 全 PASS |

- 固定シード: `StdRng::seed_from_u64(12345)`
- 較正要因なし（純粋バグ修正）

## Boy Scout 改善

- `help_successes` 累積変数の削除により、翻訳不可能な累積パターンを排除
- 2 箇所とも同一パターンで修正

## 実装手順

1. `run_kw_real_simulation`（line 1266-1324）: help_successes 変数削除、phase5 に `&new_successes` を直接渡す
2. `run_evaluation_simulation`（line 1505-1567）: 同一修正
3. FIX-D1〜D4 テスト追加
4. `cargo build` → `cargo test` → `cargo clippy`

## 物理的レビュー方法

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/simulation.rs | node "$_R/scripts/tickets/review/generate-report.js"
```

翻訳可能性 grep:
```bash
grep -n "help_successes" src/simulation.rs | grep -v "new_successes" | grep -v "//" | grep -v "#\["
```

## リスク

- **低**: 影響範囲は 2 つのシミュレーション関数内の phase5 呼び出しのみ
- `run_simulation` は無影響
