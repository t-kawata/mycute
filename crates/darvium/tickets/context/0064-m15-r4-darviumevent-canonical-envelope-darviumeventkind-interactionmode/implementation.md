# 実装サマリ: M1.5-R4 DarviumEvent canonical envelope + DarviumEventKind + InteractionMode 型定義

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/event.rs | 新規作成 | 全25種の型定義 + 8つのテストケース |
| src/lib.rs | 修正 | pub mod event; + 全型の pub use 再エクスポート（22型） |

## 実装した型定義

### 補助型（9種）
- EventId, DeliveryMode, TransportMeta, EventVisibility, EventRetention, PiiHandlingPolicy, EventPrivacy, EventSource, EventMetadata, EventCausality

### 中心型（3種）
- InteractionMode（OneWay / TwoWay）
- DarviumEventKind（13 variant）
- DarviumEvent（10フィールド canonical envelope）

### Subtype enum（11種）
- SystemEvent（4 variant）、SearchEvent（5 variant）、WorkflowExecutionEvent（4 variant）
- TrainingEvent（9 variant）、KnowledgeEvent（4 variant）、ConversationalEventEnvelope（5 variant）
- LifecycleEvent（4 variant）、GcEvent（3 variant）、RepairEvent（4 variant）
- ReciprocityEvent（8 variant）、FusionEvent（5 variant）、HitlEvent（4 variant）

## RFC 交叉参照
- RFC §12C.1-12C.3 と完全一致（10フィールド、13 variant、すべての補助型）
- RFC 未定義の subtype（SearchEvent / LifecycleEvent / FusionEvent）は最小 variant を新規定義
- ReciprocityEvent は RFC §15.10.6 ReciprocityEventKind の variant を流用

## 検証結果
- cargo check: 成功
- 全8テスト: PASS
- JSON ラウンドトリップ: 1000/1000 (100.00%)
