# 計画: チケット#139 — HELPイベント発行のプロダクション配線

## 要件の再確認

- SimulationContext に event_bus フィールド追加（注入可能）
- phase3_help_protocol の状態遷移を transition_to() 経由に置き換え
- Offered→Executing は 2段階遷移（Offered→Accepted→Executing）が必要（非合法遷移の回避）
- event_bus = None で既存動作を完全維持

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/simulation.rs | 修正 | SimulationContext に event_bus 追加、phase3_help_protocol の遷移置き換え |
| tickets/specs/0139-help.md | 修正 | [E1] の行番号・遷移記述を実際のコードに合わせて修正 |

## 計装・観測の実装計画

- テスト: simulation.rs 内の `#[cfg(test)]` モジュールに T1-T4 を追加
- --nocapture で観測出力取得
- 観測対象: イベント発行数・種類分布、遷移正当性
- 較正: 本チケットでは実施しない

## 実装手順

1. SimulationContext に event_bus を追加
2. phase3_help_protocol の 4 箇所の直接代入を transition_to に置き換え（Offered→Executing は 2段階）
3. spec Investigation の更新
4. cargo check 確認
5. テスト実装（T1-T4）
6. cargo test 全通過
7. 観察レポート保存

## 物理的レビュー方法

_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/simulation.rs tickets/specs/0139-help.md | node "$_R/scripts/tickets/review/generate-report.js"

## リスク

- 低: expect() による abort（合法遷移のみ使用するため実行パスでは絶対に発火しない）
- 低: 既存テストへの影響（event_bus: None で従来と等価動作）
- 低: 2段階遷移による Accepted 状態追加（既存コードで Accepted セッションは最終分岐で削除）
