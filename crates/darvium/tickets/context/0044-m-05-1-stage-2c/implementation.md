# 実装サマリー: M-0.5-1 メモリ内デュアルストア候補抽出及び統合・重複排除器（Stage 2c）の検証

## 変更したファイル

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/store/mod.rs` | 追加 | `merge_and_deduplicate_candidates` 関数 + 9件のユニットテスト + 2件の観測テスト |
| `src/lib.rs` | 変更 | `pub use store::merge_and_deduplicate_candidates;` を追加 |

## 実装内容

### `merge_and_deduplicate_candidates`
- シグネチャ: `(semantic: Vec<RankedCandidate>, structural: Vec<RankedCandidate>) -> Vec<RankedCandidate>`
- 重複排除キー: `workflow_id`（HashMap でグループ化）
- 重複時のスコア選択: 高い方の `blended_score` を残す（最大値保存則）
- provenance: 両ストア由来を連結（重複除去あり）
- 順序: セマンティック側の元の順序を維持し、構造側の新規候補を後方に追加
- 非破壊的: 入力リストは clone により変更されない

### テスト
- T1: 重複なしマージ（5件→5件）
- T2: 重複排除 セマンティック側高スコア（0.9→0.9）
- T3: 重複排除 構造側高スコア（0.6→0.8）
- T4: 重複排除 スコア同値（0.75→0.75）
- T5: 両方空（空→空）
- T6a/b: 片方のみ空（3件→3件）
- T7: provenance 連結検証（semantic-v1 + struct-v1）
- T8: 大量候補マージ（2,000件→1,500件）
- T9: 入力非破壊性検証
- OTS-1: カイ二乗検定（χ²=69.96, df=63, 一様性確認）
- OTS-2: 最大値保存則（10,000組、保存率100%）

## 検証結果
- `cargo test`: 275 passed, 0 failed
- `cargo clippy -- -D warnings`: 警告なし
- `cargo fmt`: フォーマット済み
