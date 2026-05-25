# M1.76-6: GC hazard with benevolence (F-7, F-8, F-9) — 実装計画

## RFC 既存実装状態検証

### RFC §15.10.4 — 式 F-7 / F-8 / F-9

当該 RFC セクションは **構造体・enum を定義せず、純粋な数式定義のみ**。代わりに式 F-7 が要求するパラメータの現行コード対応を検証:

| パラメータ | RFC 記号 | 現行フィールド | 型 | デフォルト値 | 状態 |
|---|---|---|---|---|---|
| ベースラインハザード | λ₀ | `lambda_gc_base` | f32 | `0.1` (生値) | ⚠️ 生値ハードコード |
| LifecycleScore 重み | γ_L | `gamma_lifecycle` | f32 | `0.5` (生値) | ⚠️ 生値ハードコード |
| Benevolence 重み | γ_B | `gamma_benevolence` | f32 | `0.10` (定数参照) | ✅ 一致 |
| Child protect 重み | γ_C | `gamma_child_protect` | f32 | `0.20` (定数参照) | ✅ 一致 |

**評価サマリ**: 全 4 パラメータともフィールドは存在し型も正しい。`lambda_gc_base` と `gamma_lifecycle` のデフォルト値が生値ハードコード。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 追加 | `GC_HAZARD_LAMBDA_0`, `GC_HAZARD_GAMMA_LIFECYCLE` の 2 定数 |
| `src/event.rs` | 修正 | `ReciprocityLifecyclePolicy` デフォルト値の定数参照化 (2 行) |
| `src/reciprocity.rs` | 追加 | `softplus`, `compute_gc_hazard`, `compute_gc_probability`, `compute_survival_probability` + テスト 8 件 |

## 実装手順

### Step 1: constants.rs に定数追加
### Step 2: event.rs のデフォルト値修正
### Step 3: reciprocity.rs に関数追加
### Step 4: テスト追加 (TC-1〜TC-8)

## 物理的レビュー方法

1. `run-quality-checks.js` で変更ファイルをチェック
2. `cargo test` 全テスト通過
3. `cargo test -- --nocapture` で計装出力確認
4. 翻訳可能性 grep（名詞始まり関数、1文字変数、マジックナンバー）
5. RFC §15.10.4 との無矛盾確認
