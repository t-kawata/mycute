---
ticket_id: 132
title: M1.76-KW-WIRE-B: help_session ID フォーマット統一 — compute_search_radius_inverse の実測値化
slug: m176-kw-wire-b-help-session-id-compute-search-radius-inverse
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/observation-20260528-072213.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/review.md
---
# M1.76-KW-WIRE-B: help_session ID フォーマット統一 — compute_search_radius_inverse の実測値化

## Summary

`compute_search_radius_inverse()`（kind_world.rs:2244）が常に 0.5 を返す永久スタブである問題を修正する。原因は、同関数内のパースロジック `s.strip_prefix('n').and_then(|r| r.parse().ok())` が `"n<数字>"` 形式のみを前提としており、実際の production 環境の WorkflowGraphId（`"adult-1"`, `"child-1"`, `"a1"`, `"c1"` 等）および simulation 環境の ID（`"wf-child-3"`, `"wf-adult-7"`, `"session-42"` 等）のいずれにもマッチしないため。本チケットではこのパースロジックを拡張し、全 ID フォーマットからノード番号を抽出できるようにすることで、j_search_radius_inv を実測値ベースに変更する。

## Background

RFC §15.9.2 において、j_search_radius_inv は「HELP セッション参加者間の平均 L2 距離の逆数」と定義されている。Phase 2 較正実験（experiments.md 参照）では、G1 パラメーター 14 個中 8 個がスタブであることが確認され、特に SEARCH_RADIUS_INVERSE は関数本体が完全にパラメーターを無視して 0.5 を返すスタブであった。このため s_search 因子の 25%（4 成分中の 1 つ）が仮値に固定され、真の検索性能の最適化が妨げられている。本チケットは WIRE 系列の最初の実装として、production コードの修正のみで simulation の較正精度を向上させる。

## Scope

1. `compute_search_radius_inverse()` のパースロジックを拡張し、`"n<数字>"` だけでなく実在する ID フォーマット全てに対応させる
2. パース失敗時のフォールバック戦略の追加
3. 引数シグネチャの整理（positions を引数として明示的に受け取る）
4. AllParams の G1_SEARCH_RADIUS_INVERSE を active=false に変更（関数がパラメーターを無視するため経路が無効）
5. テスト B1-B7 の追加

## Non-scope

- simulation.rs の session ID 採番ロジックの変更（シミュレーション内 ID もパースできるようにするのが本チケットの目的であり、採番方式は変更しない）
- 他のスタブ（REMOTE_EXPLORE_* x4, SEARCH_TICK_FRACTION, EVALUATE_FRACTION, REMOTE_EXPLORE_HUMAN_WEIGHT）の実装
- compute_search_radius_inverse 以外の collect_final_metrics パスの変更

## Investigation

### 発見 1: compute_search_radius_inverse のパースロジック（kind_world.rs:2244-2292）

```rust
fn compute_search_radius_inverse(
    sessions: &[crate::help::HelpSession],
    positions: &std::collections::HashMap<
        crate::types::NodeId,
        crate::spaceposition::SpacePositionEmbedding,
    >,
) -> f64 {
    if sessions.is_empty() {
        return 0.5;
    }
    // "n<数字>" → NodeId にパース
    let parse_nid = |s: &str| -> Option<crate::types::NodeId> {
        s.strip_prefix('n').and_then(|r| r.parse().ok())
    };
    let mut total_distance = 0.0f64;
    let mut counted = 0usize;
    for session in sessions {
        let from_id = match parse_nid(&session.from_workflow) {
            Some(id) => id,
            None => continue, // ★ マッチしないとスキップ
        };
        let to_id = match parse_nid(&session.to_workflow) {
            Some(id) => id,
            None => continue, // ★ マッチしないとスキップ
        };
        // ... L2 距離計算 ...
    }
    if counted == 0 {
        0.5  // ★ 全セッションスキップ = 常に 0.5
    } else {
        let mean_distance = total_distance / counted as f64;
        1.0 / (1.0 + mean_distance)
    }
}
```

`parse_nid` は `s.strip_prefix('n')` で先頭の "n" のみを削除する。このため以下は全てノーマッチとなる：
- `"adult-1"`（production HelpSession の from_workflow）
- `"child-1"`（同上、to_workflow）
- `"a1"`（テストコード help.rs:996-997）
- `"wf-child-3"`（simulation.rs:460 の SimWorkflowState.id）
- `"session-42"`（simulation.rs:563 の SimHelpSession.id）

### 発見 2: production HelpSession の from_workflow/to_workflow 形式（help.rs:247-256）

```rust
pub struct HelpSession {
    pub help_id: String,
    pub from_workflow: WorkflowGraphId,  // "adult-1" 等
    pub to_workflow: WorkflowGraphId,   // "child-1" 等
    pub current_state: HelpState,
}
```

