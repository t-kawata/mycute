# 実装計画: M1.76-KW-WIRE-B

## 要件の再確認
- `compute_search_radius_inverse()`（kind_world.rs:2244）のパースロジックを production コードのみで修正
- パース成功 → L2 距離の実測値に基づく 0〜1 の値を返す
- パース失敗 → 従来通り 0.5（フォールバック）
- G1_SEARCH_RADIUS_INVERSE を active=false に設定
- B1-B7 テスト

## RFC 既存実装状態検証

### RFC §15.9.2 j_search_radius_inv
RFC 定義（4723行）:
> HELP セッションの探索距離の逆数（社会加速度定義④に対応）。現在のアーキテクチャでは WorkflowGraphId と NodeId の対応付けが行えないため 0.5 を返す暫定実装。

現行コード（kind_world.rs:2244-2292）:
- `compute_search_radius_inverse()` 関数は存在するが、`strip_prefix('n')` ロジックにより全 ID フォーマットがマッチせず常に 0.5 を返す

| 観点 | 状態 |
|------|------|
| 関数存在 | ✅ 実装済み |
| ID パースの汎用性 | ❌ "n<数字>" のみ対応（RFC は問題を認識済み） |
| 戻り値の意味 | ❌ 常に 0.5（RFC は暫定と明記） |

**評価サマリ**: RFC 自身が暫定実装を認識しており、本チケットはその修正。矛盾なし。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/kind_world.rs` | 修正 | `compute_search_radius_inverse()` パースロジック修正 + B1-B6 テスト追加 |
| `src/kind_world.rs` | 修正 | `AllParams::default_g1()` で `active[G1_SEARCH_RADIUS_INVERSE] = false` |
| `Darvium-RFC-0001-Unified-v2.3-final.md` | 修正 | RFC §15.9.2 の「暫定実装」但し書き削除 + 更新 |

## 実装手順

### Step 1: `compute_search_radius_inverse` パースロジック修正
`parse_nid` クロージャを複数パターン対応の関数に置き換え:
1. `"n<数字>"` → 従来形式（後方互換）
2. `"wf-child-<数字>"` / `"wf-adult-<数字>"` → シミュレーション ID
3. `"session-<数字>"` → シミュレーション HELP セッション
4. `"adult-<数字>"` / `"child-<数字>"` → production ID
5. 末尾数字抽出フォールバック

### Step 2: AllParams::default_g1() の active 設定
`params.active[G1_SEARCH_RADIUS_INVERSE] = false`

### Step 3: B1-B6 テスト追加
mod tests 内に固定値 assert テスト

### Step 4: cargo build + cargo test
### Step 5: RFC 更新

## Boy Scout 改善
- `parse_nid` クロージャ → テスト可能なスタンドアロン関数に抽出

## 計装・観測の実装計画
- B1-B6: 固定値 assert テスト（観測不要）
- B7: `cargo test` 全通過確認
- 較正ループは対象外

## 物理的レビュー
```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/kind_world.rs | \
  node "$_R/scripts/tickets/review/generate-report.js"
```

## リスク
- ID パースの過剰マッチ → 末尾アンカー + パターン優先順位で対策
- active=false による既存テストパラメーター数変化 → テストアサーション確認
