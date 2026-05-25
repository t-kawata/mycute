# 実装サマリ: M1.75-10 property-based village invariant fuzzing

## 変更ファイル

### src/constants.rs
- `PROPTEST_DEFAULT_CASES: u32 = 10_000` — proptest 反復回数 (Environment Policy Knob)
- `VILLAGE_FIXTURE_DIR: &str` — failing seed fixture 出力先 (Environment Policy Knob)

### src/replay.rs (mod tests 内)
- 5 種の proptest strategy (`maturity_strategy`, `consistency_state_tag_strategy`, `adult_candidate_strategy`, `adult_candidate_list_strategy`, `help_state_strategy`)
- 7 件の新規テスト (F-1〜F-7):
  - F-1: `f1_prop_helper_assignment` — helper 数が top_k を超えないこと
  - F-2: `f2_prop_consistency_state_filter` — 非 Committed 候補が 100% フィルタされること
  - F-3: `f3_prop_help_terminal_non_reentrance` — 終端状態からの再遷移が禁止されること
  - F-4: `f4_prop_empty_village_fallback` — 全 Adult 非 Committed 時に空の LocalVillage が返ること
  - F-5: `f5_prop_maturity_classification` — 全軸非負入力で panic しないこと
  - F-6: `f6_prop_fixture_export_roundtrip` — FailingSeedEntry の JSON ラウンドトリップ
  - F-7: `f7_prop_fixture_replay_regression` — fixture ディレクトリ作成と読み込み検証
- `FailingSeedEntry` 型 — JSON 保存用の fixture データ構造

### tests/fixtures/village_invariant_failures/
- 初回テスト実行時に自動生成される fixture 保存ディレクトリ

## テスト結果
- 全 897 テスト PASS (7 new + 890 existing)
- 既存テストへの回帰なし