`WorkflowGraphId` は `pub type WorkflowGraphId = String;`（types.rs:19）。production 環境では `"adult-1"`, `"child-1"` 形式が使用される。

### 発見 3: シミュレーション内の ID 形式（simulation.rs:460, 479, 492, 563）

```rust
// SimWorkflowState の ID
id: format!("wf-child-{}", i),    // line 460
id: format!("wf-adult-{}", i),    // line 479（大人セクション）

// SimHelpSession の ID
id: format!("session-{}", session_counter),  // line 563
```

シミュレーション内では **3 種類**の ID フォーマットが混在している。

### 発見 4: 呼び出し箇所（kind_world.rs:2854）

```rust
let search_radius_inverse = compute_search_radius_inverse(&ctx.help_sessions, positions);
```

`ctx.help_sessions` は `Vec<crate::help::HelpSession>`。production 環境では HelpSession が実際に生成されるが、ID が `"n<数字>"` 形式でないため常に 0.5 が返る。シミュレーション内では HELP セッションが生成されない（Phase 2 実験の experiments.md に記録）ため、`sessions.is_empty()` で早期 return して 0.5。

### 発見 5: AllParams 結合の無意味化（kind_world.rs:342-343, 7822-7824）

```rust
// G1_SEARCH_RADIUS_INVERSE: AllParams の 12 番エントリ
pub const G1_SEARCH_RADIUS_INVERSE: usize = 12;
```

AllParams::default_g1() に SEARCH_RADIUS_INVERSE が定義されているが、関数本体がパラメーターを完全に無視しているため、Bayesian 最適化器がこのパラメーターを変動させても J_kw に一切の変化が生じない。Remove は行わないが active=false に設定し、将来の生産コード変更で再有効化できるようにする。

## Test Plan

### B1: `"n123"` 形式の正しいパース
`"n42"` を NodeId(42) に変換できること。

### B2: `"wf-child-5"` 形式の正しいパース
シミュレーション ID `"wf-child-5"` を NodeId(5) に変換できること。

### B3: `"session-42"` 形式の正しいパース
シミュレーションセッション ID `"session-42"` を NodeId(42) に変換できること。

### B4: 全セッション同一位置 → L2 距離 0 → 逆数 1.0
2 セッションの from/to が同一位置を持つ場合、`total_distance=0`、`counted=2`、`mean_distance=0`、`1.0/(1.0+0) = 1.0` を返すこと。

### B5: HELP セッション不在 → 0.5
空スライスを渡した場合、早期 return で 0.5 を返すこと。

### B6: パース失敗セッションのスキップ
3 セッション中 1 つだけパース成功する場合、その 1 つの距離のみから計算され、失敗した 2 つは無視されること。

### B7: 既存テスト全 PASS
修正後も既存のテストスイートが全て通過すること。

## 計装方法・観測対象

### 計装方法
- kind_world.rs 内の `mod tests` に B1-B6 のユニットテストを追加
- B1-B6 は固定値の assert テスト（観測不要、PASS/FAIL のみ）
- B7 は `cargo test` で全テスト通過を確認

### 観測対象
本チケットは production コードの修正のみであり、較正ループは対象外。後続の WIRE-C/D/E 完了後に統合較正を行う。

### 較正計画
本チケット単独では較正しない。WIRE 全チケット完了後、M1.76-KW4 再開時に較正。

## Boy Scout Rule — 翻訳可能性計画

1. **`compute_search_radius_inverse` のパースロジック**: 現在の `strip_prefix('n')` は 1 種類の ID 形式にしか対応しておらず、関数名が約束する「探索半径逆数の実測値計算」と実装が乖離している。正規表現または複数パターンマッチで全 ID 形式に対応させることで、関数の動作と名称を一致させる。
2. **`parse_nid` クロージャ**: 関数内クロージャとして定義されており、テスト容易性が低い。テスト可能な関数に抽出することを検討する。
3. **AllParams の SEARCH_RADIUS_INVERSE**: active=false に変更し、コメントで「現在は実測値経路で計算」と理由を明記する。

## Acceptance Criteria

- [ ] B1: `"n42"` → NodeId(42) のパースが成功する
- [ ] B2: `"wf-child-5"` → NodeId(5) のパースが成功する
- [ ] B3: `"session-42"` → NodeId(42) のパースが成功する
- [ ] B4: 同一位置セッション → 逆数 1.0 を返す
- [ ] B5: 空セッション → 0.5 を返す
- [ ] B6: パース失敗セッションをスキップし残りから計算する
- [ ] B7: 既存テスト全 PASS
- [ ] 翻訳可能性の検証が通っている

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0132-m176-kw-wire-b-help-session-id-compute-search-radius-inverse/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
