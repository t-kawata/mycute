# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/constants.rs` | 編集 | 信頼度/提案器関連の定数9件を追加（`CONFIDENCE_C_S_WEIGHT` 等） |
| `src/types.rs` | 編集 | `ConfidenceVector` 構造体を追加（`new`, `uniform`, `aggregate`, `perturb`） |
| `src/search/mod.rs` | 編集 | `pub mod mock_proposer;` を追加 |
| `src/lib.rs` | 編集 | `ConfidenceVector`, `MockProposer`, `CompositionDecision`, `decide_composition_fate` の再公開 |
| `src/search/mock_proposer.rs` | 新規 | PRNG 駆動型擬似提案器の全文実装（424行） |

## 実装内容の概要

### モジュール構造

- **`MockProposer`** (`src/search/mock_proposer.rs`): `StdRng` を内包し、決定論的再現可能な3次元信頼度ベクトルを生成する。
  - `new()` / `from_seed(u64)` / `set_seed(u64)`: 構築・リセット
  - `generate_confidence()` → `ConfidenceVector`: 一様分布から c_s, c_v, c_h を生成
  - `last_confidence()` / `generation_count()`: 状態アクセサ

- **`ConfidenceVector`** (`src/types.rs`): 3次元信頼度ベクトル C = (c_s, c_v, c_h)。
  - `new(c_s, c_v, c_h)`: 各成分を [0,1] にクランプ
  - `aggregate()`: 重み付き線形結合 (w_s·c_s + w_v·c_v + w_h·c_h)
  - `perturb(delta, rng)`: ツイン軌道用微小摂動

- **`CompositionDecision`** enum: `Refine { reason }` / `Finalize { reason }` / `Uncertain { reason }`
- **`decide_composition_fate(cv)`**: C_agg に基づく分岐決定
- **`compute_lyapunov_exponent(ref, pert, δC0, t)`**: ツイン軌道リアプノフ指数計算

### テスト

| グループ | テスト数 | 内容 |
|----------|---------|------|
| T1 | 2 | コンストラクション検証 |
| T2 | 3 | クランプ動作検証 |
| T3 | 3 | aggregate 計算検証 |
| T4 | 2 | 値域不変条件 |
| T5 | 2 | 決定論的再現性 |
| T6 | 1 | 異種シード多様性 |
| T7 | 1 | 高信頼度→Finalize |
| T8 | 1 | 低信頼度→Refine |
| T9 | 1 | 中信頼度→Uncertain |
| T10 | 3 | perturb 不変条件 |
| T11 | 2 | set_seed リセット |
| L1-L3 | 3 | リアプノフ指数 |
| OTS-1 | 1 | 5,000回分布シミュレーション |
| OTS-2 | 1 | 1,000ステップツイン軌道 |
| OTS-3 | 1 | 重みスイープ (0.20→0.60, 0.01step) |

### ドキュメンテーション

- 全公開アイテムに rustdoc コメント + 使用例
- ドキュメンテーションテスト（Doc-test）1件通過確認
- `decide_composition_fate` / `compute_lyapunov_exponent` に数式付き説明
