# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 変更内容 |
|----------|----------|
| src/event.rs | ①13個の make_*_event 公開ヘルパー関数追加 ②9個の DomainProjection コンストラクタ追加 ③initialize_domain_projections()更新（5→14 projection） ④TC-1〜TC-7テスト追加 ⑤既存テスト更新（5→14 assertions） |

## Step 1: 全13ドメインの make_*_event 公開ヘルパー関数

`make_system_event`, `make_search_event`, `make_workflow_execution_event`, `make_training_event`, `make_knowledge_event`, `make_conversational_event`, `make_lifecycle_event`, `make_gc_event`, `make_repair_event`, `make_reciprocity_event`, `make_fusion_event`, `make_hitl_event`, `make_village_event` の13関数。

各ヘルパーは `make_${domain}_event` の命名規則で `pub fn` として実装。DarviumEvent canonical envelope の全フィールドを明示的に指定する構造体リテラル形式。

## Step 2: 不足9ドメインの DomainProjection コンストラクタ追加

- system_log() — SystemEvent 全4種
- workflow_execution_log() — WorkflowExecutionEvent 全4種
- knowledge_log() — KnowledgeEvent 全4種
- conversational_log() — ConversationalEventEnvelope 全5種
- lifecycle_log() — LifecycleEvent 全4種
- gc_log() — GcEvent 全3種
- repair_log() — RepairEvent 全4種
- fusion_log() — FusionEvent 全5種
- hitl_log() — HitlEvent 全4種

initialize_domain_projections() に新規9 projection を追加し、登録数を 5→14 に更新。

## Step 3〜9: TC-1〜TC-7 テスト

- TC-1: 全14 DomainProjection コンストラクタ正常性 + interested_kinds 完全性（全64種）
- TC-2: 全13ドメイン130件 publish → replay 完全一致
- TC-3: subscribe フィルタ分別精度（明示的なドメイン kind 指定で正確性保証）
- TC-4: 全14 Projection 相互汚染ゼロ
- TC-5: 130件クロック厳密単調増加 + 重複0
- TC-6: 1300件 JSON ラウンドトリップ完全性（100%）
- TC-7: 1300件観測テスト + 一貫性スコア（1.0達成）

## 退行修正

既存テスト test_r10_initialize_domain_projections の projection 数想定を 5→14 に更新。
