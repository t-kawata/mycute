# 計画: 人口安定化のための動的淘汰圧制御

## 要件
- 生存人口が目標値を超えたら淘汰圧（lambda_gc_base）を上げ、gamma_child_protect を下げる
- 目標を下回ったらデフォルトに戻す
- ヒステリシス帯で発振防止
- フロントエントスライダーから目標人口をリアルタイム変更可能

## 変更ファイル
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/simulation.rs | 追加 | Config 6フィールド追加、compute_adjusted_policy、SimulationParams拡張、全GC呼出に調整挿入 |
| src/server.rs | 変更 | SimCommand::UpdateTargetPop + WebSocketハンドラ |
| web/cube/observation/index.html | 変更 | 目標人口スライダー追加 |
| web/cube/observation/script.js | 変更 | syncSettingsToBackend + スライダーイベント |

## 実装手順
1. ReciprocitySimulatorConfig に制御フィールド追加（simulation.rs）
2. compute_adjusted_policy 関数実装（simulation.rs）
3. SimulationParams に target_population 追加（simulation.rs, #[cfg(feature = "server")]）
4. 全 phase4_gc_survival / run_lifecycle_gc 呼出に調整挿入（simulation.rs）
5. SimCommand + WebSocket ハンドラ（server.rs）
6. フロントエンドUI（index.html + script.js）
7. テスト作成（TC1〜TC6）

## 物理的レビュー方法
- cargo check → cargo clippy → cargo test

## リスク
- target_population=None（デフォルト）で既存動作と完全互換
