# 変更したファイル一覧と実装内容の概要

## 新規作成: src/gmr.rs (~930行)
- ApplicabilityCandidate 構造体 (RFC §10.3 評価候補)
- Stage5Branch 列挙型 (Reuse/Patch/Compose/New/Abort)
- ApplicabilityChannel トレイト + 5実装 (AG-01〜AG-05)
- DeterminismScore 構造体 (SoftMin合成, RFC §10.2)
- ApplicabilityScore 構造体 (幾何平均, RFC §10.3)
- Stage5Decision 構造体 (確率的分岐, RFC §10.4)
- CapabilityGenerator トレイト + SimpleNewGenerator
- DifferentialInference 構造体 (差分推論, RFC §13)
- new_workflow_from ユーティリティ関数
- 8つの不変条件テスト (TC1-TC7) + 3つの観測テスト

## 修正: src/constants.rs
- P2 GMR定数追加: SOFT_MIN_BETA, DETERMINISM_THRESHOLD, AG01〜AG05関連定数, STAGE5_REUSE/ABORT_THRESHOLD
- 既存のApplicabilityScore定数との重複を排除

## 修正: src/composition.rs
- compose_workflows 関数追加: 2つのWorkflowGraphを統合、重複排除

## 修正: src/lib.rs
- `pub mod gmr;` 追加 (guardとhelpの間、アルファベット順)

## 修正: src/simulation.rs
- SimulationContext に `use_gmr: bool` フィールド追加
- phase5_capability_diffusion に GMR分岐追加 (try_gmr_diffusion)
- run_kw_real_simulation で use_gmr=false に初期化
- test_gmr_enabled_simulation テスト追加

## 修正: src/reciprocity.rs
- ドキュメントコメントの整形 (clippy 警告修正)
