# Darvium RFC-0001 — Unified Edition v2.3-c
## Darvium Workflow IR・GMR Retrieval Core・SearchWorkflow・グラフパッチ生成・Lifecycle / GC・Knowledge Ecosystem・Training Plane 統合仕様

**Darvium: Crystallized Ecosystems of Knowledge and Capability（知識と実務能力の結晶化された生態系）**

```
RFC番号  : Darvium-RFC-0001 (統合版)
旧番号   : RFC-0001 Rev.4 + RFC-0002 Rev.3 (統合)
ステータス: PROPOSED STANDARD — Finalizing Revision (v2.3-c)
著者     : Darvium Design Working Group
作成日   : 2026-05-19
改訂日   : 2026-05-22 (v2.3-c)
正史基盤 : Darvium RFC-0001 Unified Edition v1.8-final
RFC-0003対象: Pareto Trust・Counterfactual Replay・Darwinian Evolution・基盤モデル finetuning (本 RFC スコープ外)
```

---

## 改訂履歴

| バージョン | 主な変更 |
|------------|---------|
| v1.0 | RFC-0001 Rev.4 + RFC-0002 Rev.3 統合。矛盾解消・空白補完 |
| **v1.1** | レビュー指摘 6 点必須修正: ①GMR命題を期待値保証へ変更 ②Applicability invalidation を実装内保証へ ③atomic patch apply ④DualTemporalTrust λ 設計意図明記 ⑤cold-start trust 初期化 ⑥agentsethash 64bit 移行計画。追加改善: Stage 0 副作用包含チェック、floorT 定数参照統一、cₛ プロンプト仕様追記、ロールバック方針明記、GED 境界スムージング |
| **v1.2** | 最終完成版: ①inherit_from_parent に operational floor 追加 ②管理者 fast-track の監査ログ要件 (TrustAuditLog) ③apply_patch_atomic に楽観的並行性制御 (GraphVersion CAS) ④定数を MAX_GRAPH_NODES/MAX_COMPILED_STEPS/MAX_PATCH_OPS に分離 ⑤PatchConfidence 動的重みの切り替え条件を規範化 ⑥TrustUpdate::Human の debounce 方針追記 ⑦Trust 継承減衰係数 0.70 を OQ に登録 ⑧TRUST_INHERIT_DECAY 定数化 |
| **v1.3** | §17 マイルストーン改訂: M -1（ダミー層・ポート抽象化）フェーズを新設。各マイルストーンに実装ステップ詳細・ファイル構造・テスト観点を追記。§3 スコープ / §2 用語集に M -1 関連エントリを追加 |
| **v1.4** | graph_embedding を RFC-0001 スコープから除外し、検索経路を task_embedding + GED 中心へ再整理。WorkflowGraph の自己抽象化 / Self-Refinement / Lineage / Contribution / Determinism 推定メタデータを追加。Applicability・Patch・DB・監査ログ・マイルストーン・Open Questions を対応更新 |
| **v1.5** | 専用 graph_embedding モデルは導入せず、WorkflowDesignText / WorkflowDesignEmbedding による structural proxy retrieval を正式化。mission / workflow の双方で design text を生成・保持し、Stage 2 を task_embedding と workflow_design_embedding の Dual Retrieval に拡張。GED の役割を検索主経路から精密 reranking / structural validation / abstraction trigger へ再定義し、複数 embedding channel の version 整合ルール・DB・マイルストーンを更新 |
| **v1.6** | v1.5 の GMR Retrieval Core を保持したまま、その上位に SearchWorkflow Meta-Workflow を追加。検索を first-class workflow operation として定式化し、SearchState / SearchTrace / SearchBudget / SearchOutcome / RecursionGuard を導入。Outcome を REUSE / PATCH / COMPOSE / NEW / ABORT に拡張し、Fake-first マイルストーンを M-2〜M4 に再編。AI provider 接続前に RetrievalPrimitive・状態遷移・予算ガード・監査可能性を deterministic replay と property-based test で検証する方針を規範化 |
| **v1.7** | v1.6 の Layer 分離・Trust 4 軸・GMR / SearchWorkflow・Patch / CAS・lineage を保持したまま、SubWorkflow 資産化を明文化。Human Time と VirtualClock に基づく時間二軸モデル、workflow ごとの時間減衰重み、自然淘汰としての GC、経験値 grace period、互恵性ベース評判、親からの評判/経験値継承、resource pressure 連動淘汰、環境別ポリシー、社会加速度指標を追加。SearchWorkflow / Repository / DB / 定数 / 状態遷移 / 監査ログを対応拡張し、長期持続型ワークフロー生態系として規範化 |
| **v1.8** | v1.7 の完成品質と規範を保持したまま、LadybugDB / StructMem / Corpus2Skill を additive に統合。Knowledge Ecosystem Integration、knowledge-aware QueryDesignText、Knowledge Applicability、Knowledge Primitive Registry、SearchTrace 拡張、knowledge-aware candidate evaluation、dual-store consistency refinement、v1.8 calibration candidates を追加し、知識アクセス・証拠性・鮮度/妥当性・変異安全性・修復手順を規範化 |
| **v1.8-final** | v1.8 の規範を一切毀損せず、(1) QueryDesignText の knowledge-aware schema を正式 canonical schema として固定、(2) Knowledge Applicability の式と Annex 解釈優先順位を明文化、(3) SQLite / LadybugDB / Workflow Repository の source-of-truth 境界を明確化、(4) three-plane architecture と既存 Layer の責務境界を説明補強し、自己完結性と非曖昧性を高めた完成版 |
| **v1.9** | v1.8-final の全文・規範・責務境界・数式・型定義・付録を一切毀損せず保持したまま、Human-in-the-loop を中核に据えた Training Plane を strictly additive に統合。TrainingMission / TrainingRunLog / TrainingFeedback / PromotionCandidate / TrainingTrustProfile / CandidateKnowledgeDocument / CurriculumPolicy / TrainingAuditLog を追加し、AI発・人間発・失敗再訓練を含む自主トレーニング、human review queue、sandbox execution policy、training/prod trust 分離、段階的 promotion、training-specific lifecycle / GC、knowledge under training、migration と監査要件を規範化 |
| **v2.0** | v1.9 の全文・規範・責務境界・数式・型定義・付録を一切毀損せず保持したまま、Repository Pair / Expert Namespace / Fusion Plan / Extraction Plan / Identity Remap Table / Fusion Audit Record / Pair Birth Lifecycle を strictly additive に統合。SQLite + LadybugDB を一体として扱う synthesis fusion を first-class operation として定義し、expert selective extraction・multi-pair fusion・split / recompose・完全トレーサビリティ・actor identity extension・training / production separation・dual-store birth commit・quarantine / repair discipline を規範化 |
| **v2.0-final** | v2.0 の規範を保持したまま、fusion semantics の曖昧性を除去。knowledge object の自動 semantic merge / truth arbitration を v2.0 スコープ外として明示し、conflict は coexistence + lineage relation で扱う方針を固定。単一プロセス / 単一ノード前提を設計上の制約として再明記し、形式保証・脅威モデル・分散化・探索最適化を Annex / RFC-0003 系へ外出しする責務境界を補強して、完成版としての自己完結性を高めた |
| **v2.1** | v2.0-final の全文・規範・責務境界を毀損せず保持したまま、SearchWorkflow を mission-completion-oriented orchestration として再明確化し、単一候補失敗が即時 mission failure を意味しないこと、候補フォールバック・requery・compose・new・human review が bounded orchestration の一部であることを明文化した |
| **v2.2** | v2.1 の規範を保持したまま、WorkflowGraph / SearchWorkflowGraph の DAG 検証を作成時・登録時・更新時と、使用時・コンパイル時・実行前の双方で MUST として明文化し、さらに多層 DAG における ready frontier / concurrency-admissible set / frontier-based parallel execution obligation を追加して、toposort や compile_to_steps の線形化を逐次実行の根拠にできないことを規範化した |
| **v2.3** | v2.2 の全文・規範・責務境界・mission-completion semantics・二段 DAG 検証・多層 DAG 並列実行義務を一切毀損せず保持したまま、(1) dual-store consistency の startup repair scan と recovery invariant、LadybugDB 再試行の idempotent expectation、silent divergence の禁止を明文化し、(2) GED 境界付近の ranking stability / oscillation risk に対する replay / property-based test / calibration discipline を補強し、(3) Training Plane に safe sandbox scope 限定の optional auto-approval exception policy を補足し、(4) reuse quality・false-new rate・repair rate・review-load indicators などの補助メトリクスを前景化した strictly additive revision |
| **v2.3-c** | v2.3 の全文・規範・責務境界・mission-completion semantics・二段 DAG 検証・多層 DAG 並列実行義務・dual-store repair semantics・ranking stability discipline・safe sandbox scope auto-approval を一切毀損せず保持したまま、Conversational Knowledge Path を strictly additive に統合。ConversationalEvent / ConversationalIngestionPolicy / ConversationalClassificationProposal / ConversationalGateDecision / ConversationalMissionPayload / ConversationalFragmentMeta / ConsolidationCandidateSet / ConsolidationPolicy / ConversationalPromotionGate の型定義群、LLM proposal → deterministic gate 分離原則、multi-turn / multi-day consolidation policy と数値閾値、personalization namespace convention、privacy / retention / tombstone / repair 規約を追加し、会話入力を起点とする知識成長経路（ConversationalEvent → Fragment → CandidateKnowledgeDocument → CanonicalDocument）の全段階を数値閾値・型定義・擬似コード付きで規範化した strictly additive revision |

---

## 目次

