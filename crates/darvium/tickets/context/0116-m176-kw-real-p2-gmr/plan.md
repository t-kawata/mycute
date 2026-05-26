# Plan: M1.76-KW-REAL-P2 — GMR抽象化層

## 要件の再確認
RFC §4A.3 の 8 機構のうち、REAL 4 件（GraphPatch, apply_patch_atomic, AG-06, AG-07）を流用し、MISSING 7 機構（DeterminismScore, ApplicabilityScore/AG-01〜AG-05, Stage5Branch/Decision, compose_workflows, new_workflow_from, DifferentialInference, ApplicabilityChannel/CapabilityGenerator traits）を新規実装する。

## RFC 既存実装状態検証

### RFC §10.2 DeterminismScore (SoftMin)
- `WorkflowNode::AgentStep { determinism, side_effects, .. }` — 現行コードに determinism/side_effects フィールドなし (types.rs:53-59)
- シミュレーションでは `&[f64]` で代用

### RFC §10.3 ApplicabilityScore
- S_sem: cosine_similarity REAL (pipeline.rs:60)
- S_struct: exp(-lambda * GED) — シミュレーションでは簡略化
- ApplicabilityScore → 全フィールド MISSING

### RFC §10.4 Stage5分岐
- A >= 0.50 → REUSE、A < 0.50 → GraphPatch 生成
- Stage5Branch/Decision → MISSING

### 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/gmr.rs | 新規 | 全GMR型・トレイト・実装 + テスト |
| src/composition.rs | 拡張 | compose_workflows |
| src/constants.rs | 拡張 | GMR定数 |
| src/simulation.rs | 拡張 | Phase1/Phase5 GMR接続 |
| src/lib.rs | 拡張 | pub mod gmr |

### 実装手順
1. constants.rs: GMR定数追加
2. gmr.rs: 全GMR機構実装 + テスト(TC1-TC6)
3. composition.rs: compose_workflows
4. lib.rs: pub mod gmr
5. simulation.rs: Phase1/Phase5 GMR接続
6. cargo test + cargo clippy

### Boy Scout 改善
新規ファイルのみ、スコープ外改善不要。

### 物理的レビュー方法
run-quality-checks + 翻訳可能性 grep

### リスク
- WorkflowNode に determinism フィールドなし → &[f64] 代用、低リスク
- DifferentialInference 空パッチ生成フォールバック、中リスク