1. [概要と目的](#1-概要と目的)
2. [用語集](#2-用語集)
3. [スコープ](#3-スコープ)
4. [設計上の前提と制約](#4-設計上の前提と制約)
5. [4 層アーキテクチャ概観](#5-4-層アーキテクチャ概観)
6. [Layer 2 — Workflow IR (WorkflowGraph)](#6-layer-2--workflow-ir-workflowgraph)
7. [Layer 2 → Layer 1 コンパイル](#7-layer-2--layer-1-コンパイル)
8. [WorkflowRepository と MemoizedGraph](#8-workflowrepository-と-memoizedgraph)
9. [WorkflowDesignText / QueryDesignText](#9-workflowdesigntext--querydesigntext)
10. [TrustProfile — 4 軸信頼モデル](#10-trustprofile--4-軸信頼モデル)
11. [Applicability Check](#11-applicability-check)
12. [Layer 3a — GMR Retrieval Core](#12-layer-3a--gmr-retrieval-core)
12A. [Knowledge Primitive Registry (v1.8)](#12a-knowledge-primitive-registry-v18)
13. [Layer 3b — SearchWorkflow Engine](#13-layer-3b--searchworkflow-engine)
14. [Layer 2.5 — グラフパッチ生成](#14-layer-25--グラフパッチ生成)
15. [Layer 3c — Lifecycle / Natural Selection / GC](#15-layer-3c--lifecycle--natural-selection--gc)
16. [GMR / SearchWorkflow / Lifecycle 実行フロー全体](#16-gmr--searchworkflow--lifecycle-実行フロー全体)
16A. [Training Plane 実行フロー全体 (v1.9)](#16a-training-plane-実行フロー全体-v19)
16B. [Conversational Knowledge Path (v2.3-c)](#16b-conversational-knowledge-path-v23-c)
17. [健全性命題](#17-健全性命題)
18. [エラーハンドリングとロールバック方針](#18-エラーハンドリングとロールバック方針)
19. [性能目標](#19-性能目標)
20. [マイルストーン](#20-マイルストーン)
20A. [v1.8-final 統合補完](#20a-v18-final-統合補完)
20B. [v1.9 Training Plane 統合補完](#20b-v19-training-plane-統合補完)
21. [未解決事項 (Open Questions)](#21-未解決事項-open-questions)
22. [付録 A — 定数一覧](#22-付録-a--定数一覧)
23. [付録 B — エラー型全体](#23-付録-b--エラー型全体)
24. [付録 C — 数式インデックス](#24-付録-c--数式インデックス)
25. [補足: データベース構成：SQLite + LadybugDB](#25-補足-データベース構成sqliteladybugdb)
26. [付録 D — v1.7 / v1.8 / v1.9 追加データモデル](#26-付録-d--v17--v18--v19-追加データモデル)
27. [付録 E — v1.8 / v1.9 Calibration Candidates](#27-付録-e--v18--v19-calibration-candidates)
28. [Repository Pair / Expert Fusion 統合仕様 (v2.0-final)](#28-repository-pair--expert-fusion-統合仕様-v20-final)
29. [Fusion Core Terminology (v2.0)](#29-fusion-core-terminology-v20)
30. [Repository Pair Model](#30-repository-pair-model)
31. [Expert Boundary Model](#31-expert-boundary-model)
32. [Fusion / Extraction Operations](#32-fusion--extraction-operations)
33. [Admissibility and Safety Gates](#33-admissibility-and-safety-gates)
34. [Identity Remapping](#34-identity-remapping)
35. [Lineage and Traceability Requirements](#35-lineage-and-traceability-requirements)
36. [Training / Production Separation in Fusion](#36-training--production-separation-in-fusion)
37. [Fusion Orchestrator and Birth Commit](#37-fusion-orchestrator-and-birth-commit)
38. [Failure Handling, Quarantine, and Repair for Fusion](#38-failure-handling-quarantine-and-repair-for-fusion)
39. [Migration and Backward Compatibility for v2.0](#39-migration-and-backward-compatibility-for-v20)
40. [付録 F — v2.0 追加データモデル](#40-付録-f--v20-追加データモデル)
41. [付録 G — Fusion Invariants and Open Questions](#41-付録-g--fusion-invariants-and-open-questions)
42. [参照文献](#42-参照文献)

---

## 1. 概要と目的

Darvium は OpenFang を Layer 1 実行エンジンとして利用し、WorkflowGraph を正本とする Application Workflow 層、その再利用検索を担う GMR Retrieval Core、ならびにそれらを探索・選択する SearchWorkflow Meta-Workflow 層、さらに長期運用下で資産の寿命・淘汰・評判・継承を制御する Lifecycle / Natural Selection 層を統合した実行・探索基盤を提供する。

本 RFC で規定する主要保証は以下の 10 個である。

1. **構文的健全性** — WorkflowGraph / SearchWorkflowGraph は常に DAG であり、変数スコープと状態遷移制約が閉じている。
2. **実行的健全性** — Applicability Check と SearchGuard がエージェント互換性・副作用安全性・予算超過・再帰暴走を事前に抑止する。
3. **検索的健全性** — GMR Retrieval Core は semantic 類似 (`task_embedding`) と structural proxy 類似 (`workflow_design_embedding`) を統合し、候補不足・version 不整合・信頼不足を明示的に扱う。
4. **探索的健全性** — SearchWorkflow は REUSE / PATCH / COMPOSE / NEW / ABORT の outcome 空間を bounded search として探索し、SearchTrace により監査可能な決定履歴を残す。
5. **最適化的健全性** — 既存ワークフローの再利用・差分修正・構成的合成・新規生成を明確に分離し、期待値ベースで LLM 呼び出しコストと失敗率を削減する。
6. **生態系的健全性** — SubWorkflow を共有資産として登録し、Human Time と VirtualClock の二軸時間、経験値 grace period、互恵性ベース評判、自然淘汰としての GC、resource pressure 制御により、資産群の長期持続可能性を保つ。
7. **知識的健全性** — Knowledge Applicability、origin trace、evidence completeness、dual-store consistency により、知識アクセス・知識変異・知識昇格の安全性と説明可能性を確保する。
8. **訓練的健全性** — Training Plane は mission generation、human review、sandbox execution、feedback ingestion、promotion を first-class に扱い、本番実行系と責務・namespace・評価系を分離する。
9. **昇格的健全性** — training artifacts は sandbox only / candidate / approved / promoted / rolled back の段階を経なければ production artifacts に昇格してはならない。
10. **共同訓練健全性** — 人間は単なる例外処理の最終安全弁ではなく、訓練対象の選定、結果評価、重点領域の注入、昇格判断を行う共同訓練者として規範的に位置づけられる。

本 RFC は RFC-0001 Rev.4 を正史とし、RFC-0002 Rev.3 のグラフパッチ生成仕様を統合した v1.5 の完成度を保持しつつ、v1.6 では SearchWorkflow Meta-Workflow を追加して GMR を workflow discovery primitive として再編成した完成度を保持しつつ、v1.7 では Lifecycle / Natural Selection 層を追加して SubWorkflow 資産化、時間二軸、VirtualClock、経験値、互恵性評判、GC、継承、resource pressure、社会加速度を統合した単一規範文書である。

さらに v1.8 / v1.8-final では、LadybugDB / StructMem / Corpus2Skill を additive に統合し、Knowledge Ecosystem Integration、knowledge-aware QueryDesignText、Knowledge Applicability、Knowledge Primitive Registry、dual-store consistency、three-plane architecture の責務境界を完成形として固定した。v1.9 はこの完成形を前提に、その全文を保持したまま Human-in-the-loop を中核に据えた first-class training architecture を追加する strictly additive revision である。v2.0-final はその上に repository pair / expert fusion / quarantine discipline を重ね、v2.1 と v2.2 は SearchWorkflow の mission-completion semantics、creation-time / execution-time DAG validation、frontier-based parallel execution obligation を strictly additive に補強した。v2.3 はさらに、dual-store repair semantics、ranking stability / replay / property-based test discipline、training review load の安全な運用補助、補助評価指標の前景化を加えるが、既存の core invariant と責務境界を変更しない。v2.3-c はさらに Conversational Knowledge Path を strictly additive に追加するが、既存の core invariant と責務境界を変更しない。

本改訂でいう training とは、基盤モデル自体の parameter update ではなく、(a) ワークフロー空間の拡張、(b) ワークフロー品質の洗練、(c) 知識基盤の厚みの増大、(d) 人間の価値判断・重点領域の注入を、明示的な mission generation・mission review・sandbox execution・feedback ingestion・promotion discipline の下で制度化することを指す。

したがって v1.9 は、v1.8-final に内在していた探索・改良・レビュー・trust 更新・知識蓄積の諸機構を、Training Plane という論理平面に整理して formalize する改訂である。training primitive を一切用いない既存 v1.8 workflow の意味論、TrustProfile、SearchWorkflow、Lifecycle / GC、Knowledge Applicability、source-of-truth 境界、QueryDesignText canonical schema、GraphVersion CAS、dual-store consistency は v1.9 においても変更されてはならない (MUST NOT)。

**v1.9 確定方針**: 専用 `graph_embedding` は RFC-0001 の規範スコープから除外し、真の graph embedding・GNN encoder・その学習最適化は RFC-0003 以降へ委譲する。SearchWorkflow の COMPOSE / NEW / ABORT 分岐は bounded heuristic policy として扱い、責務・状態機械・予算・監査可能性のみを規範化する。加えて v1.7 では GC / 評判 / 社会加速度の閾値や重みは tuning 可能としつつ、時間軸分離、SubWorkflow 資産化、状態遷移、監査可能性、Soft/Hard/Tombstone の責務境界は規範として固定する。さらに v1.9 は、これらに Training Plane を strictly additive に重ねるのみとし、training artifact が promotion gate を通過するまで production selection path・production trust・canonical knowledge・WorkflowRepository source-of-truth を汚染しないことを追加規範として固定する。さらに v2.3 は、dual-store recovery は application-level discipline であり XA / distributed 2PC を意味しないこと、ranking stability と review-load は calibration / operational measurement の対象であることを補足するが、single-process / single-node 前提や training / production separation を変更しない。v2.3-c は、会話入力から長期知識への成長経路（Conversational Knowledge Path）を追加規定するが、既存の core invariant と責務境界を変更しない。

---

## 2. 用語集

| 用語 | 定義 |
|------|------|
| **WorkflowGraph** | `StableGraph<WorkflowNode, EdgeMeta>` 型の有向非巡回グラフ (DAG) |
| **MemoizedGraph** | WorkflowGraph に埋め込みベクタ・TrustProfile・Provenance を付与したリポジトリ格納単位 |
| **WorkflowDesignText** | WorkflowGraph の構造・主要ノード列・依存関係・分岐・集約・I/O・副作用・決定論性特徴を canonical schema で記述した自然言語 / 半構造化テキスト (v1.5 新設) |
| **WorkflowDesignEmbedding** | `WorkflowDesignText` を embedding 化した構造類似近似ベクトル (v1.5 新設) |
| **QueryDesignText** | mission から生成される検索用の粗いワークフロー設計記述。完全な WorkflowGraph ではない (v1.5 新設) |
| **Dual Retrieval / Bi-Vector Retrieval** | `task_embedding` と `workflow_design_embedding` の双方に基づく候補探索方式 (v1.5 新設) |
| **Structural Semantic Proxy** | 真の graph embedding ではなく、構造記述テキストの embedding を構造類似の近似表現として用いる方式 (v1.5 新設) |
| **GED** | Graph Edit Distance — グラフ間の最小編集コスト (NP 困難、近似使用) |
| **GMR** | Graph-Memoized Reasoning — 過去の成功グラフを検索・再利用する最適化手法 (arXiv:2511.15715) |
| **ApplicabilityScore A** | 類似度・決定論性・信頼の加重幾何平均。再利用可否を判定するスコア |
| **DeterminismScore D** | グラフ内 AgentStep の決定論性を副作用重み付き SoftMin で集約したスコア |
| **TrustProfile** | Operational・Semantic・Temporal・Human の 4 軸で構成される信頼プロファイル |
| **GraphPatch** | Gold グラフを Gnew に変換する差分操作列と PatchConfidence |
| **patchconfidence** | LLM 自己評価・バリデータ・履歴の幾何平均から算出されるパッチ信頼度 |
| **Layer 2.5** | GraphPatch を生成・適用するレイヤ |
| **Gold グラフ** | GMR により選択された最高 ApplicabilityScore の既存 MemoizedGraph |
| **cold-start** | 実行履歴が 0 の新規 MemoizedGraph、または新規ミッションへの初回適用状態 |
| **MUST / SHOULD / MAY** | RFC 2119 準拠 |
| **GraphVersion** | MemoizedGraph の楽観的並行性制御に使用する u64 カウンタ |
| **TrustAuditLog** | 管理者 fast-track 等の信頼値手動変更を記録する監査ログエントリ |
| **Debounce** | Human/Semantic の trust 更新時、composite スコア変動が閾値未満の場合はキャッシュ無効化をスキップするメカニズム |
| **PortTrait** | `WorkflowExecutor` / `LlmClient` など、外部依存をトレイト境界として抽象化した差し替え可能なインタフェース (v1.3 新設) |
| **FakeImpl** | PortTrait の in-process ダミー実装。OpenFang・LLM に一切接続せず、AIコスト ゼロでロジック層のテストを実現する (v1.3 新設) |
| **M -1** | OpenFang・LLM を FakeImpl に置き換えた状態でコアロジックのみを検証するフェーズ (v1.3 新設) |
| **Self-Refinement / Self-Deepening** | 既存グラフを再評価し、抽象化・決定論性改善・局所修正を行う自己改善サイクル (v1.4 新設) |
| **AbstractableSubgraph** | 反復出現や高 GED 断片から切り出され、SubWorkflow 候補となる部分グラフ (v1.4 新設) |
| **AbstractionPatch** | 部分グラフを新規 SubWorkflow へ置換する差分操作列 (v1.4 新設) |
| **WorkflowLineage** | 派生元・ルート・世代・生成方式を記録する系譜メタデータ (v1.4 新設) |
| **ContributionRecord** | グラフ改善に対する人間・モデル・レビュー実行の寄与記録 (v1.4 新設) |
| **DeterminismObservation** | 同一ノードの再実行差分を記録した決定論性観測値 (v1.4 新設) |
| **DeterminismProfile** | prior / estimated / confidence / sample_count を保持する決定論性推定プロファイル (v1.4 新設) |
| **RefinementRunLog** | Self-Refinement / Deepening / レビュー実行の入出力・トークン・結果を記録するログ (v1.4 新設) |
| **SearchWorkflow** | Application Workflow の探索・再利用・合成・新規提案を行うメタワークフロー。GMR Retrieval Core を内部 primitive として呼び出す (v1.6 新設) |
| **RetrievalPrimitive** | SearchWorkflow から呼ばれる Stage 0–4 の GMR 候補取得 API。v1.5 の GMR 検索パイプラインを純化した再利用 primitive (v1.6 新設) |
| **SearchState** | SearchWorkflow の状態機械上の状態。Init / Retrieve / Evaluate / Refine / Compose / ProposeNew / Finalize / Abort を持つ (v1.6 新設) |
| **SearchTrace** | iteration ごとの query / policy / candidate / outcome justification / budget 消費を記録する監査トレース (v1.6 新設) |
| **SearchBudget** | token・retrieval call・iteration・wall-clock を束ねた bounded search 制約 (v1.6 新設) |
| **SearchOutcome** | SearchWorkflow の最終決定。ReuseExisting / PatchExisting / ComposeExisting / GenerateNew / AbortSearch / NeedsHumanReview を含む (v1.6 新設) |
| **CompositionPlan** | 複数既存 workflow を接続して目的を満たす構成案。ComposeExisting outcome の根拠となる中間表現 (v1.6 新設) |
| **SearchRunLog** | SearchWorkflow 実行単位の開始・終了・予算超過・最終 outcome を保持するログ (v1.6 新設) |
| **RecursionGuard** | SearchWorkflow が SearchWorkflow を再帰的に呼ぶ際の深さ・再入・side-effect 境界を制御する保護機構 (v1.6 新設) |
| **VirtualClock** | Darvium 内部イベントにより単調増加するグローバル内部時間カウンタ。SystemTime とは独立した仮想時間軸 (v1.7 新設) |
| **Human Time** | 外界・社会・情報鮮度の変化を表す SystemTime ベースの時間軸 (v1.7 新設)。SystemTime は常に UTC とみなす (MUST) |
| **Virtual Time** | `VirtualClock` 差分により測定される Darvium 内部進行時間 (v1.7 新設) |
| **TimeDecayProfile** | workflow ごとの human / virtual 減衰重みと減衰率を保持する時間減衰設定 (v1.7 新設) |
| **ExperienceCount** | 実行・再利用・構成への寄与などに基づき蓄積される累積経験値。grace period 判定に用いる (v1.7 新設) |
| **Grace Period** | `experience_count < MIN_SURVIVAL_EXPERIENCE` の間、GC 対象から保護する最小生存期間 (v1.7 新設) |
| **ReputationProfile** | 直接互恵性・間接互恵性・経験値補正済み評判を保持する資産評判プロファイル (v1.7 新設) |
| **Direct Reciprocity** | ある workflow が他 workflow を有益に利用し、また利用され返す双方向関係に基づく評判成分 (v1.7 新設) |
| **Indirect Reciprocity** | 利用ネットワーク全体における中心性・信認・協力度に基づく評判成分 (v1.7 新設) |
| **LifecycleScore L(G)** | 時間鮮度・成功率・trust・使用度・評判を統合した生存スコア。GC 判定に用いる (v1.7 新設)。デフォルト重みは付録 A の `LIFECYCLE_WEIGHT_*` を用いる |
| **GcState** | Active / SoftDeleted / HardDeleteCandidate / Tombstoned の資産寿命状態 (v1.7 新設) |
| **ResourcePressure** | ストレージ・メモリ・CPU・ANN インデックス容量等の逼迫度を表すスカラーまたは複合観測値 (v1.7 新設) |
| **EnvironmentPolicy** | 本番・検証・実験などの箱庭ごとに GC / 継承 / 重みを切り替える運用ポリシー集合 (v1.7 新設) |
| **SocialAcceleration** | 再利用・構成・成功率改善がシステム全体の進化速度をどれだけ高めたかを表す上位 KPI (v1.7 新設) |
| **ConsistencyState** | LadybugDB / SQLite 間の論理整合状態。Committed / Pending / NeedsRepair / Quarantined を持つ補助状態 (v1.7 追補) |
| **RepairLog** | 異種ストア間の不整合検知・再試行・tombstone 化を記録する復旧監査ログ (v1.7 追補) |
| **Training Plane** | mission generation・human review queue・sandbox execution・feedback ingestion・promotion workflow を統合する v1.9 の論理平面 |
| **TrainingMission** | 訓練対象ミッションを first-class に表現する型。出自・レビュー状態・success criteria・sandbox policy を含む (v1.9 新設) |
| **TrainingRunLog** | training run 単位の実行記録。SearchRunLog と join 可能だが責務は分離されるログ (v1.9 新設) |
| **TrainingFeedback** | human feedback を構造化し、trust update・audit・curriculum bias に接続する型 (v1.9 新設) |
| **TrainingTrustProfile** | production TrustProfile と分離された訓練評価チャネル。sandbox operational / human / curriculum fit / safety を保持 (v1.9 新設) |
| **PromotionCandidate** | sandbox 成果を production へ昇格させる前の審査単位 (v1.9 新設) |
| **PromotionStatus** | `SandboxOnly / Candidate / Approved / Rejected / Promoted / RolledBack` からなる昇格状態 (v1.9 新設) |
| **CandidateKnowledgeDocument** | training 中に得た document を sandbox namespace に保持するための暫定知識資産 (v1.9 新設) |
| **MissionSource** | `AiGenerated / HumanSubmitted / ReplayFromProduction / DerivedFromFailure` (v1.9 新設) |
| **MissionReviewStatus** | `Pending / Approved / Rejected / Archived` (v1.9 新設) |
| **FeedbackRating** | `Good / Bad / NeedsRevision / Irrelevant / Unsafe` (v1.9 新設) |
| **FeedbackTargetScope** | `Mission / Workflow / SubWorkflow / KnowledgeObject / SearchPolicy` (v1.9 新設) |
| **TrainingArtifactState** | `TrainingOnly / PromotionCandidate / Promoted / Rejected / Tombstoned` で表される訓練資産状態 (v1.9 新設) |
| **CurriculumPolicy** | ドメイン・難易度・失敗原因・目的別に training mission の流量と優先度を調整する方針 (v1.9 新設) |
| **ConversationalEvent** | 会話入力を知識化する最初の入口となるイベント。utterance・actor・channel・コンテキスト情報を保持する (v2.3-c 新設) |
| **ConversationalIngestionPolicy** | 会話イベントの知識化を制御するポリシー。namespace template・auto-sandbox-ingest可否・PII処理・retention・カテゴリ別規則を保持する (v2.3-c 新設) |
| **ConversationalKnowledgeCategory** | 会話内容の知識分類。UserProfile / UserPreference / LongLivedProjectContext / StableConstraint / TemporaryTaskContext / FactualClaim / Reflection / RelationshipFact / Noise / Unsafe / Unknown (v2.3-c 新設) |
| **ConversationalGateAction** | 会話 ingestion の deterministic gate が出力するアクション種別。Drop / StoreRawEventOnly / StoreFragmentOnly / CreateTrainingMission / CreateTrainingMissionAndFragment / QueueForConsolidation (v2.3-c 新設) |
| **ConsolidationCandidateSet** | 複数日にわたる会話断片を束ねて図書館化候補とするための集合。semantic_coherence・trace_completeness・temporal_stability・contradiction_score を保持する (v2.3-c 新設) |
| **ConsolidationPolicy** | ConsolidationCandidateSet が CandidateKnowledgeDocument へ昇格するための数値閾値群。min_distinct_events / min_distinct_days / min_semantic_coherence 等を保持する (v2.3-c 新設) |

---

## 3. スコープ

### 3.1 In-Scope

- WorkflowGraph の型定義・バリデーション規則
- Layer 2 → Layer 1 コンパイル (`compile_to_steps`)
- WorkflowRepository・MemoizedGraph の構造と cold-start 初期化
- TrustProfile 4 軸の更新アルゴリズム (atomic 状態機械)
- Applicability Check (ハードゲート + DeterminismScore + ApplicabilityScore)
- GMR Retrieval Core (Stage 0–4)
- SearchWorkflow Meta-Workflow と agentic search loop
- Lifecycle / Natural Selection / Garbage Collection 層
- SearchWorkflow の bounded heuristic policy（閾値・policy 関数そのものは実装調整可能、ただし状態機械・ガード・監査要件は規範）
- SearchState / SearchTrace / SearchBudget / SearchOutcome / RecursionGuard
- ComposeExisting / GenerateNew / AbortSearch / NeedsHumanReview outcome 規則
- WorkflowDesignText の生成規則・保存方式・canonical format
- QueryDesignText の生成規則・複雑さ制約・キャッシュ方針
- WorkflowDesignEmbedding による structural proxy ANN 検索
- semantic 類似度と structure-description 類似度の統合式
- GED の reranking / structural validation / abstraction trigger としての再定義
- GraphPatch 生成・atomic 適用・PatchConfidence 計算
- TrustUpdate と applicability キャッシュ無効化の状態機械
- エラーハンドリングとロールバック方針
- 全定数と数値ハイパーパラメータの正本
- **PortTrait 抽象化と FakeImpl の設計指針 (v1.3 追加)**
- WorkflowGraph の自己抽象化 (AbstractableSubgraph / AbstractionPatch)
- RetrievalPrimitive API と SearchWorkflow からの利用契約
- FakeImpl / deterministic doubles / property-based test による SearchWorkflow の事前検証
- Self-Refinement / Self-Deepening の実行ログと lineage 管理
- Determinism の事前値 + 実測推定プロファイル
- ContributionRecord / WorkflowLineage / RefinementRunLog などの派生メタデータ
- SubWorkflow 資産化と共有 Repository 登録規則
- Human Time / Virtual Time / VirtualClock の時間二軸モデル
- workflow ごとの TimeDecayProfile と再推定規則
- ExperienceCount / Grace Period / ReputationProfile / GcState
- 直接互恵性・間接互恵性・経験値補正評判
- LifecycleScore / soft delete / hard delete candidate / tombstone 遷移
- resource pressure 連動淘汰加速と environment policy 差分
- SocialAcceleration 指標と tuning 用 KPI
- 単一プロセス前提における LadybugDB / SQLite 間の不整合検知・隔離・自動修復フロー
- Training Plane の責務定義と four-plane logical architecture への拡張
- TrainingMission / TrainingRunLog / TrainingFeedback / PromotionCandidate / TrainingTrustProfile / CandidateKnowledgeDocument / CurriculumPolicy / TrainingAuditLog
- AI 発 / 人間発 / failure replay による mission intake
- Human mission review queue と approve / reject / edit / priority 調整 / duplicate merge
- Sandbox execution policy と fake-first training mode
- SearchWorkflow と Training Orchestrator の統合契約
- training trust と production trust の分離・継承・昇格条件
- training-specific lifecycle / GC semantics
- knowledge under training と sandbox namespace / promotion discipline
- human communication patterns と formal object 連結
- backward compatibility / migration strategy
- **Conversational Knowledge Path: conversational event ingestion, LLM-driven policy-based classification, deterministic ingestion gate, Conversational TrainingMission construction, fragment / candidate creation, multi-turn / multi-day consolidation policy, personalization namespace convention, conversational promotion gate, and privacy / retention / tombstone / repair for conversational memory (v2.3-c 追加)**

### 3.2 Out-of-Scope (RFC-0003 に委譲)

- Pareto Trust フロンティア
- SearchWorkflow の exploration policy 最適化アルゴリズムそのもの (bandit / MCTS / RL など)
- Counterfactual Replay
- Darwinian Graph Mutation (変異率 `μᵢ = 1 − dᵢ`)
- HumanTrustLogistic から Elo への昇格 (count ≥ 50 以降)
- GNN / GIN / GraphSAGE 等による真の graph embedding を用いた検索強化
- graph embedding 用の専用 encoder 学習・最適化
- query-graph の完全構築とその graph neural encoding
- Saga 補償トランザクション (§15 で将来拡張として記述)
- TrustAuditLog の永続化バックエンド (DB スキーマは実装依存)
- 分散マルチノード間の GraphVersion 同期 (シングルプロセス前提)
- RL / bandit による GC 閾値自動学習
- graph embedding 用専用 encoder の再導入
- SocialAcceleration の最適制御アルゴリズムそのもの
- 分散 2PC / XA などのマルチストア厳密分散トランザクション
- 基盤 LLM 自体の finetuning や parameter update
- 任意の外部ツールを自動発見して無制限に組み込む機構
- human approval なしの本番自己改変の一般化
- production knowledge base への無審査自動昇格
- safety policy を回避した unrestricted self-play
- pairwise ranking / tournament ranking を用いた高度な human preference 学習最適化

---

## 4. 設計上の前提と制約

| ID | 内容 | 影響範囲 |
|----|------|---------|
| P-01 | WorkflowGraph は DAG でなければならない。`petgraph::algo::toposort` が `Err(Cycle)` を返す場合は即時拒否 | Layer 2 |
| P-02 | OpenFang REST API は OpenFang v0.4.9 以降の仕様に依存 | Layer 1 |
| P-03 | AgentStep の idempotency は Layer 2 では保証しない。SideEffect フィールドで明示 | Layer 2 |
| P-04 | WorkflowRepository は `tokio::sync::RwLock` で保護された並行アクセスを前提とする | Layer 3 |
| P-09 | MemoizedGraph への更新は `GraphVersion` による楽観的並行性制御 (CAS) を使用すること (MUST)。期待バージョンと不一致の場合は `UpdateConflict` エラーを返すこと (§8.3 参照) | Layer 3 / 2.5 |
| P-05 | 埋め込みモデルのバージョンは `Provenance.source_version` に記録し、異なるバージョン間の類似度比較は AG-05 で排除する | Layer 3 |
| P-06 | `StableGraph` を使用すること (MUST)。DiGraph はノード削除時に NodeIndex が無効化されるため使用禁止 | Layer 2 / 2.5 |
| P-07 | 新規 MemoizedGraph は cold-start trust で初期化すること (§8 参照)。Trust が 0.0 のグラフをリポジトリに登録してはならない (MUST NOT) | Layer 3 |
| P-08 | `apply_patch` は atomic に実行すること。途中失敗時はグラフを元の状態に戻さなければならない (MUST) | Layer 2.5 |
| P-10 | training artifacts は production artifacts と source-of-truth を共有してよいが、namespace・review state・promotion state・policy binding を分離しなければならない (MUST) | Training Plane |
| P-11 | AI-generated TrainingMission は原則として human review を経ずに sandbox 実行してはならない (MUST NOT) | Training Plane |
| P-12 | training で得られた workflow / subworkflow / knowledge / query pattern を production Gold として即時採用してはならない (MUST NOT) | Training Plane / Promotion |
| P-13 | training trust と production trust は別チャネルで保持しなければならない (MUST) | Trust |
| P-14 | knowledge mutation を伴う training run は sandbox namespace に限定しなければならない | Knowledge / Training |
| P-15 | v2.0 の Repository Pair / Fusion semantics は単一プロセス・単一ノード前提で規範化される。分散 consensus / replication / partition handling は本 RFC では扱わず、将来 Annex / 別 RFC に委譲する | Fusion / Repository |
| P-16 | fusion における knowledge object の semantic deduplication・truth arbitration・自動優劣判定は本 RFC スコープ外とし、v2.0-final では coexistence + lineage relation (`CONSOLIDATES` / `SUPERSEDES` 等) により扱うこと | Fusion / Knowledge |

---

## 5. 4 層アーキテクチャ概観

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 3c — Lifecycle / Natural Selection / GC              │
│  VirtualClock, TimeDecayProfile, ReputationProfile, GcState │
│  LifecycleScore, ResourcePressure, SocialAcceleration       │
├─────────────────────────────────────────────────────────────┤
│  Layer 3b — SearchWorkflow Engine                           │
│  SearchWorkflow, SearchState, SearchTrace, SearchBudget     │
│  REUSE / PATCH / COMPOSE / NEW / ABORT                      │
├─────────────────────────────────────────────────────────────┤
│  Layer 3a — GMR Retrieval Core                              │
│  WorkflowRepository, MemoizedGraph, Stage 0–4 Retrieval     │
│  task_embedding + workflow_design_embedding                 │
│  TrustProfile, ApplicabilityScore                           │
├─────────────────────────────────────────────────────────────┤
│  Layer 2.5 — GraphPatch / Composition Proposal              │
│  GraphPatch, CompositionPlan, PatchConfidence               │
│  apply_patch_atomic, TrustUpdate 連携                       │
├─────────────────────────────────────────────────────────────┤
│  Layer 2 — Workflow IR (Application / SearchWorkflow)       │
│  WorkflowNode, SearchStep, EdgeMeta, StableGraph<>          │
│  compile_to_steps, ValidationError, SearchValidationError   │
├─────────────────────────────────────────────────────────────┤
│  Layer 1 — Executor / Provider Ports                        │
│  OpenFang, WorkflowExecutor, LlmClient, EmbeddingProvider   │
└─────────────────────────────────────────────────────────────┘
```

---


### 5.5 Knowledge Ecosystem Integration (v1.8)

Revision Revision v1.8-final preserves the v1.8 knowledge ecosystem integration layer and further resolves schema, annex, source-of-truth, and architectural-boundary ambiguities without altering prior normative behavior. Revision v1.8-final therefore introduces a knowledge ecosystem integration layer without changing the foundational Layer 1 through Layer 3c responsibilities defined in v1.7. Workflow orchestration remains the source of truth for workflow graphs, trust, lifecycle, search traces, and workflow applicability. LadybugDB becomes the source of truth for knowledge objects and relations, including Fragment, MemoryEvent, MemoryConcept, CanonicalDocument, SkillNode, Chunk, Entity, and the lineage relations DERIVEDFROM, CONSOLIDATES, ABOUTCONCEPT, SUPERSEDES, MATERIALIZEDAS, and COMPILEDTOSKILL.

The integrated system SHALL be interpreted as a three-plane architecture: (a) Workflow Orchestration Plane, consisting of WorkflowGraph, GMR Retrieval Core, SearchWorkflow Engine, Lifecycle GC, and TrustProfile; (b) Knowledge Access Primitive Plane, consisting of deterministic wrappers for memorygetrecentevents, memorygetconcepts, memorygetconcepthistory, memorytraceorigin, memorypromotetodocument, skilllistchildren, skillgetchunks, skillexpandentities, skillbacktrack, and kbhybridsearch; and (c) Knowledge Persistence Plane, consisting of LadybugDB as the knowledge source of truth and optional SQLite runtime metadata for caches, queues, repair state, and route hints.

This integration is strictly additive. Existing v1.7 semantics for WorkflowGraph compilation, applicability computation, patch application, trust update, and lifecycle transitions MUST remain valid for workflows that do not invoke any knowledge primitive

Revision v1.8-final clarifies that the three-plane architecture is a logical decomposition layered over the existing v1.7 implementation stack. The Workflow Orchestration Plane remains implemented primarily by Layer 2 through Layer 3c. The Knowledge Access Primitive Plane is not an independent scheduler or repository; rather, it is the normative interface surface through which Layer 3b SearchWorkflow and Layer 3a retrieval logic invoke deterministic knowledge operations under the same timeout, audit, trust, and replay constraints that already govern AgentStep execution. The Knowledge Persistence Plane remains responsible only for persisted knowledge objects and relations plus optional runtime metadata stores, and SHALL NOT redefine the existing v1.7 workflow ownership of WorkflowGraph, GraphVersion, TrustProfile, Lifecycle state, or SearchTrace..

**v2.3-c 補足:** Conversational ingestion is an optional policy-governed extension layered over the existing Knowledge Access Primitive Plane and Training Plane. It SHALL NOT redefine ownership of canonical knowledge, WorkflowGraph, TrustProfile, Lifecycle state, SearchTrace, or training-production separation.

### 5.6 Training Plane Integration (v1.9)

Revision v1.9 extends the v1.8-final logical decomposition by adding a fourth logical plane, the **Training Plane**, while preserving all existing v1.8-final responsibilities and source-of-truth boundaries. The Training Plane formalizes mission generation, human review, sandbox execution, feedback ingestion, curriculum shaping, and promotion to production, but SHALL NOT redefine ownership of WorkflowGraph, GraphVersion, TrustProfile, Lifecycle state, SearchTrace, or canonical knowledge objects.

The integrated system SHALL therefore be interpreted as a four-plane logical architecture: (a) Workflow Orchestration Plane; (b) Knowledge Access Primitive Plane; (c) Knowledge Persistence Plane; and (d) Training Plane. The Training Plane is not an independent executor or repository. It is an orchestration extension layered over SearchWorkflow, Trust, Lifecycle, Knowledge Primitive Registry, and promotion/audit controls.

Training artifacts SHALL remain isolated from production artifacts until promotion gates, trust review, audit requirements, evidence / origin-trace requirements, CAS checks, and consistency checks are satisfied. This revision is strictly additive with respect to v1.8 search, trust, lifecycle, and knowledge semantics.

**v2.3-c 補足:** Conversational ingestion is an optional policy-governed extension layered over the existing Knowledge Access Primitive Plane and Training Plane. It SHALL NOT redefine ownership of canonical knowledge, WorkflowGraph, TrustProfile, Lifecycle state, SearchTrace, or training-production separation.

## 6. Layer 2 — Workflow IR (WorkflowGraph)

### 6.1 WorkflowNode / SideEffectSet / VarDecl

```rust
#[derive(Debug, Clone)]
enum WorkflowNode {
    AgentStep {
        agent: String,
        prompt_template: String,
        inputs: Vec<VarDecl>,
        output_var: String,
        side_effects: SideEffectSet,
        /// 決定論性スコア [0.0, 1.0]
        /// 1.0 = 完全決定論的 (純粋計算等)
        /// 0.7 = instruction-following LLM
        /// 0.5 = RAG LLM
        /// 0.0 = 外部 API 依存
        determinism: f32,
        determinism_profile: Option<DeterminismProfile>,
        timeout_secs: u32,
        error_mode: ErrorMode,
    },
    SubWorkflow {
        workflow_id: WorkflowId,
        input_mapping: HashMap<String, String>,
        output_var: String,
        error_mode: ErrorMode,
    },
}

#[derive(Debug, Clone)]
struct VarDecl {
    name: String,
    required: bool,
    var_type: VarType,  // String | Number | Json | Blob
}

/// 副作用セット。risk_score は DeterminismScore の重み計算に使用
#[derive(Debug, Clone, Default)]
struct SideEffectSet {
    writes_external_api: bool,       // 外部 API 書き込み
    sends_notification: bool,        // 通知送信
    modifies_persistent_state: bool, // DB 等の永続状態変更
    /// true の場合は AG-03 ハードゲートでブロック
    irreversible: bool,
    /// [0.0, 1.0]: writes_external_api=1.0, DB変更=0.7, 通知=0.3
    risk_score: f32,
}

impl SideEffectSet {
    /// 副作用包含チェック: self が mission_required を包含するかどうか
    /// Stage 0 フィルタで使用 (§11.2)
    fn contains(&self, required: &SideEffectSet) -> bool {
        (!required.writes_external_api || self.writes_external_api)
            && (!required.sends_notification || self.sends_notification)
            && (!required.modifies_persistent_state || self.modifies_persistent_state)
    }
}

#[derive(Debug, Clone)]
enum ErrorMode {
    Fail,
    Skip,
    Retry { max_attempts: u32, backoff_secs: u32 },
}
```

### 6.2 EdgeMeta

```rust
#[derive(Debug, Clone)]
enum EdgeMeta {
    DependsOn,
    DataFlow { from_var: String, to_var: String },
    Conditional { condition_expr: String, branch: BranchLabel },
    FanOut { branch_id: usize },
    Collect { strategy: CollectStrategy },  // WaitAll | WaitFirst | Threshold(n)
}
```

### 6.3 WorkflowGraph 型宣言 (StableGraph)

```rust
use petgraph::stable_graph::{StableGraph, NodeIndex};

/// MUST: StableGraph を使用すること (P-06)
/// DiGraph はノード削除時に NodeIndex が無効化されるため使用禁止
type WorkflowGraph = StableGraph<WorkflowNode, EdgeMeta>;

struct WorkflowRegistry {
    graphs: HashMap<WorkflowId, Arc<WorkflowGraph>>,
}
```

### 6.4 バリデーション規則

| ID | 規則 | 実装 |
|----|------|------|
| V-01 | DAG 検証: `petgraph::algo::toposort` が Ok を返すこと | `toposort(&graph, None)` |
| V-02 | ノード ID 一意性: UUID の重複禁止 | HashMap による検査 |
| V-03 | DataFlow.from_var は送信ノードの output_var と一致すること | スコープ前向き走査 |
| V-04 | DataFlow.to_var は受信ノードの inputs に含まれること | スコープ照合 |
| V-05 | SubWorkflow の workflow_id は WorkflowRegistry に存在すること | Registry lookup |
| V-06 | SubWorkflow の input/output_mapping は Registry の spec と整合すること | Registry lookup |
| V-07 | FanOut と Collect の branch_id は 1 対 1 対応すること | branch_id 集合比較 |
| V-08 | 孤立ノード (入次数 = 出次数 = 0 かつ非ルート) は禁止 | 次数検査 |

---

## 7. Layer 2 → Layer 1 コンパイル

### 7.1 CompilerContext と compile_to_steps

```rust
struct CompilerContext {
    namespace_stack: Vec<String>,
    var_scope: HashMap<String, String>,  // 論理名 → 名前空間付き名
    visited: HashSet<WorkflowId>,        // 循環参照検出用
}

fn compile_to_steps(
    graph: &WorkflowGraph,
    registry: &WorkflowRegistry,
    ctx: &mut CompilerContext,
) -> Result<Vec<OpenFangStep>, CompileError> {
    let sorted = toposort(graph, None)
        .map_err(|e| CompileError::CycleDetected(e))?;
    let mut steps = Vec::new();
    for node_idx in sorted {
        match &graph[node_idx] {
            WorkflowNode::AgentStep { agent, prompt_template, inputs, output_var, .. } => {
                let ns_output_var = ctx.namespace_var(output_var);
                let ns_prompt = ctx.resolve_vars_in_template(prompt_template)?;
                ctx.validate_inputs(inputs)?;
                steps.push(OpenFangStep {
                    agent: agent.clone(),
                    prompt: ns_prompt,
                    output_var: ns_output_var,
                });
            }
            WorkflowNode::SubWorkflow { workflow_id, input_mapping, output_var, .. } => {
                if ctx.visited.contains(workflow_id) {
                    return Err(CompileError::CircularReference(workflow_id.clone()));
                }
                ctx.visited.insert(workflow_id.clone());
                ctx.push_namespace(workflow_id);
                ctx.apply_input_mapping(input_mapping)?;
                let sub_graph = registry.get(workflow_id)
                    .ok_or_else(|| CompileError::WorkflowNotFound(workflow_id.clone()))?;
                let sub_steps = compile_to_steps(sub_graph, registry, ctx)?;
                ctx.pop_namespace();
                ctx.visited.remove(workflow_id);
                let final_output = sub_steps.last()
                    .map(|s| s.output_var.clone())
                    .ok_or(CompileError::EmptySubWorkflow)?;
                ctx.bind_output(output_var, final_output)?;
                steps.extend(sub_steps);
            }
        }
        if steps.len() > MAX_STEPS {
            return Err(CompileError::StepCountExceeded(steps.len(), MAX_STEPS));
        }
    }
    Ok(steps)
}
```

**変数名前空間規則**: SubWorkflow 内の変数は `{workflow_uuid}/{original_var_name}` 形式で名前空間化する。  
**制限**: コンパイル出力 ≤ `MAX_COMPILED_STEPS = 256`。グラフノード数 ≤ `MAX_GRAPH_NODES = 512`。パッチ操作数 ≤ `MAX_PATCH_OPS = 64`。`ctx.visited` による循環参照検出は MUST。

> **注意 (v1.2)**: v1.1 まで `MAX_STEPS` が compile 出力・グラフサイズ・clone コストを兼ねていたが、v1.2 で責務別に分離した。付録 A の定数一覧を参照。

### 7.2 エラー列挙

```rust
#[derive(Debug, thiserror::Error)]
enum CompileError {
    #[error("Cycle detected: {0:?}")]
    CycleDetected(petgraph::algo::Cycle<NodeIndex>),
    #[error("Circular SubWorkflow reference: {0:?}")]
    CircularReference(WorkflowId),
    #[error("Workflow not found: {0:?}")]
    WorkflowNotFound(WorkflowId),
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    #[error("Empty SubWorkflow")]
    EmptySubWorkflow,
    #[error("Step count {0} exceeds limit {1}")]
    StepCountExceeded(usize, usize),
}
```

---

## 8. WorkflowRepository と MemoizedGraph

```rust
struct WorkflowRepository {
    graphs: Arc<RwLock<Vec<MemoizedGraph>>>,
    index:  Arc<RwLock<AnnIndex>>,  // HNSW ベース ANN インデックス
}

struct MemoizedGraph {
    id:               WorkflowGraphId,
    graph:                     WorkflowGraph,
    task_embedding:            Vec<f32>,    // ミッション/タスク記述の埋め込み
    workflow_design_text:      String,      // canonical workflow design text (§9)
    workflow_design_embedding: Vec<f32>,    // 構造記述の embedding
    agents_et_hash:            u64,         // 64bit FNV-1a (§12.1; v1.1 変更)
    trust:            TrustProfile,
    performance:      Metrics,
    provenance:       Provenance,
    lineage:          WorkflowLineage,
    contributions:    Vec<ContributionRecord>,
    last_virtual_seen: u64,
    experience_count:  u32,
    time_decay:        TimeDecayProfile,
    reputation:        ReputationProfile,
    gc_state:          GcState,
    tombstone_ref:     Option<TombstoneRef>,
    consistency_state: ConsistencyState,
    repair_epoch:      u64,
}

struct Metrics {
    success_rate:   f32,
    avg_latency_ms: u64,
    token_cost_avg: u32,
    run_count:      u32,
    last_run_at:    SystemTime,
}

struct Provenance {
    created_at:       SystemTime,
    last_used_at:     SystemTime,  // mark_used() のみが更新する
    last_verified_at: SystemTime,  // mark_verified() のみが更新する
    source_version:   String,
    environment_hash: u64,
}

struct EmbeddingChannelVersion {
    model_version: String,
    template_version: Option<String>,
}

struct DesignTextProvenance {
    generated_at:      SystemTime,
    generator_kind:    DesignTextGeneratorKind, // DeterministicFormatter | LlmFormatter
    generator_version: String,
    template_version:  String,
}

enum DesignTextGeneratorKind {
    DeterministicFormatter,
    LlmFormatter,
}

struct EmbeddingVersions {
    task:   EmbeddingChannelVersion,
    design: EmbeddingChannelVersion,
}

struct TimeDecayProfile {
    w_human:   f32,
    w_virtual: f32,
    lambda_human_use:   f32,
    lambda_human_verify: f32,
    lambda_virtual_use: u64,
    lambda_virtual_verify: u64,
    updated_at: SystemTime,
}

struct ReputationProfile {
    direct_score:     f32,
    indirect_score:   f32,
    experience_score: f32,
    inherited_score:  f32,
    final_score:      f32,
    alpha_positive:   u32,
    beta_negative:    u32,
    last_recomputed_at: SystemTime,
}

enum GcState {
    Active,
    SoftDeleted { since: SystemTime, reason: String },
    HardDeleteCandidate { since: SystemTime, consecutive_failures: u32 },
    Tombstoned { tombstone_id: String, since: SystemTime },
}

struct TombstoneRef {
    tombstone_id: String,
    deleted_at: SystemTime,
}

enum ConsistencyState {
    Committed,
    Pending { op_id: String, phase: CommitPhase },
    NeedsRepair { op_id: String, reason: String },
    Quarantined { op_id: String, since: SystemTime },
}

enum CommitPhase {
    MetaPrepared,
    BlobPrepared,
    MetaCommitted,
    BlobCommitted,
}

struct RepairLog {
    op_id: String,
    graph_id: WorkflowGraphId,
    detected_at: SystemTime,
    reason: String,
    action: RepairAction,
}

enum RepairAction {
    RetryMetaCommit,
    RetryBlobCommit,
    MarkQuarantined,
    ConvertToTombstone,
}

struct VirtualClockState {
    current: u64,
    updated_at: SystemTime, // UTC (MUST)
}

struct EnvironmentPolicy {
    environment_name: String,
    gc_theta_soft: f32,
    gc_theta_hard: f32,
    min_survival_experience: u32,
    reputation_weight: f32,
    inheritance_rate: f32,
    pressure_mode: PressureMode,
}

enum PressureMode {
    Normal,
    Constrained,
    Emergency,
}
```

### 8.1 Provenance / VirtualClock 更新関数

`last_used_at` と `last_verified_at` は別々の関数で更新する (MUST)。意図の混乱を防ぐため同一関数内での両方更新は禁止 (MUST NOT)。加えて v1.7 では、Human Time と独立した VirtualClock を単調増加で管理し、`last_virtual_seen` を別経路で更新しなければならない (MUST)。マシン停止のみを理由に仮想時間を進めてはならない (MUST NOT)。

```rust
/// 検索・実行のたびに呼ぶ。last_used_at のみ更新
fn mark_used(prov: &mut Provenance) {
    prov.last_used_at = SystemTime::now();
}

/// patchconfidence ≥ PATCH_CONFIDENCE_THRESHOLD かつ実行成功時のみ呼ぶ
/// last_verified_at をリセットし、DualTemporalTrust の t_verify → 1.0 にする
fn mark_verified(prov: &mut Provenance) {
    prov.last_verified_at = SystemTime::now();
    // NOTE: last_used_at は更新しない。呼び出し元で mark_used() を別途呼ぶこと
}

fn advance_virtual_clock(clock: &mut VirtualClockState, delta: u64) {
    assert!(delta > 0);
    clock.current = clock.current.saturating_add(delta);
    clock.updated_at = SystemTime::now();
}

fn mark_virtual_seen(graph: &mut MemoizedGraph, clock: &VirtualClockState) {
    graph.last_virtual_seen = clock.current;
}
```

### 8.2 cold-start 初期化 (P-07)

新規 MemoizedGraph をリポジトリに登録する際は、必ず cold-start trust で初期化しなければならない (MUST)。Trust が 0.0 のグラフを登録してはならない (MUST NOT)。また `gc_state = Active`、`experience_count = 0`、`last_virtual_seen = current_virtual_clock`、`reputation.final_score = REPUTATION_COLD_START` で初期化しなければならない (MUST)。

```rust
impl TrustProfile {
    /// 完全新規グラフ (Gold なし) の初期化
    fn cold_start_new() -> Self {
        TrustProfile {
            operational: TRUST_COLD_START_OPERATIONAL,  // 0.40
            semantic:    TRUST_COLD_START_SEMANTIC,      // 0.50
            temporal:    DualTemporalTrust::default(),   // score ≈ 1.0 (作成直後)
            human:       HumanTrustLogistic::default(),  // score = 0.50
        }
    }

    /// Gold グラフから Gnew を派生させる場合の初期化 (§12.5)
    /// TRUST_INHERIT_DECAY = 0.70 (OQ-10 参照)
    fn inherit_from_parent(parent: &TrustProfile, patch_confidence: f32) -> Self {
        // operational は親の 70% を引き継ぐが、TRUST_COLD_START_OPERATIONAL を下限とする
        // 根拠: 低信頼親から派生した Gnew が AG-04 ギリギリ通過することを防ぐ
        let inherited_op = (parent.operational * TRUST_INHERIT_DECAY)
            .max(TRUST_COLD_START_OPERATIONAL);  // = 0.40 (floor)
        TrustProfile {
            operational: inherited_op,
            semantic:    patch_confidence,
            temporal:    DualTemporalTrust::default(),
            human:       HumanTrustLogistic::default(),  // score = 0.50
        }
    }
}
```

**設計根拠**: cold-start operational = 0.40 は `TRUST_HARD_GATE_THRESHOLD (0.20)` の 2 倍であり、初回実行を許可しつつ過信を避けるプリオール値として設定する。実行実績が蓄積されるにつれて EMA により実際の成功率に収束する。

`inherit_from_parent` の `TRUST_INHERIT_DECAY = 0.70` の根拠は OQ-10 として管理する。Floor 適用により、たとえ親の operational が 0.0 であっても派生グラフは 0.40 から開始され、AG-04 (composite ≥ 0.20) を安定して通過できる。

**HumanTrust の Fast-track**: 権限を持つ管理者が明示的に承認した場合、`HumanTrustLogistic.score` を `TRUST_ADMIN_FAST_TRACK (0.80)` に設定することができる (MAY)。これは B2B 環境で人間フィードバックが 50 件蓄積されるまでの過渡期に活用する。

**監査ログ要件 (v1.2 追加)**: 管理者 fast-track を適用した場合、その操作を `TrustAuditLog` に記録しなければならない (SHOULD)。B2B 環境では MUST に引き上げることを推奨する。

```rust
struct TrustAuditLog {
    graph_id:     WorkflowGraphId,
    event_type:   TrustAuditEvent,
    actor_id:     String,       // 操作した管理者の ID
    old_value:    f32,
    new_value:    f32,
    timestamp:    SystemTime,
    reason:       Option<String>,
}

enum TrustAuditEvent {
    AdminFastTrack,
    ManualOverride,
    AbstractionRequested,
    AbstractionApplied,
    AbstractionRejected,
    DeterminismSamplingStarted,
    DeterminismSamplingCompleted,
    DeterminismEstimateUpdated,
    RefinementRunExecuted,
    HumanReviewApproved,
    HumanReviewRejected,
}

fn apply_admin_fast_track(
    graph: &mut MemoizedGraph,
    actor_id: String,
    audit_log: &mut Vec<TrustAuditLog>,
    reason: Option<String>,
) {
    let old_value = graph.trust.human.score;
    graph.trust.human.score = TRUST_ADMIN_FAST_TRACK;
    graph.invalidate_applicability_cache();
    audit_log.push(TrustAuditLog {
        graph_id:   graph.id.clone(),
        event_type: TrustAuditEvent::AdminFastTrack,
        actor_id,
        old_value,
        new_value:  TRUST_ADMIN_FAST_TRACK,
        timestamp:  SystemTime::now(),
        reason,
    });
}
```

### 8.3 SubWorkflow 資産化 (v1.7)

AbstractableSubgraph から切り出された部分グラフは、元グラフ内部の局所置換にとどめてはならず、新規 `WorkflowId` を持つ独立 `WorkflowGraph` として再構成し、`MemoizedGraph` として `WorkflowRepository` に登録しなければならない (MUST)。元グラフ側は `WorkflowNode::SubWorkflow` へ置換されるが、その参照先は元グラフ専用の匿名断片ではなく、他の Application Workflow / SearchWorkflow から再利用可能な共有資産として扱わなければならない (MUST)。

SubWorkflow 資産にも通常の graph 資産と同様に、`TrustProfile`、`WorkflowLineage`、`ContributionRecord`、`WorkflowDesignText / WorkflowDesignEmbedding`、`Metrics`、`TimeDecayProfile`、`ReputationProfile`、`GcState`、`experience_count` を付与しなければならない (MUST)。新規抽象化で生成された SubWorkflow は `Grace Period` の保護対象とし、観察前に GC してはならない (MUST NOT)。

```rust
fn register_abstracted_subworkflow(
    repo: &mut WorkflowRepository,
    subgraph: WorkflowGraph,
    parent_id: WorkflowGraphId,
    patch_confidence: f32,
    clock: &VirtualClockState,
) -> MemoizedGraph {
    let workflow_id = WorkflowGraphId::new();
    MemoizedGraph {
        id: workflow_id,
        graph: subgraph,
        task_embedding: vec![],
        workflow_design_text: String::new(),
        workflow_design_embedding: vec![],
        agents_et_hash: 0,
        trust: TrustProfile::inherit_from_parent_placeholder(patch_confidence),
        performance: Metrics::cold_start(),
        provenance: Provenance::new(),
        lineage: WorkflowLineage::derived_from(parent_id),
        contributions: vec![],
        last_virtual_seen: clock.current,
        experience_count: 0,
        time_decay: TimeDecayProfile::default(),
        reputation: ReputationProfile::cold_start(),
        gc_state: GcState::Active,
        tombstone_ref: None,
        version: 0,
    }
}
```

### 8.4 GraphVersion による楽観的並行性制御 (P-09)

`apply_patch_atomic` が複数スレッドから同一グラフに並列適用された場合、後勝ちによる更新消失を防ぐために楽観的並行性制御 (Optimistic Concurrency Control) を使用する。

```rust
/// MemoizedGraph にバージョンカウンタを持たせる
struct MemoizedGraph {
    // (既存フィールド...)
    version: u64,  // 新規追加: 更新のたびにインクリメント
}

#[derive(Debug, thiserror::Error)]
enum RepositoryError {
    #[error("Version conflict: expected {expected}, found {actual}")]
    UpdateConflict { expected: u64, actual: u64 },
    #[error("Graph not found: {0:?}")]
    NotFound(WorkflowGraphId),
    #[error("Cross-store inconsistency detected")]
    CrossStoreInconsistency,
}

impl WorkflowRepository {
    /// 楽観的更新: expected_version が現在バージョンと一致する場合のみ更新を適用
    async fn update_graph_cas(
        &self,
        graph_id: WorkflowGraphId,
        new_graph: WorkflowGraph,
        expected_version: u64,
    ) -> Result<u64, RepositoryError> {
        let mut store = self.graphs.write().await;
        let entry = store.iter_mut()
            .find(|g| g.id == graph_id)
            .ok_or(RepositoryError::NotFound(graph_id))?;
        if entry.version != expected_version {
            return Err(RepositoryError::UpdateConflict {
                expected: expected_version,
                actual:   entry.version,
            });
        }
        entry.graph   = new_graph;
        entry.version += 1;
        Ok(entry.version)
    }
}
```

**呼び出しパターン**: `apply_patch_atomic` を呼ぶ前に `graph.version` を読み取り、成功後に `update_graph_cas(id, new_graph, read_version)` で CAS 更新する。`UpdateConflict` が返った場合は最新バージョンで再試行すること (SHOULD)。

**設計根拠**: `RwLock` は読み取り多数・書き込みまれの前提で良好なスループットを提供する。`apply_patch_atomic` のクローン + バリデーションは純粋計算であり、ロックを保持したまま実行する必要はない。バージョン CAS はロック解放後の更新競合を検出する安全ネットとして機能する。


---

## 9. WorkflowDesignText / QueryDesignText

### 9.1 基本原則

v1.5 では、構造類似検索を検索段階へ復帰させるため、各 `MemoizedGraph` に `WorkflowDesignText` と `WorkflowDesignEmbedding` を保持しなければならない (MUST)。`WorkflowDesignEmbedding` は真の graph embedding ではなく、フォーマット規定された構造記述テキストの embedding を structural proxy として用いるものである。

専用 `graph_embedding` フィールド、GNN encoder、または graph neural retrieval path を RFC-0001 v1.6 の実装必須要件として追加してはならない (MUST NOT)。これらは RFC-0003 以降の拡張事項である。

新しい mission に対しても、実装は `task_embedding` に加え `QueryDesignText` と `query_design_embedding` を生成しなければならない (MUST)。ただし `QueryDesignText` は検索用スケッチであり、完全な `WorkflowGraph` や実行計画の仕様として扱ってはならない (MUST NOT)。

### 9.2 Canonical schema

`WorkflowDesignText` / `QueryDesignText` の出力フォーマットは 1 つに統一しなければならない (MUST)。v1.5 では埋め込み安定性を優先し、以下の JSON 風 canonical schema を推奨標準とする。

```json
{
  "workflow_purpose": "...",
  "ordered_stages": ["..."],
  "node_list": [
    {
      "id": "n1",
      "kind": "AgentStep|SubWorkflow",
      "agent": "...",
      "inputs": ["..."],
      "output": "...",
      "side_effects": ["..."],
      "determinism": 0.7
    }
  ],
  "edge_list": [
    {"from": "n1", "to": "n2", "type": "DependsOn|DataFlow|Conditional|FanOut|Collect"}
  ],
  "branch_merge_summary": ["..."],
  "required_agents": ["..."],
  "primary_input_types": ["String", "Json"],
  "primary_output_types": ["String"],
  "side_effect_summary": ["external_api_write"],
  "determinism_summary": {"min": 0.5, "weighted_softmin_prior": 0.62}
}
```

同一または意味的に等価な `WorkflowGraph` から生成される `WorkflowDesignText` は、ノード順序、セクション順序、キー名、粒度において可能な限り正規化されていなければならない (SHOULD)。余計な散文的説明を避け、主要ノード列、依存順序、分岐、集約、主要 I/O、副作用、決定論性を一定順序で列挙すること。

### 9.3 Stored workflow 生成手順

新規 workflow 登録時、Self-Refinement 後、またはグラフ構造変更後には、実装は `WorkflowGraph` から canonical `WorkflowDesignText` を再生成しなければならない (MUST)。LLM 非依存で deterministic に生成可能な部分は deterministic formatter で生成することを推奨し、LLM formatter を使う場合は JSON スキーマ検証と deterministic fallback formatter を備えること (SHOULD)。

### 9.4 Query 側生成手順

Mission 入力からは少なくとも `mission_text`、`task_embedding`、`query_design_text`、`query_design_embedding` を導出する。`QueryDesignText` は coarse search sketch であり、完全な実行 workflow の代替ではないことを明示し、ノード数・深さ・分岐数に上限を設けなければならない (MUST)。

```rust
struct QueryRepresentation {
    mission_text:            String,
    task_embedding:          Vec<f32>,
    query_design_text:       String,
    query_design_embedding:  Vec<f32>,
    design_template_version: String,
}
```

query sketch 生成コストは full workflow generation より十分小さくなければならない (MUST)。同一または高類似 mission に対しては `query_design_text` と `query_design_embedding` のキャッシュを許可する (MAY)。

---


### 9.5 Knowledge-Aware QueryDesignText Extension (v1.8)

Revision v1.8 extends QueryDesignText with optional knowledge-aware fields used only when the mission requires knowledge retrieval or knowledge mutation. The canonical query representation MAY include: `query_type` in {`episodic`, `canonical`, `hybrid`}; `freshness_requirement` in {`recent`, `stable`, `historical`, `mixed`}; `evidence_strictness` in {`light`, `strict`, `audit-grade`}; `origin_trace_required: bool`; and `drift_sensitivity` in {`ignore`, `prefer-latest`, `show-history`}.

These fields SHALL influence retrieval and evaluation policy but SHALL NOT change the structural meaning of WorkflowGraph. When omitted, the runtime MUST default to `query_type = hybrid`, `freshness_requirement = mixed`, `evidence_strictness = light`, `origin_trace_required = false`, and `drift_sensitivity = prefer-latest`.

The stored `QueryRepresentation` structure is extended as follows:

```rust
struct QueryRepresentation {
    mission_text: String,
    task_embedding: Vec<f32>,
    query_design_text: String,
    query_design_embedding: Vec<f32>,
    design_template_version: String,
    query_type: QueryType,
    freshness_requirement: FreshnessRequirement,
    evidence_strictness: EvidenceStrictness,
    origin_trace_required: bool,
    drift_sensitivity: DriftSensitivity,
}

enum QueryType { Episodic, Canonical, Hybrid }
enum FreshnessRequirement { Recent, Stable, Historical, Mixed }
enum EvidenceStrictness { Light, Strict, AuditGrade }
enum DriftSensitivity { Ignore, PreferLatest, ShowHistory }
```

The above extension is backward-compatible: any v1.7 query representation can be upgraded by populating the default values described above.
## 10. TrustProfile — 4 軸信頼モデルと時間二軸拡張

```rust
struct TrustProfile {
    operational: f32,               // [0, 1]: 実行成功率 EMA
    semantic:    f32,               // [0, 1]: 検索一致度 EMA
    temporal:    DualTemporalTrust, // Human Time + Virtual Time blended
    human:       HumanTrustLogistic, // 補助的 human trust
}
```

### 9.1 OperationalTrust・SemanticTrust (EMA)

```rust
fn update_operational_trust(trust: &mut f32, success: bool, alpha: f32) {
    let outcome = if success { 1.0 } else { 0.0 };
    *trust = (1.0 - alpha) * *trust + alpha * outcome;
}
```

- OperationalTrust EMA α = 0.15
- SemanticTrust EMA α = 0.10

### 10.2 DualTemporalTrust + Human Time / Virtual Time

v1.6 の TemporalTrust は `last_used_at` / `last_verified_at` に基づく Human Time 減衰を規定していたが、v1.7 ではこれを Human Time と Virtual Time の二軸へ拡張する。Human Time は外界・社会・情報鮮度の変化を、Virtual Time は `VirtualClock` により観測される Darvium 内部の進行を表す。マシン停止のみを理由に Virtual Time を進めてはならない (MUST NOT)。

Human Time の SystemTime は常に UTC として扱う (MUST)。タイムゾーン変換は行わず、UNIX epoch (1970-01-01T00:00:00Z) からの経過ミリ秒で一貫して表現する。

```rust
struct DualTemporalTrust {
    lambda_use:    f32,  // Human Time /分
    lambda_verify: f32,  // Human Time /分
    alpha_blend:   f32,  // Human use / verify blend
}

impl DualTemporalTrust {
    fn score_human(&self, last_used_at: SystemTime, last_verified_at: SystemTime) -> f32 {
        let now = SystemTime::now();
        let delta_use    = now.duration_since(last_used_at)
            .unwrap_or_default().as_secs_f32() / 60.0;
        let delta_verify = now.duration_since(last_verified_at)
            .unwrap_or_default().as_secs_f32() / 60.0;
        let t_use    = (-self.lambda_use    * delta_use).exp();
        let t_verify = (-self.lambda_verify * delta_verify).exp();
        self.alpha_blend * t_use + (1.0 - self.alpha_blend) * t_verify
    }
}

fn compute_virtual_freshness(
    current_virtual_clock: u64,
    last_virtual_seen: u64,
    lambda_virtual: f32,
) -> f32 {
    let delta = current_virtual_clock.saturating_sub(last_virtual_seen) as f32;
    (-lambda_virtual * delta).exp()
}

fn compute_temporal_freshness(
    trust: &DualTemporalTrust,
    decay: &TimeDecayProfile,
    prov: &Provenance,
    current_virtual_clock: u64,
    last_virtual_seen: u64,
) -> f32 {
    let fh = trust.score_human(prov.last_used_at, prov.last_verified_at);
    let fv = compute_virtual_freshness(
        current_virtual_clock,
        last_virtual_seen,
        decay.lambda_virtual_use as f32,
    );
    (decay.w_human * fh + decay.w_virtual * fv).clamp(0.0, 1.0)
}
```

**λ 設計意図 (v1.1 明記)**:

| パラメータ | 値 (/分) | 半減期 | 設計根拠 |
|-----------|---------|--------|---------|
| `λ_use` | 0.0001 | ≈ 6,930 分 (≈ 4.8 日) | 頻繁に使われるグラフは使用間隔が短いため、比較的速く減衰してよい |
| `λ_verify` | 0.00005 | ≈ 13,860 分 (≈ 9.6 日) | 手動検証は重いコストがかかる行為であり、一度検証されたグラフは長く信頼したい。**λ_verify < λ_use** であることが設計の意図 |

`λ_verify < λ_use` を保つことは不変条件であり、実装時に入れ替えてはならない (MUST NOT)。

**mark_verified() の呼び出し条件** (P-08 参照): patchconfidence ≥ `PATCH_CONFIDENCE_THRESHOLD` かつ実行成功の両方を満たす場合のみ呼ぶこと (MUST)。

### 10.3 HumanTrustLogistic

```rust
struct HumanTrustLogistic {
    score: f32,  // 初期値 0.50
    k:     f32,  // 学習率 0.08
    scale: f32,  // ロジスティックスケール 0.30
    count: u32,
}

impl HumanTrustLogistic {
    fn default() -> Self {
        Self { score: 0.50, k: HUMAN_TRUST_K, scale: HUMAN_TRUST_SCALE, count: 0 }
    }

    /// outcome: 1.0 = thumbs-up, 0.5 = partial, 0.0 = thumbs-down
    /// 将来拡張: 5段階評価は outcome = {0.0, 0.25, 0.5, 0.75, 1.0} にマッピング可能
    fn update(&mut self, outcome: f32) {
        let expected = 1.0 / (1.0 + (-(self.score - 0.5) / self.scale).exp());
        self.score = (self.score + self.k * (outcome - expected)).clamp(0.0, 1.0);
        self.count += 1;
    }
}
```

**Elo 昇格**: count ≥ 50 になった場合の Elo ベース評価システムへの移行は RFC-0003 に委譲。MVP では常に HumanTrustLogistic を使用する。

### 10.4 TrustProfile 複合スコア

```rust
impl TrustProfile {
    fn composite(&self, prov: &Provenance, decay: &TimeDecayProfile, current_virtual_clock: u64, last_virtual_seen: u64) -> f32 {
        0.35 * self.operational
        + 0.25 * self.semantic
        + 0.20 * compute_temporal_freshness(&self.temporal, decay, prov, current_virtual_clock, last_virtual_seen)
        + 0.20 * self.human.score
    }
}
```

| 軸 | 重み | 更新契機 |
|----|------|---------|
| Operational | 0.35 | 実行完了ごと |
| Semantic | 0.25 | 検索・適用ごとに意味的乖離を測定 |
| Temporal | 0.20 | 時間経過で自動減衰。mark_verified() でリセット |
| Human | 0.20 | ユーザフィードバック |

### 10.5 TrustUpdate 状態機械 (atomic 保証)

`MemoizedGraph` への信頼更新はすべて `update_trust()` 経由で行うこと (MUST)。直接フィールド代入禁止 (MUST NOT)。

**v1.1 変更**: `TrustUpdate::Operational` を呼ぶと内部で自動的に applicability キャッシュを無効化する。呼び出し元が `Applicability` を別途呼ぶ必要はない。

```rust
enum TrustUpdate {
    Operational(bool),  // true = success, false = failure
    Human(f32),         // outcome ∈ {0.0, 0.25, 0.5, 0.75, 1.0}
    Semantic(f32),      // semantic_deviation ∈ [0.0, 1.0]
}

impl MemoizedGraph {
    fn update_trust(&mut self, update: TrustUpdate) {
        match update {
            TrustUpdate::Operational(success) => {
                update_operational_trust(&mut self.trust.operational, success, 0.15);
                // Operational 更新時は常に applicability を無効化 (内部保証)
                self.invalidate_applicability_cache();
            }
            TrustUpdate::Human(outcome) => {
                let old_composite = self.trust.composite(&self.provenance);
                self.trust.human.update(outcome);
                let new_composite = self.trust.composite(&self.provenance);
                // Debounce: composite スコアが TRUST_DEBOUNCE_DELTA (0.05) 以上変動した
                // 場合のみキャッシュを無効化する。頻繁な非同期フィードバックによる
                // 不必要な再計算を防ぐ (OQ-11 参照)
                if (new_composite - old_composite).abs() >= TRUST_DEBOUNCE_DELTA {
                    self.invalidate_applicability_cache();
                }
            }
            TrustUpdate::Semantic(score) => {
                update_semantic_ema(&mut self.trust.semantic, score, 0.10);
                self.invalidate_applicability_cache();
            }
        }
    }
}
```

---

### 10A. TrainingTrust と評価分離 (v1.9)

v1.9 では、training feedback を production TrustProfile へ直接混入させないため、training evaluation channel を分離する。

```rust
struct TrainingTrustProfile {
    operational: f32,
    human: f32,
    curriculum_fit: f32,
    safety: f32,
}
```

`TrainingTrustProfile` は sandbox success・human feedback・curriculum 適合・安全性評価を集約する補助評価であり、production `TrustProfile` の代替ではない。promotion 時に training trust を production trust に直接コピーしてはならず (MUST NOT)、floor と decay を伴う限定継承のみを許可する (MUST)。

```rust
fn inherit_training_signal_to_production(
    parent: &TrustProfile,
    training: &TrainingTrustProfile,
) -> TrustProfile {
    let inherited_human = (parent.human.score * 0.70 + training.human * 0.30)
        .clamp(0.0, 1.0)
        .max(0.50);
    TrustProfile {
        operational: (parent.operational * TRUST_INHERIT_DECAY)
            .max(TRUST_COLD_START_OPERATIONAL),
        semantic: parent.semantic,
        temporal: DualTemporalTrust::default(),
        human: HumanTrustLogistic::from_score(inherited_human),
    }
}
```

上記係数は calibration candidate であるが、**training → production の直接コピー禁止**、**floor**、**decay**、**temporal freshness reset** の 4 原則は規範である。

## 11. Applicability Check

### 10.1 ハードゲート (AG)

以下の条件をすべて AND で検査し、1 つでも失敗したら即時拒否する (MUST)。

| ID | 条件 | 拒否理由 |
|----|------|---------|
| AG-01 | ミッションに必要なエージェントが現環境に存在すること | エージェント不在 |
| AG-02 | エージェントの capability バージョンが一致すること | capability 不整合 |
| AG-03 | `side_effects.irreversible == true` のノードが存在しないこと | 不可逆副作用 |
| AG-04 | `trust.composite() >= TRUST_HARD_GATE_THRESHOLD` | Trust 下限 |
| AG-05 | `trust.operational >= TRUST_OPERATIONAL_HARD_GATE` | Operational Trust 下限 |
| AG-06 | semantic channel (`task_embedding`) の model version が query / candidate 間で互換であること、または semantic score を無効化可能であること | semantic channel 不整合 |
| AG-07 | structural proxy channel (`workflow_design_embedding`) の model / template version が query / candidate 間で互換であること、または structural score を無効化可能であること | structural channel 不整合 |

**v1.1 変更**: 旧 AG-06「Trust が 0.0 でないこと」は P-07 (cold-start 初期化の義務) と §8.2 の実装によりシステム的に保証されるため、ハードゲート規則としては削除し P-07 に統合した。AG-06 は埋め込みモデルバージョン検査 (旧 AG-05) に番号を変更。

### 10.2 DeterminismScore D (SoftMin)

```
D(G) = (−1/β) × ln( Σᵢ (wᵢ/W) × exp(−β × dᵢ) )

wᵢ = base × side_effect_multiplier
  ExternalApiWrite → ×4.0
  FileWrite        → ×2.0
  Notification     → ×1.5
  None             → ×1.0

dᵢ = effective_determinism(node) ∈ [0.0, 1.0]
β  = SOFT_MIN_BETA = 5.0
```

```rust
impl WorkflowGraph {
    fn aggregate_determinism(&self, beta: f32) -> f32 {
        let weighted_exps: Vec<(f32, f32)> = self.node_weights()
            .filter_map(|n| {
                if let WorkflowNode::AgentStep { determinism, side_effects, .. } = n {
                    let w = compute_node_weight(side_effects);
                    let d = effective_determinism(*determinism, determinism_profile.as_ref());
                    Some((w, (-beta * d.clamp(0.0, 1.0)).exp()))
                } else { None }
            }).collect();
        if weighted_exps.is_empty() { return 1.0; }
        let w_sum: f32 = weighted_exps.iter().map(|(w, _)| w).sum();
        let log_sum: f32 = weighted_exps.iter()
            .map(|(w, e)| (w / w_sum) * e)
            .sum::<f32>().ln();
        (-log_sum / beta).clamp(0.0, 1.0)
    }
}
```

D < `DETERMINISM_THRESHOLD (0.50)` の場合、ミッションが非決定論的実行を許可していなければ拒否。

### 10.3 ApplicabilityScore A (幾何平均 + floor)

```
A(Gᵢ, Gⱼ) = ∏ₖ max(vₖ, floorₖ)^αₖ

  vS = Stotal(Gq, Gᵢ)  (§12.3)
  vD = D(Gᵢ)
  vT = trust.composite(..., current_virtual_clock, last_virtual_seen)

  floorS = APPLICABILITY_FLOOR_S = 0.10
  floorD = APPLICABILITY_FLOOR_D = 0.10
  floorT = APPLICABILITY_FLOOR_T = TRUST_HARD_GATE_THRESHOLD = 0.20
           ※ v1.1: floorT は TRUST_HARD_GATE_THRESHOLD と同値に定義。
              AG-04 を通過したグラフは必ず vT ≥ floorT を満たすため、
              floor の適用は冗長だが安全ネットとして保持する。

  αS = APPLICABILITY_ALPHA_S = 0.40
  αD = APPLICABILITY_ALPHA_D = 0.30
  αT = APPLICABILITY_ALPHA_T = 0.30
```

```rust
fn compute_applicability_score(similarity: f32, determinism: f32, trust: f32) -> f32 {
    let vs = similarity.max(APPLICABILITY_FLOOR_S);
    let vd = determinism.max(APPLICABILITY_FLOOR_D);
    let vt = trust.max(APPLICABILITY_FLOOR_T);  // = TRUST_HARD_GATE_THRESHOLD
    vs.powf(APPLICABILITY_ALPHA_S)
    * vd.powf(APPLICABILITY_ALPHA_D)
    * vt.powf(APPLICABILITY_ALPHA_T)
}
```

A ≥ `APPLICABILITY_THRESHOLD (0.50)` で再利用可 (Cost = 0)。A < 0.50 で GraphPatch 生成 (§12)。

### 10.4 フロー全体

```
Input: ミッション, 候補 MemoizedGraph Gᵢ
  ↓
[AG-01〜AG-07] ← 失敗 → REJECT
  ↓ 全通過
[DeterminismScore D] ← D < 0.50 かつ非許容 → REJECT
  ↓
[ApplicabilityScore A]
  A ≥ 0.50 → REUSE (Cost = 0)
  A < 0.50 → GraphPatch 生成 (§12)
```

---


### 11.5 Knowledge Applicability Extension (v1.8)

Revision v1.8 preserves the v1.7 workflow applicability score as `A_workflow` and adds a second-stage knowledge applicability score `K` when, and only when, the evaluated candidate invokes one or more knowledge primitives or declares knowledge-bound evidence requirements. If no knowledge primitive is present, the final applicability MUST be identical to the v1.7 value.

Knowledge applicability is computed from three bounded components: freshness `F_knowledge`, version alignment `V_knowledge`, and drift alignment `D_knowledge`. `F_knowledge` SHALL be derived from evidence freshness signals such as `Chunk.stale`, `CanonicalDocument.valid_from/valid_to`, `MemoryConcept.status`, concept supersession state, and event recency decay. `V_knowledge` SHALL capture whether the retrieved evidence matches the requested version context or validity interval. `D_knowledge` SHALL capture whether the evidence selection is compatible with the query drift policy (`ignore`, `prefer-latest`, `show-history`).

The knowledge applicability scalar SHALL be computed as:

\[
K = F_{knowledge}^{0.50} \cdot V_{knowledge}^{0.30} \cdot D_{knowledge}^{0.20} 	ag{1}
\]

The final applicability SHALL be computed as:

\[
A_{final} = A_{workflow}^{0.70} \cdot K^{0.30} 	ag{2}
\]

The runtime MUST use `A_final` for candidate selection whenever knowledge applicability is active. `A_workflow` MUST still be recorded in SearchTrace for debugging, replay, and calibration.

Knowledge applicability hard gates are defined as follows:

1. If `evidence_strictness = audit-grade` and `K < 0.30`, the candidate MUST NOT be selected for REUSE, PATCH, or COMPOSE and the SearchWorkflow MUST emit `NeedsHumanReview` or `AbortSearch` with an explicit reason.
2. If `origin_trace_required = true` and the candidate produces an empty evidence set or an incomplete trace root, the candidate MUST fail knowledge applicability regardless of `A_workflow`.
3. If all retrieved evidence is stale, superseded, invalid for the requested version interval, or incompatible with the declared drift policy, the candidate MUST be treated as knowledge-inapplicable even if its workflow applicability exceeds `APPLICABILITYTHRESHOLD`.

The default calibration constants in equations (1) and (2) are normative for v1.8. Future revisions MAY recalibrate them, but such recalibration MUST be treated as a versioned change to the applicability model rather than an implementation-local tuning.
## 12. Layer 3a — GMR Retrieval Core

### 11.1 agentsethash (64bit FNV-1a)

**v1.1 変更**: agentsethash を 32bit から 64bit に変更。M3 以降の 10 万件規模での衝突率を許容範囲 (<0.01%) に維持するため。

```rust
fn compute_agentset_hash(agents: &[String]) -> u64 {
    let mut sorted = agents.to_vec();
    sorted.sort();
    let combined = sorted.join("|");
    fnv1a64(combined.as_bytes())
}

fn fnv1a64(data: &[u8]) -> u64 {
    const FNV_PRIME:  u64 = 1_099_511_628_211;
    const FNV_OFFSET: u64 = 14_695_981_039_346_656_037;
    data.iter().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ (*byte as u64)).wrapping_mul(FNV_PRIME)
    })
}
```

**衝突確率** (Birthday Paradox):

| N件 | 32bit | 64bit |
|-----|-------|-------|
| 10,000 | 1.16% | <0.0001% |
| 100,000 | ≈50% | 0.027% |

MemoizedGraph 構造体の `agents_et_hash` フィールドは `u64` に変更済み (§8)。

### 12.2 4 ステージ検索

| Stage | 手法 | 計算量 | 目的 |
|-------|------|--------|------|
| Stage 0 | agentsethash 一致 + **副作用包含チェック** | O(N) | エージェント・副作用 不適合候補を排除 |
| Stage 1 | AG-04/05/06/07 通過チェック | O(N') | Trust・embedding channel version フィルタ |
| Stage 2a | ANN (HNSW): task_embedding で top-k_sem | O(log N) | semantic 候補取得 |
| Stage 2b | ANN (HNSW): workflow_design_embedding で top-k_struct | O(log N) | structural proxy 候補取得 |
| Stage 2c | union + dedupe | O(k) | semantic / structural 候補統合 |
| Stage 3 | GED 近似 + 抽象化候補検出 + reranking | O(k) | 精密構造評価と抽象化誘発点の同定 |
| Stage 4 | ApplicabilityScore 計算・閾値判定 | O(k) | 最終再利用可否 |

**Stage 0 副作用包含チェック (v1.1 変更)**:  
旧仕様の「完全一致」から「包含チェック」に変更。候補グラフの副作用セットがミッション要求副作用を包含する場合のみ通過する。

```
通過条件: mission_required.side_effects ⊆ candidate.aggregated_side_effects
```

これにより、ミッションが `writes_external_api=false` を要求する場合に `writes_external_api=true` の候補が排除されず、パッチ生成により副作用ノードを削除した形で再利用できる。

**推奨値**: `ANN_TOP_K_SEM = 10`, `ANN_TOP_K_STRUCT = 10`。評価コストに応じて独立調整してよいが、主仕様は Dual ANN + Union Rerank とする。

### 11.3 類似度統合式と GED 境界スムージング

**v1.6 注記**: 本節の structural path は `workflow_design_embedding` と GED 近似のみを対象とする。専用 `graph_embedding` cosine・GNN reranker・graph encoder 学習は本 RFC の規範対象外であり、SearchWorkflow からも呼び出してはならない (MUST NOT)。

```
Stotal(Gᵢ, Gⱼ) = (1 − α) × Ssem + α × Sstruct   (α = 0.35)

Ssem    = cosine(task_embedding_i, task_embedding_j)
Sstruct = GED_normalized または AbstractableSubgraph-aware GED 近似
```

**v1.4 方針**: graph embedding cosine への切替は削除済みとし、GED と部分グラフ抽象化候補の検出に一本化する。大規模グラフでは `GED_GRAPH_SIZE_LIMIT` を超えた場合に `GraphNeedsAbstraction` として自己抽象化パスへ送る。

```rust
enum StructuralMatch {
    GedScore(f32),
    GraphNeedsAbstraction { candidates: Vec<AbstractableSubgraph> },
}
```

### 12.3A GED 近似アルゴリズム選択方針 (v1.9 補足)

GED は NP 困難であるため、本 RFC は近似使用を前提とする。v1.9 では、実装間のブレを抑えるため、近似アルゴリズムの選択基準を次のように補足規範化する。

1. **大規模候補の高速 rerank** では、transport-based approximation またはそれと同等の assignment/OT 系近似を第一選択とすることを推奨する (SHOULD)。これは Stage 2 の ANN 候補 union 後に多数候補を粗く並べ替える用途で、速度と安定性を優先するためである。
2. **中規模候補の精密比較** では、beam search 系近似または edit path 探索系近似を用いてよい (MAY)。これは top-n 候補の局所精査、PatchExisting / ComposeExisting 境界判断、abstractable subgraph の同定など、精度をやや優先したい場面を対象とする。
3. **大規模グラフ** では、`GED_GRAPH_SIZE_LIMIT` を超えた時点で完全比較志向の近似を打ち切り、`GraphNeedsAbstraction` へ送らなければならない (MUST)。

### 12.3B 推奨プロファイル

| プロファイル | 想定用途 | 推奨近似 | 目的 |
|---|---|---|---|
| `fast-rerank` | Stage 3 の候補粗順位付け | transport / OT 系 | 速度優先、top-k 圧縮 |
| `balanced-validate` | top-n 候補の再比較 | beam search 系 | 速度と精度の均衡 |
| `abstraction-trigger` | 大規模・高複雑度 graph | size gate + subgraph extraction | GED 深追いを避け抽象化へ送る |
| `patch-audit` | patch proposal の局所妥当性確認 | 局所 beam / edit path | 説明可能な差分確認 |

### 12.3C 規範要件

- 実装は、どの近似アルゴリズムをどの profile で使用したかを `SearchTrace` または同等の replay 可能メタデータに記録することが望ましい (SHOULD)。
- 同一 deployment 内で GED 近似戦略を silently 変更してはならない (MUST NOT)。変更時は ANN / applicability / patch quality への影響を replay で確認し、バージョン付き migration note を残すこと。
- beam width、transport regularization、最大展開数などの細部パラメータは implementation-tunable だが、**fast-rerank / balanced-validate / abstraction-trigger** の責務分離は規範として保持することを推奨する。

---


## 12A. Knowledge Primitive Registry (v1.8)

Revision v1.8 introduces a normative registry of knowledge access primitives. These primitives are first-class workflow operations executed through the same safety, timeout, determinism, and audit framework that governs AgentStep and SubWorkflow execution. Knowledge primitives are divided into read-only primitives and mutation primitives.

### 12A.1 Primitive Set

The initial v1.8 registry SHALL contain the following primitive identifiers:

- `memorygetrecentevents`
- `memorygetconcepts`
- `memorygetconcepthistory`
- `memorytraceorigin`
- `memorypromotetodocument`
- `skilllistchildren`
- `skillgetchunks`
- `skillexpandentities`
- `skillbacktrack`
- `kbhybridsearch`

All primitives except `memorypromotetodocument` SHALL be treated as read-only by default. `memorypromotetodocument` SHALL be treated as a knowledge mutation primitive that modifies persistent knowledge state. Additional primitives MAY be added in later revisions only through a registry update that declares side effects, determinism expectations, idempotency, and evidence output behavior.

### 12A.2 Workflow IR Integration

A workflow node MAY declare a knowledge primitive through the following extension to `WorkflowNode::AgentStep` metadata:

```rust
enum KnowledgePrimitiveKind {
    MemoryGetRecentEvents,
    MemoryGetConcepts,
    MemoryGetConceptHistory,
    MemoryTraceOrigin,
    MemoryPromoteToDocument,
    SkillListChildren,
    SkillGetChunks,
    SkillExpandEntities,
    SkillBacktrack,
    KbHybridSearch,
}

enum FreshnessLevel { Recent, Stable, Historical, Mixed }
enum EvidenceOutputType { None, IdsOnly, IdsWithMeta, IdsWithChunks }
```

When a knowledge primitive is attached to an AgentStep, the step SHALL additionally declare `requires_freshness_level`, `evidence_output_type`, and an idempotency class. Read-only primitives SHOULD be marked idempotent unless the underlying store cannot guarantee stable pagination or stable ranking under equal inputs. Mutation primitives MUST be marked non-idempotent unless an explicit operation fingerprint is used to deduplicate repeated writes.

### 12A.3 Evidence Bundle Contract

Every successful knowledge primitive invocation SHALL normalize its output into the following contract before control returns to SearchWorkflow evaluation:

```rust
struct KnowledgeEvidenceBundle {
    evidence_ids: Vec<String>,
    version_context: VersionContext,
    freshness_summary: FreshnessSummary,
    confidence_meta: ConfidenceMeta,
    origin_trace_ids: Vec<String>,
}
```

`evidence_ids` SHALL contain the stable identifiers of the knowledge objects that justify the step result. `version_context` SHALL capture validity and version metadata required to replay or audit the step. `freshness_summary` SHALL summarize stale flags, validity window compliance, and aggregate freshness score. `confidence_meta` SHALL summarize ranking and retrieval signals such as vector similarity, BM25 score, hybrid score, and hit counts. `origin_trace_ids` SHALL contain the transitive origin chain when traceability is requested or available.

### 12A.4 Mutation Safety Rule

Knowledge mutation primitives MUST be review-gated. A mutation primitive MUST NOT be executed unless all of the following hold:

1. `A_final >= APPLICABILITYTHRESHOLD`.
2. `K >= 0.50`.
3. The calling workflow satisfies the v1.7 trust hard gates.
4. If `origin_trace_required = true`, the mutation request contains a non-empty evidence bundle with traceable origin IDs.

If any condition fails, SearchWorkflow MUST transition to `NeedsHumanReview` or `AbortSearch` and MUST record the failure reason in SearchTrace.

### 12A.5 SearchTrace Extension

SearchTrace and SearchRunLog are extended with the following optional fields when knowledge primitives are active:

- `knowledge_evidence_ids: Vec<String>`
- `knowledge_version_context: Option<VersionContext>`
- `knowledge_freshness_summary: Option<FreshnessSummary>`
- `knowledge_query_mode: Option<QueryType>`
- `origin_trace_ids: Vec<String>`

These fields are additive and backward-compatible. Replays of legacy v1.7 runs MAY leave them empty.

**v2.3-c 補足:** The following primitives are the standard conversational memory path: `memorygetrecentevents`, `memorygetconcepts`, `memorygetconcepthistory`, `memorytraceorigin`, `memorypromotetodocument`. These primitives serve as the deterministic wrappers for conversational fragment retrieval, trace back, and promotion to canonical document. New conversational-specific primitives are not required; the existing primitive set accommodates the conversational knowledge path through policy-governed classification and deterministic gating at the ingestion layer. `kbhybridsearch` MAY additionally be used for semantic cross-modal discovery of conversational fragments.

## 13. Layer 3b — SearchWorkflow Engine

### 13.1 基本原則

v1.6 では、v1.5 の GMR 検索をそれ自体で最終意思決定を返す機構として扱うのではなく、SearchWorkflow が呼び出す `RetrievalPrimitive` として再定義する。SearchWorkflow は Application Workflow を探索対象とするメタワークフローであり、REUSE / PATCH / COMPOSE / NEW / ABORT の outcome 空間を bounded search として扱わなければならない (MUST)。

SearchWorkflow は **検索そのものを first-class workflow operation として扱う**。すなわち、mission を受け取って単発の候補検索を行うだけでなく、query 表現の再構成、candidate sufficiency の判定、構成的合成、再検索、終了判定までを明示的な状態遷移で記述しなければならない (MUST)。v1.7 ではさらに、`GcState != Active` の候補を既定で検索候補から除外し、`ReputationProfile.final_score` と `LifecycleScore` を検索再順位付けに利用してよい (MAY)。ただし `SoftDeleted` 資産の復活判定や lineage 参照のための閲覧経路は残さなければならない (MUST)。 同様に `consistency_state != Committed` の資産は SearchWorkflow の既定候補集合から除外しなければならない (MUST)。ただし repair worker と監査系 read path からの参照は許可してよい (MAY)。

### 13.2 SearchWorkflowGraph

SearchWorkflow は通常の WorkflowGraph と同様に DAG として表現してよいが、そのノード種別は探索専用でなければならない (SHOULD)。最小構成として以下の探索ステップを持つ。 v1.7 追補として、`SearchStep` は Application Workflow の `WorkflowNode` と同一グラフ内で混在させないことを推奨する (SHOULD)。すなわち `SearchWorkflowGraph` は概念上 `WorkflowGraph` と並列の探索専用 IR であり、共通の DAG 制約や validation style を共有してよいが、ノード種別レベルでは分離を保つ。

```rust
#[derive(Debug, Clone)]
enum SearchStep {
    BuildQueryStep,
    RetrieveCandidatesStep,
    EvaluateCandidatesStep,
    RefineSearchPolicyStep,
    RequeryDecisionStep,
    ComposeCandidatesStep,
    PatchProposalStep,
    NewWorkflowProposalStep,
    SelectOutcomeStep,
    RecordSearchTraceStep,
    GuardStep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchState {
    Init,
    Retrieve,
    Evaluate,
    Refine,
    Compose,
    ProposeNew,
    Finalize,
    Abort,
}
```

### 13.3 SearchWorkflow データモデル

```rust
struct SearchBudget {
    max_iterations:      u32,
    max_retrieval_calls: u32,
    max_prompt_tokens:   u64,
    max_wall_clock_ms:   u64,
}

struct RecursionGuard {
    max_depth:       u32,
    current_depth:   u32,
    allow_reentrant: bool,
}

struct SearchTrace {
    search_run_id:          String,
    iteration:              u32,
    query_text:             String,
    query_design_text_hash: u64,
    retrieval_top_k_sem:    u32,
    retrieval_top_k_struct: u32,
    candidate_ids:          Vec<WorkflowGraphId>,
    selected_gold:          Option<WorkflowGraphId>,
    selected_outcome:       Option<SearchOutcome>,
    budget_snapshot:        SearchBudgetSnapshot,
    justification:          String,
}

struct SearchBudgetSnapshot {
    iterations_used:      u32,
    retrieval_calls_used: u32,
    prompt_tokens_used:   u64,
    wall_clock_ms_used:   u64,
}

enum SearchOutcome {
    ReuseExisting { graph_id: WorkflowGraphId },
    PatchExisting { graph_id: WorkflowGraphId, patch: GraphPatch },
    ComposeExisting { plan: CompositionPlan },
    GenerateNew { proposal: WorkflowGraph },
    AbortSearch { reason: SearchAbortReason },
    NeedsHumanReview { reason: String },
}

struct CompositionPlan {
    component_graph_ids: Vec<WorkflowGraphId>,
    composition_edges:   Vec<(NodeId, NodeId, EdgeMeta)>,
    expected_inputs:     Vec<VarDecl>,
    expected_output:     VarDecl,
    confidence:          f32,
}
```

### 13.4 RetrievalPrimitive 契約

SearchWorkflow は Stage 0–4 の GMR を `RetrievalPrimitive::search_workflows()` として呼び出すこと (MUST)。RetrievalPrimitive は候補集合と rerank 結果を返す pure retrieval contract であり、REUSE / PATCH / COMPOSE / NEW の最終決定権を持ってはならない (MUST NOT)。

SearchOutcome の最終選択は `EvaluateCandidatesStep` / `RefineSearchPolicyStep` による bounded heuristic policy として扱う。v1.6 では COMPOSE / NEW / ABORT の分岐閾値を単一の数理式に固定せず、`SearchTrace` に判定根拠を残すこと、予算と安全ガードを破らないこと、同一 Fake 入力に対して deterministic replay 可能であることを優先要件とする。

```rust
trait RetrievalPrimitive {
    fn search_workflows(
        &self,
        query: &QueryRepresentation,
        policy: &RetrievalPolicy,
    ) -> Result<CandidateSet, RetrievalError>;
}

struct RetrievalPolicy {
    top_k_sem:      u32,
    top_k_struct:   u32,
    min_trust:      f32,
    allow_compose:  bool,
    allow_new:      bool,
}

struct CandidateSet {
    candidates: Vec<RankedCandidate>,
    retrieval_calls_used: u32,
}
```

### 13.5 状態遷移規則

SearchWorkflow は以下の有向状態機械に従わなければならない (MUST)。`Finalize` と `Abort` は終端状態であり、終端後に再遷移してはならない (MUST NOT)。

```text
Init -> Retrieve -> Evaluate
Evaluate -> Finalize        (REUSE / PATCH が十分)
Evaluate -> Compose         (単独候補では不十分だが組成候補あり)
Evaluate -> Refine          (候補不足・policy 改善が必要)
Compose -> Finalize         (COMPOSE 成立)
Compose -> Refine           (compose 不成立)
Refine -> Retrieve          (requery)
Refine -> ProposeNew        (既存候補再利用の期待値が低い)
ProposeNew -> Finalize      (NEW 採択)
任意状態 -> Abort           (budget / recursion / unsafe transition)
```

`Refine -> Retrieve -> Refine` が閾値回数を超えて往復する場合、実装は `SearchPolicyOscillation` として検出し `AbortSearch` または `NeedsHumanReview` に落とさなければならない (MUST)。

COMPOSE / NEW の選好ロジックは v1.6 では policy layer に属し、Theorem としては扱わない。FakeImpl と SearchTrace によって replay・比較・監査可能であることが主要品質要件である。

### 13.6 ガード条件

- SearchBudget の上限超過時は `SearchBudgetExceeded` を返し、`Abort` へ遷移すること (MUST)。
- RecursionGuard の深さ超過時は `SearchRecursionExceeded` を返し、SearchWorkflow は SearchWorkflow を再入してはならない (MUST)。
- side-effect safety invariant に反する SearchStep 遷移、たとえば review-gated でない実プロバイダ呼び出しを `GenerateNew` で即採択する経路は `UnsafeSearchTransition` として拒否すること (MUST)。
- `GenerateNew` および `ComposeExisting` の実 execution は review-gated とし、少なくとも M3 までは proposal validity のみを評価対象としてよい (SHOULD)。

### 13.7 SearchRunLog

SearchWorkflow 実行単位は `SearchRunLog` に永続化し、SearchTrace と lineage を相互参照可能にしなければならない (SHOULD)。

```rust
struct SearchRunLog {
    run_id:               String,
    mission_text:         String,
    started_at:           SystemTime,
    finished_at:          Option<SystemTime>,
    final_outcome:        Option<SearchOutcomeKind>,
    final_graph_id:       Option<WorkflowGraphId>,
    iterations_used:      u32,
    retrieval_calls_used: u32,
    prompt_tokens_used:   u64,
    wall_clock_ms_used:   u64,
    aborted_reason:       Option<String>,
}

enum SearchOutcomeKind {
    Reuse,
    Patch,
    Compose,
    New,
    Abort,
    NeedsHumanReview,
}
```


---

### 13A. Training Orchestrator (v1.9)

v1.9 は SearchWorkflow の bounded search state machine を保持したまま、その外側に human-guided training loop を配置する Training Orchestrator を推奨する。mission review、feedback ingestion、promotion review は探索状態機械そのものとは責務が異なるため、SearchWorkflow に無理に混入させるべきではない。

```text
TrainingInit
  → MissionIntake
  → HumanMissionReview
  → SandboxQueue
  → SandboxExecute(SearchWorkflow)
  → ResultReport
  → HumanFeedback
  → PromotionReview
  → {Promote | Reject | Archive | RetryTraining}
```

**規範要件**

1. mission は `AiGenerated` と `HumanSubmitted` の二系統に加え、`ReplayFromProduction` と `DerivedFromFailure` を受け入れなければならない。
2. human mission review は少なくとも `approve`、`reject`、`edit mission text`、`adjust priority`、`add human-supplied missions`、`merge duplicates` をサポートしなければならない。
3. sandbox execution は SearchWorkflow を内部 primitive として利用してよいが、`require_fake_impl_first = true` の場合、real provider 実行前に FakeImpl / deterministic doubles を通さなければならない。
4. result reporting は mission、selected outcome、使用 workflow / subworkflow、成否、latency / token cost / side-effects summary、生成 candidate の有無を人間に報告しなければならない。
5. human feedback は `Good / Bad / NeedsRevision / Irrelevant / Unsafe` を付与できなければならず、trust / audit / tagging / promotion / curriculum bias に接続しなければならない。
6. training 成果を production に反映するには PromotionCandidate を経由しなければならない。

### 13B. Human Communication Patterns (v1.9)

v1.9 は、人間向け自然言語インタラクションを formal object に対応づけることを規範的に重視する。少なくとも次の prompt pattern を想定する。

- 「自主トレーニングとして以下のミッションを試したい。不要なミッションを削除してください。」
- 「必要であれば追加ミッションも入力してください。」
- 「優先度を変更したいものがあれば調整してください。」
- 「以下の training run を完了した。Good/Bad/NeedsRevision/Irrelevant/Unsafe を選んでください。」
- 「改善してほしい観点があれば短く追記してください。」
- 「production に昇格させたい候補があれば選んでください。」

これらは UX 表現であるが、その背後では `TrainingMission`、`TrainingFeedback`、`PromotionCandidate` などの formal object に必ず変換されなければならない (MUST)。

## 14. Layer 2.5 — グラフパッチ生成

### 12.1 PatchOperation / GraphPatch

```rust
#[derive(Debug, Clone)]
enum PatchOperation {
    AddNode    { node: WorkflowNode },
    RemoveNode { node_id: NodeId },
    ReplaceNode { node_id: NodeId, new_node: WorkflowNode },
    AddEdge    { from: NodeId, to: NodeId, meta: EdgeMeta },
    RemoveEdge { from: NodeId, to: NodeId },
    UpdatePrompt        { node_id: NodeId, new_prompt: String },
    UpdateInputMapping  { node_id: NodeId, new_mapping: HashMap<String, String> },
}

struct GraphPatch {
    source_graph_id:   WorkflowGraphId,
    operations:        Vec<PatchOperation>,
    patch_confidence:  PatchConfidence,
    generated_at:      SystemTime,
    generator_version: String,
}
```

### 12.2 GraphPatchGenerator と LLM 自己評価スコア cₛ

```rust
struct GraphPatchGenerator {
    llm_client: Arc<dyn LlmClient>,
    validator:  Arc<PatchValidator>,
    history:    Arc<PatchHistory>,
}

impl GraphPatchGenerator {
    async fn generate(
        &self,
        gold: &WorkflowGraph,
        mission: &MissionSpec,
        applicability: f32,
    ) -> Result<GraphPatch, PatchError> {
        let diff_spec  = self.compute_diff_spec(gold, mission);
        let (self_score, raw_ops) = self.llm_generate(&diff_spec).await?;
        let val_score  = self.validator.score(&raw_ops, gold, mission)?;
        let hist_score = get_history_score_with_prior(&self.history, &diff_spec);
        let confidence = PatchConfidence::compute(self_score, val_score, hist_score);
        if confidence.value < PATCH_CONFIDENCE_THRESHOLD {
            return Err(PatchError::LowConfidence(confidence.value));
        }
        Ok(GraphPatch {
            source_graph_id: gold.id.clone(),
            operations: raw_ops,
            patch_confidence: confidence,
            generated_at: SystemTime::now(),
            generator_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}
```

**LLM Planner への入力コンテキスト**:

```rust
struct PatchGenerationContext {
    mission:                   String,
    source_graph_dot:          String,   // Gold グラフの DOT 形式
    node_summaries:            Vec<NodeSummary>,
    low_applicability_nodes:   Vec<NodeId>,
    unsafe_side_effect_nodes:  Vec<NodeId>,
    required_capabilities:     Vec<String>,
}
```

**cₛ (LLM 自己信頼スコア) の仕様 (v1.1 追加)**:

LLM の過信頼バイアス (参照文献 9, 10) に対処するため、以下のプロセスで cₛ を取得する。

1. **プロンプト設計**: LLM に対し「生成したパッチ操作列が元のミッション要件を満たすか、0.0〜1.0 のスコアで評価せよ。スコアが低いほど変更が不完全または危険であることを意味する」と明示する。
2. **verbalized confidence の正規化**: LLM 出力の confidence 値は `[0.0, 1.0]` にクランプし、JSON スキーマ `{"patch_ops": [...], "self_confidence": float}` で構造化出力を要求する。
3. **過信頼補正**: 初期運用フェーズ (Milestone M2) では `cₛ_adjusted = cₛ × SELF_CONF_DISCOUNT (0.85)` で補正する。M2 以降に validator スコアとの乖離データが蓄積し次第、補正係数を実績ベースで調整する (OQ-03)。
4. **wₛ の非対称化オプション (SHOULD)**: cₛ < 0.50 の場合は wₛ を 0.30 → 0.20 に動的引き下げ、wᵥ を 0.40 → 0.50 に引き上げることができる。これにより LLM が「自信なし」と言っているケースでバリデータ評価を優先できる。

### 12.3 PatchConfidence (幾何平均 + cold-start prior)

```
patchconfidence = cₛ_adjusted^wₛ × cᵥ^wᵥ × cₕ^wₕ

wₛ = 0.30 (cₛ < 0.50 時は動的に 0.20 へ引き下げ可)
wᵥ = 0.40 (cₛ < 0.50 時は動的に 0.50 へ引き上げ可)
wₕ = 0.30
cₕ の cold-start prior = 0.50
```

```rust
struct PatchConfidence {
    value:           f32,
    self_score:      f32,
    validator_score: f32,
    history_score:   f32,
}

impl PatchConfidence {
    fn compute(self_score: f32, val_score: f32, hist_score: f32) -> Self {
        const EPS: f32 = 0.01;
        let cs_adj = (self_score * SELF_CONF_DISCOUNT).max(EPS);
        let cv = val_score.max(EPS);
        let ch = hist_score.max(EPS);
        // 非対称重み調整: LLM が低自信の場合は validator を優先
        // 動的重み切り替え規則 (v1.2 規範化):
        // cₛ < 0.50: LLM が明示的に低自信 → validator 優先 (ws=0.20, wv=0.50)
        // cₛ ≥ 0.50: 通常重み (ws=0.30, wv=0.40)
        // 切り替え閾値 0.50 は PATCH_SELF_CONF_SWITCH_THRESHOLD として定数管理
        let (ws, wv) = if self_score < PATCH_SELF_CONF_SWITCH_THRESHOLD {
            (PATCH_CONFIDENCE_WS_LOW, PATCH_CONFIDENCE_WV_HIGH)  // (0.20, 0.50)
        } else {
            (PATCH_CONFIDENCE_WS, PATCH_CONFIDENCE_WV)  // (0.30, 0.40)
        };
        let wh = 0.30f32;
        let value = cs_adj.powf(ws) * cv.powf(wv) * ch.powf(wh);
        Self { value, self_score, validator_score: val_score, history_score: hist_score }
    }
}

fn get_history_score_with_prior(history: &PatchHistory, diff_spec: &DiffSpec) -> f32 {
    history.success_rate_for_similar(diff_spec).unwrap_or(PATCH_CONFIDENCE_PRIOR)
}
```

**バリデータスコア cᵥ 計算**:
- 未解決変数 1 件ごとに −0.15 (上限 3 件)
- DataFlow 辺の一貫性違反: −0.15
- [0.0, 1.0] にクランプ

**閾値**: `PATCH_CONFIDENCE_THRESHOLD = 0.75`

### 12.4 apply_patch_atomic (P-08)

**v1.1 変更**: `apply_patch` を `apply_patch_atomic` に改名・再設計。clone → apply all → validate → swap の 4 フェーズにより途中失敗時にグラフを元の状態に保つ (P-08)。

```rust
/// Atomic パッチ適用。途中失敗時は gold を変更しない。
fn apply_patch_atomic(
    gold: &WorkflowGraph,
    patch: &GraphPatch,
) -> Result<WorkflowGraph, PatchError> {
    // フェーズ1: clone (gold は不変)
    let mut g_candidate = gold.clone();
    // フェーズ2: 全操作を順次適用 (失敗時は g_candidate を drop して終了)
    for op in &patch.operations {
        apply_operation(&mut g_candidate, op)?;
        // NOTE: 失敗時は ? によりここで return Err(...)。gold は未変更のまま。
    }
    // フェーズ3: 事後バリデーション
    validate_patch_result(&g_candidate)?;
    // フェーズ4: swap (バリデーション通過後のみ caller に返す)
    Ok(g_candidate)
}

fn validate_patch_result(graph: &WorkflowGraph) -> Result<(), PatchError> {
    // DAG 検証
    toposort(graph, None).map_err(|_| PatchError::CycleCreated)?;
    // 変数スコープ検証
    validate_var_scope(graph)?;
    // SubWorkflow 参照検証
    validate_subworkflow_refs(graph)?;
    Ok(())
}
```

**設計根拠**: `StableGraph` の clone コストは O(V+E) であり、ワークフローの規模 (≤ `MAX_GRAPH_NODES = 512` ノード) では許容範囲。将来的にコピーオンライト最適化が必要な場合は RFC-0003 以降で検討する。

**CAS 更新パターン** (P-09 と連携):

```rust
async fn patch_and_register(
    repo: &WorkflowRepository,
    gold_id: WorkflowGraphId,
    patch: &GraphPatch,
    parent_trust: &TrustProfile,
    patch_conf: f32,
) -> Result<WorkflowGraphId, RepositoryError> {
    // 1. gold を読み取り、バージョンを記録
    let (gold_graph, gold_version) = repo.read_with_version(gold_id).await?;
    // 2. atomic パッチ適用 (pure computation; ロック不要)
    let new_graph = apply_patch_atomic(&gold_graph, patch)
        .map_err(|_| RepositoryError::NotFound(gold_id))?;
    // 3. CAS 更新 (バージョン不一致なら UpdateConflict)
    // Gnew は新規 ID で登録するため、競合は gold への直接更新時のみ発生
    let new_id = WorkflowGraphId::new_v4();
    repo.insert_derived(new_id, new_graph,
        TrustProfile::inherit_from_parent(parent_trust, patch_conf)).await?;
    Ok(new_id)
}
```

### 12.5 TrustUpdate 連携

```rust
async fn execute_with_trust_update(
    graph: &mut MemoizedGraph,
    patch: &GraphPatch,
    result: &ExecutionResult,
) {
    let success        = result.is_success();
    let confidence_met = patch.patch_confidence.value >= PATCH_CONFIDENCE_THRESHOLD;

    // mark_used は実行のたびに呼ぶ
    mark_used(&mut graph.provenance);

    if success && confidence_met {
        graph.update_trust(TrustUpdate::Operational(true));
        // mark_verified は成功 + 信頼度充足の場合のみ
        mark_verified(&mut graph.provenance);
    } else if !success {
        graph.update_trust(TrustUpdate::Operational(false));
        // 失敗時は mark_verified を呼ばない (last_verified_at は更新しない)
    }
    // NOTE: update_trust(Operational(...)) の内部で applicability cache を無効化済み
}
```

### 12.6 エラー列挙

```rust
#[derive(Debug, thiserror::Error)]
enum PatchError {
    #[error("Low confidence {0:.3} below threshold {}", PATCH_CONFIDENCE_THRESHOLD)]
    LowConfidence(f32),
    #[error("Patch creates a cycle")]
    CycleCreated,
    #[error("Variable scope violation: {0}")]
    VarScopeViolation(String),
    #[error("SubWorkflow reference missing: {0:?}")]
    SubworkflowRefMissing(WorkflowId),
    #[error("Source graph not found: {0:?}")]
    SourceGraphNotFound(WorkflowGraphId),
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeId),
}
```

---

## 15. Layer 3c — Lifecycle / Natural Selection / GC

### 15.1 基本原則

v1.7 では、WorkflowRepository を単なる保存箱ではなく、再利用可能資産の生態系として扱う。特に AbstractableSubgraph から生成された SubWorkflow は、局所最適化の副産物ではなく共有資産であり、検索・合成・継承・淘汰の対象として Lifecycle 管理を受けなければならない (MUST)。

GC は単純削除処理ではなく、自然淘汰として定義する。平時の長期選別と、resource pressure 下の淘汰加速を同一状態機械で扱い、瞬間的ノイズで消えないよう連続低スコア条件を持たせなければならない (MUST)。 また、SubWorkflow 資産化は無制限に行ってはならず、environment policy は 1 mission あたりの抽象化上限、最小再利用予兆、ANN index 増分上限の少なくとも 1 つを持つべきである (SHOULD)。

### 15.2 時間二軸モデル

Human Time は外界の変化、情報鮮度、社会的陳腐化を表す。全ての Human Time は UTC を基準とし (MUST)、UNIX epoch からの経過ミリ秒で表現する。Virtual Time は Darvium 内部でどれだけイベントが進行したかを表し、`VirtualClock` の増分だけで進める。

時間鮮度の基準式は次とする。

\[
F_H(G)=\exp(-\lambda_H \Delta t^H(G))
\]

\[
F_V(G)=\exp(-\lambda_V \Delta c(G))
\]

\[
F_{time}(G)=w_{human}(G)F_H(G)+w_{virtual}(G)F_V(G)
\]

ここで `w_human(G) + w_virtual(G) = 1.0` を不変条件とする (MUST)。`w_human` / `w_virtual` は workflow ごとの `TimeDecayProfile` に保持し、生成時に初期化し、Patch / Refinement / Abstraction 後に再推定してよい (SHOULD)。ML 学習は導入せず、heuristic と LLM 補助、ならびに実行履歴からの数理的更新で調整する。

### 15.3 互恵性ベース評判

人間 thumbs-up/down は補助的に残すが、v1.7 における主たる評判形成は workflow 間の互恵関係から構成する。少なくとも直接互恵性と間接互恵性を区別して保持しなければならない (MUST)。

```rust
struct ReciprocityEdge {
    from: WorkflowGraphId,
    to: WorkflowGraphId,
    useful_calls: u32,
    harmful_calls: u32,
    compose_count: u32,
    patch_help_count: u32,
    updated_at: SystemTime,
}
```

間接互恵性は利用ネットワーク上の PageRank 的中心性またはそれと同等の決定論的中心性指標で近似してよい (MAY)。ただし popularity のみで高得点化しないよう、成功率・有害依存・失敗波及を負の寄与として組み込むことを推奨する (SHOULD)。

経験値補正済み評判の代表式は次とする。

\[
R_{exp}(G)=rac{\alpha \cdot (1-e^{-k(\alpha+eta)})}{\alpha+eta}
\]

ここで `α` は有益な再利用・有益 compose・正の間接寄与、`β` は有害再利用・失敗伝播・負の寄与を表す。`α + β = 0` の場合は `REPUTATION_COLD_START` を返すこと (MUST)。

### 15.4 Experience / Grace Period

各資産は `experience_count` を持つ。これは少なくとも成功実行、失敗実行、他 workflow からの再利用、Compose への寄与、Patch 親としての寄与により増加させなければならない (MUST)。

`experience_count < MIN_SURVIVAL_EXPERIENCE` の間、当該資産を `SoftDeleted` または `HardDeleteCandidate` へ遷移させてはならない (MUST NOT)。ただしセキュリティ事故・不可逆副作用・明白な破損グラフに対する緊急隔離は別扱いとし、通常 GC と混同してはならない (MUST NOT)。

### 15.5 LifecycleScore

各 graph に対して 0〜1 の生存スコア `L(G)` を定義する。

\[
L(G)=F_{time}(G)^{\alpha_T}\cdot s(G)^{\alpha_S}\cdot T(G)^{\alpha_C}\cdot U(G)^{\alpha_U}\cdot R(G)^{\alpha_R}
\]

- `F_time(G)`: Human / Virtual blended freshness
- `s(G)`: success_rate または成功履歴由来の安定度
- `T(G)`: composite trust
- `U(G)`: run_count / reuse_count / contribution count 由来の使用度
- `R(G)`: 互恵性ベース評判

```rust
fn compute_lifecycle_score(
    time_freshness: f32,
    success_signal: f32,
    trust_signal: f32,
    usage_signal: f32,
    reputation_signal: f32,
) -> f32 {
    time_freshness.powf(LIFECYCLE_ALPHA_TIME)
        * success_signal.powf(LIFECYCLE_ALPHA_SUCCESS)
        * trust_signal.powf(LIFECYCLE_ALPHA_TRUST)
        * usage_signal.powf(LIFECYCLE_ALPHA_USAGE)
        * reputation_signal.powf(LIFECYCLE_ALPHA_REPUTATION)
}
```

`L(G)` は hard gate ではなく、Lifecycle / GC の判定と SearchWorkflow の再順位付けに用いる。ApplicabilityScore と責務を混同してはならない (MUST NOT)。

### 15.6 GC 状態遷移

GC 状態は少なくとも次の状態を持たなければならない (MUST)。

- `Active`
- `SoftDeleted`
- `HardDeleteCandidate`
- `Tombstoned`

遷移規則は次を基準とする。

```text
Active -- L(G) < THETA_SOFT and grace-exited and consecutive_low >= N --> SoftDeleted
SoftDeleted -- L(G) >= THETA_RESTORE --> Active
SoftDeleted -- L(G) < THETA_HARD and retention_elapsed and refcount == 0 --> HardDeleteCandidate
HardDeleteCandidate -- delete/tombstone transaction success --> Tombstoned or physical delete
```

`SoftDeleted` は検索候補集合から除外されるが、Repository 内には残す。`HardDeleteCandidate` は lineage・SearchTrace・TrustAuditLog・SubWorkflow 参照整合性を満たすまでは物理削除してはならない (MUST NOT)。歴史参照が必要な環境では tombstone を残すことを推奨する (SHOULD)。

### 15.7 親からの継承

子 graph は `inherit_from_parent` と整合する形で、TrustProfile だけでなく ReputationProfile と ExperienceCount の一部を継承してよい (MAY)。ただし完全継承は禁止し、必ず `INHERITANCE_RATE < 1.0` の減衰コピーでなければならない (MUST)。

```rust
fn inherit_reputation(parent: &ReputationProfile, rate: f32) -> ReputationProfile {
    ReputationProfile {
        direct_score:     parent.direct_score * rate,
        indirect_score:   parent.indirect_score * rate,
        experience_score: parent.experience_score * rate,
        inherited_score:  parent.final_score * rate,
        final_score:      (parent.final_score * rate).clamp(0.0, 1.0),
        alpha_positive:   ((parent.alpha_positive as f32) * rate) as u32,
        beta_negative:    ((parent.beta_negative as f32) * rate) as u32,
        last_recomputed_at: SystemTime::now(),
    }
}
```

継承率が高すぎると格差固定化、低すぎると親資産価値の断絶が起きるため、実装値は environment policy ごとに調整し、Open Questions では tuning 問題として管理する。

### 15.8 Resource Pressure と環境別ポリシー

ストレージ、メモリ、CPU、ANN インデックス容量などの圧迫度を観測し、`PressureMode::Normal | Constrained | Emergency` を判定して GC 強度を変化させる。

- Normal: `THETA_SOFT`, `THETA_HARD`, `MIN_SURVIVAL_EXPERIENCE` を通常値で運用。
- Constrained: `THETA_SOFT`, `THETA_HARD` を引き上げ、低価値資産の soft delete を早める。
- Emergency: 必要なら `MIN_SURVIVAL_EXPERIENCE` を一時的に引き下げて若年資産にも淘汰圧をかけるが、監査ログを必須とする。
- HNSW / ANN index node count, resident memory, graph blob size は `ResourcePressure` の観測対象に含めることを推奨する (SHOULD)。

本番・検証・実験・ローカル開発などの環境差分は `EnvironmentPolicy` で切り替える。VirtualClock 自体は環境ごとに独立させてよいが、同一 environment 内では巻き戻してはならない (MUST NOT)。

### 15.9 SocialAcceleration

DGMV の社会加速度概念を、Darvium では「他資産への貢献がシステム全体の進化速度を高める指標」として翻訳する。これは runtime gate ではなく、GC / 評判 / SearchWorkflow 調整の上位 KPI である。

少なくとも次の観測量を定義してよい (MAY)。

- REUSE / PATCH / COMPOSE / NEW 比率の推移
- SubWorkflow 資産の再利用頻度増加率
- false-new rate の低下
- success_rate 改善速度
- 1 mission あたり平均 token cost の低下

SocialAcceleration は tuning 指標であり、ApplicabilityScore や LifecycleScore の代替にしてはならない (MUST NOT)。

### 15A. Training-specific Lifecycle / GC semantics (v1.9)

training artifacts も放置すると肥大化するため、Lifecycle / GC の対象に含めなければならない。ただし本番 workflow と training-only workflow は寿命管理の意味が異なるため、同一パラメータで GC してはならない (MUST NOT)。

```rust
enum TrainingArtifactState {
    TrainingOnly,
    PromotionCandidate,
    Promoted,
    Rejected,
    Tombstoned,
}
```

少なくとも以下の区別が必要である。

- training-only graph
- promotion candidate
- promoted graph
- rejected artifact

training-only graph は、一定期間再利用されず、feedback も悪く、promotion 可能性も低いなら SoftDeleted または Tombstoned に遷移してよい。一方、replay dataset、mission lineage、promotion audit、feedback log は graph tombstone 後も監査目的で保持されなければならない (MUST)。

## 16. GMR / SearchWorkflow / Lifecycle 実行フロー全体

### 15.1 SearchWorkflow 全体フロー

```text
Input: mission, SearchBudget, RecursionGuard
  ↓
[BuildQueryStep] → QueryRepresentation
  ↓
[RetrievalPrimitive(Stage 0–4)]
  ↓
[EvaluateCandidatesStep]
  ├─ A ≥ threshold かつ単独候補で十分 → REUSE / PATCH
  ├─ 単独候補は不十分だが補完候補あり → COMPOSE
  ├─ 候補不足だが再検索余地あり → REFINE → REQUERY
  └─ 既存候補の期待値が低い → NEW / ABORT
  ↓
[RecordSearchTraceStep]
  ↓
[Finalize or Abort]
```

### 15.2 Retrieval Core 呼び出し点

SearchWorkflow は `RetrieveCandidatesStep` のたびに RetrievalPrimitive を呼ぶが、その呼び出し回数は `SearchBudget.max_retrieval_calls` で上限化しなければならない (MUST)。Compose と New proposal は retrieval の代替ではなく、Evaluate の後段にある outcome proposal である。

### 15.3 v1.5 互換フロー

以下の v1.5 フローは SearchWorkflow が `ReuseExisting` または `PatchExisting` を選んだ場合の内部 primitive としてそのまま保持する。

```
[ミッション入力]
      │
      ▼
Stage 0: agentsethash 一致 + side_effects 包含チェック
      │ 不適合 → [新規ワークフロー生成]
      ▼
Stage 1: AG-01〜AG-07 ハードゲート
      │ 全候補失敗 → [新規ワークフロー生成]
      ▼
Stage 2: ANN top-k (task_embedding + graph_embedding)
      │ k = 10 候補
      ▼
Stage 3: Sstruct 計算 (GED近似 + 境界スムージング)
      │ Stotal 計算
      ▼
Stage 4: ApplicabilityScore A = f(Stotal, D, T)
      │
      ├── A ≥ 0.50 ──────────────────→ [REUSE]
      │                                  mark_used() + mark_verified()
      │                                  update_trust(Operational(true))
      │                                  compile_to_steps → OpenFang
      │
      └── A < 0.50 ──→ [GraphPatchGenerator.generate(Gold, mission)]
                             │
                      patchconfidence ≥ 0.75
                             │ YES
                             ▼
                       apply_patch_atomic(Gold, patch) → Gnew
                       TrustProfile::inherit_from_parent(Gold, pc)
                       compile_to_steps(Gnew) → OpenFang
                       execute_with_trust_update()
                             │ NO
                             ▼
                       PatchError::LowConfidence → 人間レビュー要求
```

---

### 16.4 Knowledge-Aware Candidate Evaluation (v1.8)

When SearchWorkflow evaluates a candidate that includes knowledge primitives, the `EvaluateCandidatesStep` SHALL execute the following additional substeps after workflow applicability and before final outcome selection:

1. Collect the normalized `KnowledgeEvidenceBundle` from each knowledge primitive node participating in the candidate.
2. Compute `F_knowledge`, `V_knowledge`, and `D_knowledge` from the aggregated evidence set.
3. Compute `K` and `A_final` according to Section 11.5.
4. Apply the knowledge hard gates.
5. Persist `A_workflow`, `K`, `A_final`, evidence IDs, version context, freshness summary, and origin trace IDs into SearchTrace.

This extension SHALL NOT change the legal SearchState transitions introduced in v1.6/v1.7. It refines candidate evaluation inside the existing `Evaluate` state only. Thus, the v1.7 invariants regarding bounded search, recursion guard, unsafe transition rejection, and deterministic replay remain in effect.

When multiple candidates are otherwise tied on `A_final`, the runtime SHOULD prefer the candidate with the higher evidence completeness under the declared `evidence_strictness`, then the candidate with the stronger origin trace completeness, and only then the lower-cost candidate. This tie-break order is normative for v1.8 knowledge-aware selection.

## 16A. Training Plane 実行フロー全体 (v1.9)

### v2.3 補足: safe sandbox scope auto-approval

Training Plane は human review を中心規範として保持するが、safe sandbox scope に限定された artifact については optional な Auto-Approval Exception Policy を導入してもよい (MAY)。ただし、この policy は audit log、policy ID、scope boundary、no-production-promotion constraint を伴い、training / production separation を弱めてはならない (MUST NOT)。


```text
Input domain / failure replay / curriculum signal / human mission
  → Build TrainingMission
  → HumanMissionReview
  → Approve / Reject / Edit / Merge
  → For Approved mission:
       SandboxPolicy check
       Fake-first dry-run (optional but policy-gated)
       SearchWorkflow execute
       ResultReport
       HumanFeedback
       PromotionCandidate generate (optional)
       PromotionReview
       {Promote | Reject | Archive | RetryTraining}
  → TrainingLifecycle / GC
```

training mode では `allownew = true`、`allowcompose = true`、patch proposal 積極化、fake-first 優先、knowledge mutation の sandbox namespace 限定、side-effect heavy path の review-gated 化が推奨される。ただしこれは SearchPolicy の傾向を変えるものであり、AG hard gate、knowledge hard gate、dual-store consistency、unsafe path rejection を無効化することを意味しない。

production への昇格では、少なくとも sandbox success、human feedback、side-effect profile、evidence / origin trace、patch confidence または manual approval、CAS / consistency / audit を満たさなければならない。

### 16A.1 Human review queue 運用補足 (v1.9 補足)

Human review queue の UI / notification backend 自体は implementation-specific であっても、review 遅延が Training Plane のスループット・鮮度・優先度制御を壊さないよう、最低限の運用方針を定めることを推奨する。

- `review_timeout_secs` を超えて未処理の mission は `Pending` のまま放置せず、再通知または reviewer escalation を行うことが望ましい (SHOULD)。
- `Critical` / `High` priority mission は FIFO のみで裁かず、priority-aware dequeue を用いることを推奨する。
- 同種 mission が多数滞留している場合、batch approval / batch rejection / duplicate merge をサポートすることが望ましい (SHOULD)。
- 一定時間を超えて feedback が返らない completed run は curriculum bias へ強く反映してはならず、`feedback_pending` 相当の中立状態として扱うことを推奨する。

```rust
struct HumanReviewQueuePolicy {
    review_timeout_secs: u64,
    escalation_timeout_secs: u64,
    max_batch_review_size: u32,
    priority_aware_dequeue: bool,
    allow_batch_approval: bool,
}
```

この方針は human reviewer の応答遅延を Training Plane の不可視なボトルネックにしないための補助規範であり、TrainingMission / TrainingRunLog / PromotionCandidate の formal object 設計を補完する。

`HUMAN_REVIEW_TIMEOUT_SECS`、`HUMAN_REVIEW_ESCALATION_SECS`、`HUMAN_REVIEW_MAX_BATCH_SIZE` の推奨初期値は付録 A に記載されており、運用条件に応じて Annex E の方針に従い再キャリブレーションしてよい。

**v2.3-c 補足:** Conversational ingestion MAY be a target of the safe sandbox scope Auto-Approval Exception Policy, provided that:
- The ingested artifact remains within sandbox namespace (MUST).
- No conversational event, fragment, or candidate knowledge object may directly mutate production canonical knowledge (MUST NOT).
- Promotion auto-approval for conversational origin knowledge is prohibited (MUST NOT).
- The existing promotion discipline, trust review, origin-trace requirements, and dual-store consistency protocol apply without modification (MUST).

## 16B. Conversational Knowledge Path (v2.3-c)

Revision v2.3-c extends the four-plane logical architecture by formalizing a conversational knowledge path — a policy-governed pipeline through which human conversation with Darvium can, under explicit deterministic gate control, produce sandbox-scoped CandidateKnowledgeDocuments and, after meeting promotion gates, CanonicalDocuments.

This extension is strictly additive. It does not add new knowledge primitives to the existing §12A registry; it uses the existing `memorygetrecentevents`, `memorygetconcepts`, `memorygetconcepthistory`, `memorytraceorigin`, and `memorypromotetodocument` primitives as its retrieval and promotion instrumentation. It does not redefine the Training Plane's human review, sandbox isolation, promotion discipline, dual-store consistency, or fusion semantics.

The conversational knowledge path SHALL NOT rely on trigger phrases as the primary admission mechanism. LLM-based policy-conditioned classification is the standard proposal mechanism; deterministic gates are the standard enforcement mechanism.

#### Architecture overview

The conversational knowledge path extends the four-plane architecture by adding a vertical ingestion layer that spans all existing planes:

```text
   ┌──────────────────────────────────────────────────────────────────┐
   │              Conversational Ingestion Layer (v2.3-c)             │
   │  ConversationalEvent → LLM Proposal → Deterministic Gate →      │
   │  TrainingMission / Fragment / CandidateKnowledgeDocument         │
   └──────────────────────────────────────────────────────────────────┘
                                   │
   ┌──────────────────────────────────────────────────────────────────┐
   │                  Workflow Orchestration Plane                    │
   │  SearchWorkflow · GMR Retrieval · Patch · Compose · New · ABORT │
   └──────────────────────────────────────────────────────────────────┘
                                   │
   ┌──────────────────────────────────────────────────────────────────┐
   │               Knowledge Access Primitive Plane                   │
   │  memorygetrecentevents · memorygetconcepts · kbhybridsearch ·    │
   │  memorytraceorigin · memorypromotetodocument                     │
   └──────────────────────────────────────────────────────────────────┘
                                   │
   ┌──────────────────────────────────────────────────────────────────┐
   │               Knowledge Persistence Plane                        │
   │  LadybugDB: Fragment · MemoryEvent · CanonicalDocument           │
   │  SQLite: lineage · audit · trust · lifecycle metadata            │
   └──────────────────────────────────────────────────────────────────┘
                                   │
   ┌──────────────────────────────────────────────────────────────────┐
   │                  Training Plane (extended by v2.3-c)             │
   │  TrainingMission · Sandbox Execution · Feedback · Promotion      │
   │  CandidateKnowledgeDocument · CurriculumPolicy                   │
   └──────────────────────────────────────────────────────────────────┘
```

### 16B.1 Conversational Knowledge Ingestion

This section formalizes the entry point of conversational knowledge ingestion.

#### Required types

```rust
struct ConversationalEvent {
    event_id: String,
    session_id: String,
    user_id: String,
    actor: ConversationActor,
    utterance: String,
    timestamp: SystemTime,
    language: String,
    context_window_id: Option<String>,
    parent_event_ids: Vec<String>,
    source_channel: ConversationChannel,
}

enum ConversationActor {
    Human,
    Darvium,
    System,
}

enum ConversationChannel {
    Chat,
    VoiceTranscript,
    ImportedLog,
    EmailBridge,
    Api,
}

struct ConversationalIngestionPolicy {
    policy_id: String,
    namespace_template: String,
    allow_auto_sandbox_ingest: bool,
    require_human_review_for_promotion: bool,
    max_candidate_span_days: u32,
    min_policy_score: f32,
    min_promotion_score: f32,
    allow_raw_transcript_persistence: bool,
    pii_handling: PiiHandlingPolicy,
    retention: RetentionPolicy,
    category_rules: Vec<ConversationCategoryRule>,
    updated_at: SystemTime,
}

struct ConversationCategoryRule {
    category: ConversationalKnowledgeCategory,
    allowed_namespace_suffix: String,
    auto_ingest_to_sandbox: bool,
    eligible_for_consolidation: bool,
    eligible_for_promotion: bool,
    require_origin_trace: bool,
    minimum_distinct_events: u32,
    minimum_distinct_days: u32,
    minimum_llm_confidence: f32,
}

enum ConversationalKnowledgeCategory {
    UserProfile,
    UserPreference,
    LongLivedProjectContext,
    StableConstraint,
    TemporaryTaskContext,
    FactualClaim,
    Reflection,
    RelationshipFact,
    Noise,
    Unsafe,
    Unknown,
}

struct RetentionPolicy {
    raw_event_ttl_days: u32,
    sandbox_candidate_ttl_days: u32,
    rejected_candidate_tombstone_hours: u32,
}

enum PiiHandlingPolicy {
    Reject,
    RedactBeforePersist,
    AllowSandboxOnly,
}

struct ConversationalClassificationProposal {
    event_id: String,
    proposed_category: ConversationalKnowledgeCategory,
    policy_score: f32,
    llm_confidence: f32,
    rationale_summary: String,
    proposed_namespace: String,
    extractive_facts: Vec<String>,
    inferred_temporality: InferredTemporality,
    inferred_scope: InferredScope,
    contains_pii: bool,
    promotion_eligibility_hint: PromotionEligibilityHint,
}

enum InferredTemporality {
    Ephemeral,
    Stable,
    Historical,
    Mixed,
}

enum InferredScope {
    Personal,
    Project,
    Global,
    Ambiguous,
}

enum PromotionEligibilityHint {
    Never,
    SandboxOnly,
    ReviewRequired,
    PotentiallyPromotable,
}
```

#### Normative text

Conversational ingestion MUST NOT rely on trigger phrases as the primary admission mechanism. Implementations SHALL evaluate conversational events through a policy-conditioned classification proposal process in which an LLM or equivalent semantic reasoner assesses long-term reuse value, category, scope, temporality, privacy risk, and promotion eligibility under an explicit ingestion policy.

If `proposed_category` is `Noise` or `Unsafe`, the event MUST NOT proceed to knowledge mutation.

If `contains_pii` is true, the system SHALL follow `PiiHandlingPolicy`: `Reject` drops the event; `RedactBeforePersist` requires normalized facts to be redacted before any persistence; `AllowSandboxOnly` permits unredacted storage within sandbox scope only.

If `allow_auto_sandbox_ingest` is true, its effect is limited to safe sandbox scope. Immediate promotion to production canonical knowledge is NOT permitted.

### 16B.2 LLM-driven Classification and Deterministic Gate

This section formalizes the separation of responsibilities between the LLM proposal and the deterministic gate.

#### Required types

```rust
struct ConversationalGateDecision {
    event_id: String,
    action: ConversationalGateAction,
    target_namespace: Option<String>,
    normalized_facts: Vec<String>,
    reason_code: String,
    requires_human_review: bool,
    created_mission: Option<String>,
}

enum ConversationalGateAction {
    Drop,
    StoreRawEventOnly,
    StoreFragmentOnly,
    CreateTrainingMission,
    CreateTrainingMissionAndFragment,
    QueueForConsolidation,
}
```

#### Decision procedure

The following pseudocode SHALL serve as the normative decision procedure for the deterministic ingestion gate:

```rust
fn decide_conversational_ingest(
    event: &ConversationalEvent,
    proposal: &ConversationalClassificationProposal,
    policy: &ConversationalIngestionPolicy,
) -> ConversationalGateDecision {
    if matches!(proposal.proposed_category, ConversationalKnowledgeCategory::Noise | ConversationalKnowledgeCategory::Unsafe) {
        return drop_decision(event, "CATEGORY_REJECTED");
    }

    if proposal.contains_pii {
        match policy.pii_handling {
            PiiHandlingPolicy::Reject => return drop_decision(event, "PII_REJECTED"),
            PiiHandlingPolicy::RedactBeforePersist => {
                // normalized_facts must be redacted before any persistence
            }
            PiiHandlingPolicy::AllowSandboxOnly => {}
        }
    }

    if proposal.policy_score < policy.min_policy_score {
        return drop_decision(event, "POLICY_SCORE_TOO_LOW");
    }

    let rule = lookup_category_rule(policy, proposal.proposed_category);
    if proposal.llm_confidence < rule.minimum_llm_confidence {
        return ConversationalGateDecision {
            event_id: event.event_id.clone(),
            action: ConversationalGateAction::CreateTrainingMission,
            target_namespace: Some(proposal.proposed_namespace.clone()),
            normalized_facts: proposal.extractive_facts.clone(),
            reason_code: "LOW_CONFIDENCE_REVIEW_REQUIRED".into(),
            requires_human_review: true,
            created_mission: Some(new_training_mission_id()),
        };
    }

    if rule.auto_ingest_to_sandbox && policy.allow_auto_sandbox_ingest {
        return ConversationalGateDecision {
            event_id: event.event_id.clone(),
            action: ConversationalGateAction::CreateTrainingMissionAndFragment,
            target_namespace: Some(proposal.proposed_namespace.clone()),
            normalized_facts: proposal.extractive_facts.clone(),
            reason_code: "SANDBOX_AUTO_INGEST".into(),
            requires_human_review: false,
            created_mission: Some(new_training_mission_id()),
        };
    }

    ConversationalGateDecision {
        event_id: event.event_id.clone(),
        action: ConversationalGateAction::CreateTrainingMission,
        target_namespace: Some(proposal.proposed_namespace.clone()),
        normalized_facts: proposal.extractive_facts.clone(),
        reason_code: "REVIEW_GATED_INGEST".into(),
        requires_human_review: true,
        created_mission: Some(new_training_mission_id()),
    }
}
```

#### Editorial requirement

The classification proposal MAY be nondeterministic, but persistence, state transition, namespace assignment, promotion eligibility, and canonical exposure SHALL be governed by deterministic gates, auditable state transitions, and existing training-production separation invariants.

The following diagram illustrates the boundary between the LLM's nondeterministic proposal role and the deterministic gate's enforcement role:

```text
   ┌──────────────────────────────────┐   ┌──────────────────────────────────┐
   │   LLM (may be nondeterministic)  │   │   Deterministic Gate (code path)  │
   │                                  │   │                                  │
   │   ConversationalEvent ───────────┼──>│   decide_conversational_ingest() │
   │   │                              │   │   ├─ Category check (Noise/Unsafe)│
   │   ▼                              │   │   ├─ PII handling policy         │
   │   ClassificationProposal         │   │   ├─ Policy score threshold      │
   │   ├─ proposed_category           │   │   ├─ LLM confidence threshold   │
   │   ├─ policy_score                │   │   ├─ Auto-ingest eligibility     │
   │   ├─ llm_confidence              │   │   │                              │
   │   ├─ contains_pii                │   │   ▼                              │
   │   ├─ proposed_namespace          │   │   ConversationalGateDecision     │
   │   └─ extractive_facts            │   │   ├─ Drop                       │
   │                                  │   │   ├─ StoreRawEventOnly           │
   │                                  │   │   ├─ CreateTrainingMission       │
   │                                  │   │   └─ CreateTrainingMissionAndFrag│
   └──────────────────────────────────┘   └──────────────────────────────────┘
```

Any conversationally derived knowledge mutation SHALL be sandbox-first (MUST). No conversational event, fragment, or candidate knowledge object may directly mutate production canonical knowledge without passing the existing promotion discipline, trust review, origin-trace requirements, and dual-store consistency protocol (§25.x, §18.2). This is a hard invariant: the entire conversational knowledge path is governed by the deterministic gate, and no ad hoc mutation path outside the gate is permitted (MUST NOT).

### 16B.3 Conversational TrainingMission Construction

This section specifies the complete shape of a TrainingMission generated from conversational events.

#### Required types

```rust
struct ConversationalMissionPayload {
    mission_id: String,
    source_event_ids: Vec<String>,
    user_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    normalized_facts: Vec<String>,
    mission_text: String,
    success_criteria: Vec<String>,
    review_required: bool,
    created_at: SystemTime,
}
```

#### Normative requirements

`MissionSource::HumanSubmitted` SHALL be the standard source for conversational ingest missions.

The act of creating a TrainingMission from conversational events does not itself generate a CandidateKnowledgeDocument or a CanonicalDocument. It merely places the conversational evidence under Training Plane governance.

#### mission_text generation convention

The following template SHALL be normative:

```text
Consolidate the provided conversational evidence into a sandbox-scoped candidate knowledge object.
Preserve origin trace.
Do not infer beyond stated evidence.
Mark unresolved ambiguity explicitly.
Target namespace: {namespace}.
Target category: {category}.
```

#### success_criteria requirements

At minimum, the following success criteria SHALL be auto-populated:

- source_event_ids are all preserved in the origin trace.
- Each normalized fact has an evidence anchoring in the source events.
- Any ambiguity is explicitly marked as unresolved.
- The output appears only in the sandbox namespace.

### 16B.4 Fragment and Candidate Creation

This section specifies how conversational fragments are stored as Fragments and CandidateKnowledgeDocuments.

#### Policy principles

- Raw transcript full-text persistence is optional. If `allow_raw_transcript_persistence` is false, only normalized facts and a redacted summary SHALL be stored.
- Under sandbox namespace, conversational fragments MAY be stored as `Fragment` or `MemoryEvent` in LadybugDB.
- CandidateKnowledgeDocument SHALL be retained as a training document in sandbox namespace.

#### Required types

```rust
struct ConversationalFragmentMeta {
    fragment_id: String,
    source_event_ids: Vec<String>,
    user_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    redacted_summary: String,
    extracted_facts: Vec<String>,
    distinct_day_count: u32,
    first_seen_at: SystemTime,
    last_seen_at: SystemTime,
}
```

#### Persistence rules

`ConversationalFragmentMeta` MUST be joinable with LadybugDB Fragment / MemoryEvent.

`source_event_ids` MUST be maintained as stable IDs eligible for promotion to `origin_trace_ids`.

When a CandidateKnowledgeDocument is created, the following fields SHALL be populated per the existing v1.9 definition (§26 D.4): `knowledge_id`, `source_run_id`, `namespace`, `evidence_summary`, `origin_trace_ids`, `completeness_score`, `promotion_status`, `created_at`.

### 16B.5 Multi-turn / Multi-day Consolidation Policy

This section is the core consolidation rule. It defines the strict conditions under which scattered conversational fragments may be bundled into a single CandidateKnowledgeDocument.

#### Required types

```rust
struct ConsolidationCandidateSet {
    set_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    fragment_ids: Vec<String>,
    source_event_ids: Vec<String>,
    distinct_event_count: u32,
    distinct_day_count: u32,
    semantic_coherence: f32,
    trace_completeness: f32,
    temporal_stability: f32,
    contradiction_score: f32,
    created_at: SystemTime,
}

struct ConsolidationPolicy {
    min_distinct_events: u32,
    min_distinct_days: u32,
    min_semantic_coherence: f32,
    min_trace_completeness: f32,
    min_temporal_stability: f32,
    max_contradiction_score: f32,
    require_origin_trace: bool,
    allow_auto_candidate_creation: bool,
    allow_auto_promotion: bool,
}
```

#### Normative default thresholds

| Threshold | Default |
|---|---|
| `min_distinct_events` | 3 |
| `min_distinct_days` | 2 |
| `min_semantic_coherence` | 0.70 |
| `min_trace_completeness` | 0.80 |
| `min_temporal_stability` | 0.65 |
| `max_contradiction_score` | 0.20 |
| `require_origin_trace` | true |
| `allow_auto_candidate_creation` | true |
| `allow_auto_promotion` | false |

#### semantic_coherence definition

`semantic_coherence` SHALL be defined as the degree (0.0–1.0) to which a set of conversational fragments belongs to the same long-lived fact, preference, constraint, or project context. Implementations MAY use LLM judgment to compute this score, but the acceptance or rejection of the score SHALL be decided by a deterministic threshold against the policy-declared `min_semantic_coherence`.

#### contradiction_score safe rule

A candidate set whose `contradiction_score` exceeds `max_contradiction_score` MUST NOT be automatically canonicalized. The default safe action is either:
- Retain the CandidateKnowledgeDocuments as separate coexisting candidates, or
- Send the contradictory set to the human review queue as a `SUPERSEDES` / `CONSOLIDATES` candidate.

Destructive merge SHALL NOT be performed.

The following decision table formalizes the contradiction handling matrix:

```text
   | Contradiction Score | Auto-Canonicalize | Action           | Lineage        |
   |---------------------|-------------------|------------------|----------------|
   | <= max_contradiction| Yes               | Consolidate and  | CONSOLIDATES   |
   |   (default 0.20)    |                   | promote (if all  |                |
   |                     |                   | other gates pass)|                |
   |---------------------|-------------------|------------------|----------------|
   | > 0.20, <= 0.50    | No                | Separate         | (coexistence)  |
   |                     |                   | candidates       |                |
   |                     |                   | coexist          |                |
   |---------------------|-------------------|------------------|----------------|
   | > 0.50              | No                | Human review     | SUPERSEDES /   |
   |                     |                   | queue            | CONSOLIDATES   |
   |---------------------|-------------------|------------------|----------------|

   Destructive merge is NOT permitted at any contradiction level.
```

#### Normative consolidation condition

Multi-turn or multi-day conversational fragments MAY be consolidated into a CandidateKnowledgeDocument only when the candidate set satisfies policy-declared thresholds for semantic coherence, trace completeness, temporal stability, and contradiction tolerance (§16B.5 thresholds table). Promotion to CanonicalDocument SHALL remain separately gated through the ConversationalPromotionGate (§16B.7) and is not implied by consolidation eligibility.

#### Libraryfication stage convention

The following four stages and their cross-stage lineage relations SHALL be normative:

1. **ConversationalEvent** — raw conversational input
2. **Fragment / MemoryEvent** — normalized fragment under sandbox namespace
3. **CandidateKnowledgeDocument** — bundled candidate under sandbox namespace
4. **CanonicalDocument** — promoted canonical knowledge

Lineage relations:
- Event/Fragment → CandidateKnowledgeDocument: `DERIVEDFROM`
- Fragment bundle → CandidateKnowledgeDocument: `CONSOLIDATES`
- CandidateKnowledgeDocument → CanonicalDocument: `MATERIALIZEDAS`
- Replaced canonical / preference update: `SUPERSEDES`

The following state transition diagram illustrates the four-stage pipeline:

```text
   ┌──────────────────────┐
   │  ConversationalEvent │  (raw input from chat, voice, etc.)
   └──────────┬───────────┘
              │ LLM classification proposal
              │ Deterministic ingestion gate (§16B.2)
              ▼
   ┌──────────────────────┐
   │  Fragment /          │  (sandbox namespace)
   │  MemoryEvent         │
   └──────────┬───────────┘
              │ Multi-turn / multi-day consolidation
              │ ConsolidationPolicy thresholds check (§16B.5)
              ▼
   ┌──────────────────────┐
   │  CandidateKnowledge  │  (sandbox namespace)
   │  Document            │
   └──────────┬───────────┘
              │ ConversationalPromotionGate check (§16B.7)
              │ Dual-store commit with shared opid (§25.x)
              ▼
   ┌──────────────────────┐
   │  CanonicalDocument   │  (production namespace)
   └──────────────────────┘
```

### 16B.6 Personalization Namespace Convention

This section standardizes the namespace convention for personal knowledge learned through conversation.

#### Normative naming convention

The following forms SHALL be standard:

- `user/{user_id}/profile`
- `user/{user_id}/preferences`
- `user/{user_id}/projects/{project_id}`
- `user/{user_id}/history`
- `user/{user_id}/scratch`

#### Usage convention

| Namespace | Purpose | Promotion Permitted |
|---|---|---|
| `profile` | Long-term personal attributes, stable self-description | Conditionally |
| `preferences` | Stable tastes, preferences, communication tendencies | Conditionally |
| `projects/{project_id}` | Long-lived project context, constraints, policies | Conditionally |
| `history` | Past factual records, historical reference | Usually sandbox / review required |
| `scratch` | Temporary notes, short-term working context | Not permitted |

#### Expert Namespace alignment

- User namespaces SHALL be extractable and fuseable as v2.0 Expert Namespace.
- `scratch` and tombstoned artifacts SHALL NOT be included in the required dependency closure by default.

### 16B.7 Promotion to Canonical Document

This section formalizes the final step of libraryfication: promotion of a conversational-origin CandidateKnowledgeDocument to CanonicalDocument.

#### Policy principles

- Conversational-origin knowledge MUST NOT become a CanonicalDocument without first passing through a CandidateKnowledgeDocument stage.
- `memorypromotetodocument` is the sole mutation primitive for this transition. It SHALL only be usable after the promotion gate is satisfied.
- The dual-store consistency protocol (§25.x) applies without modification.

#### PromotionGate type

```rust
struct ConversationalPromotionGate {
    candidate_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    llm_policy_score: f32,
    completeness_score: f32,
    trace_completeness: f32,
    contradiction_score: f32,
    distinct_day_count: u32,
    training_good_ratio: f32,
    sandbox_success_rate: f32,
    requires_human_review: bool,
}
```

#### Normative conditions

A conversational-origin CandidateKnowledgeDocument MAY be promoted to CanonicalDocument only when ALL of the following are satisfied:

- `promotion_status = Approved`
- `completeness_score >= 0.80`
- `trace_completeness >= 0.80`
- `contradiction_score <= 0.20`
- `distinct_day_count >= 2`
- `training_good_ratio >= TRAINING_PROMOTION_MIN_GOOD_RATIO`
- `sandbox_success_rate >= TRAINING_PROMOTION_MIN_SUCCESS_RATE`
- `requires_human_review = false` or human approval has been recorded
- A dual-store commit intent sharing a single `op_id` has been generated

The existing training constants (`TRAINING_PROMOTION_MIN_GOOD_RATIO`, `TRAINING_PROMOTION_MIN_SUCCESS_RATE`) are calibration candidates, and their values apply to conversational-origin promotion without modification.

### 16B.8 Privacy, Retention, Tombstone, and Repair

This section formalizes operational rules specific to conversational memory.

#### Required provisions

- Raw conversational events MAY expire according to the TTL declared in `RetentionPolicy`.
- A Rejected CandidateKnowledgeDocument SHALL inherit the existing tombstone grace period (§15 GcState).
- An artifact subject to a user deletion request SHALL retain at minimum a namespace-local tombstone and audit log entry, and MUST be excluded from normal retrieval paths.
- A conversational artifact that encounters dual-store inconsistency SHALL transition to `NeedsRepair` or `Quarantined` (§18.2), and MUST NOT appear in normal REUSE / PATCH / COMPOSE paths.

## 17. 健全性命題

v1.9 では v1.8-final までの健全性命題に加え、以下を追加する。

1. **Training Isolation Invariant** — training artifact は promotion を完了するまで production selection path に混入してはならない。
2. **Human Review Invariant** — AI-generated mission は human review を経ずに実行してはならない。
3. **Promotion Discipline Invariant** — sandbox success / human feedback / audit / consistency を満たさない成果は production へ昇格してはならない。
4. **Trust Separation Invariant** — training trust は production trust に直接コピーしてはならない。
5. **Knowledge Promotion Invariant** — training knowledge mutation は sandbox namespace に留まり、production canonical knowledge へは別審査なしに昇格してはならない。

6. **Conversational Ingestion Invariant** — conversational origin knowledge は ConversationalEvent → Fragment/SandboxMemoryEvent → CandidateKnowledgeDocument → CanonicalDocument の全段階を経なければ production canonical knowledge に到達してはならない (MUST NOT)。いずれかの段階をスキップして直接 production canonical knowledge を生成する経路は、gate の存在如何にかかわらず禁止する。


### 16.1 命題の分類

v1.6 では健全性に関する記述を以下の 3 種に分類する。

| 分類 | 意味 | 例 |
|------|------|----|
| Theorem / Invariant | 実装規則と型・状態機械から導かれるべき性質。テストまたは形式検証の対象 | DAG 制約、終端状態からの再遷移禁止、CAS による更新消失検出 |
| Design Assumption | 設計上の前提として固定する性質。値の妥当性は設計判断に依存するが、RFC 本文では既定値として扱う | `λ_verify < λ_use`、cold-start trust の下限、applicability floor |
| Empirical Claim | 実装・評価により支持されるべき期待値主張。M 系マイルストーンの観測で継続検証する | Fake-first による不具合早期発見、SearchWorkflow による false-new rate 低減 |

以降の節で数式やアルゴリズムを記述する場合、証明可能な安全性主張と運用上の期待値主張を混同してはならない (MUST NOT)。SearchWorkflow の outcome policy 品質は v1.6 では Empirical Claim に属し、規範対象は状態機械・予算・監査可能性・ガード条件である。

### 16.2 命題本文

### 命題 1: GMR 期待コスト削減 (v1.1 修正)

**旧命題 (v1.0 の誤り)**: "L_t ≤ L_{t-1} が成立する" — これは一般に偽である。未知タスクやパッチ失敗が連続した場合、個別時刻で L_t > L_{t-1} となりうる。

**修正命題**:  
タスク分布が定常な環境において、十分な実行履歴が蓄積された後、GMR の期待 LLM 呼び出しコストはベースライン (全タスク新規生成) に対して単調非増加に収束する。形式的には:

> 十分大きい T に対し、E[Σₜ L_t / T] ≤ E[Σₜ L_t^baseline / T]

**直感的説明**: 再利用パス (A ≥ 0.50) では LLM 呼び出し = 0。パッチパスでも Gold グラフを出発点とするため呼び出し数 < 新規生成。TrustProfile の Temporal 減衰により品質劣化した古いグラフは自然淘汰され、高品質グラフが優先的に選択されるようになる。非定常タスク分布では保証は成立しない点に注意。

**証明**: 形式証明は OQ-09 (新設) として今後の課題とする。

### 命題 2: コンパイル健全性

`compile_to_steps(G, reg, ctx)` が `Ok(steps)` を返す場合、`steps` は OpenFang スキーマに適合し、循環参照・未解決変数・空 SubWorkflow を含まない。

**証明**: V-01 の toposort 成功が DAG を保証し、`ctx.visited` が循環 SubWorkflow を排除する。各ノードの変数バインディングは `ctx.validate_inputs()` と `ctx.bind_output()` が検証する。□

### 命題 3: Patch Atomic 性

`apply_patch_atomic(gold, patch)` が `Err(e)` を返す場合、`gold` は変更されていない。

**証明**: 実装は clone → apply → validate → swap の 4 フェーズ構造であり、フェーズ 2, 3 のエラーは `g_candidate` のドロップで終了し `gold` に到達しない。□

---

## 18. エラーハンドリングとロールバック方針

### v2.3 補足: startup repair scan

Startup repair scan は optional housekeeping ではなく、non-committed dual-store operation を normal selection path に戻す前の必須 recovery procedure である。`Pending` または partial dual-store commit が検出された場合、実装は commit intent と lineage の監査可能性を再確認し、idempotent retry、`NeedsRepair`、`Quarantined` のいずれかへ明示的に遷移させなければならない (MUST)。


### 18.2 Dual-Store Consistency Refinement (v1.8)

Revision v1.8 makes the dual-store commit contract fully normative for knowledge mutation paths. Workflow orchestration metadata and state remain authoritative in the Darvium repository, while LadybugDB remains authoritative for persisted knowledge objects. Any operation that mutates both domains SHALL be executed under a shared `opid` and SHALL follow the sequence below:

1. Write workflow-side intent and mark `ConsistencyState::Pending { opid, phase = MetaPrepared }`.
2. Write knowledge-side intent under the same `opid`.
3. Perform workflow-side mutation and knowledge-side mutation.
4. If both commits succeed, mark `ConsistencyState::Committed`.
5. If either side fails after any prepare or write has occurred, mark `ConsistencyState::NeedsRepair { opid, reason }`, append a `RepairLog`, and enqueue the operation for repair.

A workflow in `Pending`, `NeedsRepair`, or `Quarantined` state MUST NOT be selected for normal REUSE, PATCH, or COMPOSE. Such workflows MAY be inspected by audit, repair, or replay tooling only. A repair worker MAY attempt retry-commit, compensating tombstone, or quarantine according to the existing v1.7 repair model, but any successful recovery MUST preserve the original `opid`, lineage references, and SearchTrace linkage.

The runtime MUST treat the dual-store protocol as an application-level commit intent protocol rather than a database-native XA guarantee. Therefore, implementations MUST preserve enough intent and audit metadata to deterministically finish, quarantine, or tombstone any interrupted operation during startup repair scans.


### 18.x 異種ストア整合性とフェイルセーフ (v1.7 追補)

LadybugDB に保持される graph / embedding 系データと、SQLite に保持される Trust / Lifecycle / lineage / audit 系メタデータは、単一 ACID トランザクションではなく**論理コミット単位**として扱う。`apply_patch_atomic`、SubWorkflow 資産登録、GC 状態遷移、tombstone 化の各処理は、少なくとも `op_id` を持つ commit intent を先に生成し、両ストア更新後にのみ `consistency_state = Committed` へ遷移させなければならない (MUST)。

いずれか片側の書き込みが失敗した場合、当該資産を `NeedsRepair` または `Quarantined` に遷移させ、SearchWorkflow / RetrievalPrimitive の通常候補集合から除外しなければならない (MUST)。この隔離は runtime safety のための措置であり、通常 GC や trust 低下と混同してはならない (MUST NOT)。

```rust
fn commit_dual_store_update(op_id: String, graph: &mut MemoizedGraph) -> Result<(), RepositoryError> {
    graph.consistency_state = ConsistencyState::Pending {
        op_id: op_id.clone(),
        phase: CommitPhase::MetaPrepared,
    };

    sqlite_prepare(op_id.clone())?;
    ladybug_prepare(op_id.clone())?;

    match (sqlite_commit(op_id.clone()), ladybug_commit(op_id.clone())) {
        (Ok(()), Ok(())) => {
            graph.consistency_state = ConsistencyState::Committed;
            Ok(())
        }
        (meta_res, blob_res) => {
            graph.consistency_state = ConsistencyState::NeedsRepair {
                op_id: op_id.clone(),
                reason: format!("meta={:?}, blob={:?}", meta_res.err(), blob_res.err()),
            };
            enqueue_repair(op_id, graph.id.clone());
            Err(RepositoryError::CrossStoreInconsistency)
        }
    }
}
```

復旧系は起動時、および定期 repair worker により `consistency_state != Committed` の資産を走査し、(1) 片側 commit の再試行、(2) 一貫した tombstone への収束、(3) 管理者レビュー待ちの quarantine 維持、のいずれかへ遷移させなければならない (SHOULD)。`RepairLog` は `LifecycleAuditLog` と同様に監査可能でなければならない (SHOULD)。

厳密な 2PC / XA は本 RFC のスコープ外だが、単一プロセス前提では「commit intent + quarantine + startup repair scan」により、片側成功状態の放置を避けることを v1.7 の最小フェイルセーフとする。



### 15.1 Layer 2 / 2.5 のエラー処理

| エラー種別 | 原因 | 処理 |
|-----------|------|------|
| CompileError | DAG 違反・循環参照・未解決変数 | REJECT。WorkflowGraph を修正してから再試行 |
| PatchError::LowConfidence | patchconfidence < 0.75 | 人間レビューキューに積む |
| PatchError::CycleCreated | パッチ適用後に DAG 違反 | REJECT。Gold グラフから再試行 |

### 15.2 Layer 1 実行エラーと補償トランザクション

**現行方針 (M0〜M2)**: ErrorMode に従いシンプルに制御する。

| ErrorMode | 動作 |
|-----------|------|
| `Fail` | ステップ失敗時にワークフロー全体を即時中断。副作用のロールバックは行わない |
| `Skip` | 失敗ステップをスキップして後続ステップを継続 |
| `Retry` | 指定回数・バックオフで再試行。上限到達後は Fail と同様 |

**補償トランザクション (将来: Milestone M3+)**:  
外部 API 書き込みが `irreversible: false` かつ途中失敗した場合、Saga パターンによる補償操作 (逆操作ノード) の実行が求められるケースがある。現行は WorkflowNode に補償ノードを定義する仕組みを持たないため、以下を将来拡張として記録する。

```
WorkflowNode::CompensationStep {
    compensates_for: NodeId,  // 補償対象ノードの UUID
    compensation_prompt: String,
}
```

この拡張は RFC-0003 または独立 RFC として検討する。現時点では `irreversible: false` の副作用を持つステップが失敗した場合、上位レイヤ (ミッション発行者) が補償責任を持つ。

---

### 18A. Training-specific errors (v1.9)

```rust
#[derive(Debug, thiserror::Error)]
enum TrainingError {
    #[error("Mission requires human review before execution: {0}")]
    UnapprovedMission(String),
    #[error("Sandbox policy violation: {0}")]
    SandboxPolicyViolation(String),
    #[error("Promotion gate not satisfied: {0}")]
    PromotionGateViolation(String),
    #[error("Training artifact quarantined: {0}")]
    ArtifactQuarantined(String),
}
```

training-specific failure は既存の patch rollback、CAS conflict、dual-store repair、quarantine 規範を尊重した上で扱うこと。未承認 mission の実行、sandbox policy に反する external write / network side-effect / irreversible mutation、promotion gate 不成立は明示的エラーとして監査されなければならない。

## 19. 性能目標

### v2.3 補助観測指標

本節の主要性能目標に加え、RFC 準拠実装は運用品質の補助指標として、reuse quality、false-new rate、compose/new fallback frequency、repair rate、quarantine rate、rollback rate、human review queue depth、review latency、ranking stability under small patch を観測対象に含めることが望ましい (SHOULD)。これらは現時点では calibration candidate と operational metric であり、一律の固定閾値を意味しない。


| 指標 | 目標値 | 達成マイルストーン |
|------|--------|-----------------|
| LLM 呼び出し削減率 | ≥ 20% (vs ベースライン) | M2 |
| レイテンシ削減率 | ≥ 15% | M2 |
| ApplicabilityScore 適合率 (再利用後の成功率) | ≥ 95% | M2 |
| trustscore (成熟グラフ) | ≥ 0.70 | M3 |

---

### 19A. Training Plane performance and isolation (v1.9)

v1.9 の追加目標は、production path の latency / recall / applicability precision を低下させないことである。Training Plane は background / operator-driven / sandbox-first に動作し、training-only graph の過剰蓄積により ANN 汚染や search quality 低下を引き起こしてはならない。mission review queue のバッチング、fake-first による real provider call 削減、failure replay の圧縮は推奨される運用方針である。

## 20. マイルストーン

### v2.3 補足: testing discipline

マイルストーンに含まれる testing discipline は、可能であれば `GED_GRAPH_SIZE_LIMIT` 境界付近の replayable ranking drift test、small structural perturbation に対する property-based ranking stability test、startup repair scan の deterministic recovery test を含むべきである (SHOULD)。


v1.7 では、既存の Fake-first / deterministic replay 方針を保持したまま、Lifecycle / GC / Reputation / VirtualClock を段階導入する。少なくとも M0 相当で VirtualClock と `last_virtual_seen`、M1 相当で TimeDecayProfile と LifecycleScore、M2 相当で GcState / soft delete、M3 相当で reciprocity graph / reputation recompute、M4 相当で resource pressure / environment policy / social acceleration KPI を検証できるようにマイルストーンを拡張しなければならない (MUST)。

v1.6 以降の**正規マイルストーン体系は M -2〜M4** である。v1.5 以前に記載されていた M -1〜M4 の表現は履歴的文脈としてのみ参照され、競合する場合は本節の v1.6 体系を優先しなければならない (MUST)。

v1.6 では、v1.5 の M -1〜M4 を SearchWorkflow 導入に合わせて再編し、**Fake-first / deterministic replay first** を徹底する。実 AI provider や実 executor への接続は、RetrievalPrimitive・SearchState machine・budget guard・trace/audit 整合性が FakeImpl で検証された後段に配置する。

| ID | 目的 | 主な内容 |
|----|------|---------|
| M -2 | SearchWorkflow 仕様固定 | Stage 0–4 RetrievalPrimitive の純インタフェース化、FakeRetrievalPrimitive、empty set / timeout / version mismatch / deterministic replay のテスト |
| M -1.5 | Search state machine 検証 | Init / Retrieve / Evaluate / Refine / Compose / Finalize / Abort の遷移表、停止条件、requery 条件、oscillation 検出、モデル検査 |
| M -1 | Fake policy evaluator | deterministic heuristic による EvaluateCandidates / RefineSearchPolicy の実装、budget / uncertainty / trust に基づく outcome 選定 |
| M -0.5 | Fake repository / embeddings | task/design dual retrieval、union rerank、ranking drift 検査、embedding version mismatch 移行テスト |
| M0 | Composition / New proposal 基盤 | ComposeExisting / GenerateNew proposal、lineage / invalidation / proposal validity テスト |
| M0.5 | Fake LLM adapter | scripted fake LLM、JSON schema parser、malformed output recovery、same-input same-output replay |
| M1 | Human-in-the-loop review | NeedsHumanReview、SearchTrace と TrustAuditLog / SearchRunLog の整合性、manual override |
| M1.5 | Real embedding provider | 実 embedding provider 接続、ANN recall と ranking drift 検証 |
| M2 | Limited real LLM | BuildQueryStep / RefineSearchPolicyStep のみ実 LLM 接続、schema conformance と budget overrun protection |
| M2.5 | Real query-policy evaluation | nondeterminism envelope 計測、provider latency と replay baseline 比較 |
| M3 | Real proposal generation | Compose / New / Patch proposal を実 LLM で生成し、review-gated validity を評価 |
| M4 | Real executor end-to-end | OpenFang / 実 executor を含む end-to-end。ただし unsafe side-effect path は review-gated を維持 |

### 19.1 Legacy マイルストーン互換メモ

以下の旧マイルストーン表現は履歴参照用であり、v1.6 の正規実装計画ではない。

### 19.1 マイルストーン一覧

| ID | 名称 | 成果物 |
|----|------|--------|
| **M -1** | **ダミー層・ポート抽象化** | PortTrait 定義 + FakeImpl。OpenFang・LLM に未接続の状態でコアロジック全域をテスト可能にする |
| M0 | MVP | WorkflowGraph + compile_to_steps + WorkflowRepository (埋め込みなし、cold-start trust) |
| M1 | GMR 基本 | task_embedding による cosine ANN 検索 + GED + Applicability Check |
| M2 | Trust + Patch | TrustProfile 4 軸 + GraphPatchGenerator + PatchConfidence + cₛ 補正 |
| M3 | 大規模化 | HNSW 1,000+ グラフ + graph_embedding (GNN) + 補償トランザクション RFC 策定 |
| M4 | RFC-0003 | Pareto Trust + Counterfactual Replay + Darwinian Evolution |

---

### 17.2 M -1 — ダミー層・ポート抽象化

**目的**: OpenFang・LLM に一切接続しない状態で、コアロジック（バリデーション・コンパイル・Trust 数値演算・CAS）の正確さを `cargo test` のみで検証する。AI コストをゼロに保ちながら定数実験を高速に回す。

#### ポート抽象化 (PortTrait)

外部依存はすべてトレイト境界として定義し、本番実装と FakeImpl を差し替え可能にする。

```rust
// src/ports.rs

/// Layer 1 実行エンジンの差し替えポート
/// 本番: OpenFangClient  / テスト: FakeExecutor
#[async_trait]
pub trait WorkflowExecutor: Send + Sync {
    async fn execute(
        &self,
        steps: Vec<OpenFangStep>,
    ) -> Result<ExecutionResult, ExecutorError>;
}

/// LLM 呼び出しの差し替えポート
/// 本番: RealLlmClient  / テスト: FakeLlmClient
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate_patch(
        &self,
        ctx: &PatchGenerationContext,
    ) -> Result<(f32, Vec<PatchOperation>), LlmError>;
}
```

**本番コードはこの 2 つのトレイトにのみ依存する。** `GraphPatchGenerator` の `llm_client` フィールドは `Arc<dyn LlmClient>` であり、実装詳細を知らない（§12.2 参照）。

#### ダミー実装 (FakeImpl)

```rust
// src/fakes.rs

/// 常に空ステップを成功で返す。OpenFang に接続しない。
pub struct FakeExecutor {
    pub call_count: Arc<AtomicUsize>,
}

#[async_trait]
impl WorkflowExecutor for FakeExecutor {
    async fn execute(
        &self,
        _steps: Vec<OpenFangStep>,
    ) -> Result<ExecutionResult, ExecutorError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(ExecutionResult::success())
    }
}

/// 固定スコア・固定 ops を返す。LLM に接続しない。
pub struct FakeLlmClient {
    pub self_confidence: f32,             // 返す cₛ 値
    pub ops: Vec<PatchOperation>,         // 返すパッチ操作列
    pub call_count: Arc<AtomicUsize>,
}

impl FakeLlmClient {
    /// デフォルト: cₛ = 0.90、ops = []（Gold をそのまま採用）
    pub fn default_pass() -> Self { ... }
    /// 低自信ケース: cₛ = 0.40（重み切り替えの検証用）
    pub fn low_confidence() -> Self { ... }
}

#[async_trait]
impl LlmClient for FakeLlmClient {
    async fn generate_patch(
        &self,
        _ctx: &PatchGenerationContext,
    ) -> Result<(f32, Vec<PatchOperation>), LlmError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok((self.self_confidence, self.ops.clone()))
    }
}
```

#### ファイル構造

```
src/
├── constants.rs        ← 定数の唯一の正本（付録 A と 1:1 対応）
├── types.rs            ← WorkflowNode / EdgeMeta / OpenFangStep 等の純粋型
├── graph.rs            ← WorkflowGraph バリデーション (V-01〜V-08)
├── compiler.rs         ← compile_to_steps（純粋関数）
├── trust.rs            ← TrustProfile 4 軸 / DualTemporalTrust / HumanTrustLogistic
├── repository.rs       ← WorkflowRepository + CAS + cold-start
├── ports.rs            ← WorkflowExecutor / LlmClient トレイト境界 ★
└── fakes.rs            ← FakeExecutor / FakeLlmClient ダミー実装 ★
```

`ports.rs` と `fakes.rs` が M -1 の核心である。M0 以降は本番実装クレートを追加するだけでよく、コアロジック側の変更は不要。

#### M -1 でテスト可能な範囲

| テスト対象 | M -1 | 理由 |
|---|---|---|
| WorkflowGraph DAG バリデーション (V-01〜V-08) | ✅ | 純粋 petgraph 計算 |
| `compile_to_steps` 出力の正確さ | ✅ | 純粋関数 |
| SubWorkflow ネスト・循環参照検出 | ✅ | 純粋関数 |
| `MAX_COMPILED_STEPS` 制限 | ✅ | 定数チェック |
| SideEffectSet 包含チェック | ✅ | 純粋ロジック |
| TrustProfile EMA 収束 | ✅ | 数値計算のみ |
| DualTemporalTrust 減衰 (λ 操作) | ✅ | 数値計算のみ |
| HumanTrustLogistic 更新 | ✅ | 数値計算のみ |
| composite スコア重み検証 | ✅ | 数値計算のみ |
| cold-start 初期化 (P-07) | ✅ | 純粋構築 |
| GraphVersion CAS 競合検出 | ✅ | `tokio::sync::RwLock` のみ |
| Debounce しきい値動作 | ✅ | 数値比較のみ |
| `apply_patch_atomic` の atomic 性 | ✅ | clone+validate のみ |
| FakeExecutor が LLM を呼ばないこと | ✅ | `call_count == 0` で確認 |
| ANN 検索 (HNSW) | ❌ | M1（埋め込みモデル必要） |
| ApplicabilityScore の総合精度 | ❌ | M1（実データ必要） |
| LLM 自己評価スコア精度 | ❌ | M2（LLM 必要） |

#### テストコード例

```rust
// tests/m_minus1/trust_ema.rs

#[tokio::test]
async fn operational_trust_converges_after_50_successes() {
    use darvium_core::constants::*;
    let mut trust = TRUST_COLD_START_OPERATIONAL; // 0.40
    for _ in 0..50 {
        update_operational_trust(&mut trust, true, OPERATIONAL_EMA_ALPHA);
    }
    assert!((trust - 1.0).abs() < 0.01,
        "50 回成功後は 1.0 に収束するはず: actual={trust}");
}

#[tokio::test]
async fn fake_executor_never_calls_llm() {
    let executor = Arc::new(FakeExecutor::default());
    let llm      = Arc::new(FakeLlmClient::default_pass());

    // ワークフローを compile して FakeExecutor で実行
    let graph   = build_simple_graph();
    let steps   = compile_to_steps(&graph, &WorkflowRegistry::empty(),
                                   &mut CompilerContext::new()).unwrap();
    executor.execute(steps).await.unwrap();

    assert_eq!(llm.call_count.load(Ordering::SeqCst), 0,
        "M -1 では LLM は一切呼ばれてはならない");
}

#[tokio::test]
async fn cas_detects_concurrent_update_conflict() {
    let repo = WorkflowRepository::in_memory();
    let id   = repo.insert(build_simple_graph(), TrustProfile::cold_start_new())
                   .await.unwrap();
    // バージョン 0 を読み取り
    let (_, v0) = repo.read_with_version(id).await.unwrap();
    // 同バージョンで 2 回更新 → 2 回目は UpdateConflict
    repo.update_graph_cas(id, build_simple_graph(), v0).await.unwrap();
    let err = repo.update_graph_cas(id, build_simple_graph(), v0).await.unwrap_err();
    assert!(matches!(err, RepositoryError::UpdateConflict { .. }));
}
```

#### 定数実験の進め方

M -1 は定数チューニングの実験場でもある。`constants.rs` の値を変更し `cargo test` を実行するだけで、AIコストゼロ・数秒以内にすべての数値的挙動を確認できる。

```
実験サイクル:
  1. constants.rs の定数を変更（例: TRUST_INHERIT_DECAY = 0.50 → 0.80）
  2. cargo test --test m_minus1          # 全テスト実行（秒単位）
  3. 結果を記録（EMA 収束速度・CAS 競合率等）
  4. 次の値へ → 1 に戻る
```

---

### 17.3 M0 — MVP（OpenFang 実接続）

**前提**: M -1 の全テストがグリーン。

**スコープ**: WorkflowGraph + `compile_to_steps` + WorkflowRepository の実接続版。埋め込みなし・cold-start trust のみ。

#### 実装ステップ

1. **`OpenFangClient` 実装** (`src/openFang_client.rs`)  
   `WorkflowExecutor` トレイトを実装。`POST /v1/workflows` に `Vec<OpenFangStep>` を送信し、`ExecutionResult` を返す。タイムアウト・リトライは `ErrorMode` に従う。

2. **統合テスト追加** (`tests/m0/`)  
   ローカル OpenFang インスタンス（Docker）に対して `compile_to_steps → execute` の疎通を確認。

3. **cold-start trust 登録確認**  
   `WorkflowRepository::insert` で `Trust::cold_start_new()` が正しく設定されることを統合テストで検証。

**M0 では LLM は引き続き FakeLlmClient を使用する。** Trust 4 軸・GraphPatch は対象外。

**M -1 との違い**:

| | M -1 | M0 |
|---|---|---|
| OpenFang 接続 | ❌ FakeExecutor | ✅ 実接続 |
| LLM 呼び出し | ❌ FakeLlmClient | ❌ 引き続き Fake |
| テスト方法 | `cargo test` のみ | Docker 統合テスト追加 |
| 目的 | ロジックの数学的正確さ | Layer 1 疎通確認 |

---

### 17.4 M1 — GMR 基本（埋め込み検索）

**前提**: M0 の統合テストがグリーン。

**スコープ**: `task_embedding` による cosine ANN 検索 + GED 近似 + Applicability Check（AG-01〜AG-06 + DeterminismScore）。

#### 実装ステップ

1. **埋め込みパイプライン** (`src/embedding.rs`)  
   ミッション文字列 → `task_embedding: Vec<f32>`。埋め込みモデルバージョンを `Provenance.source_version` に記録（AG-06 対応）。

2. **AnnIndex (HNSW) 統合** (`src/ann_index.rs`)  
   LadybugDB の HNSW インデックスに `task_embedding` を登録。`Stage 2` の `top-k = ANN_TOP_K` 検索を実装。

3. **GED 近似** (`src/ged.rs`)  
   ノード数 ≤ `GED_GRAPH_SIZE_LIMIT (50)` の場合に近似 GED を計算。境界スムージング (§11.3) を実装。

4. **Applicability Check 統合** (`src/applicability.rs`)  
   `compute_applicability_score` を Stage 4 に組み込む。`A ≥ APPLICABILITY_THRESHOLD (0.50)` で REUSE、未満で GraphPatch パスへ分岐（この時点ではまだ FakeLlmClient）。

5. **Stage 0 副作用包含チェック**  
   `SideEffectSet::contains` を Stage 0 フィルタに組み込む（§11.2）。

#### テスト追加

- `task_embedding` の cosine 検索が上位 k 件を正しく返すことを確認
- GED スムージング境界（ノード数 45〜55）でスコアが連続していることをプロパティテストで確認
- AG-06（埋め込みモデルバージョン不一致）で候補が除外されることを確認

---

### 17.5 M2 — Trust + Patch（LLM 実接続）

**前提**: M1 の ANN 検索・Applicability Check がグリーン。

**スコープ**: TrustProfile 4 軸フル稼働 + GraphPatchGenerator + PatchConfidence + cₛ 補正 + `RealLlmClient` 実装。

#### 実装ステップ

1. **`RealLlmClient` 実装** (`src/llm_client.rs`)  
   `LlmClient` トレイトを実装。`PatchGenerationContext` を JSON プロンプトに変換し、LLM に送信。`{"patch_ops": [...], "self_confidence": float}` の構造化出力を受信・パース。cₛ_adjusted = cₛ × `SELF_CONF_DISCOUNT (0.85)` を適用。

2. **PatchConfidence 動的重み** (`src/patch_confidence.rs`)  
   `cₛ < PATCH_SELF_CONF_SWITCH_THRESHOLD (0.50)` の場合に `(ws, wv) = (0.20, 0.50)` へ切り替え（§12.3 規範化）。

3. **TrustProfile フル統合**  
   `update_trust` / `mark_used` / `mark_verified` / Debounce / GraphVersion CAS を本番フローに接続（§9.5, §8.2, §8.3）。`TrustAuditLog` の SQLite 永続化。

4. **PatchHistory** (`src/patch_history.rs`)  
   SQLite の `patch_history` テーブルに実行結果を記録。`get_history_score_with_prior` が実績データを返すようになる。

5. **低信頼パスの人間レビューキュー**  
   `PatchError::LowConfidence` が発生した場合の通知インタフェースを実装。`PATCH_CONFIDENCE_THRESHOLD = 0.75` 未満のパッチをキューに積む。

#### コスト管理

`RealLlmClient` 切り替え後は AI コストが発生する。以下のガードを必ず設ける：

- `FakeLlmClient` を使い続けるテストは `#[cfg(test)]` フラグで分離
- LLM を実際に呼ぶ統合テストは `cargo test --features integration_llm` でのみ実行
- CI は原則として M -1 テスト（`cargo test`）のみ実行し、LLM 統合テストは手動または週次バッチで実行

---

### 17.6 M3 — 大規模化

**スコープ**: HNSW 1,000+ グラフ対応 + `graph_embedding` (GNN; Milestone M1 では `task_embedding` のみ) + 補償トランザクション RFC 策定。

#### 実装ステップ

1. **graph_embedding (GNN)** (`src/graph_embedding.rs`)  
   GIN または GraphSAGE で `WorkflowGraph` → `Vec<f32>` の構造埋め込みを生成。LadybugDB の HNSW インデックスに追加登録。`Sstruct` の計算が GED からベクトルコサインへ切り替わる境界 (§11.3) を活用。

2. **大規模インデックス性能チューニング**  
   `ANN_TOP_K` の引き上げ（10 → 20 程度）を検討（OQ の検討事項）。HNSW の `ef_construction` / `M` パラメータをプロファイリングして調整。

3. **補償トランザクション RFC 草稿**  
   `WorkflowNode::CompensationStep` (§15.2) を正式仕様化する独立 RFC または RFC-0003 サブセクションを策定。

---

### 17.7 M4 — RFC-0003

**スコープ**: Pareto Trust フロンティア + Counterfactual Replay + Darwinian Graph Mutation (変異率 `μᵢ = 1 − dᵢ`)。本 RFC のスコープ外。RFC-0003 に委譲。

---

## 20B. v1.9 Training Plane 統合補完

v1.9 統合時の重点検証項目は以下である。

1. training primitive を使わない既存 v1.8 workflow が完全に同一挙動を保つこと。
2. AI-generated mission が human review を必ず経由すること。
3. sandbox knowledge mutation が production namespace に漏れないこと。
4. training trust が production trust を直接汚染しないこと。
5. PromotionCandidate を経由しない production 昇格が不可能であること。
6. training-only graph の GC が production graph の selection quality を下げないこと。

v2.3-c では、以下を追加の重点検証項目とする。

7. conversational ingestion の全段階（event → fragment → candidate → canonical）が Observational Testing First の対象となること。
8. LLM proposal と deterministic gate の責務分離が正しく実装されていること（gate の code path は replay 可能でなければならない）。

## 21. 未解決事項 (Open Questions)

### 21.1 重点 OQ (v1.9 補足)

- **OQ-09: 命題 1 の形式証明** — 理想的には形式証明が望ましいが、価値主張の初期根拠としては、bounded search / replay / failure-rate reduction を示す早期の実証実験で代替してよい。v1.9 時点では proof-first より evidence-first の順序が妥当である。
- **OQ-10: `TRUST_INHERIT_DECAY = 0.70` の根拠** — 実装前または実装初期に、少なくとも 0.50〜0.90 の範囲で感度分析を行い、promotion success・rollback rate・false confidence inflation を比較することを推奨する。
- **Elo 昇格 (`count >= 50`)** — v1.8 の out-of-scope を維持しつつも、HumanTrustLogistic の飽和挙動、count 依存の更新安定性、極端評価への過敏性を replay データで確認してから導入判断すべきである。

- OQ-v1.7-06: `ConsistencyState::NeedsRepair` から tombstone へ収束させる既定 retry 回数
- OQ-v1.7-07: SubWorkflow 資産化上限と HNSW 増分上限の環境別既定値
- OQ-v1.7-08: parameter taxonomy に基づく半自動 calibration の導入タイミング


- OQ-v1.7-01: `w_human` / `w_virtual` 初期推定の最適ヒューリスティクス
- OQ-v1.7-02: `MIN_SURVIVAL_EXPERIENCE` の環境別既定値
- OQ-v1.7-03: 直接互恵性 / 間接互恵性 / 成功率を統合する評判重み
- OQ-v1.7-04: resource pressure に応じた `THETA_SOFT` / `THETA_HARD` の上げ幅
- OQ-v1.7-05: tombstone を強制する環境と物理削除を許容する環境の分界


v1.6 では Open Questions を**実装停止要因ではなく、既定値を持つチューニング論点**として扱う。特に `λ_use` / `λ_verify`、Applicability の α / floor、`SELF_CONF_DISCOUNT` は本 RFC の既定値で固定して実装してよく、変更が必要な場合は backward-compatible patch RFC または後続改訂で調整する。

したがって、本節の OQ は v1.6 の規範性を妨げない。High / Medium / Low のうち、v1.6 Finalizing Revision では High は「アーキテクチャ未確定」を意味せず、「実測により将来再調整し得る優先監視項目」を意味する。

| ID | 質問 | 対象箇所 | 優先度 |
|----|------|---------|--------|
| OQ-01 | DualTemporalTrust の λ_use / λ_verify の最適値は何か。現在の値は経験則 | §9.2 | High |
| OQ-02 | ApplicabilityScore の αS=0.40, αD=0.30, αT=0.30 の根拠。A/B テスト計画必要 | §10.3 | High |
| OQ-03 | cₛ の SELF_CONF_DISCOUNT=0.85 の妥当性。M2 実績データで調整予定 | §12.2 | High |
| OQ-04 | GED 近似アルゴリズムの選定: Optimal Transport vs Hungarian vs A* 近似 | §11.2 Stage 3 | Medium |
| OQ-05 | graph_embedding の GNN モデル選定: GIN vs GraphSAGE (Milestone M3) | §8 | Medium |
| OQ-06 | patchconfidence < 0.75 時の人間レビューインタフェース仕様 | §12.6 | Medium |
| OQ-07 | HumanTrustLogistic から Elo への昇格基準 (count ≥ 50 以外の指標) | §9.3 | Low |
| OQ-08 | GED 境界スムージングのブレンド幅 [45, 55] の妥当性。ノード数分布による調整余地 | §11.3 | Low |
| OQ-09 | GMR 期待コスト削減命題の形式証明。タスク分布の定常性仮定の緩和 | §14 | Low |
| OQ-10 | `TRUST_INHERIT_DECAY = 0.70` の根拠。実験的に決定すべきか？信頼継承の decay parameter として他の値 (0.50, 0.80) との比較検証が必要 | §8.2 | Medium |
| OQ-11 | `TRUST_DEBOUNCE_DELTA = 0.05` の妥当性。Human フィードバックのバッチ更新パターンに依存する。非同期フィードバックの想定頻度によっては 0.02 や 0.10 が適切な可能性 | §9.5 | Low |

---

## 22. 付録 A — 定数一覧

v1.9 では最低限、以下の追加定数群を推奨する。

| 定数 | 既定値 | 意図 |
|---|---:|---|
| `TRAINING_HUMAN_REVIEW_REQUIRED` | true | AI-generated mission の human review 必須 |
| `TRAINING_PROMOTION_MIN_GOOD_RATIO` | 0.70 | Good 優勢判定 |
| `TRAINING_PROMOTION_MIN_SUCCESS_RATE` | 0.80 | sandbox success 閾値 |
| `TRAINING_TRUST_INHERIT_ALPHA` | 0.30 | training human signal を production へ混ぜる上限 |
| `TRAINING_TOMBSTONE_GRACE_HOURS` | 72 | candidate / rejected artifact の短期保護 |
| `LIFECYCLE_WEIGHT_FRESHNESS` | 0.22 | Human Time / Virtual Time 鮮度の寄与 |
| `LIFECYCLE_WEIGHT_SUCCESS` | 0.24 | operational success の寄与 |
| `LIFECYCLE_WEIGHT_TRUST` | 0.24 | trust 複合値の寄与 |
| `LIFECYCLE_WEIGHT_USAGE` | 0.15 | reuse / compose / contribution 頻度の寄与 |
| `LIFECYCLE_WEIGHT_REPUTATION` | 0.15 | reciprocity / indirect reputation の寄与 |

LifecycleScore の初期デフォルト重みは、実装ブレを避けるため、上記を v1.9 の推奨既定値とする。これらは将来の calibration candidate ではあるが、少なくとも v1.9 系 deployment では明示的な versioned override なしに変更してはならない (MUST NOT)。

これらの数値は calibration candidate であり、定数の存在意図と境界条件を規範化し、具体値は運用調整してよい。


### A.x v2.3-c 追加定数

v2.3-c では、会話取り込みに関する以下の定数を追加する。

| 定数 | 既定値 | 意図 |
|---|---:|---|
| `CONVERSATIONAL_CONSOLIDATION_MIN_EVENTS` | 3 | Consolidation に最低限必要な異なる会話イベント数 |
| `CONVERSATIONAL_CONSOLIDATION_MIN_DAYS` | 2 | Consolidation に最低限必要な異なる日数 |
| `CONVERSATIONAL_CONSOLIDATION_MIN_COHERENCE` | 0.70 | 断片間の意味的一貫性下限 |
| `CONVERSATIONAL_CONSOLIDATION_MIN_TRACE` | 0.80 | origin trace 網羅率下限 |
| `CONVERSATIONAL_CONSOLIDATION_MIN_STABILITY` | 0.65 | 時間的安定性下限 |
| `CONVERSATIONAL_CONSOLIDATION_MAX_CONTRADICTION` | 0.20 | 矛盾スコア上限（超過時は自動 canonicalization 禁止） |
| `CONVERSATIONAL_CONTRADICTION_COEXISTENCE_DEFAULT` | true | 矛盾検出時の既定動作（coexistence、destructive merge は禁止） |


### A.x 定数の分類 (v1.7 追補)

実装・運用の見通しを高めるため、定数は次の 3 群に分類して管理することを推奨する。

- Safety Invariants: hard gate 下限、状態機械の禁止遷移、名前空間制約など、原則として頻繁に変えない定数。
- Environment Policy Knobs: `THETA_SOFT`, `THETA_HARD`, `MIN_SURVIVAL_EXPERIENCE`, `INHERITANCE_RATE` など、環境差分で変える定数。
- Calibration Candidates: `APPLICABILITY_ALPHA_*`, `LIFECYCLE_ALPHA_*`, `TRUST_INHERIT_DECAY` など、M3 以降に履歴ベース半自動調整の候補となる定数。

本分類はアルゴリズム変更ではなく、運用責務の明確化を目的とする。

**本版 v1.3 が正本。他バージョンに記載の定数と矛盾する場合は本表を優先する。**

### 信頼スコア系

| 定数名 | 値 | 調整ガイド |
|---|---|---|
| `TRUST_HARD_GATE_THRESHOLD` | 0.20 | composite信頼スコアの絶対下限（AG-04）。**上げると**信頼実績のないグラフが弾かれやすくなり安全性が増すが、新規グラフが使われにくくなる。**下げると**低信頼グラフも通過しやすくなり多様な再利用が起きるが品質リスクが増す。本番環境では0.25〜0.30への引き上げが安全 |
| `TRUST_OPERATIONAL_HARD_GATE` | 0.15 | 実行成功率の絶対下限（AG-05）。**上げると**失敗率の高いグラフを早期に排除できる。新しいグラフが実績を積む前に弾かれやすくなるため、warm-upが遅い環境では低めに保つと良い |
| `TRUST_COLD_START_OPERATIONAL` | 0.40 | 新規グラフが登録された瞬間のoperational初期値。**上げると**新規グラフが即座により多くの再利用候補として選ばれるが過信頼になるリスクがある。**下げると**新規グラフは慎重に扱われ実績が出てから選ばれるようになる。ハードゲート（0.20）の2倍という設計意図を崩さないよう0.30〜0.60の範囲が妥当 |
| `TRUST_COLD_START_SEMANTIC` | 0.50 | 新規グラフの初期semantic信頼値。意味的な一致度の実績がない状態の中立値として0.50が設定されている。特に理由がなければ変更不要 |
| `TRUST_ADMIN_FAST_TRACK` | 0.80 | 管理者が手動承認した際にHumanTrustに強制設定される値。**上げると**管理者承認が強い信頼として反映されcomposite全体を底上げできる。B2B環境でフィードバックが50件集まる前の過渡期に高めに設定すると立ち上がりが早くなる |
| `TRUST_INHERIT_DECAY` | 0.70 | 派生グラフ（Gnew）が親（Gold）のoperationalを引き継ぐ割合。**上げると**（例：0.85）親の高信頼をほぼそのまま受け継ぐため派生グラフが早く信頼を得られるが、低品質な親から生まれた子も高信頼になるリスクがある。**下げると**（例：0.50）派生グラフは親とほぼ無関係にゼロから信頼を積み上げるため安全だが立ち上がりが遅い。親グラフの品質管理が十分できている環境では高め、不確かな環境では低めに設定する |
| `TRUST_DEBOUNCE_DELTA` | 0.05 | Human/Semanticのフィードバック更新後にcompositeスコアがこの値未満しか変動しなかった場合、キャッシュ無効化をスキップする閾値。**小さくすると**（例：0.01）細かいフィードバックでもキャッシュが再計算されるため鮮度は上がるが再計算コストが増える。**大きくすると**（例：0.10）キャッシュが安定するがフィードバックの反映が遅くなる。高頻度フィードバック環境では大きめが有利 |

### TrustProfile更新アルゴリズム系

| 定数名 | 値 | 調整ガイド |
|---|---|---|
| `OPERATIONAL_EMA_ALPHA` | 0.15 | 実行成功/失敗をoperational信頼に反映する速さ（EMA係数）。**上げると**（例：0.30）直近の実行結果が強く反映され、連続失敗で信頼が急落・連続成功で急回復する。変動の激しい環境に向く。**下げると**（例：0.05）過去の実績が長く保持され安定するが、品質劣化への反応が遅れる。実行頻度が低い環境では低めが安定 |
| `SEMANTIC_EMA_ALPHA` | 0.10 | semantic信頼の更新速さ。operationalより低めに設定されているのは意味的一致度が急変しにくい性質を反映している。基本的にOPERATIONAL_EMA_ALPHAより低く保つことが設計意図 |
| `TEMPORAL_LAMBDA_USE` | 0.0001/分 | 最後に使用されてからの時間経過でtemporal信頼が減衰する速さ（半減期約4.8日）。**上げると**使われないグラフが数日で急速に陳腐化し、活発に使われるグラフだけが生き残る。更新頻度の高いドメインに向く。**下げると**長期間使われなくてもtemporal信頼が保たれる。バッチ処理など実行頻度が低い環境に向く |
| `TEMPORAL_LAMBDA_VERIFY` | 0.00005/分 | 最後に手動検証されてからの減衰速さ（半減期約9.6日）。**必ずLAMBDA_USEより小さく保つこと**（設計上の不変条件）。検証コストが高い環境では小さくして検証の価値を長持ちさせる。検証が頻繁にできる環境では大きくしても良い |
| `TEMPORAL_ALPHA_BLEND` | 0.35 | use側減衰（lambda_use）とverify側減衰（lambda_verify）のブレンド比。0.35はuse側の重みで、verify側が0.65。**上げると**使用頻度をより重視するため活発に使われるグラフが優遇される。**下げると**手動検証の履歴をより重視するため、使用頻度が低くても品質保証されたグラフが選ばれやすくなる |
| `HUMAN_TRUST_K` | 0.08 | 1回のthumb up/downがHumanTrust値をどれだけ動かすかの学習率。**上げると**（例：0.15）数回のフィードバックで信頼値が大きく変化し、少人数でも評価が反映されやすい。**下げると**（例：0.03）多数のフィードバックが集まるまで信頼値が安定し、単発の誤評価に頑健になる。ユーザー数が少ないフェーズでは低めが安全 |
| `HUMAN_TRUST_SCALE` | 0.30 | ロジスティック関数の感度を決めるスケール係数。**上げると**スコアが0.5付近に集中しやすくなり極端な値になりにくい（保守的）。**下げると**フィードバックの差異がスコアに大きく出る（感度が高い） |

### TrustProfile複合スコア重み系

4つの重みは合計1.0になる必要があります。**どれかを上げたら他を下げる必要があります。**

| 定数名 | 値 | 調整ガイド |
|---|---|---|
| `COMPOSITE_WEIGHT_OPERATIONAL` | 0.35 | 実行成功率の重み。**上げると**実際に動いた実績を最重視するようになり、よく使われるグラフが選ばれやすくなる。信頼性が最優先の本番環境に向く |
| `COMPOSITE_WEIGHT_SEMANTIC` | 0.25 | 意味的一致度の重み。**上げると**ミッションとの意味的な近さを重視するため、新しいタスクに対して意味的に近いグラフが積極的に選ばれるようになる。ドメイン多様性が高い環境に向く |
| `COMPOSITE_WEIGHT_TEMPORAL` | 0.20 | 鮮度（使用・検証からの経過時間）の重み。**上げると**古いグラフが強く排除され常に新鮮なグラフが優先される。情報の陳腐化が早いドメインに向く。**下げると**実績ある古いグラフも長く使われ続ける |
| `COMPOSITE_WEIGHT_HUMAN` | 0.20 | 人間フィードバックの重み。**上げると**ユーザー評価が信頼に強く反映される。ユーザーが専門家でフィードバックの質が高い環境に向く。フィードバックが少ない初期フェーズでは低めにして実行実績（operational）を重視する方が安定する |

### ApplicabilityScore系

| 定数名 | 値 | 調整ガイド |
|---|---|---|
| `APPLICABILITY_THRESHOLD` | 0.50 | **最も影響の大きい調整値。** このスコア以上で完全再利用（LLM呼び出しゼロ）、未満でGraphPatch生成へ分岐する。**上げると**（例：0.65）厳密に似たグラフのみ再利用するためLLMへの依存が増すが出力品質が安定する。**下げると**（例：0.35）多少異なるグラフも再利用候補になりコストが下がるが、ズレたグラフが選ばれるリスクがある。チューニングの第一候補 |
| `APPLICABILITY_ALPHA_S` | 0.40 | ApplicabilityScoreにおける類似度（Stotal）の指数重み。**上げると**類似度の高さがスコアに強く反映され、意味的・構造的に近いグラフが強く優遇される |
| `APPLICABILITY_ALPHA_D` | 0.30 | 決定論性の重み。**上げると**副作用が多い・非決定論的なグラフが強く排除される。外部API呼び出しを含むワークフローが多い環境では高めが安全 |
| `APPLICABILITY_ALPHA_T` | 0.30 | 信頼スコアの重み。**上げると**信頼実績のあるグラフが圧倒的に優遇され、新規グラフはほぼ選ばれなくなる。安定運用フェーズに向く |
| `APPLICABILITY_FLOOR_S/D/T` | 0.10/0.10/0.20 | 各スコアの下限クランプ値。幾何平均でゼロが含まれると全体がゼロになるのを防ぐ安全ネット。基本的に変更不要だが、floorTはTRUST_HARD_GATE_THRESHOLDと同値に保つこと |

### DeterminismScore系

| 定数名 | 値 | 調整ガイド |
|---|---|---|
| `SOFT_MIN_BETA` | 5.0 | SoftMinの集約の鋭さ。**上げると**（例：10.0）グラフ内で最も決定論性が低いノードのスコアに全体が引っ張られるようになる（ボトルネック重視）。**下げると**（例：2.0）全ノードの平均的な決定論性が反映される（均等視）。外部API呼び出しなど危険なノードが1つあれば全体を下げたい場合は高めに設定する |
| `DETERMINISM_THRESHOLD` | 0.50 | この値未満のDeterminismScoreを持つワークフローを非決定論的として拒否する閾値。**上げると**より決定論的なワークフローのみが選ばれるようになり再現性が上がる。副作用リスクを厳しく管理したい本番環境向き。**下げると**RAGや外部APIを含む多様なワークフローも通過できるようになる |

### 類似度検索系

| 定数名 | 値 | 調整ガイド |
|---|---|---|
| `ANN_TOP_K` | 10 | Stage 2のANN検索で取得する候補数。**上げると**（例：20）より多くの候補からStage 3/4で精密選択できるためヒット率が上がるが、GED計算コストがk倍増える。**下げると**（例：5）高速だが最良候補を見逃すリスクがある。リポジトリが1万件を超えた段階で引き上げ検討が推奨される |
| `SIMILARITY_ALPHA` | 0.35 | Stotalにおける構造的類似度Sstructの重み（意味的類似度は1-α=0.65）。**上げると**グラフの形状・構造の一致を重視するようになり、同じエージェント構成のワークフローが優遇される。**下げると**ミッションの意味的な近さだけで選ばれやすくなり構造が違っても再利用が起きる。序盤（グラフ密度が低い時期）は低め、成熟期は高めが効果的 |
| `GED_GRAPH_SIZE_LIMIT` | 50 | GED精密計算とgraph_embeddingコサイン類似度の切り替えノード数閾値。GEDはNP困難なため大きなグラフには近似しか使えない。**上げると**より大きなグラフまでGEDで精密比較するが計算コストが指数的に増える。**下げると**小さいグラフもembeddingで高速比較するが精度が落ちる |
| `GED_BLEND_MARGIN` | 5 | 境界付近（±5ノード）でGEDとembeddingをブレンドするスムージング幅。小さくすると切り替えが急峻になりスコアに不連続が生じやすい。変更の必要性は低い |

### PatchConfidence系

| 定数名 | 値 | 調整ガイド |
|---|---|---|
| `PATCH_CONFIDENCE_THRESHOLD` | 0.75 | パッチの自動適用を許可する信頼度の下限。**上げると**（例：0.85）より確実なパッチのみ自動適用され安全だが、人間レビューに回される頻度が増える。**下げると**（例：0.60）より積極的にパッチが自動適用されてスループットが上がるが、品質リスクが増す。本番初期は高め、実績が出てから徐々に下げるのが推奨パターン |
| `SELF_CONF_DISCOUNT` | 0.85 | LLMの自己評価スコアcₛにかける補正係数（過信頼対策）。**上げると**（例：1.0）LLMの自己評価をそのまま信用する。LLMのキャリブレーションが十分なモデルを使っている場合に有効。**下げると**（例：0.70）LLMの自信を強く割り引いてvalidatorとhistoryに依存するようになる。M2以降に実際の過信頼データが蓄積したら実績ベースで調整する（OQ-03） |
| `PATCH_SELF_CONF_SWITCH_THRESHOLD` | 0.50 | LLMの自己評価がこの値未満の場合に重みをWS_LOW/WV_HIGHに切り替える閾値。**上げると**（例：0.65）より高い自信を要求するようになりvalidator優先が発動しやすくなる。保守的な運用に向く |
| `PATCH_CONFIDENCE_WS` / `WS_LOW` | 0.30 / 0.20 | LLM自己評価スコアの通常時/低自信時の重み。WS_LOWはLLMが「自信なし」と言っている場合にLLMへの依存を下げるための値。この2つの差を大きくするほど低自信時にvalidatorへの依存が強まる |
| `PATCH_CONFIDENCE_WV` / `WV_HIGH` | 0.40 / 0.50 | バリデータスコアの通常時/LLM低自信時の重み。バリデータの信頼性が高い環境（厳密なスキーマ検証が実装されている場合）は高めに設定すると良い |
| `PATCH_CONFIDENCE_PRIOR` | 0.50 | パッチ履歴がない状態（cold-start）での履歴スコアcₕの初期値。**上げると**履歴なしでも楽観的にパッチを試みるようになる。**下げると**初回パッチ適用のハードルが上がり慎重になる |

---

## 23. 付録 B — エラー型全体

v1.9 では既存の `ValidationError`、`CompileError`、`RepositoryError`、`SearchValidationError` 等に加え、`TrainingError` を追加する。training plane 導入によって既存エラー型の意味論を再定義してはならない。


```rust
// Layer 2: バリデーションエラー
#[derive(Debug, thiserror::Error)]
enum ValidationError {
    #[error("Cycle detected involving node {0}")]
    Cycle(Uuid),
    #[error("Duplicate node ID: {0}")]
    DuplicateNodeId(Uuid),
    #[error("Unresolved variable '{var}' at node {node_id}")]
    UnresolvedVariable { var: String, node_id: Uuid },
    #[error("Unresolved SubWorkflow: {0:?}")]
    UnresolvedSubWorkflow(WorkflowId),
    #[error("FanOut/Collect mismatch for branch_id {0}")]
    FanOutCollectMismatch(usize),
    #[error("Isolated node: {0}")]
    IsolatedNode(Uuid),
    #[error("Invalid mapping in SubWorkflow {node_id}: {detail}")]
    InvalidMapping { node_id: Uuid, detail: String },
}

// Layer 2: コンパイルエラー (§7.2 参照)
// Layer 2.5: パッチエラー (§12.6 参照)
// Layer 3: リポジトリエラー (§8.3 参照)
#[derive(Debug, thiserror::Error)]
enum RepositoryError {
    #[error("Version conflict: expected {expected}, found {actual}")]
    UpdateConflict { expected: u64, actual: u64 },
    #[error("Graph not found: {0:?}")]
    NotFound(WorkflowGraphId),
    #[error("Cross-store inconsistency detected")]
    CrossStoreInconsistency,
}
```

---

## 24. 付録 C — 数式インデックス

v1.9 は v1.8-final の Applicability / Knowledge Applicability / Temporal Freshness / LifecycleScore 数式を変更しない。TrainingTrust → ProductionTrust の写像は reference design であり、最終係数は calibration candidate とする。


| 式番号 | 数式 | 箇所 |
|--------|------|------|
| (1) | GMR cost: G_T = (L, C(G_T, Inc(G_T))) | §1 |
| (2) | Stotal(Gᵢ, Gⱼ) = (1−α)×Ssem + α×Sstruct | §11.3 |
| (3) | D(G) = (−1/β) × ln(Σᵢ (wᵢ/W) exp(−β dᵢ)) | §10.2 |
| (4) | A = ∏ₖ max(vₖ, floorₖ)^αₖ | §10.3 |
| (5) | TrustProfile.composite = 0.35×op + 0.25×sem + 0.20×temp + 0.20×hum | §9.4 |
| (6) | DualTemporalTrust = α×exp(−λ_use Δt_use) + (1−α)×exp(−λ_verify Δt_verify) | §9.2 |
| (7) | HumanTrustLogistic: h_{n+1} = h_n + k(outcome − E); E = σ((h−0.5)/s) | §9.3 |
| (8) | patchconfidence = cₛ_adj^wₛ × cᵥ^wᵥ × cₕ^wₕ | §12.3 |
| (9) | EMA: trust_{n+1} = (1−α)×trust_n + α×outcome | §9.1 |
| (10) | E[Σₜ L_t / T] ≤ E[Σₜ L_t^baseline / T] (定常タスク分布下) | §14 |
| (11) | inherited_op = max(parent.op × TRUST_INHERIT_DECAY, TRUST_COLD_START_OPERATIONAL) | §8.2 |

---

## 25. 補足: データベース構成：SQLite + LadybugDB

v1.9 では、training-specific metadata を SQLite runtime metadata または同等の workflow-side metadata store に保持してよい。一方、knowledge object の source-of-truth は引き続き LadybugDB にあり、training knowledge は sandbox namespace 付き knowledge object または `CandidateKnowledgeDocument` として segregate されるべきである。

workflow-side training metadata の例:

- TrainingMission table
- TrainingRunLog table
- TrainingFeedback table
- PromotionCandidate table
- TrainingAuditLog table
- CurriculumQueue table

これらは WorkflowRepository の graph blob source-of-truth を置き換えるものではなく、join / audit / queue / review / promotion を支える補助ストアである。

v2.3-c では、会話メタデータについても同様に以下の workflow-side 推奨テーブルを追加する。

- ConversationalEventLog table
- ConversationalProposalLog table
- ConsolidationRunLog table

これらの詳細スキーマは付録 D (§26 D.6) に定義する。knowledge object （Fragment / CandidateKnowledgeDocument / CanonicalDocument）の source-of-truth は引き続き LadybugDB にあり、会話由来の知識も sandbox namespace 下で LadybugDB に保持される。


### 25.x クロスストア書き込み規約 (v1.7 追補)

SQLite は trust / lifecycle / lineage / audit の正本、LadybugDB は graph / embedding / ANN index の正本として役割分離してよい。ただし更新操作は常に `op_id` 単位で結び付け、両ストアの commit 成否を `ConsistencyState` に反映しなければならない (MUST)。

推奨手順は次の通りである。

1. SQLite に commit intent を記録する。
2. LadybugDB 側の graph / embedding / index 更新を実施する。
3. SQLite 側の最終 commit と audit 反映を完了する。
4. 両方成功した場合のみ `Committed` とする。
5. 途中失敗時は `NeedsRepair` へ遷移し、通常検索から除外する。

この順序は規範上の唯一解ではないが、少なくとも「途中失敗が検知不能な実装」を許してはならない (MUST NOT)。



v1.4 では graph_embedding を LadybugDB の必須要件から外し、LadybugDB 側は WorkflowGraph 本体・task_embedding・HNSW インデックスを中心に保持する。SQLite 側には v1.3 の Trust / Provenance / Metrics / PatchHistory / TrustAuditLog に加え、WorkflowLineage・RefinementRunLog・ContributionRecord・DeterminismObservation・DeterminismProfile・AbstractionHistory を保持する。

### 前提

RFC-0001はDBスキーマを意図的にスコープ外としており、永続化バックエンドは実装者に委ねられている。LadybugDBがHNSWをネイティブサポートしている前提で、RFC-0001の全スコープ（M3まで）をフルローカルで網羅できる。

***

### 役割分担

| コンポーネント | 担当DB | 理由 |
|---|---|---|
| `WorkflowGraph`（ノード・エッジ構造） | **LadybugDB** | グラフ構造をネイティブに保存でき、PatchOperation（AddNode/RemoveNode/AddEdge等）がそのままグラフDB操作に対応する |
| `task_embedding`（ミッション意味ベクトル） | **LadybugDB** | HNSWインデックスと同じDB内に置くことでStage 2 ANN検索が完結する |
| `graph_embedding`（グラフ形状ベクトル、M1以降） | **LadybugDB** | 同上 |
| `AnnIndex`（HNSWインデックス） | **LadybugDB** | ネイティブ対応のため外部ライブラリ不要 |
| `TrustProfile`（4軸スコア数値） | **SQLite** | フラットな数値カラムで直接対応できる |
| `Provenance`（時刻・バージョン文字列） | **SQLite** | 単純なカラム保存で十分 |
| `GraphVersion`（CAS用カウンタ） | **SQLite** | `UPDATE ... WHERE version = ?`で原子的CASが実現できる |
| `TrustAuditLog`（監査ログ） | **SQLite** | 追記のみのシンプルなテーブルで十分。RFC内で「バックエンドは実装依存」と明示されている |
| `PatchHistory`（パッチ成功率履歴） | **SQLite** | 集計クエリ（成功率計算）が得意 |
| `Metrics`（実行統計） | **SQLite** | success_rate / avg_latency_ms等の数値集計に向く |

***

### テーブル・スキーマのイメージ

**SQLite側**

```sql
-- MemoizedGraphのメタデータ
CREATE TABLE memoized_graphs (
    id              TEXT PRIMARY KEY,   -- WorkflowGraphId（UUID）
    ladybug_graph_id TEXT NOT NULL,     -- LadybugDB側のグラフIDへの外部キー
    version         INTEGER NOT NULL DEFAULT 0,  -- GraphVersion（CAS用）
    -- TrustProfile
    trust_operational  REAL NOT NULL,
    trust_semantic     REAL NOT NULL,
    trust_last_used_at INTEGER NOT NULL,     -- UNIXタイムスタンプ
    trust_last_verified_at INTEGER NOT NULL,
    trust_human_score  REAL NOT NULL,
    trust_human_count  INTEGER NOT NULL,
    -- Metrics
    success_rate    REAL NOT NULL,
    avg_latency_ms  INTEGER NOT NULL,
    token_cost_avg  INTEGER NOT NULL,
    run_count       INTEGER NOT NULL,
    last_run_at     INTEGER NOT NULL,
    -- Provenance
    created_at      INTEGER NOT NULL,
    source_version  TEXT NOT NULL,
    environment_hash INTEGER NOT NULL
);

-- TrustAuditLog
CREATE TABLE trust_audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    graph_id    TEXT NOT NULL,
    event_type  TEXT NOT NULL,  -- 'AdminFastTrack' | 'ManualOverride'
    actor_id    TEXT NOT NULL,
    old_value   REAL NOT NULL,
    new_value   REAL NOT NULL,
    timestamp   INTEGER NOT NULL,
    reason      TEXT
);

-- PatchHistory
CREATE TABLE patch_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source_graph_id TEXT NOT NULL,
    diff_spec_hash  TEXT NOT NULL,  -- DiffSpecのハッシュ（類似検索用）
    success         INTEGER NOT NULL,  -- 0 or 1
    patch_confidence REAL NOT NULL,
    applied_at      INTEGER NOT NULL
);
```

**LadybugDB側**

```
ノード: WorkflowNode
  - agent, prompt_template, determinism,
    side_effects（各フラグ）, timeout_secs, error_mode

エッジ: EdgeMeta
  - DependsOn / DataFlow / Conditional / FanOut / Collect

ベクトルプロパティ:
  - task_embedding: Vec<f32>   （HNSWインデックス対象）
  - graph_embedding: Vec<f32>  （HNSWインデックス対象、M1以降）
```

***

### データの流れ

```
[新規ワークフロー登録]
  1. LadybugDBにグラフ構造・埋め込みを保存 → ladybug_graph_id取得
  2. SQLiteにメタデータ・TrustProfile・GraphVersionを保存
     （ladybug_graph_idを外部キーとして持つ）

[GMR検索 Stage 0〜1]
  SQLiteで agentsethash / TrustProfile のフィルタリング

[GMR検索 Stage 2]
  LadybugDBのHNSWで task_embedding + graph_embedding の近傍検索

[GMR検索 Stage 3〜4]
  LadybugDBからグラフ構造を取得してGED計算
  SQLiteからTrustProfileを取得してApplicabilityScore計算

[パッチ適用・新規登録]
  apply_patch_atomic（純粋計算、DB操作なし）
  → LadybugDBに新Gnewグラフを保存
  → SQLiteにGnewのメタデータを新規INSERT
  → GoldのGraphVersionはSQLiteのCASで保護
    （UPDATE ... WHERE version = expected_version）

[TrustProfile更新]
  SQLiteのMemoizedGraphレコードをトランザクション更新
  debounce条件を満たす場合のみapplicabilityキャッシュ無効化
```

***

### GraphVersion CASの実装パターン

RFC-0001のP-09（楽観的並行性制御）はSQLiteの行ロックなしで以下のように実現できる：

```sql
UPDATE memoized_graphs
SET graph = ?, version = version + 1
WHERE id = ? AND version = ?;
-- 影響行数が0ならUpdateConflictエラーを返す
```

競合が起きた場合はRFC仕様通り最新バージョンで再試行する。

***

### マイルストーンごとの対応状況

| マイルストーン | SQLite | LadybugDB |
|---|---|---|
| **M -1** | インメモリ `RwLock<Vec<MemoizedGraph>>` のみ（DB不要） | 不要（FakeImpl で完結） |
| **M0** | メタデータ・Trust・Provenance保存のみ | グラフ構造保存のみ（ベクトル不要） |
| **M1** | Trust・AuditLogフル稼働 | task_embeddingのHNSW検索が加わる |
| **M2** | PatchHistory追加 | パッチ派生グラフの保存が増える |
| **M3** | 変更なし | graph_embeddingのHNSWインデックス追加、大規模化対応 |


---

## 26. 付録 D — v1.7 / v1.8 / v1.9 追加データモデル

### D.1 v1.7 追加データモデル

本付録の既存 v1.7 データモデル定義はそのまま維持される。

### D.2 v1.8 追加データモデル

v1.8 では、KnowledgeEvidenceBundle、VersionContext、FreshnessSummary、ConfidenceMeta、knowledge-aware QueryRepresentation 拡張、ならびに SearchTrace / SearchRunLog の knowledge フィールド拡張が追加される。これらはすべて additive であり、既存 v1.7 実装は空値または既定値によって後方互換に移行できる。


```rust
struct LifecycleAuditLog {
    graph_id: WorkflowGraphId,
    old_state: GcState,
    new_state: GcState,
    lifecycle_score: f32,
    resource_pressure: f32,
    actor: Option<String>,
    timestamp: SystemTime,
    reason: String,
}

struct SocialAccelerationSnapshot {
    measured_at: SystemTime,
    reuse_ratio: f32,
    patch_ratio: f32,
    compose_ratio: f32,
    new_ratio: f32,
    false_new_rate: f32,
    subworkflow_reuse_growth: f32,
    success_improvement_velocity: f32,
}
```



### D.1 SearchWorkflow コア型

```rust
struct SearchWorkflowSpec {
    budget: SearchBudget,
    recursion_guard: RecursionGuard,
    initial_policy: RetrievalPolicy,
}

struct SearchRepositoryFields {
    workflow_kind: WorkflowKind,
    search_policy_json: Option<String>,
    latest_search_run_id: Option<String>,
}

enum WorkflowKind {
    Application,
    Search,
}
```

### D.2 SearchTrace / SearchRunLog

```rust
struct SearchTraceEntry {
    run_id:             String,
    iteration:          u32,
    state:              SearchState,
    query_text_hash:    u64,
    query_design_hash:  u64,
    selected_candidate: Option<WorkflowGraphId>,
    selected_outcome:   Option<SearchOutcomeKind>,
    budget_snapshot:    SearchBudgetSnapshot,
    justification_hash: u64,
}
```

### D.3 Compose / New proposal

```rust
struct NewWorkflowProposal {
    mission_text:        String,
    proposed_graph:      WorkflowGraph,
    confidence:          f32,
    requires_human_gate: bool,
}
```

### D.4 v1.9 Candidate Knowledge Handling

```rust
struct CandidateKnowledgeDocument {
    knowledge_id: String,
    source_run_id: String,
    namespace: String,
    evidence_summary: String,
    origin_trace_ids: Vec<String>,
    completeness_score: f32,
    promotion_status: PromotionStatus,
    created_at: SystemTime,
}
```

### D.5 v1.9 Curriculum / Audit 補助型

```rust
struct CurriculumPolicy {
    domain_weights: HashMap<String, f32>,
    difficulty_weights: HashMap<String, f32>,
    failure_class_weights: HashMap<String, f32>,
    objective_weights: HashMap<String, f32>,
    updated_at: SystemTime,
}

struct TrainingAuditLog {
    mission_id: Option<String>,
    run_id: Option<String>,
    candidate_id: Option<String>,
    actor_id: Option<String>,
    event_type: String,
    timestamp: SystemTime,
    reason: Option<String>,
}
```

### D.6 v2.3-c 会話ログ推奨スキーマ

v2.3-c では、会話イベント・分類提案・統合実行の各ログに対して以下の推奨スキーマを定義する。これらは実装自由度を妨げない推奨構造であり、SQLite runtime metadata store または同等の workflow-side store に保持してよい。

```rust
struct ConversationalEventLog {
    event_id: String,
    session_id: String,
    user_id: String,
    actor: String,
    timestamp: SystemTime,
    channel: String,
    redacted_text: String,
    raw_text_ref: Option<String>,
    policy_id: String,
}

struct ConversationalProposalLog {
    event_id: String,
    proposed_category: String,
    policy_score: f32,
    llm_confidence: f32,
    contains_pii: bool,
    proposed_namespace: String,
    created_at: SystemTime,
}

struct ConsolidationRunLog {
    run_id: String,
    namespace: String,
    candidate_set_id: String,
    candidate_id: Option<String>,
    semantic_coherence: f32,
    trace_completeness: f32,
    contradiction_score: f32,
    decision: String,
    created_at: SystemTime,
}
```

## 27. 付録 E — v1.8 / v1.9 Calibration Candidates

The following constants are normative in v1.8 but are explicitly designated as future calibration candidates: knowledge applicability exponents in Section 11.5, the mutation safety threshold `K >= 0.50`, the audit-grade hard gate `K < 0.30`, and evidence-completeness tie-break policies in Section 16.4. Implementations MUST NOT silently change these values within a v1.8 deployment; any change requires explicit versioning, migration notes, and replay/evaluation evidence.

In addition, v1.9 designates the following as training-related calibration candidates: training trust → production trust inheritance ratio, sandbox success / Good ratio thresholds, candidate tombstone grace period, curriculum weight decay, AI-generated mission auto-approval exception scope, and promotion rollback granularity. In addition, recommended initial values for human review SLA (review timeout, escalation timeout, max batch size) are provided in Annex A and SHALL be treated as calibration candidates rather than hard guarantees. These parameters MAY evolve only through explicit versioned revision rather than implementation-local drift.

This annex exists to preserve the design discipline established in v1.7: parameters MAY evolve, but only through explicit RFC-level revision rather than ad hoc implementation drift.

In addition, v2.3-c designates the following as conversational calibration candidates: consolidation thresholds (min_distinct_events, min_distinct_days, min_semantic_coherence, min_trace_completeness, min_temporal_stability, max_contradiction_score), LLM confidence threshold for auto-sandbox-ingest, promotion completeness thresholds, and contradiction coexistence policy. These parameters SHALL be treated as calibration candidates with explicit versioned defaults rather than implementation-local drift. Initial normative defaults are provided in §16B.5 and §22 (v2.3-c 追加定数).


## 28. 参照文献

1. Yash Raj Singh (2024). "Graph-Memoized Reasoning: Foundations for Structured Workflow Reuse in Intelligent Systems." arXiv:2511.15715
2. RightNow-AI/openfang — Open-source Agent Operating System (Rust). GitHub.
3. petgraph::algo::toposort — Docs.rs.
4. petgraph::stable_graph::StableGraph — Docs.rs.
5. RFC 9923 — The FNV Non-Cryptographic Hash Algorithm. IETF.
6. Graph edit distance — Wikipedia.
7. "Enhancing Graph Edit Distance Computation." VLDB Endowment.
8. "Computing Approximate Graph Edit Distance via Optimal Transport." arXiv.
9. "Taming Overconfidence in LLMs: Reward Calibration in RLHF." arXiv.
10. "Mind the Confidence Gap: Overconfidence, Calibration, and LLMs." arXiv.
11. LogSumExp (SoftMin) — Wikipedia.
12. "Balancing stability and flexibility: investigating a dynamic K value in the Elo rating system."
13. Darwin Gödel Machine. arXiv:2505.22954. (RFC-0003 関連)
14. "Patch Graph Rewriting." ICGT 2020.
15. RVPO: "Risk-Sensitive Alignment via Variance Regularization." arXiv. (SoftMin の LogSumExp 正当化)
16. Darvium v1.9 策定指示書. Human-guided training architecture, promotion discipline, and Training Plane integration.



---

## 28. Repository Pair / Expert Fusion 統合仕様 (v2.0-final)

Revision v2.0 preserves the full normative content of v1.9 without weakening, deleting, or redefining any prior guarantee, source-of-truth boundary, trust rule, lifecycle rule, training invariant, applicability equation, patch rule, audit requirement, or repair discipline. v2.0 is a strictly additive revision that introduces repository-pair-level synthesis fusion as a first-class operation over the already established four-plane logical architecture.

The purpose of this revision is not to define a destructive database merge. The purpose is to define safe birth, selective extraction, fusion, split, and recomposition of repository pairs composed of SQLite and LadybugDB, while preserving full lineage, contribution history, actor traceability, training / production separation, and dual-store operational integrity.

A Repository Pair in v2.0 SHALL be interpreted as a portable operational individual composed of: (a) SQLite-side workflow, trust, lifecycle, audit, training, and runtime metadata; and (b) LadybugDB-side knowledge objects, relations, and origin-bearing evidence structures. Fusion operations SHALL create a new output pair and MUST NOT destructively mutate existing input pairs in place.

This revision further introduces the concept of Expert Namespace as the primary semantic selection unit for extraction and fusion. Expert assets SHALL be selected by explicit manifest and closure policy rather than by ad hoc file copying, raw prefix matching, or implementation-local heuristics that would make lineage and admissibility non-deterministic.

For avoidance of ambiguity, v2.0-final does NOT define automatic semantic merge, automatic truth arbitration, or confidence-weighted winner selection between conceptually similar knowledge objects imported from different source pairs. When two or more knowledge objects appear semantically overlapping, the default-safe rule is coexistence under regenerated identities plus explicit lineage relations such as `CONSOLIDATES` or `SUPERSEDES`; destructive collapse into a single canonical object is out of scope for this revision.

## 29. Fusion Core Terminology (v2.0)

| Term | Definition |
|------|------------|
| **Repository Pair** | SQLite + LadybugDB を source-of-truth 境界を保ったまま一体として扱う可搬個体 |
| **Expert Namespace** | 専門家を識別する主要 namespace。抽出・融合・分割の選択単位 |
| **Expert Manifest** | Expert を構成する workflow / subworkflow / knowledge / training / audit 資産束と closure policy を宣言する formal object |
| **Extraction Plan** | 単一 Repository Pair から指定 Expert を切り出して新 pair を誕生させる宣言的計画 |
| **Fusion Plan** | 複数 Repository Pair から指定 Expert 群を選択し、新 pair を誕生させる宣言的計画 |
| **Fusion Result Pair** | extraction / fusion / split / recompose の結果として新規作成される output Repository Pair |
| **SplitPairByExpert** | 一つの pair を expert 単位で複数新 pair に分割する operation |
| **RecomposePair** | 一つの pair 内の複数 expert の namespace / policy / root を再編して新 pair を作る operation |
| **Lineage Preservation Rule** | source pair / source expert / source object / source actor / source run / source feedback への到達可能性を lossless に保持する規則 |
| **IdentityRemapTable** | source ID から target ID への完全再写像を保持する表 |
| **Fusion Audit Record** | extraction / fusion / split / recompose 実行自体の監査記録 |
| **Pair Birth Lifecycle** | output pair が draft / materializing / pending commit / committed / quarantined / failed などの状態を経る誕生状態機械 |
| **Primary Membership** | ある Expert Namespace に直接所属する資産 |
| **Required Dependency Closure** | root asset を正常に機能させるために必須の外部 workflow / knowledge / relation 閉包 |
| **Optional Contextual Closure** | audit / trace / training context のうち説明可能性・再現性のために付随させる任意閉包 |
| **Training-aware Fusion** | training artifact を明示 policy の下で sandbox namespace に保持したまま扱う fusion |
| **Birth Commit** | output Repository Pair の SQLite 側 / Ladybug 側 materialization 完了後に shared operation intent を確定させる application-level commit |

## 30. Repository Pair Model

v2.0 defines Repository Pair as a first-class repository-level object without altering the pre-existing ownership boundaries of v1.9. SQLite SHALL remain authoritative for workflow graphs, WorkflowLineage, TrustProfile, Lifecycle state, SearchTrace, TrainingMission, TrainingRunLog, TrainingFeedback, PromotionCandidate, TrainingAuditLog, runtime queues, repair state, and fusion-side metadata. LadybugDB SHALL remain authoritative for knowledge objects, knowledge relations, origin trace structures, evidence lineage, and knowledge-level supersession / consolidation relations.

A Repository Pair SHALL therefore be treated as a logically coupled pair of sources of truth rather than as a monolithic database image. Any v2.0 extraction or fusion implementation that collapses these boundaries into an implementation-local merged store MAY exist as a transient execution detail, but the normative ownership boundary after output pair birth MUST remain identical to v1.9.

The repository-level operations introduced in v2.0 are:

1. `ExtractExpert`
2. `FuseExperts`
3. `SplitPairByExpert`
4. `RecomposePair`

Each such operation SHALL produce one or more new output pairs. Existing input pairs MUST remain immutable with respect to their canonical persisted content except for optional append-only audit trails recording that they were used as lineage ancestors in a fusion operation.

## 31. Expert Boundary Model

An Expert in v2.0 SHALL be defined as a semantically coherent asset bundle identified primarily by namespace and secondarily by manifest-declared roots and closure policy. Namespace alone MUST NOT be treated as sufficient for safe extraction unless the manifest or migration rule has first established root workflow, root knowledge, and closure semantics.

The normative expert boundary model SHALL include at least the following three layers:

1. **Primary Membership** — assets directly belonging to the namespace.
2. **Required Dependency Closure** — external subworkflows, shared knowledge objects, and lineage-critical relations required for correct execution or explanation.
3. **Optional Contextual Closure** — audit logs, search traces, refinement logs, training logs, and similar material included for reproducibility or explainability.

The implementation MUST be able to answer, for any extracted or fused expert, which assets are primary, which are required dependencies, and which are optional context. Failure to preserve this classification SHALL be treated as a traceability defect.

### 31.1 Recommended ExpertManifest shape

```json
{
  "expert_id": "expert://finance/ledger-auditor",
  "namespace": "finance.ledger_auditor",
  "kind": "Production",
  "root_workflow_ids": ["wf_001", "wf_014"],
  "root_knowledge_ids": ["kb_1001", "kb_1099"],
  "includes_training_artifacts": false,
  "required_dependency_policy": "closure_required",
  "optional_context_policy": "include_audit_and_lineage",
  "selection_policy": {
    "allow_soft_deleted": false,
    "allow_training_only": false,
    "require_consistency_state": ["Committed"]
  }
}
```

The above schema is illustrative, but any normative alternative SHALL preserve equivalent expressive power.

## 32. Fusion / Extraction Operations

Fusion and extraction SHALL be defined as declarative repository transformations driven by formal plan objects. They SHALL NOT be specified merely as file-copy procedures, SQL dump-and-load procedures, or implementation-local batch scripts.

### 32.1 ExtractExpert

`ExtractExpert` SHALL accept a single input pair and one or more Expert Namespace selections and SHALL produce a new pair whose asset set is the closure of the selected experts under the declared dependency and context policy.

### 32.2 FuseExperts

`FuseExperts` SHALL accept two or more input pairs and one or more Expert Namespace selections from each pair and SHALL produce a new pair whose contents are determined solely by the declared plan, admissibility constraints, remap policy, lineage policy, and training policy.

### 32.3 SplitPairByExpert

`SplitPairByExpert` SHALL partition an input pair by manifest-defined expert boundaries and SHALL create one or more new output pairs. Shared dependency assets MAY be copied into multiple children if required for closure, but such duplication MUST preserve ancestry through lineage references and remap tables.

### 32.4 RecomposePair

`RecomposePair` SHALL allow namespace rewrite, root reorganization, or policy cleanup within a single input pair while still requiring non-destructive output-pair birth, identity remap, lineage preservation, and admissibility validation.

## 33. Admissibility and Safety Gates

The admissibility model for fusion SHALL be at least as strict as the normal v1.9 candidate selection path. Assets that are in `Pending`, `NeedsRepair`, or `Quarantined` consistency state MUST NOT silently enter a normal production fusion result.

The recommended default admissibility rules are normative unless explicitly overridden by a higher-risk operation mode with audit and human review:

| Condition | Default behavior | Allowed exception |
|-----------|------------------|-------------------|
| `ConsistencyState = Pending` | Reject | None |
| `ConsistencyState = NeedsRepair` | Reject | Repair-mode extraction only |
| `ConsistencyState = Quarantined` | Reject | Audit-mode only |
| `TrainingArtifactState = TrainingOnly` | Reject | Sandbox fusion only |
| `GcState = SoftDeleted` | Optional | Explicit opt-in |
| `GcState = Tombstoned` | Reject as active asset | Ancestor reference only |
| `PromotionStatus = Candidate / Rejected / RolledBack` into production pair | Human review required | None |

A fusion implementation MUST surface admissibility rejection explicitly in `FusionAuditRecord` and MUST NOT downgrade such rejections to silent omission when that omission would break required closure or lineage completeness.

## 34. Identity Remapping

Primary object IDs in the output pair SHOULD be regenerated by default in order to avoid cross-pair collisions and implicit aliasing. Partial reuse of source IDs MAY be permitted by future revisions, but v2.0 SHALL treat full regeneration with explicit trace table as the default-safe policy.

An `IdentityRemapTable` SHALL preserve, at minimum, the following fields:

| Field | Meaning |
|-------|---------|
| `source_pair_id` | 元 pair |
| `source_store` | `sqlite` or `ladybug` |
| `source_object_type` | workflow / knowledge / runlog / audit / relation / training object 等 |
| `source_id` | 元 ID |
| `target_pair_id` | 新 pair |
| `target_id` | 新 ID |
| `preserved_namespace` | 元 namespace |
| `remap_reason` | `extract` / `fuse` / `split` / `recompose` |

The output pair SHALL be considered traceability-incomplete if any materialized object lacks either: (a) a remap entry; or (b) an explicit declaration that the object is newly born in the target pair and therefore has no source ancestor.

### 34.1 Illustrative remap examples

```text
pair_A.workflow:wf_001   -> pair_C.workflow:wf_c_9001
pair_A.knowledge:kb_1001 -> pair_C.knowledge:kb_c_2001
pair_B.workflow:wf_777   -> pair_C.workflow:wf_c_9002
pair_B.audit:trainlog_55 -> pair_C.audit:trainlog_c_801
```

## 35. Lineage and Traceability Requirements

Full traceability is one of the central normative goals of v2.0. Cryptographic immutability remains out of scope, but logical and structural reachability to ancestry SHALL be preserved.

For the purposes of this RFC, lineage preservation means that the output pair SHALL retain a lossless procedure by which a current object can be traced to source pair, source object, source actor, source run, and source feedback whenever such ancestry exists in the inputs.

### 35.1 Workflow-side lineage

At minimum, the following workflow-side structures SHALL be preserved by remapped transfer or ancestor reference:

- `WorkflowLineage`
- `ContributionRecord`
- `Provenance`
- `PatchHistory`
- `TrustAuditLog`
- `LifecycleAuditLog`
- `SearchTrace` / `SearchRunLog`
- `RefinementRunLog`

### 35.2 Knowledge-side lineage

At minimum, the following knowledge-side lineage SHALL be preserved:

- knowledge objects themselves
- origin trace identifiers
- evidence summary and completeness fields
- lineage relations such as `DERIVEDFROM`, `CONSOLIDATES`, `SUPERSEDES`, `MATERIALIZEDAS`
- training-derived knowledge fields such as namespace, origin trace ids, and promotion state when represented as candidate knowledge artifacts

### 35.3 Training-side lineage

At minimum, the following training-side structures SHALL be preserved when included by plan policy:

- `TrainingMission`
- `TrainingRunLog`
- `TrainingFeedback`
- `PromotionCandidate`
- `TrainingAuditLog`
- `CurriculumPolicy` or curriculum relation metadata

### 35.4 Actor identity extension

v2.0 SHALL strengthen actor traceability beyond the v1.9 `actor_id: String` minimum. An implementation SHALL choose at least one of the following normative strategies:

1. Define `actor_id` as a stable reference resolvable against an external identity registry that yields public key and display information.
2. Introduce an `ActorRef` structure carrying stable actor reference, public key reference, and display-name snapshot.

Recommended shape:

```rust
struct ActorRef {
    actor_id: String,
    public_key_ref: String,
    display_name_snapshot: Option<String>,
    identity_provider: String,
}
```

### 35.5 Contribution quantification extension

v2.0 SHOULD extend contribution accounting to support actor-level and user-impact-level traceability.

```rust
struct ContributionRecordV2 {
    contribution_id: String,
    contributor: ActorRef,
    contribution_kind: ContributionKind,
    affected_user_count: u32,
    impact_score: f32,
    source_run_ids: Vec<String>,
    source_feedback_ids: Vec<String>,
    created_at: SystemTime,
    namespace: String,
}
```

## 36. Training / Production Separation in Fusion

The v1.9 Training Isolation Invariant, Promotion Discipline Invariant, Trust Separation Invariant, and Knowledge Promotion Invariant SHALL remain fully valid under v2.0 fusion semantics. No fusion operation may be defined in a way that silently bypasses promotion gates.

A `FusionPlan` or `ExtractionPlan` SHALL include an explicit training policy. Recommended values include:

- `exclude_training_only`
- `include_promoted_only`
- `include_candidates_with_human_gate`
- `sandbox_all_training`

For production-directed `FuseExperts`, the default SHOULD be `exclude_training_only`. For research or sandbox fusion, `sandbox_all_training` MAY be selected explicitly, but the resulting output pair MUST remain outside the normal production selection path unless and until promotion requirements are satisfied.

## 37. Fusion Orchestrator and Birth Commit

The following orchestrator shape is strongly recommended as a normative decomposition boundary.

```rust
struct FusionPlan { /* formal object */ }
struct ExtractionPlan { /* formal object */ }
struct ExpertManifest { /* expert boundary */ }
struct IdentityRemapTable { /* old -> new */ }
struct FusionAuditRecord { /* operation log */ }

trait PairFusionOrchestrator {
    fn validate_inputs(&self, plan: &FusionPlan) -> Result<(), FusionError>;
    fn compute_closure(&self, plan: &FusionPlan) -> Result<ClosureSet, FusionError>;
    fn remap_identities(&self, closure: &ClosureSet) -> IdentityRemapTable;
    fn materialize_sqlite_side(&self, closure: &ClosureSet, remap: &IdentityRemapTable) -> Result<(), FusionError>;
    fn materialize_ladybug_side(&self, closure: &ClosureSet, remap: &IdentityRemapTable) -> Result<(), FusionError>;
    fn attach_lineage(&self, closure: &ClosureSet, remap: &IdentityRemapTable) -> Result<(), FusionError>;
    fn finalize_birth(&self) -> Result<FusionResultPair, FusionError>;
}
```

Recommended execution order:

1. plan validation
2. admissibility filtering
3. expert closure computation
4. conflict scan
5. full identity remap generation
6. workflow-side materialization
7. knowledge-side materialization
8. lineage / audit / training linkage materialization
9. output pair consistency validation
10. birth finalize
11. fusion audit append

The output pair SHALL NOT enter the production selection path before birth finalize and post-materialization consistency validation have succeeded.

### 37.1 Birth commit discipline

Fusion birth commit SHALL follow the same application-level integrity philosophy as the v1.9 dual-store intent protocol. The precise commit phases MAY differ from knowledge mutation commit phases, but the implementation MUST persist enough intent, remap metadata, lineage linkage, and repair metadata to deterministically finish, quarantine, or tombstone an interrupted pair birth.

At minimum, a birth operation SHALL record:

- operation id
- input pair set
- selected expert set
- output pair target id
- remap policy
- lineage policy
- training policy
- current birth phase
- repair / quarantine reason if interrupted

## 38. Failure Handling, Quarantine, and Repair for Fusion

A failed or interrupted fusion SHALL be treated as a repository-level consistency event, not as an ignorable best-effort batch failure. If SQLite-side or Ladybug-side materialization completes only partially, the runtime MUST record a repairable failure state and MUST prevent the partially born pair from entering normal production retrieval.

Recommended failure states include `BirthPending`, `BirthNeedsRepair`, `BirthQuarantined`, and `BirthTombstoned`. Implementations MAY encode these using existing consistency or lifecycle machinery so long as the semantic distinction remains auditable.

The repair worker for fusion MAY attempt retry, quarantine, or compensating tombstone. Any successful repair MUST preserve the original fusion operation id, remap table, source-pair references, and lineage linkage. Silent rebirth under a fresh unrelated id without ancestor continuity is forbidden.

## 39. Migration and Backward Compatibility for v2.0

v2.0 MUST remain backward-compatible with v1.9 repository semantics. Existing v1.9 pairs that do not declare explicit Expert Manifest objects SHALL remain valid repository pairs.

However, extraction and fusion involving legacy pairs require a migration rule. The recommended migration policy is:

1. infer provisional expert boundaries from namespace, workflow roots, and knowledge roots;
2. mark the inferred manifest as provisional;
3. require human review when closure ambiguity or ownership ambiguity remains;
4. forbid irreversible production fusion when the provisional manifest cannot satisfy traceability requirements.

v1.9 `actor_id`-only logs SHALL remain legal historical records. v2.0 implementations SHOULD enrich them via external registry resolution or `ActorRef` augmentation at migration time where feasible, but MUST NOT rewrite historical semantics.

## 40. 付録 F — v2.0 追加データモデル

```rust
struct RepositoryPairId(String);

struct FusionPlan {
    fusion_plan_id: String,
    operation: FusionOperation,
    inputs: Vec<FusionInputPair>,
    output: FusionOutputSpec,
    selection_constraints: FusionSelectionConstraints,
    id_remap_policy: IdRemapPolicy,
    human_review_required: bool,
    reason: String,
}

enum FusionOperation {
    ExtractExpert,
    FuseExperts,
    SplitPairByExpert,
    RecomposePair,
}

struct FusionInputPair {
    pair_id: RepositoryPairId,
    sqlite_snapshot: String,
    ladybug_snapshot: String,
    experts: Vec<String>,
}

struct FusionOutputSpec {
    target_pair_id: RepositoryPairId,
    output_namespace_policy: OutputNamespacePolicy,
    lineage_policy: LineagePolicy,
    training_policy: FusionTrainingPolicy,
}

enum OutputNamespacePolicy {
    PreserveOriginal,
    RewriteTo(String),
    PrefixWith(String),
}

enum LineagePolicy {
    PreserveFull,
    PreserveByAncestorReference,
}

enum FusionTrainingPolicy {
    ExcludeTrainingOnly,
    IncludePromotedOnly,
    IncludeCandidatesWithHumanGate,
    SandboxAllTraining,
}

struct FusionSelectionConstraints {
    require_consistency_state: Vec<ConsistencyStateTag>,
    reject_quarantined: bool,
    reject_needs_repair: bool,
    allow_tombstoned_context: bool,
}

enum IdRemapPolicy {
    FullRegenerateWithTraceTable,
}

struct ExpertManifest {
    expert_id: String,
    namespace: String,
    kind: ExpertKind,
    root_workflow_ids: Vec<WorkflowGraphId>,
    root_knowledge_ids: Vec<String>,
    includes_training_artifacts: bool,
    required_dependency_policy: RequiredDependencyPolicy,
    optional_context_policy: OptionalContextPolicy,
    selection_policy: ExpertSelectionPolicy,
}

enum ExpertKind {
    Production,
    Sandbox,
    Hybrid,
}

enum RequiredDependencyPolicy {
    ClosureRequired,
    ExplicitOnly,
}

enum OptionalContextPolicy {
    ExcludeAll,
    IncludeAuditAndLineage,
    IncludeFullReproducibilityContext,
}

struct ExpertSelectionPolicy {
    allow_soft_deleted: bool,
    allow_training_only: bool,
    require_consistency_state: Vec<ConsistencyStateTag>,
}

enum ConsistencyStateTag {
    Committed,
    Pending,
    NeedsRepair,
    Quarantined,
}

struct IdentityRemapEntry {
    source_pair_id: RepositoryPairId,
    source_store: StoreKind,
    source_object_type: SourceObjectType,
    source_id: String,
    target_pair_id: RepositoryPairId,
    target_id: String,
    preserved_namespace: Option<String>,
    remap_reason: RemapReason,
}

struct IdentityRemapTable {
    entries: Vec<IdentityRemapEntry>,
}

enum StoreKind { Sqlite, Ladybug }

enum SourceObjectType {
    Workflow,
    Knowledge,
    RunLog,
    Audit,
    Relation,
    TrainingObject,
    PromotionObject,
}

enum RemapReason {
    Extract,
    Fuse,
    Split,
    Recompose,
}

struct FusionAuditRecord {
    fusion_op_id: String,
    plan_id: String,
    operation: FusionOperation,
    input_pair_ids: Vec<RepositoryPairId>,
    output_pair_id: RepositoryPairId,
    selected_experts: Vec<String>,
    lineage_policy: LineagePolicy,
    training_policy: FusionTrainingPolicy,
    result_state: FusionResultState,
    actor: Option<ActorRef>,
    created_at: SystemTime,
    reason: String,
}

enum FusionResultState {
    BirthPending,
    BirthCommitted,
    BirthNeedsRepair,
    BirthQuarantined,
    BirthTombstoned,
    Rejected,
}
```

## 41. 付録 G — Fusion Invariants and Open Questions

### 41.1 Fusion invariants

1. source-of-truth preservation invariant: workflow/trust/lifecycle/training metadata ownership remains on SQLite; knowledge ownership remains on LadybugDB.
2. non-destructive fusion invariant: input pairs MUST NOT be destructively modified.
3. full traceability invariant: target objects MUST remain ancestrally reachable through remap or explicit birth declaration.
4. training separation invariant: training-only artifacts MUST NOT silently enter production pairs.
5. admissibility invariant: quarantined / pending / needs-repair assets MUST NOT silently enter normal production fusion.
6. birth integrity invariant: partially materialized output pairs MUST remain outside the normal production selection path.
7. actor traceability invariant: audit and contribution history MUST retain actor reachability at least to stable external reference.

### 41.2 Open questions

- Whether pair-level reputation should remain purely derived from constituent asset metrics or gain explicit pair-level trust.
- Whether provisional manifest inference should be standardized further for legacy v1.9 repositories.
- Whether future revisions should allow selective semantic consolidation of knowledge objects under stricter evidence rules.
- Whether pair-birth lifecycle should reuse `ConsistencyState` directly or introduce a dedicated birth-state machine.

### 41.3 Deferred annex / future RFC responsibilities

The following topics are intentionally acknowledged but remain outside the normative closure of v2.0-final:

- **Formal guarantees annex** — proof obligations for applicability stability, trust convergence, lifecycle equilibrium, safety, and liveness.
- **Threat model annex** — malicious actor, prompt injection, knowledge poisoning, fusion poisoning, and training corruption.
- **Distributed architecture annex** — multi-node replication, consensus, partition handling, and remote repair coordination.
- **Exploration theory RFC-0003 scope** — search policy optimization, MCTS / bandit / RL selection theory, Pareto trust, and Darwinian evolution.

These omissions are deliberate scope boundaries rather than accidental gaps. Any future formalization SHALL preserve the v2.0-final source-of-truth, traceability, training-separation, and non-destructive-fusion invariants.


## 41A. v2.3 Operational Clarifications

本節は v2.3 で追加された strictly additive clarification をまとめる。ここで追加される内容は、v2.2 までに固定された Workflow IR、SearchWorkflow、DAG 二段検証、frontier-based parallel execution、Knowledge Ecosystem、Training Plane、Fusion semantics を変更するものではなく、既存実装の recovery discipline、calibration discipline、operational policy、measurement guidance を一段だけ明確化する補強である。

### 41A.1 Dual-store recovery invariant

Darvium の dual-store consistency discipline は、single-process / single-node 前提の application-level commit intent + recovery / repair discipline であり、XA、分散 2PC、multi-node consensus、あるいは汎用分散 transaction manager を規範化するものではない。

`ConsistencyState != Committed` を持つ dual-store operation は、normal selection path へ復帰する前に recovery / repair path を通過しなければならない。実装は、pending or partially applied dual-store operation を committed とみなしてはならない (MUST NOT)。

Startup recovery は optional housekeeping ではなく、システム再起動後に `Pending` / `NeedsRepair` / partial commit を可観測で監査可能な状態へ収束させる必須防衛線である。RFC 準拠実装は、通常運用再開前に non-committed dual-store operations を走査し、repair の要否を判定しなければならない (MUST)。

推奨 recovery sequence は次のとおりである。

1. `ConsistencyState = Pending` または commit phase 未完了の operation を列挙する。
2. SQLite-side commit intent、対象 pair / object lineage、LadybugDB-side materialization の有無を監査可能に再確認する。
3. 再適用可能な場合は idempotent retry を行う。
4. 再適用不能または監査不能な場合は `NeedsRepair` に遷移させる。
5. 修復不能、出所不明、lineage 不完全、または repeated retry で収束しない場合は `Quarantined` に遷移させる。

LadybugDB-side repair / retry path は idempotent に設計されるべきである (SHOULD)。反復 recovery attempt は canonical state を重複 materialize したり、silent divergence を生んだりしてはならない (MUST NOT)。

Any operation that cannot be repaired into a committed, auditable state MUST transition to `NeedsRepair` or `Quarantined` rather than being treated as committed.

### 41A.2 Ranking stability and replay discipline

GED / approximation boundary 付近の挙動は retrieval architecture 自体の変更理由ではなく、calibration / testing / replay discipline により監視・改善されるべき境界挙動である。v2.3 は graph embedding や新しい最適化器を導入しない。

RFC 準拠実装は、少なくとも pre-production calibration において、small structural perturbation、rename-only patch、edge-local modification、GED size threshold 近傍の candidate set に対する ranking drift を replayable trace と property-based test で観測できるようにすることが望ましい (SHOULD)。

Small structural perturbations SHOULD NOT cause unbounded ranking oscillation without being surfaced by calibration metrics, replay traces, or tests. ただし、この要求は retrieval architecture の normative shape を v1.5–v2.2 から変更するものではなく、あくまで calibration discipline を補強するものである。

### 41A.3 Training review load optional policy

Human-in-the-loop は v1.9 以降の中心規範であり、v2.3 においても変わらない。したがって、human review queue の負荷軽減は human review の否定ではなく、安全に限定された運用補助としてのみ許容される。

実装は、明示的に定義された safe sandbox scope に限り、Auto-Approval Exception Policy を optional policy として導入してもよい (MAY)。この例外 policy は、少なくとも namespace、artifact kind、side-effect envelope、resource budget、external write 禁止、production promotion 不可の条件で bounded に定義されなければならない。

Auto-approved training artifact は、auto-approved である事実、適用された policy ID、理由、scope boundary、実行 trace を audit log に残さなければならない (MUST)。この optional policy は training trust / production trust separation、promotion gate、human override 権限を弱めてはならない (MUST NOT)。

### 41A.4 Measurement guidance

v2.3 は、latency / token / trust だけでなく、再利用品質・修復頻度・レビュー負荷・探索安定性も implementation quality を測る補助指標として前景化する。これらは現時点では calibration candidate または operational metric であり、固定閾値を一律に要求するものではない。

推奨される観測対象は少なくとも次を含む。

- reuse quality / successful reuse rate
- false-new rate
- compose fallback frequency / new fallback frequency
- repair rate / quarantine rate / rollback rate
- human review queue depth / review latency / auto-approval fraction within safe scope
- ranking stability under small patch and boundary cases

## 41B. v2.3 Milestone and Calibration Addenda

### 41B.1 Milestone addendum

M-1, M0, M1 の testing plan は、可能であれば次を含むよう補強されるべきである。

- `GED_GRAPH_SIZE_LIMIT` 前後の candidate に対する replayable ranking drift test
- rename-only patch、single-edge patch、small compose perturbation に対する property-based ranking stability test
- startup repair scan の deterministic recovery test (`Pending -> retry -> Committed`, `Pending -> NeedsRepair`, `NeedsRepair -> Quarantined`)
- safe sandbox scope policy の audit completeness test

### 41B.2 Calibration addendum

付録 E の calibration candidate には、必要に応じて次を追加してよい。

- ranking stability score near GED boundary
- oscillation sensitivity under repeated refine / requery loops
- false-new rate versus successful compose / reuse recovery
- repair convergence time and quarantine escalation rate
- review-load indicators and safe-scope auto-approval utilization


## 42. 参照文献

- 既存 v1.9 の参照文献をそのまま継承する。
- 追加で、repository transformation / provenance-preserving merge / lineage-preserving knowledge integration に関する文献を将来補充してよい。
