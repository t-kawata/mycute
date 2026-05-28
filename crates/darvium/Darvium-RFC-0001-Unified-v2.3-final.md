# Darvium RFC-0001 — Unified Edition v2.3-j
## Darvium Workflow IR・GMR Retrieval Core・SearchWorkflow・グラフパッチ生成・Lifecycle / GC・Knowledge Ecosystem・Training Plane 統合仕様

**Darvium: Crystallized Ecosystems of Knowledge and Capability（知識と実務能力の結晶化された生態系）**

```
RFC番号  : Darvium-RFC-0001 (統合版)
旧番号   : RFC-0001 Rev.4 + RFC-0002 Rev.3 (統合)
ステータス: PROPOSED STANDARD — Finalizing Revision (v2.3-j)
著者     : Darvium Design Working Group
作成日   : 2026-05-19
改訂日   : 2026-05-25 (v2.3-j)
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
| **v1.8-final** | v1.8 の規範を一切毀損せず、(1) QueryDesignText の knowledge-aware schema を正式 canonical schema として固定、(2) Knowledge Applicability の式と Annex 解釈優先順位を明文化、(3) SQLite / LadybugDB (Repository Pair) の source-of-truth 境界を明確化、(4) three-plane architecture と既存 Layer の責務境界を説明補強し、自己完結性と非曖昧性を高めた完成版 |
| **v1.9** | v1.8-final の全文・規範・責務境界・数式・型定義・付録を一切毀損せず保持したまま、Human-in-the-loop を中核に据えた Training Plane を strictly additive に統合。TrainingMission / TrainingRunLog / TrainingFeedback / PromotionCandidate / TrainingTrustProfile / CandidateKnowledgeDocument / CurriculumPolicy / TrainingAuditLog を追加し、AI発・人間発・失敗再訓練を含む自主トレーニング、human review queue、sandbox execution policy、training/prod trust 分離、段階的 promotion、training-specific lifecycle / GC、knowledge under training、migration と監査要件を規範化 |
| **v2.0** | v1.9 の全文・規範・責務境界・数式・型定義・付録を一切毀損せず保持したまま、Repository Pair / Expert Namespace / Fusion Plan / Extraction Plan / Identity Remap Table / Fusion Audit Record / Pair Birth Lifecycle を strictly additive に統合。SQLite + LadybugDB を一体として扱う synthesis fusion を first-class operation として定義し、expert selective extraction・multi-pair fusion・split / recompose・完全トレーサビリティ・actor identity extension・training / production separation・dual-store birth commit・quarantine / repair discipline を規範化 |
| **v2.0-final** | v2.0 の規範を保持したまま、fusion semantics の曖昧性を除去。knowledge object の自動 semantic merge / truth arbitration を v2.0 スコープ外として明示し、conflict は coexistence + lineage relation で扱う方針を固定。単一プロセス / 単一ノード前提を設計上の制約として再明記し、形式保証・脅威モデル・分散化・探索最適化を Annex / RFC-0003 系へ外出しする責務境界を補強して、完成版としての自己完結性を高めた |
| **v2.1** | v2.0-final の全文・規範・責務境界を毀損せず保持したまま、SearchWorkflow を mission-completion-oriented orchestration として再明確化し、単一候補失敗が即時 mission failure を意味しないこと、候補フォールバック・requery・compose・new・human review が bounded orchestration の一部であることを明文化した |
| **v2.2** | v2.1 の規範を保持したまま、WorkflowGraph / SearchWorkflowGraph の DAG 検証を作成時・登録時・更新時と、使用時・コンパイル時・実行前の双方で MUST として明文化し、さらに多層 DAG における ready frontier / concurrency-admissible set / frontier-based parallel execution obligation を追加して、toposort や compile_to_steps の線形化を逐次実行の根拠にできないことを規範化した |
| **v2.3** | v2.2 の全文・規範・責務境界・mission-completion semantics・二段 DAG 検証・多層 DAG 並列実行義務を一切毀損せず保持したまま、(1) dual-store consistency の startup repair scan と recovery invariant、LadybugDB 再試行の idempotent expectation、silent divergence の禁止を明文化し、(2) GED 境界付近の ranking stability / oscillation risk に対する replay / property-based test / calibration discipline を補強し、(3) Training Plane に safe sandbox scope 限定の optional auto-approval exception policy を補足し、(4) reuse quality・false-new rate・repair rate・review-load indicators などの補助メトリクスを前景化した strictly additive revision |
| **v2.3-c** | v2.3 の全文・規範・責務境界・mission-completion semantics・二段 DAG 検証・多層 DAG 並列実行義務・dual-store repair semantics・ranking stability discipline・safe sandbox scope auto-approval を一切毀損せず保持したまま、Conversational Knowledge Path を strictly additive に統合。ConversationalEvent / ConversationalIngestionPolicy / ConversationalClassificationProposal / ConversationalGateDecision / ConversationalMissionPayload / ConversationalFragmentMeta / ConsolidationCandidateSet / ConsolidationPolicy / ConversationalPromotionGate の型定義群、LLM proposal → deterministic gate 分離原則、multi-turn / multi-day consolidation policy と数値閾値、personalization namespace convention、privacy / retention / tombstone / repair 規約を追加し、会話入力を起点とする知識成長経路（ConversationalEvent → Fragment → CandidateKnowledgeDocument → CanonicalDocument）の全段階を数値閾値・型定義・擬似コード付きで規範化した strictly additive revision |
| **v2.3-d** | v2.3-c の全文・規範・責務境界・Conversational Knowledge Path を一切毀損せず保持したまま、§12B HumanChannel Communication Abstraction を strictly additive に統合。HumanChannel トレイト (notify/communicate/reconnect)、InteractionHandle ブロッキング待機機構、FakeHumanChannel (テスト用ダブル)、StdinoutChannel (参照実装)、MetadataStore HITL 永続化 4 メソッド + DDL定義、クラッシュリカバリプロトコル、状態機械の形式的定義、較正パラメータ 3 項目、観測計画 6 指標を追加。SideEffectSet.sends_notification コメント補足、DeterminismScore に HITL Communicate コスト係数 ×3.0 新設、TrustAuditEvent に HITL outcome variant 拡充、§13A Training Orchestrator と §13B Human Communication Patterns に HumanChannel 層構造補足とデータ型マッピング、§16A.1 HumanReviewQueuePolicy に HumanRequest.timeout 接続、マイルストーン表に M-0.5-4 追記。HITL を「命」として位置づけるための通信基盤を完備し、M1 Human-in-the-loop review の前提条件を整備した strictly additive revision |
| **v2.3-e** | v2.3-d の全文・規範・責務境界・HumanChannel 基盤・Conversational Knowledge Path・Operational Clarifications を一切毀損せず保持したまま、§41B Child Support Villages and HELP Consensus Extension を strictly additive に統合。spacepositionembedding に基づく動的 locality、Child / Adult / Local Village の定義、HelpProposal / HelpOffer / HelpDecision / HelpExecution / HelpSuccess の 5 段階 HELP 合意プロトコル、child-targeted TrainingMission 拡張、helper weighting と bounded remote exploration、child growth と reciprocity / reputation 連携、stability / dynamicity の二軸評価、village 向け calibration candidate・operational metrics・replay / perturbation / property-based test 規律を追加し、training-production separation・ApplicabilityScore・legal SearchState transitions・dual-store consistency・promotion / repair invariants を一切変更しない strictly additive revision |
| **v2.3-f** | v2.3-e の全文・規範・責務境界・Child Support Villages and HELP Extension を一切毀損せず保持したまま、**直接互恵性 (Direct Reciprocity)** と **間接互恵性 (Indirect Reciprocity)** がワークフローの生存確率、支援優先度、成熟促進、淘汰抑制に系統的に影響する数理モデルを strictly additive に統合。Reciprocity contribution の分解 (F-1〜F-3)、ReputationProfile の再定義 (F-4〜F-5)、benevolence-aware GC hazard / survival probability (F-7〜F-9)、child protection との接続 (F-10)、HELP helper weighting への benevolence 項 (F-11〜F-13)、child growth / maturation の数式化 (F-14〜F-15)、multi-objective calibration objective (F-16)、ReciprocityEvent / ReciprocityLifecyclePolicy データ型、pure function validation / deterministic replay / perturbation / synthetic ecosystem simulation / human-reviewed calibration の 5 段階較正ループ、regression guard metrics、単調性テスト・replay test・perturbation test・property-based test のテスト規律を追加し、v2.3-f 用 Calibration Candidates とマイルストーンを拡充 |
| **v2.3-g** | v2.3-f の全文・規範・責務境界・Conversational Knowledge Path・HumanChannel 基盤・Child Support Villages and HELP Extension・Direct/Indirect Reciprocity 数理モデルを一切毀損せず保持したまま、Darvium Event Architecture を strictly additive に統合。VirtualClock を「commit 済み DarviumEvent 列の順序番号」として再定義し、DarviumEvent canonical envelope・DarviumEventKind extensible taxonomy・InteractionMode {OneWay, TwoWay}・DarviumEventBus trait・InteractionStore 汎用 API を新設。HumanChannel を DarviumEventBus/InteractionStore 上の HITL-specific adapter へ再構成し、StoredInteraction を InteractionRecord&lt;HitlPayload&gt; へ一般化。StdinoutChannel を StdinoutEventChannel へ拡張し、canonical JSON Lines プロトコルを定義。全 log 型を DarviumEvent projection として位置づけ直し、外部 subscribe 経路 (stdin/stdout + WebSocket) を規範化。既存の HITL 実行意味論・InteractionHandle.wait()・MetadataStore crash recovery は後方互換を完全保持する strictly additive revision |
| **v2.3-h** | v2.3-g の全文・規範・責務境界・Event Architecture を一切毀損せず保持したまま、GMR Retrieval Core を最上階 WorkflowGraph に対する 4 層検索方式へ改訂。WorkflowDesignEmbedding / QueryDesignEmbedding を optional compatibility field へ格下げし、構造類似検索の主手段を top-level WorkflowGraph に対する GED 系検索へ移行。4 層 retrieval（Semantic → Metadata → Cheap GED → Full GED）を normative 化し、cheap GED と full GED の責務分離を明文化。ApplicabilityScore の構造成分を design embedding cosine から GED 正規化類似度へ一本化。SQLite metadata layer を first-class layer として formalize し、TopLevelGraphMetadata / CheapGedSignature データ型を新設。実験計画 Annex を追加し、較正候補を GED 関連パラメータで拡充。旧 structural proxy retrieval パラメータを deprecated に移行。 |
| **v2.3-i** | v2.3-h の全文・規範・責務境界・Event Architecture・4 層検索方式を一切毀損せず保持したまま、StructMem / Corpus2Skill を概念参照から実装対象の知識基盤機構へ昇格させ、二重 Preset Registry アーキテクチャ (BakedPresetRegistry / MutablePresetRegistry)、12 段階起動時検証手順、名前空間予約・依存方向制約、ResolvedWorkflowRegistry、Root Preset 保護を strictly additive に統合。StructMem (MemoryEvent → Fragment → MemoryConcept → CanonicalDocument) および Corpus2Skill (Chunk → Entity → SkillNode) の形成理論を実装規定として追加し、起動時検証・GC 保護・Event Architecture 拡張・Knowledge Primitive 分類・データモデル拡充を併せて統合した strictly additive revision。 |
| **v2.3-j** | v2.3-i の全文・規範・責務境界・StructMem/Corpus2Skill 実装規定・Preset Registry アーキテクチャを一切毀損せず保持したまま、WorkflowRepository の責務と名称を是正する用語改訂。§2 用語集に WorkflowCache および Repository Pair の定義を追加し、旧 WorkflowRepository を runtime cache / in-memory index として再定義。SQLite + LadybugDB から成る Repository Pair を MemoizedGraph 群の canonical persistence の正本とし、WorkflowCache はその runtime cache として位置づける。§8 の構造体定義・擬似コード・説明文を WorkflowCache / RepositoryPair 区分に更新し、RepositoryError を PersistenceError + CacheError へ再編。SearchWorkflow / GMR の検索フローを「DB 主導 + cache 加速」として明文化。全節の source-of-truth 誤記を補正した用語是正改訂。 |

---

## 目次

1. [概要と目的](#1-概要と目的)
2. [用語集](#2-用語集)
3. [スコープ](#3-スコープ)
4. [設計上の前提と制約](#4-設計上の前提と制約)
5. [4 層アーキテクチャ概観](#5-4-層アーキテクチャ概観)
6. [Layer 2 — Workflow IR (WorkflowGraph)](#6-layer-2--workflow-ir-workflowgraph)
7. [Layer 2 → Layer 1 コンパイル](#7-layer-2--layer-1-コンパイル)
8. [WorkflowCache と MemoizedGraph](#8-workflowcache-と-memoizedgraph)
9. [WorkflowDesignText / QueryDesignText](#9-workflowdesigntext--querydesigntext)
10. [TrustProfile — 4 軸信頼モデル](#10-trustprofile--4-軸信頼モデル)
11. [Applicability Check](#11-applicability-check)
12. [Layer 3a — GMR Retrieval Core](#12-layer-3a--gmr-retrieval-core)
12A. [Knowledge Primitive Registry (v1.8)](#12a-knowledge-primitive-registry-v18)
12B. [HumanChannel Communication Abstraction (v2.3-d)](#12b-humanchannel-communication-abstraction-v23-d)
12C. [Darvium Event Architecture (v2.3-g)](#12c-darvium-event-architecture-v23-g)
12D. [External Event Subscription (v2.3-g)](#12d-external-event-subscription-v23-g)
12E. [Event Projection Framework (v2.3-g)](#12e-event-projection-framework-v23-g)
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
27A. [付録 G — v2.3-h 4 層検索実験計画](#27a-付録-g--v23-h-4-層検索実験計画)
28. [リポジトリペア / エキスパートフュージョン統合仕様 (v2.0-final)](#28-リポジトリペア--エキスパートフュージョン統合仕様-v20-final)
29. [フュージョンコア用語集 (v2.0)](#29-フュージョンコア用語集-v20)
30. [リポジトリペアモデル](#30-リポジトリペアモデル)
31. [エキスパート境界モデル](#31-エキスパート境界モデル)
32. [フュージョン / 抽出操作](#32-フュージョン--抽出操作)
33. [許容性と安全ゲート](#33-許容性と安全ゲート)
34. [ID 再写像](#34-id-再写像)
35. [系統とトレーサビリティ要件](#35-系統とトレーサビリティ要件)
36. [フュージョンにおけるトレーニング/プロダクション分離](#36-フュージョンにおけるトレーニングプロダクション分離)
37. [フュージョンオーケストレーターと誕生コミット](#37-フュージョンオーケストレーターと誕生コミット)
38. [フュージョンの障害処理、隔離、修復](#38-フュージョンの障害処理隔離修復)
39. [v2.0 の移行と後方互換性](#39-v20-の移行と後方互換性)
40. [付録 F — v2.0 追加データモデル](#40-付録-f--v20-追加データモデル)
41. [付録 G — フュージョン不変条件と未解決事項](#41-付録-g--フュージョン不変条件と未解決事項)
42. [参照文献](#42-参照文献)

---

## 1. 概要と目的

Darvium は OpenFang を Layer 1 実行エンジンとして利用し、WorkflowGraph を正本とする Application Workflow 層、その再利用検索を担う GMR Retrieval Core、ならびにそれらを探索・選択する SearchWorkflow Meta-Workflow 層、さらに長期運用下で資産の寿命・淘汰・評判・継承を制御する Lifecycle / Natural Selection 層を統合した実行・探索基盤を提供する。

本 RFC で規定する主要保証は以下の 10 個である。

1. **構文的健全性** — WorkflowGraph / SearchWorkflowGraph は常に DAG であり、変数スコープと状態遷移制約が閉じている。
2. **実行的健全性** — Applicability Check と SearchGuard がエージェント互換性・副作用安全性・予算超過・再帰暴走を事前に抑止する。
3. **検索的健全性** — GMR Retrieval Core は semantic 類似 (`task_embedding`) と最上階 WorkflowGraph に対する GED 系構造類似（metadata filter + cheap GED + full GED）を統合し、候補不足・version 不整合・信頼不足を明示的に扱う。
4. **探索的健全性** — SearchWorkflow は REUSE / PATCH / COMPOSE / NEW / ABORT の outcome 空間を bounded search として探索し、SearchTrace により監査可能な決定履歴を残す。
5. **最適化的健全性** — 既存ワークフローの再利用・差分修正・構成的合成・新規生成を明確に分離し、期待値ベースで LLM 呼び出しコストと失敗率を削減する。
6. **生態系的健全性** — SubWorkflow を共有資産として登録し、Human Time と VirtualClock の二軸時間、経験値 grace period、互恵性ベース評判、自然淘汰としての GC、resource pressure 制御により、資産群の長期持続可能性を保つ。
7. **知識的健全性** — Knowledge Applicability、origin trace、evidence completeness、dual-store consistency により、知識アクセス・知識変異・知識昇格の安全性と説明可能性を確保する。
8. **訓練的健全性** — Training Plane は mission generation、human review、sandbox execution、feedback ingestion、promotion を first-class に扱い、本番実行系と責務・namespace・評価系を分離する。
9. **昇格的健全性** — training artifacts は sandbox only / candidate / approved / promoted / rolled back の段階を経なければ production artifacts に昇格してはならない。
10. **共同訓練健全性** — 人間は単なる例外処理の最終安全弁ではなく、訓練対象の選定、結果評価、重点領域の注入、昇格判断を行う共同訓練者として規範的に位置づけられる。
11. **Event Architecture 健全性** — 全 DarviumEvent は DarviumEventBus を通過しなければならない (MUST)。VirtualClock は commit 済み DarviumEvent 列の順序番号であり、いかなる domain subsystem も VirtualClock を直接更新してはならない (MUST NOT)。DarviumEvent は OneWay / TwoWay の interaction semantics を持つ。全ての TwoWay interaction は crash-safe persistent session recovery を実装しなければならない (MUST)。外部 observer は標準入出力または WebSocket により DarviumEvent を subscribe できなければならない (MUST)。

本 RFC は RFC-0001 Rev.4 を正史とし、RFC-0002 Rev.3 のグラフパッチ生成仕様を統合した v1.5 の完成度を保持しつつ、v1.6 では SearchWorkflow Meta-Workflow を追加して GMR を workflow discovery primitive として再編成した完成度を保持しつつ、v1.7 では Lifecycle / Natural Selection 層を追加して SubWorkflow 資産化、時間二軸、VirtualClock、経験値、互恵性評判、GC、継承、resource pressure、社会加速度を統合した単一規範文書である。

さらに v1.8 / v1.8-final では、LadybugDB / StructMem / Corpus2Skill を additive に統合し、Knowledge Ecosystem Integration、knowledge-aware QueryDesignText、Knowledge Applicability、Knowledge Primitive Registry、dual-store consistency、three-plane architecture の責務境界を完成形として固定した。v1.9 はこの完成形を前提に、その全文を保持したまま Human-in-the-loop を中核に据えた first-class training architecture を追加する strictly additive revision である。v2.0-final はその上に repository pair / expert fusion / quarantine discipline を重ね、v2.1 と v2.2 は SearchWorkflow の mission-completion semantics、creation-time / execution-time DAG validation、frontier-based parallel execution obligation を strictly additive に補強した。v2.3 はさらに、dual-store repair semantics、ranking stability / replay / property-based test discipline、training review load の安全な運用補助、補助評価指標の前景化を加えるが、既存の core invariant と責務境界を変更しない。v2.3-c はさらに Conversational Knowledge Path を strictly additive に追加するが、既存の core invariant と責務境界を変更しない。v2.3-g はさらに、これらすべての層に横断的な Darvium Event Architecture を追加し、VirtualClock の意味論を完了させる。

v2.3-i はさらに、StructMem / Corpus2Skill を v1.8 以来の概念参照から実装対象の知識基盤機構へ昇格させる。すなわち StructMem (MemoryEvent → Fragment → MemoryConcept → CanonicalDocument) および Corpus2Skill (Chunk → Entity → SkillNode) を、LadybugDB と SQLite 上に実体化される knowledge object 形成理論として full-spec 化する。併せて二重 Preset Registry アーキテクチャ (BakedPresetRegistry / MutablePresetRegistry / ResolvedWorkflowRegistry) を新設し、12 段階起動時検証手順・名前空間予約・依存方向制約・Root Preset 保護を規範化する。これらの追加は既存の core invariant と責務境界を変更しない。

本改訂でいう training とは、基盤モデル自体の parameter update ではなく、(a) ワークフロー空間の拡張、(b) ワークフロー品質の洗練、(c) 知識基盤の厚みの増大、(d) 人間の価値判断・重点領域の注入を、明示的な mission generation・mission review・sandbox execution・feedback ingestion・promotion discipline の下で制度化することを指す。

したがって v1.9 は、v1.8-final に内在していた探索・改良・レビュー・trust 更新・知識蓄積の諸機構を、Training Plane という論理平面に整理して formalize する改訂である。training primitive を一切用いない既存 v1.8 workflow の意味論、TrustProfile、SearchWorkflow、Lifecycle / GC、Knowledge Applicability、source-of-truth 境界、QueryDesignText canonical schema、GraphVersion CAS、dual-store consistency は v1.9 においても変更されてはならない (MUST NOT)。

**v1.9 確定方針**: 専用 `graph_embedding` は RFC-0001 の規範スコープから除外し、真の graph embedding・GNN encoder・その学習最適化は RFC-0003 以降へ委譲する。SearchWorkflow の COMPOSE / NEW / ABORT 分岐は bounded heuristic policy として扱い、責務・状態機械・予算・監査可能性のみを規範化する。加えて v1.7 では GC / 評判 / 社会加速度の閾値や重みは tuning 可能としつつ、時間軸分離、SubWorkflow 資産化、状態遷移、監査可能性、Soft/Hard/Tombstone の責務境界は規範として固定する。さらに v1.9 は、これらに Training Plane を strictly additive に重ねるのみとし、training artifact が promotion gate を通過するまで production selection path・production trust・canonical knowledge・Repository Pair の source-of-truth を汚染しないことを追加規範として固定する。さらに v2.3 は、dual-store recovery は application-level discipline であり XA / distributed 2PC を意味しないこと、ranking stability と review-load は calibration / operational measurement の対象であることを補足するが、single-process / single-node 前提や training / production separation を変更しない。v2.3-c は、会話入力から長期知識への成長経路（Conversational Knowledge Path）を追加規定するが、既存の core invariant と責務境界を変更しない。

---

## 2. 用語集

| 用語 | 定義 |
|------|------|
| **WorkflowGraph** | `StableGraph<WorkflowNode, EdgeMeta>` 型の有向非巡回グラフ (DAG) |
| **MemoizedGraph** | WorkflowGraph に埋め込みベクタ・TrustProfile・Provenance を付与した Repository Pair 上の永続化単位 |
| **WorkflowDesignText** | WorkflowGraph の構造・主要ノード列・依存関係・分岐・集約・I/O・副作用・決定論性特徴を canonical schema で記述した自然言語 / 半構造化テキスト (v1.5 新設) |
| **QueryDesignText** | mission から生成される検索用の粗いワークフロー設計記述。完全な WorkflowGraph ではない (v1.5 新設) |
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
| **GcState** | Protected / Active / SoftDeleted / HardDeleteCandidate / Tombstoned の資産寿命状態。Protected は root preset 等の GC 完全除外対象に割り当てられる (v1.7 新設、Protected は v2.3-i 追加) |
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
| **DarviumEvent** | Darvium 世界内で観測・記録される全出来事の canonical envelope。event_id / virtual_clock / created_at / causality / interaction_mode / kind / payload / transport_meta / visibility / retention / privacy をフィールドとして持つ (v2.3-g 新設) |
| **DarviumEventKind** | DarviumEvent の extensible subtype 分類。SearchEvent / WorkflowExecutionEvent / TrainingEvent / KnowledgeEvent / ConversationalEventEnvelope / LifecycleEvent / GcEvent / RepairEvent / ReciprocityEventEnvelope / FusionEvent / HitlEvent / PresetRegistryEvent / SystemEvent / ExtensionEvent を含む (v2.3-g 新設、PresetRegistryEvent は v2.3-i 追加) |
| **InteractionMode** | DarviumEvent の interaction semantics を表す直交軸。OneWay (publish-only, fire-and-forget) または TwoWay (interaction_id, 状態遷移, timeout, reconnect, pending recovery を持つ) (v2.3-g 新設) |
| **DarviumEventBus** | VirtualClock の唯一の authority。全 DarviumEvent の commit, persistence, fan-out, replay を提供する中心コンポーネント。いかなる domain も VirtualClock を直接更新してはならない (v2.3-g 新設) |
| **InteractionStore** | TwoWay interaction の永続化と復旧を提供する汎用ストア API。MetadataStore の HITL interaction メソッドを一般化したもの (v2.3-g 新設) |
| **EventProjection** | DarviumEvent Bus 上の event 列から materialize される domain-specific ビュー。SearchTrace / SearchRunLog / TrainingRunLog / TrustAuditLog / RepairLog / ReciprocityEvent 等は EventProjection の具体例 (v2.3-g 新設) |
| **EventChannel** | DarviumEvent の外部 subscribe / publish を提供する transport 抽象。StdinoutEventChannel および WebSocketEventChannel が標準実装 (v2.3-g 新設) |
| **TwoWayInteraction** | 一対の DarviumEvent (Request + Response) で構成される interaction。interaction_id / Pending→AwaitingExternal→Resolved 等の状態機械 / timeout / reconnect / pending session recovery を持つ (v2.3-g 新設) |
| **TopLevelGraphMetadata** | 最上階 WorkflowGraph から抽出される軽量メタデータ集合。node/edge/source/sink/longest_path/label_histogram/determinism/side_effect/agent 要約を含む。SQLite metadata filter (Stage 2) の入力として使用される (v2.3-h 新設) |
| **CheapGedSignature** | cheap GED (Stage 3) の replayable deterministic 入力を構成する graph signature。topological label ordering, degree histogram, reachability sketch, path hash multiset を含む (v2.3-h 新設) |
| **TopLevelQueryMetadata** | QueryDesignText から deterministic formatter で導出される query 側の top-level graph sketch metadata。metadata filter (Stage 2) の query 側入力 (v2.3-h 新設) |
| **4-Layer Retrieval (v2.3-h)** | v2.3-h で normative 化された検索パイプライン。Layer S (semantic mission retrieval) → Layer M (SQLite metadata filter) → Layer G1 (cheap GED filter) → Layer G2 (full GED rerank) の 4 層で構成される。旧 Dual Retrieval を置換 (v2.3-h 新設) |
| **PresetWorkflow** | ビルド時にバイナリに baked されるか、起動時に外部ソースからロードされる事前定義ワークフロー。BakedPresetRegistry または MutablePresetRegistry に属する (v2.3-i 新設) |
| **BakedPresetRegistry** | バイナリにコンパイル時に埋め込まれる immutable な PresetWorkflow 群。起動時に展開・検証され、発見不可・検証失敗・内容破損は boot-fatal である。StructMem / Corpus2Skill root preset はこの registry に属する (v2.3-i 新設) |
| **MutablePresetRegistry** | 起動時にファイルシステムからロードされるユーザー拡張可能な PresetWorkflow 群。検証失敗エントリは隔離 (quarantine) されるが、registry 全体の起動を阻止しない (v2.3-i 新設) |
| **ResolvedWorkflowRegistry** | BakedPresetRegistry と MutablePresetRegistry の runtime 統合。名前空間衝突解決・source provenance 追跡・依存方向検証を提供する (v2.3-i 新設) |
| **SystemPresetRoot** | StructMem / Corpus2Skill 等の system-critical な PresetWorkflow を指すルート。GcState::Protected により GC から常時保護される (v2.3-i 新設) |
| **ImmutableRoot** | 起動後に一切変更できない PresetWorkflow のルート。BakedPresetRegistry に属する全 workflow は ImmutableRoot 下にある。名前空間 `platform.*` / `builtin.*` / `system.*` が予約される (v2.3-i 新設) |
| **RootPinned** | PresetWorkflow が解決時に自身のルートとして認識するルートノードの識別子。GC 保護対象を決定する (v2.3-i 新設) |
| **CapabilityFamily** | capability の機能的分類を表す enum。`StructMem` / `Corpus2Skill` / `Search` / `Training` / `General` の 5 値を取る (v2.3-i 新設) |
| **RegistrySource** | PresetWorkflow が読み込まれた registry ソースを表す enum。`BakedPlatform` (platform-provided baked) / `MutableUser` (user-provided mutable) / `MutableWorkspace` (workspace-level mutable) (v2.3-i 新設) |
| **ArtifactOriginKind** | MemoizedGraph の出自種別を表す enum。`PresetSystem` / `PresetUser` / `SearchGenerated` / `TrainingDerived` / `FusionDerived` / `Conversational` / `Manual` の 7 値を取る (v2.3-i 新設) |
| **PresetRootPolicy** | PresetWorkflow のルート保護ポリシー。`RootPinned` (GC から常時保護) / `RootUnpinned` (通常の GC 対象) / `RootAncestorPinned` (先祖が pinned の場合に保護) (v2.3-i 新設) |
| **PresetValidationReason** | PresetWorkflow 検証失败の理由を表す 12 種の列挙子。`InvalidPresetSchema` / `DuplicateWorkflowId` / `ReservedNamespaceViolation` / `WorkflowNotFound` / `CrossRegistryDependencyViolation` / `CircularReference` / `InvalidInputMapping` / `OutputBindingMismatch` / `BootCriticalPresetMissing` / `BootCriticalPresetInvalid` / `MutableOverrideForbidden` / `PresetPolicyViolation` (v2.3-i 新設) |
| **PresetValidationFailure** | PresetWorkflow の検証失敗を表す型。`workflowid` / `source` (RegistrySource) / `source_path` / `reasons` (Vec&lt;PresetValidationReason&gt;) / `detected_at` を含む (v2.3-i 新設) |
| **WorkflowCache** | Repository Pair 上に永続化された MemoizedGraph 群の runtime cache / in-memory index。source-of-truth ではなく、検索高速化・局所再利用・compile-time / retrieval-time 参照のための in-memory working set を提供する。MemoizedGraph の canonical persistence, consistency, repair, quarantine, availability は Repository Pair により担保される (v2.3-j 新設) |
| **Repository Pair** | SQLite と LadybugDB により MemoizedGraph・WorkflowGraph・lineage・trust・consistency state を保持する永続化ペア。SQLite は trust / lifecycle / lineage / audit の正本、LadybugDB は graph / embedding / ANN index / knowledge object の正本として役割分離する。WorkflowCache はこの Repository Pair 上のデータの runtime cache として動作する。v2.0 の Fusion 操作における可搬個体としての Repository Pair 概念と同一であり、§28-§39 の Fusion 仕様における Repository Pair モデルと整合する (v2.3-j 新設) |
| **Cache Residency** | WorkflowCache に保持されている状態。永続化状態ではなく runtime residency を指す (v2.3-k 新設) |
| **Cache Eviction** | WorkflowCache から MemoizedGraph を in-memory で除去する操作。Repository Pair 上の canonical persistence を削除する意味を持たない (v2.3-k 新設) |
| **Cache TTL Policy** | Provenance.last_used_at と last_virtual_seen に基づいて eviction 候補化するポリシー (v2.3-k 新設) |
| **Pinned Cache Entry** | GcState::Protected または preset root policy により eviction 禁止となる cache entry (v2.3-k 新設) |
| **Cache Pressure State** | ResourcePressure と EnvironmentPolicy.pressure_mode から導出される cache-side eviction aggressiveness 状態 (v2.3-k 新設) |

**補注 — WorkflowRegistry 系用語との区別:** `ResolvedWorkflowRegistry` は BakedPresetRegistry + MutablePresetRegistry の runtime 統合であり、compiler の `registry.get(workflowid)` が参照する PresetWorkflow 用 registry である。`WorkflowCache` は Repository Pair 上の全 MemoizedGraph (ユーザー生成・検索生成・training 由来等) の runtime cache であり、PresetWorkflow と動的生成ワークフローの両方の in-memory 高速参照を提供する。両者は異なる概念であり、名前空間も責務も分離される。

## 3. スコープ

### 3.1 In-Scope

- WorkflowGraph の型定義・バリデーション規則
- Layer 2 → Layer 1 コンパイル (`compile_to_steps`)
- WorkflowCache・MemoizedGraph の構造と cold-start 初期化
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
- SubWorkflow 資産化と共有 Repository Pair 登録規則
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
- **WorkflowCache eviction policy, residency control, TTL, periodic cleanup, and preset-safe retention rules (v2.3-k 追加)**
- **Event-driven cache invalidation / eviction on GcState transitions and repository repair state changes (v2.3-k 追加)**

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
- **OS-level memory reclamation strategy itself, allocator tuning, and kernel-specific page cache behavior; this RFC only specifies application-level WorkflowCache eviction semantics (v2.3-k 追加)**

---

## 4. 設計上の前提と制約

| ID | 内容 | 影響範囲 |
|----|------|---------|
| P-01 | WorkflowGraph は DAG でなければならない。`petgraph::algo::toposort` が `Err(Cycle)` を返す場合は即時拒否 | Layer 2 |
| P-02 | OpenFang REST API は OpenFang v0.4.9 以降の仕様に依存 | Layer 1 |
| P-03 | AgentStep の idempotency は Layer 2 では保証しない。SideEffect フィールドで明示 | Layer 2 |
| P-04 | WorkflowCache は `tokio::sync::RwLock` で保護された並行アクセスを前提とする | Layer 3 |
| P-09 | MemoizedGraph への更新は `GraphVersion` による楽観的並行性制御 (CAS) を使用すること (MUST)。期待バージョンと不一致の場合は `CacheError::CasConflict` を返すこと (§8.3 参照) | Layer 3 / 2.5 |
| P-05 | 埋め込みモデルのバージョンは `Provenance.source_version` に記録し、異なるバージョン間の類似度比較は AG-05 で排除する | Layer 3 |
| P-06 | `StableGraph` を使用すること (MUST)。DiGraph はノード削除時に NodeIndex が無効化されるため使用禁止 | Layer 2 / 2.5 |
| P-07 | 新規 MemoizedGraph は cold-start trust で初期化すること (§8 参照)。Trust が 0.0 のグラフを Repository Pair に登録してはならない (MUST NOT) | Layer 3 |
| P-08 | `apply_patch` は atomic に実行すること。途中失敗時はグラフを元の状態に戻さなければならない (MUST) | Layer 2.5 |
| P-10 | training artifacts は production artifacts と source-of-truth を共有してよいが、namespace・review state・promotion state・policy binding を分離しなければならない (MUST) | Training Plane |
| P-11 | AI-generated TrainingMission は原則として human review を経ずに sandbox 実行してはならない (MUST NOT) | Training Plane |
| P-12 | training で得られた workflow / subworkflow / knowledge / query pattern を production Gold として即時採用してはならない (MUST NOT) | Training Plane / Promotion |
| P-13 | training trust と production trust は別チャネルで保持しなければならない (MUST) | Trust |
| P-14 | knowledge mutation を伴う training run は sandbox namespace に限定しなければならない | Knowledge / Training |
| P-15 | v2.0 の Repository Pair / Fusion semantics は単一プロセス・単一ノード前提で規範化される。分散 consensus / replication / partition handling は本 RFC では扱わず、将来 Annex / 別 RFC に委譲する | Fusion / Repository |
| P-16 | fusion における knowledge object の semantic deduplication・truth arbitration・自動優劣判定は本 RFC スコープ外とし、v2.0-final では coexistence + lineage relation (`CONSOLIDATES` / `SUPERSEDES` 等) により扱うこと | Fusion / Knowledge |
| P-17 | WorkflowCache は source-of-truth ではなく揮発 cache であり、WorkflowCache からの eviction は Repository Pair 上の canonical persistence を変更してはならない (MUST NOT) (§8.0 参照) | Layer 3a — WorkflowCache |
| P-18 | GcState::Protected の MemoizedGraph、および ArtifactOriginKind::PresetSystem または PresetRootPolicy::RootPinned | RootAncestorPinned に該当する preset-derived graph は WorkflowCache eviction 対象にしてはならない (MUST NOT) | Layer 3a — WorkflowCache |
| P-19 | GcState::Tombstoned の graph は WorkflowCache に残存してはならない (MUST NOT) | Layer 3a — WorkflowCache / Layer 3c — GC |
| P-20 | 実装は、WorkflowCache に対して periodic eviction もしくは capacity-bound eviction の少なくとも一方を実装しなければならない (MUST) | Layer 3a — WorkflowCache |
| P-21 | ConsistencyState != Committed の graph は normal retrieval hot set から除外しなければならず、eviction 候補選定においては保守的に扱わなければならない (MUST) | Layer 3a — GMR Retrieval |

---

## 4A. Darvium シミュレーション主要メカニズム — 10 セクション 56 機構

**このセクションの目的**: 社会加速度（Kind World）シミュレーションを完全に成立させるために必要なメカニズムを過不足なくリストする。各機構は RFC 該当セクションおよび数式への参照を持つ。本リストはシミュレーションに直接関与する機構のみで構成され、インフラ機構（ANN 検索パイプライン、Training Plane、イベントバス、二重ストア、健全性不変条件、学習ループ、層間連携等）は除外する。

**選別基準**: 「シミュレーションの 6 フェーズ（個人生成 → 位置・村更新 → HELP 相互支援 → 互恵性・生存 → 能力拡散 → J_kw 測定）のいずれかで直接参照・計算される機構」のみを含める。これらは Kind World 較正ループ（M1.76-KW4）において値が変化し、J_kw に影響を与える。

---

### 4A.0 J_kw 較正パラメーター完全カタログ（Calibration Parameter Catalog）

**このセクションの目的**: J_kw_social = s_growth × s_density × s_topology × s_search × s_fairness × s_speed の数値に影響を与える全ての定数を網羅的にカタログ化する。本カタログは `src/constants.rs` の定数定義と `src/simulation.rs` のフェーズ実装の二重トレースから生成された。定数は「探索済み（Searched）」「未探索（Unsearched）」「ハードコード（Hardcoded）」の 3 種に分類される。

凡例:
- **(S)** = 探索済み — MagnificentSevenParams に含まれ Bayesian Pareto 最適化の探索パラメーター
- **(U)** = 未探索 — `constants.rs` に定数として定義されているが、現在の最適化で探索されていない
- **(H)** = ハードコード — フェーズ関数内に数値リテラルとして直書きされ、定数化すらされていない

影響因子列は該当定数が主にどの因子（s_growth, s_density, s_topology, s_search, s_fairness, s_speed）を通じて J_kw に影響するかを示す。

---
#### 4A.0.1 GC Hazard 系（F-7・F-8・F-9・F-10）— 9 定数

GC hazard λ_i^GC = softplus(λ_0 - γ_lifecycle·L(G)_i - γ_benevolence·B_i - γ_child_protect·C_i^protect)

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 1 | `GC_HAZARD_LAMBDA_0` | 1.0 | **(S)** | s_growth | `to_sim_config()` でオーバーライド; `compute_gc_hazard()` |
| 2 | `GC_HAZARD_GAMMA_BENEVOLENCE` | 0.10 | **(S)** | s_growth | `to_sim_config()` でオーバーライド; `compute_gc_hazard()` |
| 3 | `GC_HAZARD_GAMMA_LIFECYCLE` | 1.0 | **(U)** | s_growth | `Default` 値のみ使用; `compute_gc_hazard()` |
| 4 | `GC_HAZARD_GAMMA_CHILD_PROTECT` | 8.0 | **(U)** | s_growth | `Default` 値のみ使用; `compute_gc_hazard()` |
| 5 | `CHILD_PROTECT_ETA1` | 0.50 | **(U)** | s_growth | `compute_child_protection()` の η₁ |
| 6 | `CHILD_PROTECT_ETA2` | 0.30 | **(U)** | s_growth | `compute_child_protection()` の η₂ |
| 7 | `CHILD_PROTECT_ETA3` | 0.20 | **(U)** | s_growth | `compute_child_protection()` の η₃ |
| 8 | `MIN_SURVIVAL_EXPERIENCE` | 5 | **(U)** | s_growth | Grace Period: experience < 5 の child は GC 対象外 |
| 9 | `LIFECYCLE_WEIGHT_BENEVOLENCE` | 0.15 | **(U)** | s_growth | lifecycle 計算における慈悲スコア重み |

---
#### 4A.0.2 Lifecycle Score 系（§4A.7・§4A.9）— 7 定数

LifecycleScore L(G)_i = (freshness × success × trust × usage × reputation)^(1/5)

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 10 | `HUMAN_FRESHNESS_HALFLIFE_MS` | 86,400,000 | **(U)** | s_growth | `compute_blended_freshness()` F_H 指数減衰 |
| 11 | `VIRTUAL_FRESHNESS_HALFLIFE` | 100.0 | **(U)** | s_growth | `compute_blended_freshness()` F_V 指数減衰 |
| 12 | `EXPERIENCE_NORMALIZATION_SCALE` | 10.0 | **(U)** | s_growth | `compute_experience_normalization()` スケール |
| 13 | `EXPERIENCE_NORMALIZATION_OFFSET` | 1.0 | **(U)** | s_growth | `compute_experience_normalization()` オフセット（FIX-B3 追加） |
| 14 | `PHASE4_FRESHNESS_HUMAN_WEIGHT` | **0.5** | **(U)** | s_growth | `phase4_gc_survival()` の人時重み |
| 15 | `PHASE4_LIFECYCLE_SUCCESS_STUB` | **0.5** | **(U)** | s_growth | `phase4_gc_survival()` の成功成分（P6 未完成スタブ） |
| 16 | `compute_mean_freshness human_weight` | **0.0** | **(H)** | s_growth | kind_world.rs:2195 — メトリクス計算パスの人時重み。phase4（#14）とは異なり pure virtual freshness |

---
#### 4A.0.3 信頼・評判系（F-1・F-2・F-3・F-4・F-5・F-6）— 22 定数

直接互恵性 R_i^dir = σ(Σ(α_h·H + α_hs·HS - α_r·RJ - α_d·DM)·decay(Δt, ρ_dir))  (F-1)
間接互恵性 R_i^ind = σ(β₁·C + β₂·A + β₃·U + β₄·Q - β₅·B)  (F-2)
慈悲総和 B_i = w_dir·R_dir + w_ind·R_ind + w_rep·Rep  (F-3)

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 17 | `REPUTATION_THETA_DIR` | 0.35 | **(S)** | s_fairness | `to_sim_config()` でオーバーライド; F-4 直接互恵性重み |
| 18 | `REPUTATION_THETA_IND` | 0.35 | **(S)** | s_fairness | `to_sim_config()` でオーバーライド; F-4 間接互恵性重み |
| 19 | `REPUTATION_THETA_EXP` | 0.20 | **(U)** | s_fairness | `Default` 値のみ使用; F-4 経験値重み |
| 20 | `REPUTATION_THETA_INHERIT` | 0.10 | **(U)** | s_fairness | `Default` 値のみ使用; F-4 継承重み |
| 21 | `REPUTATION_KAPPA_E` | 0.01 | **(U)** | s_fairness | `Default` 値のみ使用; F-5 正規化飽和率 |
| 22 | `RECIPROCITY_DIRECT_DECAY_RHO` | 0.01 | **(U)** | s_fairness | `Default` 値のみ使用; F-1 直接互恵性時間減衰 |
| 23 | `RECIPROCITY_ALPHA_HELP` | 1.0 | **(U)** | s_fairness | `compute_direct_reciprocity()` F-1 α_h — HELP 提供重み |
| 24 | `RECIPROCITY_ALPHA_SUCCESS` | 2.0 | **(U)** | s_fairness | `compute_direct_reciprocity()` F-1 α_hs — HELP 成功重み |
| 25 | `RECIPROCITY_ALPHA_REJECT` | 1.0 | **(U)** | s_fairness | `compute_direct_reciprocity()` F-1 α_r — 拒否ペナルティ重み |
| 26 | `RECIPROCITY_ALPHA_HARM` | 2.0 | **(U)** | s_fairness | `compute_direct_reciprocity()` F-1 α_d — 害ペナルティ重み |
| 27 | `INDIRECT_BETA_CENTRALITY` | 1.0 | **(U)** | s_fairness | `compute_indirect_reciprocity()` F-2 β₁ — 中心性重み |
| 28 | `INDIRECT_BETA_VILLAGE_PARTICIPATION` | 1.0 | **(U)** | s_fairness | `compute_indirect_reciprocity()` F-2 β₂ — 村参加度重み |
| 29 | `INDIRECT_BETA_ACCEPTED_RATE` | 1.0 | **(U)** | s_fairness | `compute_indirect_reciprocity()` F-2 β₃ — 受諾率重み |
| 30 | `INDIRECT_BETA_SUCCESS_RATE` | 2.0 | **(U)** | s_fairness | `compute_indirect_reciprocity()` F-2 β₄ — 成功率重み |
| 31 | `INDIRECT_BETA_HARM_SCORE` | 2.0 | **(U)** | s_fairness | `compute_indirect_reciprocity()` F-2 β₅ — 害スコア重み |
| 32 | `REPUTATION_WEIGHT_DIRECT` | 0.35 | **(U)** | s_fairness | `compute_benevolence_score()` F-3 w_dir — 直接互恵性重み |
| 33 | `REPUTATION_WEIGHT_INDIRECT` | 0.35 | **(U)** | s_fairness | `compute_benevolence_score()` F-3 w_ind — 間接互恵性重み |
| 34 | `REPUTATION_WEIGHT_REPUTATION` | 0.30 | **(U)** | s_fairness | `compute_benevolence_score()` F-3 w_rep — 評判重み |
| 35 | `TRUST_INHERIT_DECAY` | 0.90 | **(U)** | s_growth/s_fairness | `inherit_trust()` の減衰係数; §4A.8 能力拡散で使用 |
| 36 | `PHASE5_REPUTATION_INHERIT_DECAY` | **0.7** | **(U)** | s_fairness | `phase5_capability_diffusion()` の評判継承減衰 |
| 37 | `BENEVOLENT_TOP_FRACTION` | 0.2 | **(U)** | s_fairness | 慈悲的集団定義 上位 20% |
| 38 | `BENEVOLENT_BOTTOM_FRACTION` | 0.2 | **(U)** | s_fairness | 非慈悲的集団定義 下位 20% |

---
#### 4A.0.4 Helper Quality / Softmax 系（F-11・F-12）— 8 定数

Helper Quality Q(h,c,M) = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 39 | `HELP_SOFTMAX_TAU` | 1.0 | **(S)** | s_density | `to_sim_config()` でオーバーライド; F-12 softmax 温度 |
| 40 | `HELP_QUALITY_SUITABILITY_WEIGHT` | 1.0 | **(U)** | s_density | F-11 w_s — ミッション適合性重み |
| 41 | `HELP_QUALITY_TRUST_WEIGHT` | 1.0 | **(U)** | s_density | F-11 w_t — 信頼重み |
| 42 | `HELP_QUALITY_REPUTATION_WEIGHT` | 1.0 | **(U)** | s_density | F-11 w_r — 評判重み |
| 43 | `HELP_WEIGHT_BENEVOLENCE` | 0.20 | **(U)** | s_density | F-11 w_b — 慈悲重み |
| 44 | `HELP_QUALITY_CHILD_NEED_WEIGHT` | 2.0 | **(U)** | s_density | F-11 w_n — 子ニーズ重み |
| 45 | `HELP_QUALITY_DISTANCE_PENALTY` | 1.0 | **(U)** | s_density | F-11 w_d — 距離ペナルティ重み |
| 46 | `CHILD_HELPEE_BIAS_FACTOR` | 2.0 | **(U)** | s_density | phase3 の child helpee 選択バイアス |

---
#### 4A.0.5 遠隔探索系（F-13）— 4 定数

ε_remote = ε_base + a₁·N_child + a₂·B_avg、ただし ε_remote ≤ ε_max

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 47 | `REMOTE_EXPLORATION_BASE` | 0.05 | **(U)** | s_search | F-13 ε_base |
| 48 | `REMOTE_EXPLORATION_MAX` | 0.40 | **(U)** | s_search | F-13 ε_max — 上限 |
| 49 | `REMOTE_EXPLORATION_NEED_COEFF` | 1.0 | **(U)** | s_search | F-13 a₁ — 子ニーズ係数 |
| 50 | `REMOTE_EXPLORATION_BENEVOLENCE_COEFF` | 1.0 | **(U)** | s_search | F-13 a₂ — 慈悲係数 |

---
#### 4A.0.6 HELP 提供・受理系 — 14 定数

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 51 | `HELP_OFFER_QUALITY_WEIGHT` | 1.0 | **(U)** | s_density | 提供判断の品質重み |
| 52 | `HELP_OFFER_LOAD_PENALTY` | 0.5 | **(U)** | s_density | 提供判断の負荷ペナルティ |
| 53 | `HELP_OFFER_RISK_PENALTY` | 0.3 | **(U)** | s_density | 提供判断のリスクペナルティ |
| 54 | `HELP_OFFER_THRESHOLD` | 0.0 | **(U)** | s_density | 提供判断の閾値 |
| 55 | `HELP_ACCEPT_NEED_GAMMA1` | 0.4 | **(U)** | s_density | 受理判断のニーズ重み γ₁ |
| 56 | `HELP_ACCEPT_NEED_GAMMA2` | 0.3 | **(U)** | s_density | 受理判断のニーズ重み γ₂ |
| 57 | `HELP_ACCEPT_NEED_GAMMA3` | 0.3 | **(U)** | s_density | 受理判断のニーズ重み γ₃ |
| 58 | `HELP_ACCEPT_QUALITY_WEIGHT` | 1.0 | **(U)** | s_density | 受理判断の品質重み |
| 59 | `HELP_ACCEPT_UNCERTAINTY_WEIGHT` | 0.5 | **(U)** | s_density | 受理判断の不確実性重み |
| 60 | `HELP_ACCEPT_AUTONOMY_PENALTY` | 0.3 | **(U)** | s_density | 受理判断の自律性ペナルティ |
| 61 | `HELP_ACCEPT_THRESHOLD` | 0.0 | **(U)** | s_density | 受理判断の閾値 |
| 62 | `PHASE3_HELP_LOAD_LEVEL` (should_offer_help) | **0.3** | **(U)** | s_density | `phase3_help_protocol()` の HELP 負荷水準 |
| 63 | `PHASE3_HELP_RISK_LEVEL` (should_offer_help) | **0.2** | **(U)** | s_density | `phase3_help_protocol()` の HELP リスク水準 |
| 64 | `PHASE3_HELPER_BENEVOLENCE_FALLBACK` / `PHASE3_SUCCESS_BV_COEFF` / `PHASE3_SUCCESS_BASE` | **0.5/0.5/0.3** | **(U)** | s_growth/s_density | `phase3_help_protocol()` の HELP 成功判定パラメーター群 |

---
#### 4A.0.7 Child Growth / Maturation 系（F-14・F-15）— 9 定数

Child Growth: ΔG_ij = μ₁·M_ij + μ₂·H_ij + μ₃·B_ij - μ₄·F_ij
Maturation: P_mature = σ(ν₀ + ν₁·E + ν₂·T + ν₃·R + ν₄·B_helper)

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 65 | `CHILD_GROWTH_MU_MISSION_SUCCESS` | 0.60 | **(U)** | s_growth | F-14 μ₁ — mission success 重み |
| 66 | `CHILD_GROWTH_MU_HELP_SUCCESS` | 0.40 | **(U)** | s_growth | F-14 μ₂ — help success 重み |
| 67 | `CHILD_GROWTH_MU_HELPER_BENEVOLENCE` | 0.30 | **(U)** | s_growth | F-14 μ₃ — helper benevolence 重み |
| 68 | `CHILD_GROWTH_MU_FAILURE_BURDEN` | 0.20 | **(U)** | s_growth | F-14 μ₄ — failure burden 重み |
| 69 | `MATURATION_NU_BIAS` | -2.0 | **(U)** | s_growth | F-15 ν₀ — bias 項 |
| 70 | `MATURATION_NU_EXPERIENCE` | 1.0 | **(U)** | s_growth | F-15 ν₁ — 経験値重み |
| 71 | `MATURATION_NU_TRUST` | 1.0 | **(U)** | s_growth | F-15 ν₂ — 信頼重み |
| 72 | `MATURATION_NU_REPUTATION` | 1.0 | **(U)** | s_growth | F-15 ν₃ — 評判重み |
| 73 | `MATURATION_NU_HELPER_BENEVOLENCE` | 0.30 | **(U)** | s_growth | F-15 ν₄ — helper benevolence 重み |

---
#### 4A.0.8 成人閾値系（41B-4）— 3 定数

成人判定条件: experience_count ≥ E_adult AND trust ≥ T_adult AND reputation ≥ R_adult

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 74 | `E_ADULT_THRESHOLD` | 20 | **(U)** | s_growth | 成人経験値閾値 |
| 75 | `T_ADULT_THRESHOLD` | 0.70 | **(U)** | s_growth | 成人信頼閾値 |
| 76 | `R_ADULT_THRESHOLD` | 0.70 | **(U)** | s_growth | 成人評判閾値 |

---
#### 4A.0.9 村形成系（41B-6・41B-7）— 2 定数

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 77 | `VILLAGE_DISTANCE_THRESHOLD` | 0.2 | **(U)** | s_topology | 村クラスタリングの距離閾値 |
| 78 | `VILLAGE_MIN_SIZE` | 3 | **(U)** | s_topology | 最小村サイズ |

---
#### 4A.0.10 人口生成系（Phase 1）— 2 ハードコード

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 79 | `phase1 position perturbation` | **0.1** | **(H)** | s_density | `phase1_population_growth()` の位置摂動 |
| 80 | `phase1 child embedding perturbation` | **0.05** | **(H)** | s_density | `phase1_population_growth()` の子ノード埋込摂動 |

---
#### 4A.0.11 GMR 能力拡散系 — 3 定数

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 81 | `GMR_DIFFUSION_PROBABILITY` | 0.30 | **(U)** | s_growth | `try_gmr_diffusion()` の拡散確率 |
| 82 | `SOFT_MIN_BETA` | 5.0 | **(U)** | s_growth | `DeterminismScore::compute()` の SoftMin β |
| 83 | `DETERMINISM_THRESHOLD` | 0.50 | **(U)** | s_growth | GMR 拡散の決定論閾値 |

---
#### 4A.0.12 メトリクス計算系 — 9 定数

これらの定数は J_kw の各因子を計算する `collect_final_metrics()` およびヘルパー関数内で使用される。

| # | 定数名 | 値 | 分類 | 影響因子 | 使用箇所 |
|---|--------|-----|------|---------|---------|
| 84 | `ECOSYSTEM_GRID_DIVISIONS` | 10 | **(U)** | s_density | `compute_capability_coverage()` グリッド分割数 |
| 85 | `KW_ACCEL_K_NEAREST` | 5 | **(U)** | s_topology | `compute_cluster_coefficient()` 近傍数 k |
| 86 | `KW_ACCEL_DENSITY_RADIUS` | 0.3 | **(U)** | s_topology | `compute_local_density()` 半径 |
| 87 | `KW_ACCEL_NODE_DENSITY_MAX` | 50.0 | **(U)** | s_topology | `compute_mean_node_density()` 正規化定数 |
| 88 | `KW4_CONVERGENCE_THRESHOLD` | 0.1 | **(U)** | s_speed | `check_convergence()` 収束判定閾値 |
| 89 | `KW4_OBSERVATION_INTERVAL` | 10 | **(U)** | s_speed | `check_convergence()` 観測間隔 |
| 90 | `KW4_EVALUATION_POPULATION_SIZE` | 400 | **(U)** | s_growth/s_density/s_topology | `evaluate_single()` の評価人口。エージェント数が全相互作用・密度・トポロジーに影響 |
| 91 | `KW4_SIMULATION_TICKS` | 200 | **(U)** | s_speed | `compute_s_speed()` 内で使用。s_speed = 1 - ttc / total_ticks の total_ticks |
| 92 | `compute_search_radius_inverse fallback` | **0.5** | **(U)** | s_search | kind_world.rs:2039 — 空セッション時のフォールバック値。実関数は L2 距離に基づく正規化逆数（1/(1+mean_distance)）を計算。 |

---
#### 4A.0.13 フェーズ設定系 — 2 定数（探索済み）

| # | 定数名 | 値範囲 | 分類 | 影響因子 | 使用箇所 |
|---|--------|--------|------|---------|---------|
| 93 | `gc_interval`（config） | [1, 10] | **(S)** | s_growth | シミュレーションの GC 実行間隔 |
| 94 | `child_ratio`（config） | [0.05, 0.25] | **(S)** | s_growth | 生成時の子ノード比率 |

---
#### 4A.0.14 因子別カバレッジサマリ

| 因子 | J_kw の式 | 探索済み | 未探索 | ハードコード | 合計 |
|------|-----------|---------|--------|------------|------|
| s_growth | j_pop_growth·j_lifecycle·j_child_survival·j_freshness | 5 | 29 | 1 | 35 |
| s_density | j_benevolence·j_reciprocity·j_help·j_reuse·j_local_density | 1 | 18 | 1 | 20 |
| s_topology | j_village·j_churn·j_interaction·j_diffusion·j_coverage·j_fairness_ratio | 0 | 5 | 0 | 5 |
| s_search | j_execution_success·j_cost_efficiency·j_fidelity·j_nest_depth | 0 | 5 | 0 | 5 |
| s_fairness | j_benevolent_vs_non_benevolent | 2 | 19 | 0 | 21 |
| s_speed | 1 - ttc / total_ticks | 1 | 3 | 0 | 4 |

**主要な発見**:
1. 現在の Bayesian Pareto 最適化（MagnificentSevenParams）は全影響定数の約 **7%（7/94）** しかカバーしていない
2. s_growth が全因子中最多の 35 定数に支配されており、GC hazard・Lifecycle・Child growth・Maturation の各機構が集中している
3. s_fairness が 21 定数で 2 番目に大きく、F-1〜F-3（直接互恵性 α_h〜α_d 4 種・間接互恵性 β₁〜β₅ 5 種・慈悲総和 w_dir〜w_rep 3 種）の計 12 定数を含む
4. WIRE-E（M1.76-KW-WIRE-E）により 7 箇所のハードコード値が定数化・未探索（U）に変更された。残り 3 箇所のハードコード値（#16 `compute_mean_freshness human_weight=0.0`、#79 `phase1 position perturbation=0.1`、#80 `phase1 child embedding perturbation=0.05`）はスコープ外（メトリクス計算パス／Phase1 初期化パラメーター）のため現状維持。
5. 未探索定数の大部分は `ReciprocityLifecyclePolicy` のデフォルト値（`Default` impl から読み込まれる）がそのまま使用されており、`to_sim_config()` でオーバーライドされていない
6. 12 の直接・間接互恵性定数（RECIPROCITY_ALPHA_* / INDIRECT_BETA_* / REPUTATION_WEIGHT_*）は `ReciprocityLifecyclePolicy` 構造体に属さず、`compute_direct_reciprocity()` 等の関数内で直接 `constants::*` を参照している

**将来の較正拡張指針**: 各因子の最小値（ボトルネック）を特定し、該当因子に最も強い影響を与える未探索定数から優先的に探索範囲に追加する。現在のボトルネックは s_search（~0.39）であり、F-13 遠隔探索係数（REMOTE_EXPLORATION_*）が最優先の追加候補である。

---

個人は WorkflowGraph として表現される。1 個の WorkflowGraph = 1 人の「人」である。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 1 | **WorkflowNode::AgentStep** — 個人の基本行動単位 | §12, §13 | — | 1 個の AgentStep = 1 回の action。agent, prompt_template, inputs, output_var, side_effects, determinism から成る。個人の「行動」を表現する最小単位。 |
| 2 | **WorkflowNode::SubWorkflow** — 子（能力・知識）の生成 | §41B | — | 別の WorkflowGraph を子として spawn する。**「子を産む」= 人口増加の基本メカニズム。** 子は親の能力空間の近傍に位置する。 |
| 3 | **WorkflowGraph = DiGraph<WorkflowNode, EdgeMeta>** — 個人の内部構造 | §12 | — | DAG 構造。1 個の WorkflowGraph が 1 人の「人」を構成する。AgentStep が行動、SubWorkflow が子。人口カウントは全 WorkflowGraph の総数。 |
| 4 | **EdgeMeta** — 行動間の関係 | §12 | — | DependsOn, DataFlow, Conditional, FanOut, Collect の 5 種。個人内部の行動の流れを規定する（個人間の互恵性ではない）。 |
| 5 | **MemoizedGraph** — 実行完了状態の WorkflowGraph | §12 | — | WorkflowGraph + outputs + memoized 完了状態。子生成・能力継承の実体。spacepositionembedding を持つ。 |

### 4A.2 位置・村（Position & Village）— 5 機構

各個人（MemoizedGraph）は 3 次元能力空間内の位置を持ち、ユークリッド距離に基づいて村を形成する。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 6 | **SpacePositionEmbedding** — 個人の 3 次元生態的位置 | §12, §41B | — | 各 MemoizedGraph が持つ `Option<[f32; 3]>`。能力空間内での個人のニッチを表す。村クラスタリングの基礎。子は親の近傍に位置する。 |
| 7 | **位置更新 (41B-1)** — 経験による位置変化 | §15.1, §41B | 41B-1 | 経験値獲得に伴う能力空間内での位置の更新。成長・学習により個人の「位置」が変化する。 |
| 8 | **位置分解 (41B-2)** — 合成位置の成分分解 | §41B | 41B-2 | 複合 WorkflowGraph の位置を成分別に分解。COMPOSE 等で合成された個人の位置計算に使用。 |
| 9 | **子供・成人定義 (41B-3, 41B-4)** — 経験による成熟度 | §41B | 41B-3, 41B-4 | 経験値 experience_count に基づく子供/成人の判定。子供保護・村形成・GC の基礎条件。 |
| 10 | **村形成 (41B-6, 41B-7)** — 類似個人の凝集 | §15.5, §41B | 41B-6, 41B-7 | k-means クラスタリング（k = round(N / target_village_size)）により全個人を村に割り当て。target_village_size はシミュレーション設定で調整可能な較正パラメータ（デフォルト 50.0）。村のサイズ・密度は J_diffusion(s_{density}) を通じて間接的に J_kw に寄与。 |

### 4A.3 GMR・能力拡張（GMR & Capability Expansion）— 8 機構

GMR（Goal-Mediated Reasoning）の検索結果評価部（Stage 0・Stage 5）と能力生成分岐。シミュレーションでは ANN/GED パイプライン（Stage 1-4）を省略し、簡略化された検索モデルを使用する。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 11 | **ハードゲート AG-01〜AG-07** — 不変条件による事前フィルタ | §12 | — | 7 個の適用可能性ゲート（consistency, capacity, latency, author, staleness, dependency, cyclic）。シミュレーション内での能力検索の事前条件判定。 |
| 12 | **DeterminismScore D(G)** — 決定論スコア | §13 | SoftMin | 各 AgentStep の determinism 値の SoftMin 合成。出力の予測可能性を測る。GMR 検索の品質指標の 1 つ。 |
| 13 | **ApplicabilityScore A** — 適用可能性スコア | §13 | 幾何平均 | 類似度・決定論・有用性の 3 指標の幾何平均。REUSE/PATCH/COMPOSE/NEW/ABORT の分岐判断に使用。 |
| 14 | **Stage 5 分岐** — 5 方向の適用判断 | §12, §13 | — | REUSE（そのまま使用）/ PATCH（修正して使用）/ COMPOSE（複数結合）/ NEW（新規作成）/ ABORT（不適切）。シミュレーションでは確率的またはスコアベースで分岐を選択。 |
| 15 | **COMPOSE 分岐** — 複数知識の合成 | §13 | — | 2 つ以上の既存 WorkflowGraph を合成して新しい能力を生成。**人口増加メカニズムその 1。** |
| 16 | **NEW 分岐** — 完全新規知識の生成 | §13 | — | 新規 WorkflowGraph を作成。**人口増加メカニズムその 2。** |
| 17 | **Differential Inference** — 差分推論 | §13, §15.3 | — | 既存 WorkflowGraph からの微小変異で新しい WorkflowGraph を生成。**人口増加メカニズムその 3。** |
| 18 | **GraphPatch / GraphPatchSet** — 能力拡張の適用 | §14 | — | GMR 検索結果を元に WorkflowGraph に適用する差分パッチ。能力拡張の実体としてシミュレーション内で使用。 |

### 4A.4 ワークフロー実行（Workflow Execution）— 3 機構

WorkflowGraph を実際に「実行する」ための機構。シミュレーション内で個人が行動する際の枠組み。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 19 | **compile_to_steps** — WorkflowGraph → 実行可能ステップ | §12 | — | WorkflowGraph（DAG）をトポロジカルソートし、直列化された実行ステップ列に変換。各 AgentStep を順次実行可能にする。 |
| 20 | **SideEffectSet** — 副作用の宣言と管理 | §12 | — | AgentStep が外部に及ぼす副作用（DB 書込み・API 呼出し等）の宣言。シミュレーションでは簡略化された副作用モデルで代用。 |
| 21 | **ErrorMode** — エラーハンドリング戦略 | §12 | — | AgentStep 実行失敗時の振舞い（FailFast / Retry / Skip / Fallback）。シミュレーション内での個人の行動成否に影響。 |

### 4A.5 HELP 相互支援（HELP Protocol）— 8 機構

個人間の相互利益プロトコル。5 段階の状態遷移と支援者選択の確率的機構を含む。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 22 | **HELP Proposal** — 支援提案 | §41B | — | HelpProposalEvent として発行される支援の申出。支援者・被支援者・支援内容を含む。**Kind World の相互利益の起点。** |
| 23 | **HELP Offer** — 支援申出への応答 | §41B | — | Proposal に対する受諾/拒否/代替案の応答。支援者は Helper Quality Score に基づいて応答を判断。 |
| 24 | **HELP Decision** — 支援決定 | §41B | — | 支援内容の最終確定。支援者のリソース配分を決定。 |
| 25 | **HELP Execution** — 支援実行 | §41B | — | 実際の支援行動の実行。支援者の WorkflowGraph の一部として実行される。 |
| 26 | **HELP Success** — 支援完了と報酬 | §41B | — | 支援完了と相互利益の確定。双方の互恵性スコア（R_dir, R_ind）が更新される。 |
| 27 | **Helper Quality Score (F-11)** — 支援者品質 | §41B | F-11 | Q_h = α·capability + β·benevolence + γ·availability。支援者選択の品質スコア。**慈悲 benevolence が明示的に含まれる。** |
| 28 | **Softmax Helper Selection (F-12)** — 確率的支援者選択 | §41B | F-12 | P(select h) = exp(Q_h / τ) / Σ exp(Q_j / τ)。温度 τ で探索と活用を制御。**較正パラメータ softmax_temperature が対応。** |
| 29 | **Remote Exploration (F-13)** — 遠隔村への探索 | §41B | F-13 | 異なる村に属する個人間の HELP 確率。村外への支援が発生する条件を規定。村間相互作用の基盤。 |

### 4A.6 互恵性・生存（Reciprocity & Survival）— 9 機構

**Kind World の中核。** 互恵性（慈悲）が GC ハザードを低下させ、生存確率を向上させる因果連鎖。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 30 | **Direct Reciprocity R_dir (F-1)** — 直接互恵性 | §15.10 | F-1 | 「自分が助けた相手から将来助けられる」。過去の相互支援履歴に基づく直接的な互恵スコア。 |
| 31 | **Indirect Reciprocity R_ind (F-2)** — 間接互恵性 | §15.10 | F-2 | 「A を助けた人が B から助けられる」。評判を介した間接的な互恵スコア。 |
| 32 | **Benevolence Aggregate B_i (F-3)** — 慈悲総和 | §15.10 | F-3 | B_i = w_dir · R_dir + w_ind · R_ind + w_rep · Rep_i。慈悲の 3 成分合成値。GC ハザード低下に直接寄与。**較正パラメータ w_dir, w_ind, gamma_benevolence が対応。** |
| 33 | **Reputation Score Rep_i (F-4)** — 評判値 | §15.10 | F-4 | 観測可能な行動履歴から計算される個人の評判。定期的に再計算される。間接互恵性の計算基盤。 |
| 34 | **ReciprocityScore 構造** — 互恵性データ | §15.10 | — | 個人ごとの mutual_aid_count, received_aid_count, last_interaction_tick, reputation_score を保持。シミュレーション内で各 tick 更新される。 |
| 35 | **Experience Normalization (F-5)** — 経験値非線形正規化 | §15.3 | F-5 | 経験値の増加に伴う非線形正規化。初期の急成長と成熟後の飽和をモデル化。LifecycleScore の trust 成分の計算に使用。 |
| 36 | **GC Hazard λ^GC (F-7)** — 削除ハザード | §15.10 | F-7 | λ^GC_i = softplus(λ₀ - γ_L·L_i - γ_B·B_i - γ_C·C_protect_i)。**B_i（慈悲）が高いほどハザード低下 = 生存率向上。較正パラメータ lambda_gc_base, gamma_benevolence が対応。** |
| 37 | **GC Probability P_gc (F-8)** — 削除確率 | §15.10 | F-8 | P_gc_i = 1 - exp(-λ^GC_i · Δt)。各 tick における GC 削除確率。ハザードから確率への変換。 |
| 38 | **Survival Probability P_survive (F-9)** — 生存確率 | §15.10 | F-9 | P_survive_i = exp(-λ^GC_i · Δt)。**Kind World の根幹。慈悲が高い社会ほど生存確率が高まる。** |

### 4A.7 ライフサイクル・成熟（Lifecycle & Maturation）— 8 機構

個人の誕生から成長、そして GC（自然淘汰）までの一連のライフサイクル。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 39 | **LifecycleScore L(G)** — 個人の適合度 | §15.3 | — | Geometric Mean of (freshness, success, trust, usage, reputation)。自然淘汰における「適応度」。GC ハザード計算に使用。 |
| 40 | **5 状態 GC 機械** — 段階的削除プロセス | §15.3 | — | Protected → Active → SoftDeleted → HardDeleteCandidate → Tombstoned。個人の削除は一気に行わず段階的に進行する。 |
| 41 | **GC Interval** — GC 評価周期 | §15.10 | — | GC 評価を毎 tick 行わず一定間隔（gc_interval）で実行。**較正パラメータ gc_interval が対応。** |
| 42 | **Child Protection C_protect (F-10)** — 子供保護 | §15.10, §41B | F-10 | 子供（経験不足の WorkflowGraph）への GC 保護。is_child フラグ + experience_count < MIN_SURVIVAL_EXPERIENCE で発動。 |
| 43 | **Minimum Survival Experience** — 最低生存経験値 | §15.10, §41B | — | experience_count が閾値未満の WorkflowGraph は GC 削除から保護。新規生成された個人のグレイス期間を定義。 |
| 44 | **experience_count** — 個人の経験値 | §15.3, §15.10 | — | 各 WorkflowGraph が持つ経験カウンタ。実行回数・成功回数・相互作用回数によって増加。子供/成人判定・LifecycleScore 計算の基礎。 |
| 45 | **Child Growth (F-14)** — 子供の成長 | §41B | F-14 | 子供の experience_count が時間経過 + 支援受領で増加する関数。成長曲線をモデル化。 |
| 46 | **Maturation Probability (F-15)** — 成人化確率 | §41B | F-15 | 子供が成人（フル参加者）に移行する確率。経験値と経過時間の関数。**較正パラメータ child_ratio が間接的に影響。** |

### 4A.8 信頼・継承（Trust & Inheritance）— 2 機構

新規生成された個人が親から信頼・評判を継承する機構。能力拡散の基盤。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 47 | **SubWorkflow Trust Inheritance** — 子への信頼継承 | §15.3, §15.10 | — | 新しい WorkflowGraph（子）は親から初期信頼値を継承。成長のスタートラインを保証し、世代間の能力伝播を実現。 |
| 48 | **Reputation Inheritance** — 評判継承 | §15.10 | — | 子は親の評判の一部を初期評判として継承。継承率は減衰係数で制御。**評判の世代間伝播 = Kind World の持続性の基盤。** |

### 4A.9 時間・鮮度（Time & Freshness）— 2 機構

シミュレーション内の時間進行を管理する二軸時間モデル。LifecycleScore の freshness 成分の計算に使用。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 49 | **Human Time + Virtual Time 二軸** | §15.3 | — | UTC（人間時間）と VirtualClock（シミュレーション内仮想時刻）の 2 軸で時間を管理。シミュレーション tick が Virtual Time に相当。 |
| 50 | **Blended Freshness F_time** | §15.3 | — | F_time = w_H · F_H + w_V · F_V。人間時間と仮想時刻の混合による Freshness 評価。古い個人ほどスコア低下。 |

### 4A.10 J_kw 社会加速度測定（Social Acceleration Measurement）— 7 機構

Kind World の成立度合いを定量化する目的関数。5 因子の**乗算結合（product form）**と 5 因子最小値ゲートから構成される。加重和では一部の因子の不全が他の因子でマスクされるが、乗算結合では全因子が J_kw に multiplicative に寄与する。これにより全 10 セクション・57 機構の状態が過不足なく J_kw に反映される。

| # | 機構 | RFC § | 数式 | 説明 |
|---|------|-------|------|------|
| 51 | **J_kw 目的関数** — 社会加速度総合指標 | §15.9.2 | 5 因子乗算 | J_kw = s_growth × s_density × s_topology × s_search × s_fairness。**較正ループの最大化目標。** |
| 52 | **5 因子最小値ゲート** — 成立条件チェック | §15.9.2 | — | is_kind_world = (J_kw > 0.8) ∧ (min(S)ᵢ > 0.6)。**全 5 因子 > 0.6 かつ J_kw > 0.8 で Kind World 達成。** |
| 53 | **s_growth — 人口増加速度因子**（旧 S_viability） | §15.9.2 | 4 成分算術平均 | (j_pop_growth + j_lifecycle + j_child_survival + j_freshness) / 4。4A.1(5機構)・4A.7(8)・4A.9(2) の計 15 機構を捕捉。 |
| 54 | **s_density — 多層密度因子**（旧 S_capability） | §15.9.2 | 5 成分算術平均 | (j_cov + j_diffusion + j_reuse + j_nest_depth + j_node_density) / 5。4A.2(5機構)・4A.3(8) の計 13 機構＋社会加速度定義②用 2 機構を捕捉。 |
| 55 | **s_topology — 空間クラスター因子**（旧 S_cooperation） | §15.9.2 | 6 成分算術平均 | (j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density) / 6。4A.5(8機構)・4A.6(9)・4A.8(2) の計 19 機構＋社会加速度定義③用 2 機構を捕捉。 |
| 56 | **s_search — 探索効率因子**（旧 S_efficiency） | §15.9.2 | 4 成分算術平均 | (j_cost + j_execution + j_search_radius_inv + j_reasoning_steps_inv) / 4。4A.4(3機構)＋社会加速度定義④用 2 機構を捕捉。 |
| 57 | **s_fairness — 構造的公平性因子** | §15.9.2 | 1 成分 | 1.0 - j_penalty。4A.6(慈悲的優位性) を捕捉。 |

---

**総計: 10 セクション（4A.1–4A.10）、57 機構。**

収録セクション: シミュレーション個人(5) + 位置・村(5) + GMR・能力拡張(8) + ワークフロー実行(3) + HELP 相互支援(8) + 互恵性・生存(9) + ライフサイクル・成熟(8) + 信頼・継承(2) + 時間・鮮度(2) + J_kw 社会加速度測定(7) = 57

**除外した機構（インフラ・支援素材）**: GMR Stage 1-4（ANN/SQLite/GED/Cache）、KnowledgePrimitive 群、EventBus 群、SearchWorkflow 状態機械・AutoRagConfig 等学習ループ要素、Training Plane 全般、Conversational Knowledge、二重ストア一貫性、Health Invariants（H-1〜H-7）、学習フィードバックループ、Layer 1/Layer 4 連携。

**参照**: 本リストはシミュレーション実装着手前および完了後に、§5（4 層アーキテクチャ概観）、§12–§13（WorkflowGraph / GMR / SearchWorkflow）、§15（Lifecycle / Kind World / J_kw）、§41B（村/HELP）とのクロスチェックに使用すること。上記除外機構は本番実装では必要となるが、シミュレーションでは簡略化または abstract 化して扱う。

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
│  WorkflowCache, MemoizedGraph, 4-Layer Retrieval             │
│  semantic → metadata → cheap GED → full GED                 │
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


**v2.3-k 補足 — WorkflowCache の Residency / Eviction 責務:**

Layer 3a (GMR Retrieval Core) の WorkflowCache は、揮発性の in-memory 加速層であり、明示的な Residency (常駐) および Eviction (追出) ポリシーを持つ。Repository Pair は唯一の canonical persistence authority であり、WorkflowCache からの eviction は永続化データに影響しない。詳細は §8 (WorkflowCache と MemoizedGraph) に規定する。

### 5.5 知識エコシステム統合 (v1.8)

Revision v1.8-final は v1.8 知識エコシステム統合層を維持し、スキーマ・付属情報・正本・アーキテクチャ境界に関する曖昧性をさらに解消するが、以前の規範的動作は変更しない。したがって Revision v1.8-final は、v1.7 で定義された Layer 1 から Layer 3c までの基本的責務を変更することなく、知識エコシステム統合層を導入する。ワークフローオーケストレーションは、ワークフローグラフ・信頼・ライフサイクル・検索トレース・ワークフロー適用性の正本であり続ける。

**v2.3-i 再定義: StructMem / Corpus2Skill 知識基盤機構**

v1.8 で導入された LadybugDB は、従来「知識オブジェクト」として一括りにされていた Fragment、MemoryEvent、MemoryConcept、CanonicalDocument、SkillNode、Chunk、Entity を保持する。v2.3-i はこれらを **2 つの独立した知識形成理論の実体化** として formalize する:

- **StructMem (構造的記憶形成)**: 会話入力・システム内部観測から抽出された断片的記憶 (MemoryEvent) を Fragment として集約し、複数 Fragment から抽象概念 (MemoryConcept) を形成し、検証・昇格を経て CanonicalDocument として確定する知識階層。形成経路: MemoryEvent → Fragment → MemoryConcept → CanonicalDocument。
- **Corpus2Skill (コーパスからの技能抽出)**: 構造化/非構造化文書 (Chunk) からドメイン知識単位 (Entity) を抽出し、それらの実行可能なワークフロー表現 (SkillNode) へとコンパイルする技能形成。形成経路: Chunk → Entity → SkillNode。

これらの 2 機構は実装対象の知識基盤機構であり、LadybugDB を一次永続化先、SQLite をメタデータ・キャッシュ・経路ヒントストアとする。両機構の完全な実装規定は §25 (データベース構成) および §16B (Conversational Knowledge Path) で補足される。既存の知識アクセスプリミティブ平面は、これらの上に重なる決定論的ラッパーとして維持される。

**重要: md ファイル非依存の明確化 (v2.3-i):** StructMem / Corpus2Skill の normative implementation は markdown file parsing ではない。markdown や JSON は authoring / interchange / distribution form として利用しうるが、runtime の正本は LadybugDB の知識オブジェクト関係グラフ、SQLite の管理メタデータ、および起動時検証済みの Workflow IR (PresetWorkflow) である。knowledge object の意味論はファイルレイアウトではなく、object relation と policy により定義される。この原則は §25 (データベース構成) の説明とも整合する。

**workflow root と knowledge root の区別 (v2.3-i):** StructMem / Corpus2Skill では、以下の 2 種類の root が併存しうる。両者は同一概念圏に属するが、識別子およびライフサイクルは分離されうる:
- **workflow root**: capability 実行 graph の root。BakedPresetRegistry に属する PresetWorkflow として表現され、capability の起動・実行・制御を司る。
- **knowledge root**: ontology / policy / skill taxonomy / consolidation rule などの知識オブジェクトの root。LadybugDB 上の知識オブジェクト関係グラフにより表現され、GC 保護 (GcState::Protected) または明示的な root policy により管理される。

LadybugDB は、Fragment、MemoryEvent、MemoryConcept、CanonicalDocument、SkillNode、Chunk、Entity などの知識オブジェクトと、DERIVEDFROM、CONSOLIDATES、ABOUTCONCEPT、SUPERSEDES、MATERIALIZEDAS、COMPILEDTOSKILL の系統関係の正本となる。

統合システムは3平面アーキテクチャとして解釈される SHALL: (a) **ワークフローオーケストレーション平面** — WorkflowGraph、GMR Retrieval Core、SearchWorkflow Engine、Lifecycle GC、TrustProfile で構成される。(b) **知識アクセスプリミティブ平面** — memorygetrecentevents、memorygetconcepts、memorygetconcepthistory、memorytraceorigin、memorypromotetodocument、skilllistchildren、skillgetchunks、skillexpandentities、skillbacktrack、kbhybridsearch の決定論的ラッパーで構成される。(c) **知識永続化平面** — 知識の正本としての LadybugDB と、キャッシュ・キュー・修復状態・経路ヒントのためのオプションの SQLite 実行時メタデータで構成される。

この統合は厳密に追加的である。知識プリミティブを呼び出さないワークフローに対しては、WorkflowGraph コンパイル・適用性計算・パッチ適用・信頼更新・ライフサイクル遷移の既存 v1.7 セマンティクスが有効であり続けなければならない (MUST)。

Revision v1.8-final は、3平面アーキテクチャが既存の v1.7 実装スタックの上に重なる論理的分解であることを明確にする。ワークフローオーケストレーション平面は、引き続き主として Layer 2 から Layer 3c によって実装される。知識アクセスプリミティブ平面は独立したスケジューラやリポジトリではなく、Layer 3b SearchWorkflow および Layer 3a 検索ロジックが、AgentStep 実行を既に統治しているのと同じタイムアウト・監査・信頼・リプレイ制約の下で決定論的知識操作を呼び出す規範的インターフェース面である。知識永続化平面は、永続化された知識オブジェクトと関係、およびオプションの実行時メタデータストアのみを責務とし、WorkflowGraph、GraphVersion、TrustProfile、ライフサイクル状態、SearchTrace に関する既存の v1.7 ワークフロー所有権を再定義してはならない (SHALL NOT)。

**v2.3-c 補足:** Conversational ingestion は、既存の知識アクセスプリミティブ平面および Training Plane の上に重なるオプションのポリシー管理拡張である。これは、正規知識、WorkflowGraph、TrustProfile、ライフサイクル状態、SearchTrace、または訓練-本番分離の所有権を再定義してはならない (SHALL NOT)。

### 5.6 Training Plane 統合 (v1.9)

リビジョン v1.9 は v1.8-final の論理的分解を拡張し、第4の論理平面である **Training Plane** を追加するが、既存の v1.8-final の責務および正本境界はすべて維持する。Training Plane は、ミッション生成、人間レビュー、サンドボックス実行、フィードバック取り込み、カリキュラム形成、本番への昇格を形式化するが、SHALL NOT WorkflowGraph、GraphVersion、TrustProfile、ライフサイクル状態、SearchTrace、または正準知識オブジェクトの所有権を再定義してはならない。

したがって、統合システムは4平面論理アーキテクチャとして解釈されなければならない (SHALL): (a) ワークフローオーケストレーション平面、(b) 知識アクセスプリミティブ平面、(c) 知識永続化平面、(d) Training Plane。Training Plane は独立した実行基盤やリポジトリではない。SearchWorkflow、Trust、Lifecycle、Knowledge Primitive Registry、および昇格/監査制御の上に重なるオーケストレーション拡張である。

Training 成果物は、昇格ゲート、信頼レビュー、監査要件、エビデンス/起点トレース要件、CAS チェック、および一貫性チェックが満たされるまで、本番成果物から隔離されたまま維持されなければならない (SHALL)。本リビジョンは v1.8 の検索、信頼、ライフサイクル、および知識セマンティクスに対して厳密に追加的である。

**v2.3-c 補足:** Conversational ingestion は、既存の知識アクセスプリミティブ平面および Training Plane の上に重なるオプションのポリシー管理拡張である。SHALL NOT 正準知識、WorkflowGraph、TrustProfile、ライフサイクル状態、SearchTrace、または学習-本番分離の所有権を再定義してはならない。

### 5.7 Event Architecture Cross-Cutting Layer (v2.3-g)

v2.3-g は、上記の4層＋4平面アーキテクチャのすべてに横断する **Event Architecture** 層を追加する。Event Architecture は既存の層や平面の責務を変更せず、それらの状態遷移・相互作用・時間進行を統一的なイベント基盤の上に記録する。

**設計原則:**
- **DarviumEventBus** は VirtualClock の唯一の authority であり、全 DarviumEvent の commit, persistence, fan-out, replay を提供する (MUST)。
- いかなる domain subsystem も VirtualClock を直接更新してはならない (MUST NOT)。VirtualClock は Event Bus commit によってのみ進む。
- 既存の直接書き込みログテーブル（SearchTrace、SearchRunLog、TrainingRunLog、TrustAuditLog、RepairLog 等）は、DarviumEvent から materialize される **EventProjection** として再解釈される。
- **HITL/Interaction 基盤**（HumanChannel、InteractionHandle、MetadataStore crash recovery）は後方互換を保持したまま、Event Bus 上の汎用 interaction 抽象として一般化される。
- 既存の4層スタック（Layer 1〜3c）と4平面（Workflow / Knowledge / Training / Conversational）は有効であり、Event Architecture はそれらを補完する横断的コミット基盤である (§12C 参照)。

### 5.8 Preset Registry 層 (v2.3-i)

v2.3-i は、上記の4層＋4平面＋Event Architecture に加え、**Preset Registry 層** を導入する。Preset Registry 層は、以下の 3 つの registry で構成される二重 architecture を持つ:

- **BakedPresetRegistry**: コンパイル時にバイナリに埋め込まれる immutable な PresetWorkflow 群。platform-critical であり、展開・検証の失敗はプロセス起動の fatal とする。StructMem / Corpus2Skill の root preset を含む。
- **MutablePresetRegistry**: 起動時にファイルシステムから読み込まれるユーザー拡張可能な PresetWorkflow 群。検証失敗エントリは quarantine されるが、registry 全体の起動を阻止しない。
- **ResolvedWorkflowRegistry**: Baked + Mutable の runtime 統合。名前空間衝突解決・source provenance 追跡・依存方向検証を提供する。

Preset Registry 層は、既存の WorkflowCache (ユーザー生成・保存済み MemoizedGraph 群の runtime cache) とは別の論理 registry である。PresetWorkflow は起動時に Load-once / Verify-once され、root preset は GC 保護 (GcState::Protected) を受ける。詳細は §8 (WorkflowCache と MemoizedGraph) に規定する。

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
    sends_notification: bool,        // 通知送信 (HumanChannel::notify() に対応)
    has_hitl_communicate: bool,      // 双方向 HITL (HumanChannel::communicate() に対応, v2.3-d)
    modifies_persistent_state: bool, // DB 等の永続状態変更
    /// true の場合は AG-03 ハードゲートでブロック
    irreversible: bool,
    /// [0.0, 1.0]: writes_external_api=1.0, DB変更=0.7, 通知=0.3, HITL Communicate=0.5
    risk_score: f32,
}

impl SideEffectSet {
    /// 副作用包含チェック: self が mission_required を包含するかどうか
    /// Stage 0 フィルタで使用 (§11.2)
    fn contains(&self, required: &SideEffectSet) -> bool {
        (!required.writes_external_api || self.writes_external_api)
            && (!required.sends_notification || self.sends_notification)
            && (!required.has_hitl_communicate || self.has_hitl_communicate)
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

// v2.3-i: CompileError は creation-time validation と compile-time validation の二段構えにおける compile-time 側のエラー群である。
// creation-time (preset startup validation) は PresetValidationReason (§8) が担当し、IR 形式・名前空間・依存方向・boot-criticality を検証する。
// CompileError は、creation-time を通過した workflow を Layer 1 命令列へコンパイルする過程で生じる residual error を扱う。
// 両者の責務分割: PresetValidationReason は「登録可否」、CompileError は「実行可否」を判定する。
```

---

## 8. WorkflowCache と MemoizedGraph

**v2.3-j 再定義:** `WorkflowCache` は、SQLite + LadybugDB から構成される Repository Pair 上に永続化された MemoizedGraph 群の runtime cache である。WorkflowCache は source-of-truth ではなく、検索高速化・局所再利用・compile-time / retrieval-time 参照のための in-memory working set を提供する。MemoizedGraph の canonical persistence, consistency, repair, quarantine, and availability は Repository Pair により担保されなければならない (MUST)。

Mission を受けた SearchWorkflow / RetrievalPrimitive は、論理的には Repository Pair 上の MemoizedGraph 全体を検索対象とする。WorkflowCache はその部分集合を保持する加速機構であり、cache miss は Repository Pair からの lazy load により解決されなければならない (MUST)。

runtime のワークフロー lookup は、これとは別に BakedPresetRegistry + MutablePresetRegistry を統合した `ResolvedWorkflowRegistry` (§8.9) が提供する。compiler の `registry.get(workflowid)` は原則として ResolvedWorkflowRegistry に対して行われ、WorkflowCache は検索高速化・局所再利用・compile-time / retrieval-time 参照のための in-memory working set を担う。永続化・整合性・修復は Repository Pair が責務を持つ。

**v2.3-k 補足 — Cache Residency and Eviction Semantics:**

WorkflowCache は lazy load によりエントリが増加するが、unbounded growth を許可するわけではない。WorkflowCache の各エントリは揮発性の residency object であり、Repository Pair 上に `ConsistencyState::Committed` として存在する限り、cache miss 時に `get_or_load` により再ロード・再常駐化が可能である。eviction とは Repository Pair 上の graph 削除ではなく、WorkflowCache からの in-memory dereference を意味する。P-17〜P-21 の制約に従い、eviction は永続化データに影響してはならない (MUST NOT)。

```rust
struct WorkflowCache {
    working_set: Arc<RwLock<Vec<MemoizedGraph>>>,
    ann_hint:    Arc<RwLock<AnnHotIndex>>,  // 最近の検索パターンに最適化された ANN ヒント
    policy:      CachePolicy,
    // v2.3-k: Cache Residency / Eviction 制御フィールド
    max_entries:        usize,                // 最大エントリ数 (0 = 無制限)
    max_bytes:          usize,                // 最大推定バイト数 (0 = 無制限)
    default_ttl_human:  Duration,             // ヒューマンタイム TTL
    default_ttl_virtual: u64,                 // 仮想時間 TTL (VirtualClock ticks)
    eviction_interval:  Duration,             // periodic eviction 間隔
    residency_meta:     HashMap<WorkflowGraphId, CacheResidencyMeta>,
    eviction_policy:    EvictionPolicy,
}

struct RepositoryPair {
    sqlite: SqliteStore,
    ladybug: LadybugStore,
}

enum CachePolicy {
    Default,
    Pinned { workflow_ids: Vec<WorkflowGraphId> },
    Preload { workflow_ids: Vec<WorkflowGraphId> },
}

// v2.3-k: Eviction Policy (CachePolicy とは別軸の eviction 設定)
enum EvictionPolicy {
    /// eviction を一切行わない (legacy 互換)
    Disabled,
    /// TTL + capacity に基づく標準 eviction
    Standard {
        protect_presets: bool,             // デフォルト true
        enable_periodic_eviction: bool,    // 周期タスクによる eviction を有効にする
        enable_ttl_eviction: bool,         // TTL ベース eviction を有効にする
        evict_on_pressure: bool,           // ResourcePressure 駆動 eviction を有効にする
        ttl_human:          Duration,      // ヒューマンタイム TTL 上書き (None で default_ttl_human)
        ttl_virtual:        u64,           // 仮想時間 TTL 上書き (None で default_ttl_virtual)
    },
    /// ResourcePressure に対して積極的に eviction する
    Aggressive {
        protect_presets: bool,
        pressure_watermark: f64,           // 0.0〜1.0, これを超えると強制 eviction
    },
}

// v2.3-k: 各 cache entry の residency メタデータ
struct CacheResidencyMeta {
    graphid:              WorkflowGraphId,
    loaded_at:            SystemTime,
    last_cache_hit_at:    SystemTime,
    last_cache_hit_vt:    u64,
    estimated_bytes:      usize,
    eviction_exempt:      bool,
    last_eviction_reason: Option<String>,
}

// v2.3-k: eviction 操作のレポート
struct EvictionReport {
    scanned:               usize,
    evicted:               usize,
    skipped_protected:     usize,
    skipped_non_committed: usize,
    freed_estimated_bytes: usize,
}

// v2.3-k: eviction 理由の分類
enum EvictionReason {
    TtlExpiredHuman,
    TtlExpiredVirtual,
    CapacityPressure,
    ResourcePressure,
    GcStateTransition,
    ManualCleanup,
}

/// Repository Pair 上の AnnIndex の hot subset。
/// 最近の検索パターンに基づき WorkflowCache が保持する ANN ヒントであり、
/// 完全な AnnIndex は LadybugDB (Repository Pair) 上の HNSW インデックスである。
type AnnHotIndex = AnnIndex;

struct MemoizedGraph {
    id:               WorkflowGraphId,
    graph:                     WorkflowGraph,
    task_embedding:            Vec<f32>,    // ミッション/タスク記述の埋め込み
    workflow_design_text:      String,      // canonical workflow design text (§9)
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
    top_metadata:           TopLevelGraphMetadata,     // v2.3-h: 最上階 DAG メタデータ
    cheap_ged_signature:    CheapGedSignature,         // v2.3-h: cheap GED 用 replayable signature
    // v2.3-i: Preset Registry 拡張フィールド
    artifact_origin_kind:   ArtifactOriginKind,        // 出自: PresetSystem / PresetUser / SearchGenerated 等
    preset_source_info:     Option<PresetSourceInfo>,  // PresetWorkflow の場合の baked/mutable 情報
    root_policy:            PresetRootPolicy,          // root 保護ポリシー
    capability_family:      CapabilityFamily,          // StructMem / Corpus2Skill / Search / Training / General
    registry_source:        Option<RegistrySource>,    // BakedPlatform / MutableUser / MutableWorkspace
    // v2.3-i: CAS 用の楽観的バージョンカウンタ (§8.4)
    version: u64,
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
    presetlineage:    Option<String>,  // 元 preset の workflowid (v2.3-i 追加)
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

/// v2.3-h: 最上階 WorkflowGraph の軽量メタデータ（SQLite metadata filter Stage 2 入力）
struct TopLevelGraphMetadata {
    top_node_count:         u16,
    top_edge_count:         u16,
    top_source_count:       u16,
    top_sink_count:         u16,
    top_longest_path_len:   u16,
    top_max_width:          u16,
    top_label_histogram:            Vec<(String, u16)>,
    top_edge_type_histogram:         Vec<(String, u16)>,
    top_determinism_summary:         f32,
    top_sideeffect_summary:          SideEffectSet,     // RFC §6.1 SideEffectSet 参照
    top_agentsethash:                u64,           // top-level agent family summary
    top_layer_signature:             Vec<u64>,
}

/// v2.3-h: cheap GED 用 replayable deterministic graph signature
struct CheapGedSignature {
    topo_rank_labels:        Vec<u64>,
    indegree_histogram:      Vec<u16>,
    outdegree_histogram:     Vec<u16>,
    ancestor_bitset_sketch:  Vec<u64>,
    descendant_bitset_sketch: Vec<u64>,
    path_hash_multiset:      Vec<(u64, u16)>,
    signature_version:       String,
}

/// v2.3-h: metadata filter の query 側入力、QueryDesignText から deterministic formatter で導出
struct TopLevelQueryMetadata {
    top_query_node_count:         u16,
    top_query_edge_count:         u16,
    top_query_source_count:       u16,
    top_query_sink_count:         u16,
    top_query_longest_path_len:   u16,
    top_query_label_histogram:    Vec<(String, u16)>,
    top_query_agent_set_hash:     u64,
    top_query_side_effect_kind:   SideEffectSet,     // RFC §6.1 SideEffectSet 参照
    metadata_format_version:      String,
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
    /// v2.3-i: root preset 等の GC 完全除外対象
    Protected { reason: String },
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
// v2.3-g 補足: VirtualClock は「commit 済み DarviumEvent 列の順序番号」として解釈しなければならない (MUST)。
// VirtualClock は DarviumEventBus の Event commit によってのみ進む。
// いかなる application code も advance_virtual_clock を直接呼んではならない (MUST NOT)。

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

// v2.3-i: Preset Registry データ型

/// MemoizedGraph の出自種別
enum ArtifactOriginKind {
    PresetSystem,        // BakedPresetRegistry 由来の system preset
    PresetUser,          // MutablePresetRegistry 由来の user preset
    SearchGenerated,     // SearchWorkflow により生成
    TrainingDerived,     // Training Plane 由来
    FusionDerived,       // Fusion 操作由来
    Conversational,      // Conversational Knowledge Path 由来
    Manual,              // 手動登録
}

/// PresetWorkflow のソース情報
struct PresetSourceInfo {
    registry_source: RegistrySource,
    preset_metadata: PresetMetadata,
    loaded_at:       SystemTime,
    validated_at:    SystemTime,
}

/// PresetWorkflow が読み込まれた registry ソース
enum RegistrySource {
    BakedPlatform,      // platform-provided baked preset
    MutableUser,        // user-provided mutable preset
    MutableWorkspace,   // workspace-level mutable preset
}

/// capability の機能的分類
enum CapabilityFamily {
    StructMem,
    Corpus2Skill,
    Search,
    Training,
    General,
}

/// PresetWorkflow の root 保護ポリシー
enum PresetRootPolicy {
    RootPinned,            // GC から常時保護 (GcState::Protected)
    RootUnpinned,          // 通常の GC 対象
    RootAncestorPinned,    // 先祖が pinned の場合に保護
}

/// PresetWorkflow のメタデータ
struct PresetMetadata {
    workflow_id:   String,
    version:       String,
    family:        CapabilityFamily,
    description:   String,
    dependencies:  Vec<String>,       // 依存 workflow_id 一覧
    authors:       Vec<String>,
    created_at:    SystemTime,
}

/// PresetWorkflow 検証失敗の理由
enum PresetValidationReason {
    InvalidPresetSchema,
    DuplicateWorkflowId,
    ReservedNamespaceViolation,
    WorkflowNotFound,
    CrossRegistryDependencyViolation,
    CircularReference,
    InvalidInputMapping,
    OutputBindingMismatch,
    BootCriticalPresetMissing,
    BootCriticalPresetInvalid,
    MutableOverrideForbidden,
    PresetPolicyViolation,
}

/// PresetWorkflow の検証失敗（診断用完全情報）
struct PresetValidationFailure {
    workflowid:     Option<String>,
    source:         RegistrySource,
    source_path:    Option<String>,
    reasons:        Vec<PresetValidationReason>,
    detected_at:    SystemTime,
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

/// v2.3-g 改訂: advance_virtual_clock は DarviumEventBus の内部実装詳細としてのみ使用される。
/// いかなる application code もこの関数を直接呼んではならない (MUST NOT)。
/// Event Bus は commit ごとに VirtualClock を 1 以上単調増加させなければならない (MUST)。
/// 同一 event に対して重複 commit を行ってはならない (MUST NOT)。
/// replay は既存 event を再利用し、VirtualClock を再増加させてはならない (MUST NOT)。
fn advance_virtual_clock(clock: &mut VirtualClockState, delta: u64) {
    // INTERNAL: DarviumEventBus 実装のみが呼び出せる。
    // いかなる application code も直接呼んではならない。
    assert!(delta > 0);
    clock.current = clock.current.saturating_add(delta);
    clock.updated_at = SystemTime::now();
}

fn mark_virtual_seen(graph: &mut MemoizedGraph, clock: &VirtualClockState) {
    graph.last_virtual_seen = clock.current;
}
```

**v2.3-k 補足 — Cache Hit Tracking と TTL Policy:**

cache hit 時には `Provenance.last_used_at` に加えて、cache residency metadata 側の `last_cache_hit_at` / `last_cache_hit_vt` も更新しなければならない (MUST)。TTL 判定には `last_used_at` と `last_virtual_seen` を使用してよい (MAY) が、preset-protected entry (P-18) には TTL を適用してはならない (MUST NOT)。

### 8.2 cold-start 初期化 (P-07)

新規 MemoizedGraph を Repository Pair に登録する際は、必ず cold-start trust で初期化しなければならない (MUST)。Trust が 0.0 のグラフを登録してはならない (MUST NOT)。また `gc_state = Active`、`experience_count = 0`、`last_virtual_seen = current_virtual_clock`、`reputation.final_score = REPUTATION_COLD_START` で初期化しなければならない (MUST)。

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
    HumanReviewNeedsRevision,
    HumanReviewIrrelevant,
    HumanReviewUnsafe,
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

AbstractableSubgraph から切り出された部分グラフは、元グラフ内部の局所置換にとどめてはならず、新規 `WorkflowId` を持つ独立 `WorkflowGraph` として再構成し、`MemoizedGraph` として Repository Pair に永続化し、WorkflowCache に登録しなければならない (MUST)。元グラフ側は `WorkflowNode::SubWorkflow` へ置換されるが、その参照先は元グラフ専用の匿名断片ではなく、他の Application Workflow / SearchWorkflow から再利用可能な共有資産として扱わなければならない (MUST)。

SubWorkflow 資産にも通常の graph 資産と同様に、`TrustProfile`、`WorkflowLineage`、`ContributionRecord`、`WorkflowDesignText`、`Metrics`、`TimeDecayProfile`、`ReputationProfile`、`GcState`、`experience_count` を付与しなければならない (MUST)。新規抽象化で生成された SubWorkflow は `Grace Period` の保護対象とし、観察前に GC してはならない (MUST NOT)。

```rust
/// MemoizedGraph を構築して返す。呼び出し元は Repository Pair への非同期永続化および
/// WorkflowCache への登録を別途行うこと (SHOULD)。
fn register_abstracted_subworkflow(
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

`apply_patch_atomic` が複数スレッドから同一グラフに並列適用された場合、後勝ちによる更新消失を防ぐために楽観的並行性制御 (Optimistic Concurrency Control) を使用する。MemoizedGraph は `version: u64` フィールドを持ち (§8 構造体定義参照)、更新のたびにインクリメントされる。

```rust
/// WorkflowCache 層のエラー（インメモリ操作・CAS 競合）
#[derive(Debug, thiserror::Error)]
enum CacheError {
    #[error("Version conflict: expected {expected}, found {actual}")]
    CasConflict { expected: u64, actual: u64 },
    #[error("Graph not found in cache: {0:?}")]
    NotFound(WorkflowGraphId),
    #[error("Lazy load from Repository Pair failed: {0}")]
    LoadFailed(String),
    // v2.3-k: Cache Residency / Eviction エラー
    #[error("Capacity exceeded: max_entries={max_entries}, max_bytes={max_bytes}")]
    CapacityExceeded { max_entries: usize, max_bytes: usize },
    #[error("Protected entry eviction forbidden: {0:?}")]
    ProtectedEvictionForbidden(WorkflowGraphId),
    #[error("Eviction invariant violation: {0}")]
    EvictionInvariantViolation(String),
}

/// Repository Pair 永続化層のエラー（デュアルストア一貫性）
#[derive(Debug, thiserror::Error)]
enum PersistenceError {
    #[error("Cross-store inconsistency detected: {0}")]
    CrossStoreInconsistency(String),
    #[error("SQLite operation failed: {0}")]
    SqliteError(String),
    #[error("LadybugDB operation failed: {0}")]
    LadybugError(String),
    #[error("Repository Pair not found: {0}")]
    PairNotFound(String),
}

impl WorkflowCache {
    /// 楽観的更新: expected_version が現在バージョンと一致する場合のみ更新を適用
    async fn update_graph_cas(
        &self,
        graph_id: WorkflowGraphId,
        new_graph: WorkflowGraph,
        expected_version: u64,
    ) -> Result<u64, CacheError> {
        let mut store = self.working_set.write().await;
        let entry = store.iter_mut()
            .find(|g| g.id == graph_id)
            .ok_or(CacheError::NotFound(graph_id))?;
        if entry.version != expected_version {
            return Err(CacheError::CasConflict {
                expected: expected_version,
                actual:   entry.version,
            });
        }
        entry.graph   = new_graph;
        entry.version += 1;
        Ok(entry.version)
    }

    /// Repository Pair から MemoizedGraph を lazy load する
    ///
    /// v2.3-k 補足: 呼び出し前に capacity guard を評価し、必要なら eviction pass を実行する。
    /// 新規エントリ追加時は preset-safe guard を維持したまま max_entries/max_bytes を超過しない
    /// ことを確認する。超過時に非 protected エントリを十分に eviction できない場合は
    /// CacheError::CapacityExceeded を返す。
    async fn get_or_load(
        &self,
        graph_id: WorkflowGraphId,
        pair: &RepositoryPair,
    ) -> Result<MemoizedGraph, CacheError> {
        // cache hit チェック
        {
            let store = self.working_set.read().await;
            if let Some(g) = store.iter().find(|g| g.id == graph_id) {
                // cache hit → residency_meta の last_cache_hit_at / last_cache_hit_vt を更新
                if let Some(meta) = self.residency_meta.get(&graph_id) {
                    // meta.last_cache_hit_at = SystemTime::now();
                    // meta.last_cache_hit_vt = current_vt;
                }
                return Ok(g.clone());
            }
        }
        // cache miss → Repository Pair から load
        let graph = pair.load(graph_id.clone())
            .await
            .map_err(|e| CacheError::LoadFailed(e.to_string()))?;
        // capacity guard: 新規エントリ追加前に capacity 制約を確認
        {
            let mut store = self.working_set.write().await;
            // 現在のエントリ数・推定バイト数をチェック
            if self.max_entries > 0 && store.len() >= self.max_entries {
                // 非 protected エントリを eviction して空きを作る
                drop(store); // 一時的にロック解放
                let report = self.evict_to_capacity().await;
                if report.evicted == 0 {
                    return Err(CacheError::CapacityExceeded {
                        max_entries: self.max_entries,
                        max_bytes: self.max_bytes,
                    });
                }
                let mut store = self.working_set.write().await;
            }
            store.push(graph.clone());
            // residency_meta を新規作成または更新
            // self.residency_meta.insert(graph_id.clone(), CacheResidencyMeta {
            //     graphid: graph_id.clone(),
            //     loaded_at: SystemTime::now(),
            //     last_cache_hit_at: SystemTime::now(),
            //     last_cache_hit_vt: current_vt,
            //     estimated_bytes: graph.estimated_bytes(),
            //     eviction_exempt: self.is_eviction_protected(&graph),
            //     last_eviction_reason: None,
            // });
        }
        Ok(graph)
    }
}

// v2.3-k: eviction 関連 API 群 (疑似コード)
impl WorkflowCache {
    /// 保護判定: この graph が eviction 禁止かどうかを返す。
    /// GcState::Protected, ArtifactOriginKind::PresetSystem,
    /// PresetRootPolicy::RootPinned | RootAncestorPinned のいずれかに該当する場合は true。
    fn is_eviction_protected(&self, graph: &MemoizedGraph) -> bool {
        match graph.gc_state {
            GcState::Protected { .. } => return true,
            _ => {}
        }
        if graph.artifact_origin_kind == ArtifactOriginKind::PresetSystem { return true; }
        match graph.root_policy {
            PresetRootPolicy::RootPinned | PresetRootPolicy::RootAncestorPinned => return true,
            PresetRootPolicy::RootUnpinned => {}
        }
        false
    }

    /// graph_id から保護判定を行うヘルパー。
    fn is_eviction_protected_by_graph_id(&self, graph_id: &WorkflowGraphId) -> bool {
        let store = self.working_set.read().await;
        if let Some(graph) = store.iter().find(|g| g.id == *graph_id) {
            self.is_eviction_protected(graph)
        } else {
            false // cache に存在しないものは保護対象外
        }
    }

    /// 1 エントリを eviction する。
    fn evict_one(&self, graph_id: WorkflowGraphId, reason: EvictionReason) -> Result<EvictionReport, CacheError> {
        let mut store = self.working_set.write().await;
        let idx = store.iter().position(|g| g.id == graph_id)
            .ok_or(CacheError::NotFound(graph_id))?;
        if self.is_eviction_protected(&store[idx]) {
            return Err(CacheError::ProtectedEvictionForbidden(graph_id));
        }
        let estimated = store[idx].estimated_bytes();
        store.remove(idx);
        // residency_meta も削除
        self.residency_meta.remove(&graph_id);
        Ok(EvictionReport { scanned: 1, evicted: 1, skipped_protected: 0, skipped_non_committed: 0, freed_estimated_bytes: estimated })
    }

    /// TTL 期限切れエントリを一括 eviction する。
    fn evict_expired(&self, now: SystemTime, current_vt: u64) -> EvictionReport {
        // last_cache_hit_at / last_cache_hit_vt と TTL 設定を比較し、
        // 期限切れかつ非 protected のエントリを除去する。
        todo!("evict_expired — scan + filter + remove")
    }

    /// ResourcePressure に基づく eviction を実行する。
    fn evict_for_pressure(&self, pressure: ResourcePressure, env: &EnvironmentPolicy) -> EvictionReport {
        // PressureMode::Constrained 以上で非 protected エントリを段階的に eviction
        todo!("evict_for_pressure — pressure-driven eviction")
    }

    /// max_entries / max_bytes を超過しないよう eviction する。
    fn evict_to_capacity(&self) -> EvictionReport {
        // 現在のエントリ数・推定バイト数と max_entries/max_bytes を比較し、
        // 超過している場合は非 protected エントリを LRU 順に eviction する。
        // 十分に eviction できない場合は CacheError::CapacityExceeded を返す。
        todo!("evict_to_capacity — capacity-bound eviction")
    }

    /// GcState 遷移に対応する cache eviction を実行する。
    fn handle_gc_state_transition(&self, graph_id: WorkflowGraphId, old_state: GcState, new_state: GcState) -> Result<(), CacheError> {
        match new_state {
            GcState::Tombstoned { .. } => {
                // Tombstoned 遷移時は cache からの除去を必須とする (P-19)。
                // protected でも除去する (tombstone は全保護より優先)。
                let mut store = self.working_set.write().await;
                store.retain(|g| g.id != graph_id);
                self.residency_meta.remove(&graph_id);
                Ok(())
            }
            GcState::SoftDeleted { .. } | GcState::HardDeleteCandidate { .. } => {
                // 非 protected のみ eviction 試行
                if !self.is_eviction_protected_by_graph_id(&graph_id) {
                    self.evict_one(graph_id, EvictionReason::GcStateTransition)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl RepositoryPair {
    /// MemoizedGraph を永続層から読み込む
    async fn load(&self, graph_id: WorkflowGraphId) -> Result<MemoizedGraph, PersistenceError> {
        // SQLite からメタデータ読み取り、LadybugDB から graph/embedding 読み取り
        // 両ストアの整合性を確認して Committed 状態の MemoizedGraph を返す
        todo!("RepositoryPair::load — dual-store load with consistency check")
    }

    /// デュアルストアコミット
    async fn commit_dual_store_update(&self, op_id: String, graph: &mut MemoizedGraph) -> Result<(), PersistenceError> {
        graph.consistency_state = ConsistencyState::Pending {
            op_id: op_id.clone(),
            phase: CommitPhase::MetaPrepared,
        };

        self.sqlite_prepare(op_id.clone()).map_err(PersistenceError::SqliteError)?;
        self.ladybug_prepare(op_id.clone()).map_err(PersistenceError::LadybugError)?;

        match (self.sqlite_commit(op_id.clone()), self.ladybug_commit(op_id.clone())) {
            (Ok(()), Ok(())) => {
                graph.consistency_state = ConsistencyState::Committed;
                Ok(())
            }
            (meta_res, blob_res) => {
                graph.consistency_state = ConsistencyState::NeedsRepair {
                    op_id: op_id.clone(),
                    reason: format!("meta={:?}, blob={:?}", meta_res.err(), blob_res.err()),
                };
                self.enqueue_repair(op_id, graph.id.clone());
                Err(PersistenceError::CrossStoreInconsistency(
                    "Dual-store commit failed; repair enqueued".into()
                ))
            }
        }
    }

    fn sqlite_prepare(&self, _op_id: String) -> Result<(), String> { Ok(()) }
    fn ladybug_prepare(&self, _op_id: String) -> Result<(), String> { Ok(()) }
    fn sqlite_commit(&self, _op_id: String) -> Result<(), String> { Ok(()) }
    fn ladybug_commit(&self, _op_id: String) -> Result<(), String> { Ok(()) }
    fn enqueue_repair(&self, _op_id: String, _graph_id: WorkflowGraphId) {}
}
```

**呼び出しパターン**: `apply_patch_atomic` を呼ぶ前に `graph.version` を読み取り、成功後に `update_graph_cas(id, new_graph, read_version)` で CAS 更新する。`CacheError::CasConflict` が返った場合は最新バージョンで再試行すること (SHOULD)。

**設計根拠**: `RwLock` は読み取り多数・書き込みまれの前提で良好なスループットを提供する。`apply_patch_atomic` のクローン + バリデーションは純粋計算であり、ロックを保持したまま実行する必要はない。バージョン CAS はロック解放後の更新競合を検出する安全ネットとして機能する。


### 8.5 BakedPresetRegistry (v2.3-i)

BakedPresetRegistry は、バイナリにコンパイル時に埋め込まれる immutable な PresetWorkflow 群である。StructMem / Corpus2Skill の root preset を含み、以下の特性を持つ:

- **Platform-critical**: 発見不可・展開失敗・検証失敗はプロセス起動を fatal とする。
- **Immutable**: 実行中の追加・削除・変更は一切不可。変更が必要な場合はバイナリ再ビルドを要する。
- **Boot-fatal**: 1 エントリでも検証に失敗した場合、プロセスは起動してはならない (MUST)。

Baked preset の名前空間は `platform.*` / `builtin.*` / `system.*` に予約され、MutablePresetRegistry からの予約名使用は禁止される。

```rust
/// v2.3-i: BakedPresetRegistry — immutable, platform-critical, boot-fatal
struct BakedPresetRegistry {
    presets: Vec<PresetWorkflow>,
}

struct PresetWorkflow {
    metadata:  PresetMetadata,
    workflow:  WorkflowGraph,
    root_policy: PresetRootPolicy,
}

impl BakedPresetRegistry {
    /// ビルド時にバイナリセグメントから展開する。失敗は boot-fatal
    fn load_from_binary_segment(data: &[u8]) -> Result<Self, PresetValidationFailure> {
        // 1. バイナリセグメントをパース
        // 2. 各 PresetWorkflow をスキーマ検証
        // 3. 依存関係を解決
        // 4. 循環依存を検出
        // 5. DAG 検証
        // 6. 名前空間予約違反をチェック
        // 7. 全エントリ成功時のみ Ok、1 つでも失敗時は Err で即時 fatal
        todo!()
    }

    /// PresetWorkflow を ID で参照
    fn get(&self, workflow_id: &str) -> Option<&PresetWorkflow> {
        self.presets.iter().find(|p| p.metadata.workflow_id == workflow_id)
    }
}
```

### 8.6 MutablePresetRegistry (v2.3-i)

MutablePresetRegistry は、起動時にファイルシステムから読み込まれるユーザー拡張可能な PresetWorkflow 群である。以下の特性を持つ:

- **User-extensible**: 運用者は設定ファイルまたは専用ディレクトリに preset workflow を配置できる。
- **Graceful degradation**: 検証失敗エントリは registry から隔離 (quarantine) されるが、残りの正常エントリは利用可能とし、registry 全体の起動を阻止しない。
- **Load-once / Verify-once**: 起動時に一度だけ読み込まれ検証される。runtime での追加/更新は起動時とは別の hot-reload 機構（本 RFC スコープ外）を要する。

```rust
/// v2.3-i: MutablePresetRegistry — ユーザー拡張可能、graceful degradation
struct MutablePresetRegistry {
    presets:     Vec<PresetWorkflow>,
    quarantined: Vec<PresetValidationFailure>,
    source_dir:  PathBuf,
}

impl MutablePresetRegistry {
    /// ファイルシステムからプリセットをスキャン・ロード・検証
    fn load_from_directory(source_dir: &Path) -> Self {
        let mut presets = Vec::new();
        let mut quarantined = Vec::new();
        for entry in scan_preset_files(source_dir) {
            match PresetWorkflow::validate_and_parse(&entry) {
                Ok(preset) => presets.push(preset),
                Err(failure) => quarantined.push(failure),
            }
        }
        MutablePresetRegistry { presets, quarantined, source_dir: source_dir.to_path_buf() }
    }

    fn presets(&self) -> &[PresetWorkflow] { &self.presets }
    fn quarantined_failures(&self) -> &[PresetValidationFailure] { &self.quarantined }
}
```

### 8.7 12段階起動時検証手順 (v2.3-i)

Preset Registry の起動時検証は以下の 12 段階で逐次実行されなければならない (MUST)。検証順序は依存関係を考慮し、前方の段階で失敗した場合は後方の段階を実行せずに当該エントリを却下する。

| 段階 | 名称 | 対象 | 失敗時動作 |
|------|------|------|-----------|
| 1 | Baked Expand | BakedPresetRegistry のバイナリセグメント展開 | Boot-fatal |
| 2 | Baked Parse & Validate | 各 PresetWorkflow のスキーマ・形式検証 | Boot-fatal |
| 3 | Boot-Critical Check | root preset (StructMem/Corpus2Skill) の存在確認 | Boot-fatal |
| 4 | Mutable Scan | ファイルシステム上のプリセットファイル列挙 | スキップ可 (warning) |
| 5 | Mutable Parse | 各ファイルのパース | 該当エントリのみ quarantine |
| 6 | Schema Validation | 必須フィールド・型・値範囲の検証 | 該当エントリのみ quarantine |
| 7 | Graph Validation | DAG 性・ノード制約・エッジ制約の検証 | 該当エントリのみ quarantine |
| 8 | Cross-Reference Validation | 依存関係の解決可能性・循環依存検出 | 該当エントリのみ quarantine (baked 間は fatal) |
| 9 | Policy Validation | 名前空間予約違反・CapabilityFamily 制約 | 該当エントリのみ quarantine (baked 間は fatal) |
| 10 | Accept / Reject | 全 baked エントリの受理または fatal / mutable 正常エントリの受理 + quarantine 一覧の診断ログ出力 | 部分的可動 |
| 11 | Resolve | Baked + Mutable の統合・衝突解決 | 衝突は優先ルールで解決 (fatal 回避) |
| 12 | Diagnostic Log | registry 要約・quarantine 一覧・解決後一覧の診断ログ出力 | 常に成功 |

```rust
/// v2.3-i: 起動時検証手順のエントリポイント
fn run_startup_validation(
    baked: &mut BakedPresetRegistry,
    mutable: &mut MutablePresetRegistry,
) -> ResolvedWorkflowRegistry {
    // Step 1-3: Baked の展開・検証 (boot-fatal)
    // Step 4-6: Mutable のスキャン・パース・スキーマ検証
    // Step 7-9: グラフ・クロスリファレンス・ポリシー検証
    // Step 10: Accept/Reject 判定
    // Step 11: 解決・統合
    // Step 12: 診断ログ出力
    todo!()
}
```

### 8.8 依存方向制約・名前空間予約 (v2.3-i)

**依存方向制約**

PresetWorkflow 間の依存関係は以下の方向制約に従わなければならない (MUST):

| 依存元 | 依存先 | 許可 |
|--------|--------|------|
| baked | baked | MUST (同じ registry 内の他 baked preset への依存常時許可) |
| mutable | baked | MAY (baked preset に依存する mutable preset の作成許可) |
| mutable | mutable | MAY (同じ MutableRegistry 内の他 mutable preset への依存許可) |
| baked | mutable | MUST NOT (baked preset が mutable preset に依存してはならない) |

baked → mutable 依存の禁止は、platform-critical なワークフローがユーザー設定の消失・破損により機能不全に陥ることを防ぐ設計上の必須制約である。

**名前空間予約**

以下の名前空間は BakedPresetRegistry のために予約される。MutablePresetRegistry のエントリはこれらの名前空間を使用してはならない (MUST NOT):

| 名前空間 | 用途 | 例 |
|----------|------|-----|
| `platform.*` | プラットフォーム基盤 preset | `platform.structmem.core`, `platform.corpus2skill.core` |
| `builtin.*` | ビルドイン汎用 preset | `builtin.search.default`, `builtin.training.default` |
| `system.*` | システム内部管理 preset | `system.gc.policy`, `system.event.default` |

名前空間予約違反が検出された場合、PresetValidationReason::ReservedNamespaceViolation として報告され、MutablePresetRegistry では該当エントリが quarantine される。BakedPresetRegistry では boot-fatal となる。

### 8.9 ResolvedWorkflowRegistry (v2.3-i)

ResolvedWorkflowRegistry は BakedPresetRegistry と MutablePresetRegistry の runtime 統合を提供する。以下の責務を持つ:

- **二重 registry の統合**: baked + mutable の全 PresetWorkflow を単一の解決済みビューとして提供する。
- **名前空間衝突解決**: baked → mutable の優先順位で衝突を解決する。baked が同名を持つ場合、mutable 側はエラーとせず警告ログを出力した上で無視する。
- **Source provenance 追跡**: 各 PresetWorkflow が BakedPlatform / MutableUser / MutableWorkspace のいずれに由来するかを追跡する。
- **依存方向検証**: §8.8 の依存方向制約に違反する登録を禁止する。

```rust
/// v2.3-i: Baked + Mutable の runtime 統合
struct ResolvedWorkflowRegistry {
    baked:  BakedPresetRegistry,
    mutable: MutablePresetRegistry,
}

impl ResolvedWorkflowRegistry {
    /// ワークフローを解決。baked が優先される
    fn resolve(&self, workflow_id: &str) -> Option<&PresetWorkflow> {
        // baked → mutable の順で探索
        self.baked.get(workflow_id)
            .or_else(|| self.mutable.presets().iter()
                .find(|p| p.metadata.workflow_id == workflow_id))
    }

    /// 全解決済みワークフロー一覧
    fn all_resolved(&self) -> Vec<&PresetWorkflow> {
        self.baked.presets.iter()
            .chain(self.mutable.presets().iter())
            .collect()
    }

    /// Provenance 追跡
    fn source_of(&self, workflow_id: &str) -> Option<RegistrySource> {
        if self.baked.get(workflow_id).is_some() {
            Some(RegistrySource::BakedPlatform)
        } else if self.mutable.presets().iter().any(|p| p.metadata.workflow_id == workflow_id) {
            self.mutable.presets().iter()
                .find(|p| p.metadata.workflow_id == workflow_id)
                .map(|_| RegistrySource::MutableUser)  // 簡略化
        } else {
            None
        }
    }
}
```

### 8.10 JSON Preset Schema 例示 (v2.3-i)

PresetWorkflow の authoring / interchange format として JSON を利用する場合、以下のフィールドを最低限含めること (normative)。名称は実装ごとに調整してよいが、意味論は維持すること。

```json
{
  "workflowid": "platform.structmem.consolidate.v1",
  "kind": "PresetWorkflow",
  "preset_source": "baked",
  "preset_scope": "platform",
  "preset_trust_class": "trusted",
  "boot_critical": true,
  "immutable_root": true,
  "root_pinned": true,
  "depends_on": ["platform.structmem.fragment_ingest.v1"],
  "knowledge_capability": "StructMem",
  "version": "1",
  "graph": { "...": "existing WorkflowGraph JSON schema" }
}
```

上記のフィールドが表現する意味論:

| フィールド | 必須 | 意味 |
|-----------|------|------|
| `workflowid` | MUST | 一意のワークフロー識別子。名前空間予約規則に従う |
| `preset_source` | MUST | `baked` (BakedPresetRegistry) / `mutable` (MutablePresetRegistry) の別 |
| `preset_scope` | MUST | 名前空間スコープ: `platform` / `builtin` / `system` / `user` / `workspace` / `org` |
| `preset_trust_class` | MUST | `trusted` (baked default) / `untrusted` (mutable default) |
| `boot_critical` | MUST | `true` の場合、起動時検証失敗は boot-fatal |
| `immutable_root` | MUST | `true` の場合、runtime での変更を禁止 |
| `root_pinned` | SHOULD | `true` の場合、GC からの保護対象 |
| `depends_on` | SHOULD | 依存する PresetWorkflow の workflowid 一覧 |
| `knowledge_capability` | SHOULD | `StructMem` / `Corpus2Skill` / `Search` / `Training` / `General` |
| `version` | SHOULD | preset schema version |
| `graph` | MUST | 既存の WorkflowGraph JSON schema に従う graph 本体 |

この JSON format は authoring / interchange / distribution のための表現であり、runtime の正本は BakedPresetRegistry (binary-embedded) または MutablePresetRegistry (validated IR) である。JSON file 自体は起動時検証 (§8.7) を通過するまでは runtime registry に含まれない。

---

## 9. WorkflowDesignText / QueryDesignText

### 9.1 基本原則

v2.3-h では、Darvium の検索はミッション優先で構造基盤型（mission-first and structure-grounded）である。Semantic retrieval が意図的に適合する候補を絞り込み、最上階 WorkflowGraph に対する構造検索（metadata pruning、cheap GED pruning、full GED ranking）が構造的適合性を評価する。下位 SubWorkflow グラフは実行および洗練のための資産であり、第一次検索対象ではない。

各 `MemoizedGraph` は canonical で replayable かつ auditable な構造記述として `WorkflowDesignText` を保持する。構造類似検索の主手段は最上階 WorkflowGraph に対する GED 系検索である。

WorkflowDesignText と QueryDesignText は、トップレベルのワークフローの意図と構造を記述する、正準的で再実行可能かつ監査可能なテキスト記述である。
本リビジョンにおいて、これらは単体では主要な構造的検索メトリクスを定義してはならない。
主要な構造的検索は、トップレベルの WorkflowGraph に対して、メタデータフィルタリング、軽量な GED フィルタリング、および完全な GED ランキングによって計算されなければならない。

専用 `graph_embedding` フィールド、GNN encoder、または graph neural retrieval path を RFC-0001 v1.6 の実装必須要件として追加してはならない (MUST NOT)。これらは RFC-0003 以降の拡張事項である。

新しい mission に対しては、実装は `task_embedding` に加え `QueryDesignText` を生成しなければならない (MUST)。ただし `QueryDesignText` は検索用スケッチであり、完全な `WorkflowGraph` や実行計画の仕様として扱ってはならない (MUST NOT)。

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

Mission 入力からは少なくとも `mission_text`、`task_embedding`、`query_design_text` を導出する。`QueryDesignText` は coarse search sketch であり、完全な実行 workflow の代替ではないことを明示し、ノード数・深さ・分岐数に上限を設けなければならない (MUST)。

```rust
struct QueryRepresentation {
    mission_text:            String,
    task_embedding:          Vec<f32>,
    query_design_text:       String,
    design_template_version: String,
    top_query_metadata:      TopLevelQueryMetadata,  // v2.3-h: metadata filter query input
    cheap_ged_signature:     CheapGedSignature,      // v2.3-h: cheap GED query signature
}
```

query sketch 生成コストは full workflow generation より十分小さくなければならない (MUST)。同一または高類似 mission に対しては `query_design_text` のキャッシュを許可する (MAY)。

---


### 9.5 知識対応 QueryDesignText 拡張 (v1.8)

Revision v1.8 は QueryDesignText にオプションの知識対応フィールドを追加する。これらのフィールドは、ミッションが知識検索または知識変更を必要とする場合にのみ使用される。正規クエリ表現は以下を含んでもよい (MAY): `query_type`（値: `episodic`、`canonical`、`hybrid`）、`freshness_requirement`（値: `recent`、`stable`、`historical`、`mixed`）、`evidence_strictness`（値: `light`、`strict`、`audit-grade`）、`origin_trace_required: bool`、`drift_sensitivity`（値: `ignore`、`prefer-latest`、`show-history`）。

これらのフィールドは検索および評価ポリシーに影響を与える SHALL が、WorkflowGraph の構造的意味を変更してはならない (SHALL NOT)。省略時は、ランタイムはデフォルト値として `query_type = hybrid`、`freshness_requirement = mixed`、`evidence_strictness = light`、`origin_trace_required = false`、`drift_sensitivity = prefer-latest` を使用しなければならない (MUST)。

保存される `QueryRepresentation` 構造体は以下のように拡張される：

```rust
struct QueryRepresentation {
    mission_text: String,
    task_embedding: Vec<f32>,
    query_design_text: String,
    design_template_version: String,
    query_type: QueryType,
    freshness_requirement: FreshnessRequirement,
    evidence_strictness: EvidenceStrictness,
    origin_trace_required: bool,
    drift_sensitivity: DriftSensitivity,
    top_query_metadata: TopLevelQueryMetadata,   // v2.3-h
    cheap_ged_signature: CheapGedSignature,       // v2.3-h
}

enum QueryType { Episodic, Canonical, Hybrid }
enum FreshnessRequirement { Recent, Stable, Historical, Mixed }
enum EvidenceStrictness { Light, Strict, AuditGrade }
enum DriftSensitivity { Ignore, PreferLatest, ShowHistory }
```

上記の拡張は後方互換性を持つ: 任意の v1.7 クエリ表現は、上述のデフォルト値を設定することでアップグレード可能である。
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

/// v2.3-g 補足: current_virtual_clock は DarviumEventBus::current_virtual_clock() から取得しなければならない (MUST)。
/// いかなる domain code も VirtualClock 値を直接操作してはならない (MUST NOT)。
/// last_virtual_seen は Event Bus commit 時の event.virtual_clock 値によって更新される。
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
| AG-07 | structural channel: `cheap_ged_signature_version` と `ged_cost_model_version` が query / candidate 間で互換であること、または structural score を無効化可能であること (v2.3-h: design embedding 互換検査から移行) | structural channel 不整合 |

**v1.1 変更**: 旧 AG-06「Trust が 0.0 でないこと」は P-07 (cold-start 初期化の義務) と §8.2 の実装によりシステム的に保証されるため、ハードゲート規則としては削除し P-07 に統合した。AG-06 は埋め込みモデルバージョン検査 (旧 AG-05) に番号を変更。

### 10.2 DeterminismScore D (SoftMin)

```
D(G) = (−1/β) × ln( Σᵢ (wᵢ/W) × exp(−β × dᵢ) )

wᵢ = base × side_effect_multiplier
  ExternalApiWrite → ×4.0
  FileWrite        → ×2.0
  Notification     → ×1.5
  HITL Communicate  → ×3.0
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

### 10.3 ApplicabilityScore A (幾何平均 + floor, v2.3-h GED 一本化)

v2.3-h では構造類似度を design embedding cosine から **full GED 正規化類似度**へ一本化する。semantic similarity と structural similarity は以下で定義される。

**Semantic similarity (cosine):**

\[
S_{sem}(q,G)=\max\left(0,\frac{\langle e_q,e_G\rangle}{\|e_q\|\|e_G\|}\right) \tag{6}
\]

**Full GED similarity:**

\[
S_{struct}(q,G)=\exp(-\lambda\widetilde{GED}(q,G)) \tag{7}
\]

ここで \(\widetilde{GED}(q,G)\) は top-level DAG の正規化 GED である (§12 Stage 4)。

**総合類似度:**

\[
S_{total}(q,G)=\alpha S_{sem}(q,G)+(1-\alpha)S_{struct}(q,G),\quad \alpha\in[0,1] \tag{8}
\]

**ApplicabilityScore:**

```
A_workflow(G) = ∏ₖ max(vₖ, floorₖ)^αₖ

  vS = Stotal(Gq, Gᵢ)  (v2.3-h: GED-based, 式(8))
  vD = D(Gᵢ)
  vT = trust.composite(..., current_virtual_clock, last_virtual_seen)

  floorS = APPLICABILITY_FLOOR_S = 0.10
  floorD = APPLICABILITY_FLOOR_D = 0.10
  floorT = APPLICABILITY_FLOOR_T = TRUST_HARD_GATE_THRESHOLD = 0.20

  αS = APPLICABILITY_ALPHA_S = 0.40
  αD = APPLICABILITY_ALPHA_D = 0.30
  αT = APPLICABILITY_ALPHA_T = 0.30
```

数式表現:

\[
A_{workflow}(q,G)=\max(S_{total},f_S)^{\alpha_S}\max(D_G,f_D)^{\alpha_D}\max(T_G,f_T)^{\alpha_T} \tag{9}
\]

knowledge-aware 拡張が有効な場合:

\[
A_{final}(q,G)=A_{workflow}(q,G)^{\beta}\cdot K(q,G)^{1-\beta} \tag{10}
\]

推奨初期値 (calibration candidates): \(\alpha=0.45\), \(\lambda=4.0\), \(\beta=0.70\)。

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


### 11.5 知識適用性拡張 (v1.8)

Revision v1.8 は v1.7 ワークフロー適用性スコアを `A_workflow` として維持し、評価対象の候補が1つ以上の知識プリミティブを呼び出すか、知識に束縛されたエビデンス要件を宣言する場合にのみ、第2段階の知識適用性スコア `K` を追加する。知識プリミティブが存在しない場合、最終適用性は v1.7 の値と同一でなければならない (MUST)。

知識適用性は3つの有界成分から計算される: 鮮度 `F_knowledge`、バージョン整合性 `V_knowledge`、ドリフト整合性 `D_knowledge`。`F_knowledge` は `Chunk.stale`、`CanonicalDocument.valid_from/valid_to`、`MemoryConcept.status`、概念置換状態、イベント新近性減衰などのエビデンス鮮度シグナルから導出される SHALL。`V_knowledge` は、取得されたエビデンスが要求されたバージョンコンテキストまたは有効期間に一致するかを捕捉する SHALL。`D_knowledge` は、エビデンス選択がクエリドリフトポリシー（`ignore`、`prefer-latest`、`show-history`）と互換性があるかを捕捉する SHALL。

知識適用性スカラーは以下のように計算される SHALL:

\[
K = F_{knowledge}^{0.50} \cdot V_{knowledge}^{0.30} \cdot D_{knowledge}^{0.20} \tag{1}
\]

最終適用性は以下のように計算される SHALL:

\[
A_{final} = A_{workflow}^{0.70} \cdot K^{0.30} \tag{2}
\]

ランタイムは、知識適用性が有効な場合、候補選択に `A_final` を使用しなければならない (MUST)。`A_workflow` はデバッグ・リプレイ・較正のために SearchTrace に記録され続けなければならない (MUST)。

知識適用性のハードゲートは以下のように定義される:

1. `evidence_strictness = audit-grade` かつ `K < 0.30` の場合、その候補は REUSE、PATCH、COMPOSE のために選択されてはならず (MUST NOT)、SearchWorkflow は明示的な理由を伴って `NeedsHumanReview` または `AbortSearch` を発行しなければならない (MUST)。
2. `origin_trace_required = true` であり、候補が空のエビデンス集合または不完全なトレースルートを生成する場合、その候補は `A_workflow` の値にかかわらず知識適用性に失敗しなければならない (MUST)。
3. 取得された全エビデンスが古い、置換済み、要求されたバージョン期間に対して無効、または宣言されたドリフトポリシーと非互換である場合、その候補はワークフロー適用性が `APPLICABILITYTHRESHOLD` を超えていても知識非適用として扱われなければならない (MUST)。

式(1)および(2)のデフォルト較正定数は v1.8 の規範値である。将来のリビジョンで再較正してもよい (MAY) が、そのような再較正は実装ローカルのチューニングではなく、適用性モデルに対するバージョン管理された変更として扱われなければならない (MUST)。

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

### 12.2 v2.3-h: 5 ステージ検索（4 層 retrieval）

v2.3-h では旧 Dual Retrieval を廃止し、最上階 WorkflowGraph に対する 4 層検索（Semantic → Metadata → Cheap GED → Full GED）を normative 化する。

| Stage | 名称 | 主対象 | 目的 | 規範 |
|-------|------|--------|------|------|
| Stage 0 | hard gates | side effects / trust / version | 明白な非適格候補の除外 | MUST |
| Stage 1 | semantic retrieval | task_embedding | ミッション意味での coarse retrieval | MUST |
| Stage 2 | metadata filter | SQLite top-level metadata | cheap metadata による候補削減 | MUST |
| Stage 3 | cheap GED filter | top-level WorkflowGraph | lower-bound / approximate structural pruning | SHOULD, candidate count exceeds threshold のとき MUST |
| Stage 4 | full GED rerank | top-level WorkflowGraph | exact / bounded structural ranking | MUST |
| Stage 5 | applicability evaluation | A_workflow / K / trust / determinism | action decision (REUSE/PATCH/COMPOSE/NEW/ABORT) | MUST |

**v2.3-j 補足: WorkflowCache と Repository Pair の検索フロー区分:**

上記 5 ステージ検索は、論理的には Repository Pair (SQLite + LadybugDB) 上の全 MemoizedGraph を検索対象とする。WorkflowCache はその部分集合を保持する加速機構として以下の役割を担う:

1. Stage 1 (semantic retrieval) の ANN 検索は、LadybugDB 上の HNSW インデックスを主対象とし、WorkflowCache の ann_hint を hot-path の高速ヒントとして利用する (MAY)。
2. Stage 2 (metadata filter) は SQLite 上の TopLevelGraphMetadata に対して SQL フィルタとして実施される。WorkflowCache は通過候補の in-memory 高速参照を提供する。
3. Stage 3/4 (cheap GED / full GED) のグラフ構造参照が必要な場合、LadybugDB 上の WorkflowGraph 本体を参照する。
4. 検索で特定された候補 ID に対応する MemoizedGraph が WorkflowCache に存在する場合はそれを利用する。cache miss の場合は Repository Pair から lazy load し、hot candidate を WorkflowCache に昇格させてもよい (MAY)。

この区分により、WorkflowCache 単独が検索空間であるという誤解と、全件インメモリ保持が前提であるという誤解を除去する。

**Stage 0 副作用包含チェック (v1.1 変更)**:  
旧仕様の「完全一致」から「包含チェック」に変更。候補グラフの副作用セットがミッション要求副作用を包含する場合のみ通過する。

```
通過条件: mission_required.side_effects ⊆ candidate.aggregated_side_effects
```

これにより、ミッションが `writes_external_api=false` を要求する場合に `writes_external_api=true` の候補が排除されず、パッチ生成により副作用ノードを削除した形で再利用できる。

#### Stage 1: Semantic Mission Retrieval

入力 mission_text から `task_embedding` を生成し、最上階 WorkflowGraph に対応する candidate workflow 集合に対して semantic retrieval を行う。ここでの目的は、ミッション意味が大きく異なる workflow を除外することである。

\[
C_{sem}(q)=\operatorname{TopK}_{G\in\mathcal{R}} S_{sem}(q,G;task) \tag{11}
\]

- 入力: `task_embedding(q)`
- index: semantic ANN または exact cosine over task_embedding
- 出力: `C_sem(q)`
- サイズ上限: `K_SEM`

#### Stage 2: SQLite Metadata Filter

semantic 上で残った候補に対して、SQLite に保存された最上階 DAG の cheap な metadata を使ってフィルタリングする。ここではグラフ本体を Rust 側へロードせず、保存済みのメタ特徴だけで候補数を削減する。

標準 scored filter:

\[
M(q,G)=w_v\Delta_V(q,G)+w_e\Delta_E(q,G)+w_l\Delta_L(q,G)+w_p\Delta_P(q,G)+w_s\Delta_S(q,G) \tag{12}
\]

ここで

- \(\Delta_V\): node count difference normalized
- \(\Delta_E\): edge count difference normalized
- \(\Delta_L\): label histogram distance
- \(\Delta_P\): longest path / layer signature distance
- \(\Delta_S\): side effect summary mismatch penalty

`C_meta(q)` は最小 `M(q,G)` の top `K_META` とする。

- 入力: `C_sem(q)` と `TopLevelQueryMetadata(q)`
- 処理: SQLite predicate / scored filter
- 出力: `C_meta(q)`

**v2.3-i 拡張 (capability family filter):** Stage 2 の metadata filter は、各候補の `capability_family` フィールド (MemoizedGraph の top-level metadata) を追加のフィルタ条件として使用してよい (MAY)。例えば、StructMem に関連する検索クエリに対して `capability_family = StructMem` を持つ候補を優先的に残す、あるいは Training 由来の候補を通常検索から除外する等のポリシーが可能である。このフィルタは `M(q,G)` の additive penalty 項として実装し、capability 不一致にペナルティを課す scored filter とすることを推奨する。詳細な能力別フィルタリングポリシーは実装定義であり、本 RFC の規範範囲外である。

#### Stage 3: Cheap GED Filter

cheap GED lower bound を \(LB(q,G)\) とし、以下を満たす。

\[
LB(q,G) \le GED(q,G) \tag{13}
\]

cheap GED 候補集合は:

\[
C_{cheap}(q)=\{G\in C_{meta}(q)\mid LB(q,G) \le \tau_{cheap}(q)\} \tag{14}
\]

または top `K_CHEAP` 方式:

\[
C_{cheap}(q)=\operatorname{TopK}_{G\in C_{meta}(q)} -LB(q,G) \tag{15}
\]

cheap GED の構成要素:
- node/edge count lower bound
- label multiset mismatch lower bound
- topological layer mismatch lower bound
- ancestor/descendant reachability sketch mismatch lower bound
- bounded path-hash multiset mismatch lower bound

cheap GED は replayable deterministic function であり、乱数や hidden ANN 由来の近似を使ってはならない (MUST NOT)。

- 入力: `C_meta(q)` + `CheapGedSignature`
- 処理: lower-bound / approximate structural pruning
- 出力: `C_cheap(q)`
- 規範: SHOULD, ただし候補数が `CHEAPGED_ENABLE_THRESHOLD` 超過時は MUST

#### Stage 4: Full GED Rerank

full GED 候補集合について最上階 DAG に対する node alignment + edge edit cost を含む deterministic cost search を実行する。

\[
G^*_1,\dots,G^*_k = \operatorname{TopK}_{G\in C_{cheap}(q)} -GED(q,G) \tag{16}
\]

推奨 edit cost モデル:

\[
GED(q,G)=\min_{\pi\in\Pi(q,G)} \Bigg(\sum_{u\in V_q} c_V(u,\pi(u)) + \sum_{e\in E_q} c_E(e,\pi(e)) + c_{ins/del}(\pi)\Bigg) \tag{17}
\]

ノード置換コスト:

\[
c_V(u,v)=\eta_k \mathbf{1}[kind(u)\ne kind(v)] + \eta_a(1-J_A(u,v)) + \eta_i(1-J_I(u,v)) + \eta_o(1-J_O(u,v)) + \eta_d|det(u)-det(v)| \tag{18}
\]

ここで \(J_A\): agent/tag set Jaccard、\(J_I\): input type set Jaccard、\(J_O\): output type set Jaccard。

エッジ置換コスト:

\[
c_E(e,f)=\eta_t\mathbf{1}[type(e)\ne type(f)] + \eta_b\mathbf{1}[branch(e)\ne branch(f)] \tag{19}
\]

ノード削除・挿入は定数コスト、ただし side effect を持つノードは高コスト:

\[
c_{del}(u)=\delta_0 + \delta_{se}\cdot risk(u),\qquad c_{ins}(v)=\iota_0 + \iota_{se}\cdot risk(v) \tag{20}
\]

- 入力: `C_cheap(q)` + top-level WorkflowGraph
- 処理: exact / bounded structural ranking
- 出力: top-K_FULL ranked candidates
- 規範: MUST

#### Stage 5: Applicability Evaluation

候補 workflow ごとに、semantic 類似、GED 類似、DeterminismScore、TrustProfile、および必要時には Knowledge Applicability を統合し、最終的な REUSE / PATCH / COMPOSE / NEW / ABORT 判断へ接続する。詳細は §11.3 式(8)-(10) を参照。

**推奨値**: `K_SEM = 20`, `K_META = 50`, `K_CHEAP = 20`, `K_FULL = 10`。評価コストに応じて独立調整してよいが、主仕様は 4 層 retrieval + applicability decision とする。

### 11.3 類似度統合式と GED 境界スムージング（v2.3-h）

**v2.3-h 変更**: 本節の structural path は旧 `workflow_design_embedding` から **top-level DAG の full GED 正規化類似度**へ移行された。専用 `graph_embedding` cosine・GNN reranker・graph encoder 学習は本 RFC の規範対象外であり、SearchWorkflow からも呼び出してはならない (MUST NOT)。類似度の定義詳細は §11.3 式(6)-(8) を参照すること。

```
Stotal(q, G) = α × Ssem(q, G) + (1 − α) × Sstruct(q, G)   (v2.3-h: α = 0.45, §11.3 式(8))

Ssem    = cosine(task_embedding_q, task_embedding_G)           (§11.3 式(6))
Sstruct = exp(−λ × ~GED(q, G))                                 (§11.3 式(7))
          top-level DAG 正規化 GED、cheap GED は直接の構成要素ではない
```

**v1.4 方針 (v2.3-h 補足)**: graph embedding cosine への切替は削除済み。v2.3-h では cheap GED (Stage 3) と full GED (Stage 4) が分離された。大規模グラフでは `GED_GRAPH_SIZE_LIMIT` を超えた場合に `GraphNeedsAbstraction` として自己抽象化パスへ送る。

```rust
enum StructuralMatch {
    CheapGedScore(f32),              // v2.3-h: Stage 3 cheap GED lower bound
    FullGedScore(f32),               // v2.3-h: Stage 4 full GED ranked score
    GraphNeedsAbstraction { candidates: Vec<AbstractableSubgraph> },
}
```

### 12.3A GED 近似アルゴリズム選択方針 (v2.3-h 補足)

GED は NP 困難であるため、本 RFC は近似使用を前提とする。v2.3-h では cheap GED (Stage 3) と full GED (Stage 4) の責務を明確に分離する。

- **Cheap GED (Stage 3)**: full node alignment を含まない lower-bound または replayable approximation。候補数が `CHEAPGED_ENABLE_THRESHOLD` を超える場合に必須 (MUST)。transport-based approximation またはそれと同等の assignment/OT 系近似を cheap GED の optional implementation として使用してよい (MAY)。full GED より大幅に高速でなければならない (MUST)。
- **Full GED (Stage 4)**: node alignment / edit path search を含む正規 ranking 距離。beam search 系近似または edit path 探索系近似を用いてよい (MAY)。cheap GED 通過後の候補に対して精密な構造順位付けを行う。
- **GraphNeedsAbstraction**: `GED_GRAPH_SIZE_LIMIT` を超えた時点で完全比較志向の近似を打ち切り、`GraphNeedsAbstraction` へ送らなければならない (MUST)。v2.3-h では top-level 55 node regime が通常系であり、abstraction trigger は exception path である。
- Cheap GED は pruning 専用であり、最終順位確定に単独使用してはならない (MUST NOT)。
- Full GED は top-k ranking と structural validation に使用する。

### 12.3B 推奨プロファイル（v2.3-h cheap/full 分離対応）

| プロファイル | 検索層 | 想定用途 | 推奨近似 | 目的 |
|---|---|---|---|---|
| `fast-rerank` | Stage 3 (cheap GED) | metadata 通過後の構造的粗フィルタ | transport / OT 系 | 速度優先、top-k 圧縮 |
| `balanced-validate` | Stage 4 (full GED) | cheap GED 通過後の精密再比較 | beam search 系 | 速度と精度の均衡 |
| `abstraction-trigger` | exception path | 大規模・高複雑度 graph (size gate) | size gate + subgraph extraction | GED 深追いを避け抽象化へ送る |
| `patch-audit` | post-retrieval | patch proposal の局所妥当性確認 | 局所 beam / edit path | 説明可能な差分確認 |

### 12.3C 規範要件（v2.3-h 補足）

- cheap GED と full GED の双方は **deterministic** でなければならない (MUST)。乱数や hidden ANN 由来の近似を使ってはならない (MUST NOT)。
- tie-break は `WorkflowGraphId` の安定順序で固定すること (MUST)。
- 実装は、どの近似アルゴリズムをどの profile で使用したかを `SearchTrace` または同等の replay 可能メタデータに記録することが望ましい (SHOULD)。cost model version は SearchTrace, SearchRunLog, TrainingRunLog に残すこと。
- 同一 deployment 内で GED 近似戦略を silently 変更してはならない (MUST NOT)。変更時は retrieval recall / applicability / patch quality / ranking stability への影響を replay で確認し、バージョン付き migration note を残すこと。
- cheap GED skip が発生した場合も、その理由（candidate count below threshold）を trace に残すこと。
- beam width、transport regularization、最大展開数などの細部パラメータは implementation-tunable だが、**fast-rerank (cheap GED)** / **balanced-validate (full GED)** / **abstraction-trigger** の責務分離は規範として保持することを推奨する。
- 下位 DAG を retrieval front-channel に使用してはならない (MUST NOT) が、post-selection explanation と patch proposal では参照してよい (MAY)。

### 12.3D v2.3-h 参照実装疑似コード

以下の疑似コードは 4 層 retrieval パイプライン全体と Applicability 評価の参照実装を示す。定数名は §27 の較正候補に対応する。

```rust
fn retrieve_top_level_candidates(
    q: &QueryRepresentation,
    cache: &WorkflowCache,
    pair: &RepositoryPair,
    k: usize,
) -> Vec<Candidate> {
    // Stage 1: Semantic Mission Retrieval (cache 経由、miss 時は RepositoryPair から lazy load)
    let c_sem = semantic_topk(&q.task_embedding, cache, pair, K_SEM);

    // Stage 2: SQLite Metadata Filter
    let c_meta = sqlite_metadata_filter(&q.top_query_metadata, c_sem, K_META);

    // Stage 3: Cheap GED Filter (lower-bound approximation)
    let c_cheap = if c_meta.len() > CHEAPGED_ENABLE_THRESHOLD {
        cheap_ged_filter(&q.cheap_ged_signature, c_meta, K_CHEAP)
    } else {
        c_meta
    };

    // Stage 4: Full GED Rerank (node alignment + edit path)
    let ranked = full_ged_rerank(q, c_cheap, K_FULL);
    ranked
}

fn evaluate_candidate(
    q: &QueryRepresentation,
    g: &MemoizedGraph,
    full_ged: f32,
) -> ApplicabilityOutcome {
    let s_sem = cosine(&q.task_embedding, &g.task_embedding).max(0.0);
    let s_struct = (-STRUCT_GED_LAMBDA * normalize_ged(full_ged, q, g)).exp();
    let s_total = SIMILARITY_ALPHA * s_sem + (1.0 - SIMILARITY_ALPHA) * s_struct;
    let d = g.graph.aggregate_determinism(SOFTMIN_BETA);
    let t = g.trust.composite(
        g.provenance.clone(),
        g.time_decay.clone(),
        current_virtual_clock(),
        g.last_virtual_seen,
    );
    let a_workflow = compute_applicability_score(s_total, d, t);
    if g.knowledge_applicability.is_some() {
        finalize_with_knowledge(q, g, a_workflow)
    } else {
        ApplicabilityOutcome {
            score: a_workflow,
            decision: classify(a_workflow),
        }
    }
}
```

---


## 12A. 知識プリミティブレジストリ (v1.8)

Revision v1.8 は知識アクセスプリミティブの規範的レジストリを導入する。これらのプリミティブは、AgentStep および SubWorkflow の実行を統治するのと同じ安全性・タイムアウト・決定論・監査フレームワークを通じて実行される第一級のワークフロー操作である。知識プリミティブは読み取り専用プリミティブと変更プリミティブに分割される。

### 12A.1 プリミティブ集合

初期 v1.8 レジストリは以下のプリミティブ識別子を含む SHALL。v2.3-i では各プリミティブに CapabilityFamily に基づく分類を付与する:

**StructMem 系 (記憶形成):**
- `memorygetrecentevents`   — 直近の MemoryEvent を取得 (StructMem)
- `memorygetconcepts`       — 抽象概念 (MemoryConcept) 一覧を取得 (StructMem)
- `memorygetconcepthistory` — 概念の履歴を取得 (StructMem)
- `memorytraceorigin`       — 発信元を遡及追跡 (StructMem)
- `memorypromotetodocument` — Fragment を CanonicalDocument へ昇格 (StructMem, 変更)

**Corpus2Skill 系 (技能抽出):**
- `skilllistchildren`        — SkillNode の子階層を一覧 (Corpus2Skill)
- `skillgetchunks`           — Entity の基盤 Chunk を取得 (Corpus2Skill)
- `skillexpandentities`      — SkillNode から Entity を展開 (Corpus2Skill)
- `skillbacktrack`           — Entity → Chunk の逆経路追跡 (Corpus2Skill)

**共通:**
- `kbhybridsearch`           — 知識ベース横断検索 (StructMem + Corpus2Skill)

`memorypromotetodocument` を除くすべてのプリミティブは、デフォルトで読み取り専用として扱われる SHALL。`memorypromotetodocument` は、永続的知識状態を変更する知識変更プリミティブとして扱われる SHALL。追加のプリミティブは、副作用・決定論的期待値・冪等性・エビデンス出力動作を宣言するレジストリ更新を通じてのみ、後のリビジョンで追加されてもよい (MAY)。

### 12A.2 Workflow IR 統合

ワークフローノードは、`WorkflowNode::AgentStep` メタデータへの以下の拡張を通じて知識プリミティブを宣言してもよい (MAY):

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

知識プリミティブが AgentStep にアタッチされた場合、そのステップは追加で `requires_freshness_level`、`evidence_output_type`、および冪等性クラスを宣言しなければならない (SHALL)。読み取り専用プリミティブは、基盤ストアが同一入力に対して安定したページネーションや安定したランキングを保証できない場合を除き、冪等としてマークされるべきである (SHOULD)。変更プリミティブは、明示的な操作フィンガープリントを使用して繰り返し書き込みを重複排除しない限り、非冪等としてマークされなければならない (MUST)。

### 12A.3 エビデンスバンドル契約

すべての成功した知識プリミティブ呼び出しは、制御が SearchWorkflow 評価に戻る前に、その出力を以下の契約に正規化しなければならない (SHALL):

```rust
struct KnowledgeEvidenceBundle {
    evidence_ids: Vec<String>,
    version_context: VersionContext,
    freshness_summary: FreshnessSummary,
    confidence_meta: ConfidenceMeta,
    origin_trace_ids: Vec<String>,
}
```

`evidence_ids` は、ステップ結果を正当化する知識オブジェクトの安定した識別子を含む SHALL。`version_context` は、ステップをリプレイまたは監査するために必要な有効性およびバージョンメタデータを捕捉する SHALL。`freshness_summary` は、古さフラグ・有効期間準拠・総合鮮度スコアを要約する SHALL。`confidence_meta` は、ベクトル類似度、BM25スコア、ハイブリッドスコア、ヒット数などのランキングおよび検索シグナルを要約する SHALL。`origin_trace_ids` は、トレーサビリティが要求されているか利用可能な場合、推移的発信元チェーンを含む SHALL。

### 12A.4 変更安全規則

知識変更プリミティブはレビューゲートされなければならない (MUST)。変更プリミティブは、以下のすべてが成立しない限り実行されてはならない (MUST NOT):

1. `A_final >= APPLICABILITYTHRESHOLD`.
2. `K >= 0.50`.
3. 呼び出し元ワークフローが v1.7 信頼ハードゲートを満たしていること。
4. `origin_trace_required = true` の場合、変更リクエストにトレース可能な発信元 ID を持つ非空のエビデンスバンドルが含まれていること。

いずれかの条件が失敗した場合、SearchWorkflow は `NeedsHumanReview` または `AbortSearch` に遷移しなければならず (MUST)、失敗理由を SearchTrace に記録しなければならない (MUST)。

### 12A.5 SearchTrace 拡張

SearchTrace および SearchRunLog は、知識プリミティブが有効な場合、以下のオプションフィールドで拡張される:

- `knowledge_evidence_ids: Vec<String>`
- `knowledge_version_context: Option<VersionContext>`
- `knowledge_freshness_summary: Option<FreshnessSummary>`
- `knowledge_query_mode: Option<QueryType>`
- `origin_trace_ids: Vec<String>`

これらのフィールドは追加的かつ後方互換性がある。レガシー v1.7 実行のリプレイでは、これらを空のままにしてもよい (MAY)。

**v2.3-c 補足:** 以下のプリミティブが標準的な会話メモリパスである: `memorygetrecentevents`、`memorygetconcepts`、`memorygetconcepthistory`、`memorytraceorigin`、`memorypromotetodocument`。これらのプリミティブは、会話フラグメント検索・トレースバック・正規文書への昇格のための決定論的ラッパーとして機能する。新しい会話固有のプリミティブは不要であり、既存のプリミティブ集合は、取り込み層におけるポリシー管理された分類と決定論的ゲーティングを通じて会話知識経路に対応する。`kbhybridsearch` は、会話フラグメントの意味的クロスモーダル発見のために追加で使用されてもよい (MAY)。
### 12B. HumanChannel Communication Abstraction (v2.3-d)

#### 12B.1 動機と設計原則

Darvium における HITL (Human-In-The-Loop) は単なる通知機能ではなく、ワークフロー実行中に人間の判断が必要な場合に実行を完全待機させ、人間からの応答によって再開するための基盤抽象である。

**設計原則:**

1. **Transport 抽象化**: 通知・双方向通信・再接続を統一的に扱う `HumanChannel` トレイトを定義する。具体的な通信手段（標準入出力、WebSocket、HTTP、Slack/Teams/LINE/Email 等）は当該トレイトの実装として差し替え可能とする。
2. **ブロッキング待機**: `InteractionHandle::wait()` は OS スケジューラレベルでのブロッキングにより CPU リソースを消費せず、タイムアウト付き（`Some(dur)`）および無制限（`None`）の両方をサポートする。
3. **クラッシュ回復可能性**: 全 `HumanChannel` 実装は `reconnect()` を提供し、システム終了・再起動後も未解決のインタラクションを回復可能でなければならない (MUST)。
4. **永続化との責務分離**: `HumanChannel` は transport のみを抽象化し、ストレージへの永続化は上位レイヤー（Orchestrator）の責務とする。`MetadataStore` に 4 メソッド（store/load/list_pending/resolve）を追加して永続化を受け持つ。
5. **一貫性のある error 伝播**: reader スレッドの I/O エラー（不正 JSON、EOF、Mutex poison）は内部 `mpsc::Receiver<Result<HumanOutcome, DarviumError>>` を通じて呼び出し元に伝播される。

#### 12B.2 データ型

全 HITL データ型は `crate::types` に定義され、以下の構造を持つ。

```rust
/// 人間への依頼内容。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanRequest {
    pub subject: String,                   // 概要タイトル
    pub body: String,                      // 詳細説明
    pub context: serde_json::Value,        // 機械可読なコンテキスト情報
    pub timeout: Option<std::time::Duration>,  // 応答待機の推奨最大時間
}

/// 人間との双方向通信の結果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HumanOutcome {
    Responded(HumanResponse),
    TimedOut,
    Unreachable(String),  // interaction_id 不一致等、回復不能なプロトコルエラー
}

/// 人間からの応答内容。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanResponse {
    pub decision: HumanDecision,
    pub comment: Option<String>,
    /// RFC §13A 規範要件2に従い、人間がミッション文面を編集可能とする。
    pub revised_body: Option<String>,
}

/// 人間の判断。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HumanDecision {
    Approved,
    Rejected,
    NeedsRevision,
    Irrelevant,
    Unsafe,
}

// v2.3-g 一般化: StoredInteraction → InteractionRecord<HitlPayload> の型エイリアス。
// 後方互換のため StoredInteraction としての公開インタフェースは保持される。
pub type StoredInteraction = InteractionRecord<HitlPayload>;

/// 汎用 InteractionPayload トレイト (§12C.8 で正式定義、前方参照)。
pub trait InteractionPayload: Clone + Serialize + Deserialize {
    type Outcome: Clone + Serialize + Deserialize;
}

/// 汎用インタラクションレコード (v2.3-g 新設、§12C.8 も参照)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InteractionRecord<TPayload: InteractionPayload> {
    pub interaction_id: String,
    pub payload: TPayload,
    pub outcome: Option<TPayload::Outcome>,
    pub status: InteractionStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

/// HITL ドメインのペイロード。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HitlPayload {
    pub request: HumanRequest,
}

impl InteractionPayload for HitlPayload {
    type Outcome = HumanOutcome;
}

/// StoredInteraction 後方互換アクセサ（既存コードの変更を防ぐ）。
impl StoredInteraction {
    pub fn request(&self) -> &HumanRequest { &self.payload.request }
    pub fn outcome(&self) -> &Option<HumanOutcome> { &self.outcome }
}

/// v2.3-g 拡張: TwoWay 状態機械の全状態をカバーする 7 状態。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionStatus {
    Pending,
    AwaitingExternal,
    Resolved,
    TimedOut,
    Unreachable,
    ChannelClosed,
    Aborted,
}
```

#### 12B.3 HumanChannel トレイト

```rust
/// v2.3-g 再定義: HumanChannel は DarviumEventBus / InteractionStore の上に構築された
/// HITL-specific transport adapter である (§12C, §12D 参照)。
///
/// adapter 変換:
///   notify()       → DarviumEventKind::Hitl(HitlEvent::NotificationRequested)
///                     を InteractionMode::OneWay で EventBus::publish() する
///   communicate()  → DarviumEventKind::Hitl(HitlEvent::InteractionRequested)
///                     を InteractionMode::TwoWay で EventBus::open() する
///   reconnect()    → InteractionStore::reconnect_interaction() を呼ぶ façade
///
/// 後方互換のためトレイトメソッドシグネチャは変更しない。既存の HumanChannel 実装は
/// すべてそのままコンパイル可能であり、EventBus 統合は adapter 層として透過的に機能する。
pub trait HumanChannel: Send + Sync {
    /// 一方向通知（fire-and-forget）。
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError>;

    /// 双方向通信（応答待機）。 interaction_id（Uuid::new_v4()）を発行し、
    /// 呼び出し元に InteractionHandle を即時返却する。
    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>;

    /// 永続化された interaction_id とリクエストからインタラクションを再接続する。
    /// プロセス再起動後に呼ばれる。request は MetadataStore から復元された
    /// 元のリクエスト全文である。全実装がこのメソッドを提供しなければならない (MUST)。
    fn reconnect(&self, interaction_id: uuid::Uuid, request: &HumanRequest)
        -> Result<InteractionHandle, DarviumError>;
}
```

`reconnect()` が `request: &HumanRequest` を引数に取ることで、チャネル実装はストレージに依存せず transport のみに専念できる。MetadataStore から復元したリクエストを Orchestrator が渡す。

#### 12B.4 InteractionHandle — ブロッキング待機機構

```rust
pub struct InteractionHandle {
    pub(crate) interaction_id: uuid::Uuid,
    rx: std::sync::mpsc::Receiver<Result<HumanOutcome, DarviumError>>,
}

impl InteractionHandle {
    pub fn interaction_id(&self) -> &uuid::Uuid;

    /// 応答をブロッキング待機する。
    /// - Some(dur): recv_timeout(dur) を使用。超過で Ok(TimedOut)。
    /// - None: recv() を使用。無制限待機。
    /// - チャネル切断は Err(HumanChannelClosed) として伝播。
    /// - reader スレッドからの I/O エラーは Err(HumanChannelIo) として伝播。
    pub fn wait(self, timeout: Option<std::time::Duration>)
        -> Result<HumanOutcome, DarviumError>;
}
```

`TimedOut` はエラーではなく `HumanOutcome` の一値である。タイムアウトとプロトコルエラーは呼び出し元で異なるハンドリングが可能になる。

#### 12B.5 インタラクション状態機械

形式化のため、インタラクションの状態遷移を以下の状態機械として定義する。

**状態集合:**

```
S = { Idle, Pending, Resolved, TimedOut, Unreachable, ChannelClosed }
```

**初期状態:** `Idle`（communicate() 呼び出し前）

**状態遷移:**

```
Idle → communicate() → Pending   // interaction_id 発行、MetadataStore 書込
Pending → reader応答 → Resolved   // 応答受信、MetadataStore 更新
Pending → timeout経過 → TimedOut  // recv_timeout のタイムアウト
Pending → プロセス終了 → Idle     // クラッシュ。MetadataStore に Pending が残る
Idle → list_pending + reconnect() → Pending  // 再起動後回復
Pending → reconnect() 応答 → Resolved        // 再接続後の応答
Pending → interaction_id 不一致 → Unreachable // プロトコル誤用
Pending → mpsc 切断 → ChannelClosed          // reader スレッド異常終了
```

**不変条件:**
- `MetadataStore` 内のレコードは `Pending` または `Resolved` のみをとる。
- プロセス終了時にメモリ上の `InteractionHandle` は消失するが、`MetadataStore` の `Pending` レコードは生存する。
- `reconnect()` が成功した場合、当該 interaction_id の `MetadataStore` レコードは変更されない（更新は `resolve_human_interaction()` が行う）。

#### 12B.6 クラッシュリカバリプロトコル

プロセス再起動を超えたインタラクションの生存を保証するため、以下のプロトコルを規範とする。

**通常フロー:**
```
1. Orchestrator が channel.communicate(request) を呼ぶ
2. HumanChannel 実装が interaction_id = Uuid::new_v4() を発行
3. Orchestrator が MetadataStore.store_human_interaction() に保存 (status=Pending)
4. 人間が応答 → reader スレッドが受信 → InteractionHandle.wait() が解決
5. Orchestrator が MetadataStore.resolve_human_interaction() で更新
```

**クラッシュ後回復フロー:**
```
1. プロセス再起動
2. Orchestrator が MetadataStore.list_pending_human_interactions() を呼ぶ
3. 各 Pending レコードに対して:
   a. channel.reconnect(id, &record.request) を呼ぶ
   b. チャネル実装が request を人間に再通知
   c. handle.wait(timeout) で応答を待機
4. 応答受信後、MetadataStore.resolve_human_interaction() で更新
```

**スケール保証:** このプロトコルは以下の全シナリオで回復を保証する。

| シナリオ | 回復方法 |
|---------|---------|
| communicate() 直後に Darvium プロセスクラッシュ | MetadataStore に Pending レコード生存。list_pending → reconnect |
| StdinoutChannel で外部アプリも同時クラッシュ | MetadataStore にリクエスト全文が残っている。reconnect で再通知 |
| 再起動後に別のチャネル実装に差し替え | MetadataStore 抽象化により透過。同じ channel.reconnect() で回復。**検証条件**: 異種チャネル間（FakeHumanChannel ↔ StdinoutChannel）のクロス回復テストでこの保証を確認すること（M1-4 で検証） |
| 長時間応答なし（人間が離席中） | wait(Some(timeout)) で TimedOut 検出。エスカレーション or 再通知 |

#### 12B.7 MetadataStore 統合

`MetadataStore` トレイトに以下の 4 メソッドを追加する。

```rust
pub trait MetadataStore {
    // === 既存メソッド（store_search_trace, load_search_trace, ...）===

    // === HumanChannel インタラクション永続化 (v2.3-d) — HITL shim ===
    fn store_human_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError>;
    fn load_human_interaction(&self, interaction_id: &str) -> Result<StoredInteraction, DarviumError>;
    fn list_pending_human_interactions(&self) -> Result<Vec<StoredInteraction>, DarviumError>;
    fn resolve_human_interaction(&self, interaction_id: &str, outcome: &HumanOutcome)
        -> Result<(), DarviumError>;

    // === 汎用 Interaction API (v2.3-g, §12C.7 参照) ===
    // InteractionStore の MetadataStore 実装。
    // 既存の store_human_interaction 等は HITL-domain shim として引き続き利用可能。
    fn store_interaction(&self, record: &dyn AnyInteractionRecord)
        -> Result<(), DarviumError>;
    fn load_interaction(&self, interaction_id: &str)
        -> Result<Box<dyn AnyInteractionRecord>, DarviumError>;
    fn list_pending_interactions(&self)
        -> Result<Vec<Box<dyn AnyInteractionRecord>>, DarviumError>;
    fn resolve_interaction(&self, interaction_id: &str, outcome: &dyn AnyOutcome)
        -> Result<(), DarviumError>;
    fn abort_interaction(&self, interaction_id: &str)
        -> Result<(), DarviumError>;
    fn reconnect_interaction(&self, interaction_id: &str)
        -> Result<Box<dyn AnyInteractionRecord>, DarviumError>;
}
```

`InMemoryMetadataStore` は `HashMap<String, StoredInteraction>` でこれらを実装する。
`AnyInteractionRecord` と `AnyOutcome` は §12C.8 で定義される type-erased トレイトである。

**SQLite DDL 定義（後段チケットで実装）:**

```sql
CREATE TABLE IF NOT EXISTS human_interactions (
    interaction_id TEXT PRIMARY KEY NOT NULL,   -- UUID v4
    request_json   TEXT NOT NULL,                -- HumanRequest を JSON シリアライズ
    outcome_json   TEXT,                         -- HumanOutcome を JSON シリアライズ（Resolved 時のみ）
    status         TEXT NOT NULL DEFAULT 'Pending',  -- 'Pending' | 'Resolved'
    created_at     INTEGER NOT NULL,             -- Unix エポック秒
    updated_at     INTEGER NOT NULL              -- 最終更新時刻
);

CREATE INDEX idx_human_interactions_status ON human_interactions(status);
```

**ストア責務の住み分け:**

| ストア | 責務 | HITL との関係 |
|--------|------|-------------|
| `GraphStore` (LadybugDB) | ワークフローグラフ、埋め込みベクトル、知識オブジェクト | HITL インタラクションは知識オブジェクトではないため非対象 |
| `MetadataStore` (SQLite) | メタデータ、信頼スコア、監査ログ、Training/Fusion メタデータ | **HITL インタラクションはここに属する**。リクエスト・応答・状態はメタデータであり LadybugDB の対象ではない |

#### 12B.8 FakeHumanChannel（テスト用ダブル）

v2.3-g 補足: FakeHumanChannel は `FakeInteractionTransport` (§12C) の HITL 専用ラッパーとして再解釈される。後方互換のために FakeHumanChannel としての公開インタフェースは保持される。

```rust
/// 個別インタラクションの内部レコード。
enum InteractionRecord {
    Pending { request: HumanRequest },
    Resolved(HumanOutcome),
}

pub struct FakeHumanChannel {
    sent_count: std::sync::atomic::AtomicU64,
    requests_sent: std::sync::Mutex<Vec<HumanRequest>>,
    preloaded: std::sync::Mutex<std::collections::VecDeque<HumanOutcome>>,
    interactions: std::sync::Mutex<std::collections::HashMap<uuid::Uuid, InteractionRecord>>,
}

impl FakeHumanChannel {
    pub fn export_interactions(&self) -> Vec<StoredInteraction>;
    pub fn reset(&self);
}
```

**動作仕様:**

| メソッド | 動作 |
|---------|------|
| `notify()` | 常に `Ok(())`。`requests_sent` + `sent_count` を更新。HashMap には追加しない |
| `communicate()` | interaction_id 発行 → HashMap に Pending 保存 → プリロードキューから取り出し（空なら panic）→ Resolved 更新 → tx に即時送信 → handle 返却 |
| `reconnect(id, request)` | HashMap 検索 → 見つかれば既存 outcome を返す。見つからなければプリロードキューから取り出し（新インスタンス＝クラッシュ後回復の模擬）。キューも空なら `Err(HumanChannelIo)` |

`reconnect()` は同一インスタンス内の復旧（HashMap 参照）とプロセス再起動後の復旧（新インスタンス + プリロードキュー）の両方をカバーする設計とする。

#### 12B.9 StdinoutChannel（標準入出力 参照実装）

`StdinoutChannel` は標準入出力を用いた `HumanChannel` の具象実装であり、同一ローカルマシン上の外部アプリケーションが HITL の Human 側を担うことを可能にする。

```rust
pub struct StdinoutChannel<R, W> {
    reader: std::sync::Arc<std::sync::Mutex<R>>,
    writer: std::sync::Mutex<W>,
    session: std::sync::Mutex<()>,  // 同時呼び出し直列化
}
```

**JSON Lines プロトコル:**

```
# notify():
→ {"type":"notify","interaction_id":"xxx","request":{...}}
# （応答なし）

# communicate():
→ {"type":"communicate","interaction_id":"xxx","request":{...}}
← {"interaction_id":"xxx","outcome":{...}}

# reconnect():
→ {"type":"reconnect","interaction_id":"xxx","request":{...}}
← {"interaction_id":"xxx","outcome":{...}}
```

**アーキテクチャ上の要点:**

- reader は `Arc<Mutex<R>>` で包まれ、`communicate()` / `reconnect()` 内で別スレッドに委譲される。write（同期的）→ handle 即時返却 → read（別スレッド）→ mpsc 経由で解決、という非同期読み取りパターンを実現する。
- `session: Mutex<()>` は複数の `communicate()` / `reconnect()` が同時に呼ばれた場合の write-read 系列を直列化し、応答の取り違えを防止する。
- reader スレッド内でのエラー（EOF、不正 JSON、Mutex poison、interaction_id 不一致）は `tx.send(Err(HumanChannelIo(...)))` によって呼び出し元に伝播される。

##### 12B.9a StdinoutEventChannel 拡張 (v2.3-g)

v2.3-g では StdinoutChannel に加え、Darvium Event Bus と直接通信可能な `StdinoutEventChannel` を追加する。標準入出力を介した Event Channel プロトコルは以下の 7 種類の canonical JSON Lines メッセージから構成される。

**Canonical JSON Lines プロトコル (v2.3-g):**

```
# event.publish (OneWay event → EventBus):
→ {"type":"event.publish","event_kind":"...","payload":{...}}
# （応答なし。成功時は ack が非同期的に到着する場合がある）

# interaction.open (TwoWay interaction 開始):
→ {"type":"interaction.open","interaction_id":"xxx","event_kind":"...","payload":{...}}
← {"type":"ack","interaction_id":"xxx","status":"opened"}

# interaction.reply (TwoWay 応答):
→ {"type":"interaction.reply","interaction_id":"xxx","outcome":{...}}
← {"type":"ack","interaction_id":"xxx","status":"resolved"}

# interaction.reconnect (回復要求):
→ {"type":"interaction.reconnect","interaction_id":"xxx","event_kind":"...","payload":{...}}
← {"type":"ack","interaction_id":"xxx","status":"reconnected","outcome":{...}}

# subscribe (イベント購読):
→ {"type":"subscribe","event_kinds":["system.*","hitl.*"]}
← {"type":"ack","subscription_id":"sub_xxx"}

# error (プロトコルエラー):
← {"type":"error","code":"...","message":"..."}
```

**後方互換性:**

旧 HITL-only JSON Lines プロトコル（§12B.9）は以下のマッピングで v2.3-g プロトコルに変換される：

| 旧 type | 変換先 | 備考 |
|---------|--------|------|
| `notify` | `event.publish` + `HitlEvent::NotificationRequested` | 送信内容は同一 |
| `communicate` | `interaction.open` + `HitlEvent::InteractionRequested` | interaction_id は UUIDv4 |
| `reconnect` | `interaction.reconnect` | プロトコル上は同一 |

旧プロトコルのみを話す外部プロセスとの互換性のため、`StdinoutEventChannel` は初期化時に互換モードを選択できる (`CompatMode::Enabled / Disabled`)。

#### 12B.10 較正パラメータ (Calibration Candidates)

| 定数 | 既定値 | 意図 | 調整ガイド |
|---|---|---|---|
| `HITL_COMMUNICATE_COST_MULTIPLIER` | 3.0 | 双方向 HITL の DeterminismScore コスト係数 | **上げると** HITL を含むワークフローの決定論性スコアが低下し、再利用候補から外れやすくなる。**下げると** HITL のコスト影響が軽減されるが、人間待機が頻発する |
| `HITL_DEFAULT_TIMEOUT_SECS` | 3600 | communicate() のデフォルトタイムアウト秒数 | **小さくすると** 未応答のインタラクションが早期に TimedOut になりエスカレーションが促進される。**大きくすると** 人間の応答をより長く待つが、滞留インタラクションが増加する |
| `HITL_RECONNECT_BACKOFF_SECS` | 5.0 | reconnect 失敗時の再試行間隔 | **小さくすると** 再試行が頻繁になり負荷が増す。**大きくすると** 回復が遅延する |

#### 12B.11 観測計画 (Observation Metrics)

| 指標 | 計測方法 | 目的 |
|------|---------|------|
| インタラクション完了率 | `Resolved / (Resolved + TimedOut + Unreachable)` | チャネル健全性の基本指標 |
| タイムアウト率 | `TimedOut / total` | タイムアウト設定の妥当性評価 |
| Unreachable 率 | `Unreachable / total` | プロトコル誤用・設定ミスの検出 |
| 滞留時間分布 | `updated_at - created_at`（要約統計量: 中央値・P90・P99） | 人間の応答速度の実測値。較正パラメータの根拠データ |
| クラッシュリカバリ成功率 | `reconnect()` 成功数 / 再起動後総試行数 | 回復プロトコルの信頼性評価 |
| MetadataStore 整合性 | `list_pending()` 全件に対する reconnect 試行の成否率 | ストアとチャネルの一貫性監視 |

上記の観測指標は `FakeHumanChannel` の `AtomicU64` / `Mutex<HashMap>` および OTS (Observational Test Suite) により `println!` + `--nocapture` 経由で構造化テキスト出力される。

#### 12B.12 依存関係とモジュール構成

```
Cargo.toml: cargo add uuid@1 --features v4  （serde + serde_json は既存）

src/types.rs:                   HumanRequest, HumanOutcome, HumanResponse,
                                HumanDecision, StoredInteraction, InteractionStatus
src/human_channel.rs:           HumanChannel trait, InteractionHandle,
                                FakeHumanChannel, StdinoutChannel
src/store/metadata_store.rs:    4 メソッド追加 + InMemoryMetadataStore 実装
src/error.rs:                   2 バリアント追加 (HumanChannelIo, HumanChannelClosed)
src/lib.rs:                     pub mod human_channel; + pub use で公開
src/event_bus.rs:               DarviumEvent, DarviumEventKind, InteractionMode,
                                DarviumEventBus trait, FakeEventBus (§12C)
src/interaction_store.rs:       InteractionStore trait, InteractionRecord,
                                InMemoryInteractionStore (§12C)
src/event_channel.rs:           EventChannel trait, StdinoutEventChannel,
                                WebSocketEventChannel (§12D)
```

`StoredInteraction` が `types.rs` にあるため、`human_channel` と `metadata_store` は互いに依存せず、両方とも `types.rs` にのみ依存する。循環依存が発生しない。

#### 12B.13 M-0.5-4 実装範囲と M1 以降への委譲

| 範囲 | 本チケット (M-0.5-4) | 後続チケット (M1 以降) |
|------|---------------------|----------------------|
| HumanChannel トレイト定義 | 全実装 | — |
| InteractionHandle | 全実装 | — |
| FakeHumanChannel | 全実装 + export_interactions() | — |
| StdinoutChannel | 全実装 | — |
| MetadataStore メソッド定義 | 4 メソッド追加 + InMemory 実装 | JsonMetadataStore 簡易永続化（M1-4 で実装） |
| DDL 定義 | 設計確定 | JsonMetadataStore ファイル永続化（M1-4 で実装） |
| 起動時回復ループ（単一 Pending 擬似サイクル） | テストで擬似サイクル検証 (T10-7, T10-8) | 本格実装は M1-4 へ委譲 |
| 複数 Pending 一括回復 | 未実装（M-0.5-4 は単一のみ） | M1-4 で全件 list_pending → reconnect × N → resolve |
| StdinoutChannel クロスインスタンス回復 | 未実装（同一プロセス内のみ検証） | M1-4 でプロセス再起動越え回復を検証 |
| TimedOut 状態からの回復経路 | 未定義（状態機械に TimedOut はあるが回復経路なし） | M1-4 で再通知経路を設計・実装 |
| 回復中競合状態テスト | 未実装 | M1-4 でタイミング競合の一貫性検証 |
| WebSocketChannel / HttpChannel / etc. | Non-scope | 後段チケット |
| HumanReviewQueue | Non-scope | M1-1 |

---
## 12C Darvium Event Architecture (v2.3-g)

v2.3-g で導入される Darvium Event Architecture は、Darvium のすべての状態遷移を通過させる中心的な Event Bus 層である。§5.7 で定義された横断層として、既存の全コンポーネント（WorkflowGraph、HumanChannel、MetadataStore）と直交して動作し、監査可能性・再現可能性・外部連携の統一基盤を提供する。

### 12C.1 DarviumEvent — Canonical Envelope

すべてのイベントは以下の canonical envelope で表現しなければならない (MUST)。

```rust
pub struct DarviumEvent {
    pub event_id: EventId,              // UUIDv4
    pub kind: DarviumEventKind,         // イベント種別（下記 taxonomy）
    pub interaction_mode: InteractionMode, // OneWay / TwoWay (kind と直交, MUST)
    pub payload: serde_json::Value,     // 種別固有のペイロード
    pub causality: EventCausality,      // 因果関係情報
    pub metadata: EventMetadata,        // 経路情報・タイムスタンプ
    pub transport_meta: Option<TransportMeta>, // 外部配信制御
    pub visibility: EventVisibility,    // 購読可視性制御
    pub retention: EventRetention,      // 保持ポリシー
    pub privacy: EventPrivacy,          // PII・sandbox 制御
}

pub struct EventCausality {
    pub parent_event_id: Option<EventId>,   // 直接の原因イベント
    pub root_event_id: Option<EventId>,     // ルート原因イベント
    pub trace_ref: Option<String>,          // トレース識別子
    pub mission_id: Option<String>,         // 関連ミッション
    pub workflow_id: Option<String>,        // 関連ワークフロー
    pub run_id: Option<String>,             // 関連実行
}

pub struct EventMetadata {
    pub clock: u64,                  // commit 時の VirtualClock 値
    pub timestamp: SystemTime,       // commit 時刻 (UTC, MUST)
    pub source: EventSource,         // 発行元コンポーネント識別子
}

pub enum EventSource {
    System,
    HumanChannel,
    Orchestrator,
    External { channel_id: String },
    Test,
}

pub struct TransportMeta {
    pub delivery_mode: DeliveryMode,
    pub reply_to: Option<String>,
    pub ttl_seconds: Option<u64>,
}

pub enum DeliveryMode {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

pub enum EventVisibility {
    Public,        // 全 subscriber に可視
    Protected,     // 認証済み subscriber のみ可視
    Internal,      // EventBus 内部のみ
}

pub struct EventRetention {
    pub persist: bool,               // 永続化対象
    pub ttl_days: Option<u64>,       // 保持日数（None = 無期限）
}

pub struct EventPrivacy {
    pub contains_pii: bool,
    pub sandbox_only: bool,
    pub pii_handling: PiiHandlingPolicy, // §16B.1 参照
}

pub type EventId = String;           // UUIDv4
```

### 12C.2 DarviumEventKind — Event Taxonomy

`DarviumEventKind` は全イベント種別を列挙する enum であり、新種別の追加は additive にのみ行わなければならない (MUST)。

```rust
pub enum DarviumEventKind {
    System(SystemEvent),
    Search(SearchEvent),
    WorkflowExecution(WorkflowExecutionEvent),
    Training(TrainingEvent),
    Knowledge(KnowledgeEvent),
    Conversational(ConversationalEventEnvelope),
    Lifecycle(LifecycleEvent),
    Gc(GcEvent),
    Repair(RepairEvent),
    Reciprocity(ReciprocityEvent),
    Fusion(FusionEvent),
    Hitl(HitlEvent),
    PresetRegistry(PresetRegistryEvent),  // v2.3-i: Preset Registry 検証・登録・衝突解決
    Extension(String),                // 将来拡用 escape hatch
}

pub enum SystemEvent {
    ClockAdvanced,
    SnapshotTaken,
    ReplayCompleted,
    StartupCompleted,
}

pub enum WorkflowExecutionEvent {
    Started,
    Completed,
    Failed,
    Retried,
}

pub enum TrainingEvent {
    MissionGenerated,
    HumanReviewRequested,
    HumanReviewCompleted,
    SandboxExecutionStarted,
    SandboxExecutionCompleted,
    FeedbackIngested,
    PromotionCandidateCreated,
    PromotionApproved,
    PromotionRejected,
}

pub enum KnowledgeEvent {
    FragmentCreated,
    CandidateConsolidated,
    CanonicalPromoted,
    OriginTraceUpdated,
}

pub enum ConversationalEventEnvelope {
    UtteranceReceived,
    Classified,
    GateDecided,
    Consolidated,
    Promoted,
}

pub enum GcEvent {
    SoftDeleted,
    HardDeleteCandidate,
    Tombstoned,
}

// v2.3-k: GcEvent の具体的ペイロード例 (DarivumEventKind::Gc の payload として使用)
pub struct GraphGcStateChanged {
    pub graphid:   WorkflowGraphId,
    pub old_state: GcState,
    pub new_state: GcState,
    pub reason:    Option<String>,
}

**v2.3-k 補足 — WorkflowCache による GcEvent 購読:**

WorkflowCache は DarviumEventBus 上の `GcEvent` を subscribe しなければならない (MUST)。以下の GcState 遷移を受信した場合、preset-protected (P-18) でない限り速やかに cache eviction を試みなければならない (MUST):

- `SoftDeleted`: 非 protected エントリの cache eviction を試行する。
- `HardDeleteCandidate`: 同上。より積極的に eviction 候補とする。
- `Tombstoned`: cache からの完全除去を実行する。Tombstoned 遷移時は cache からの除去完了をもって invariant (P-19) として扱う。protected 設定より優先し、強制除去する。

これらの eviction は `WorkflowCache::handle_gc_state_transition` を用いて実装する (§8.4 参照)。

pub enum RepairEvent {
    InconsistencyDetected,
    RetryAttempted,
    TombstoneApplied,
    RepairCompleted,
}

pub enum HitlEvent {
    NotificationRequested,    // OneWay: 通知送信
    InteractionRequested,     // TwoWay: HITL インタラクション開始
    InteractionResolved,      // TwoWay: HITL 応答完了
    ChannelReconnected,       // TwoWay: チャネル再接続
}

/// v2.3-i: Preset Registry イベント
pub enum PresetRegistryEvent {
    StartupValidationStarted,      // 起動時検証開始
    StartupValidationCompleted,    // 起動時検証完了 (検証結果サマリ付き)
    PresetAccepted,                // プリセット受理
    PresetQuarantined,             // プリセット隔離 (PresetValidationFailure 付き)
    CollisionResolved,             // 名前空間衝突解決 (優先ルールの適用結果付き)
}
```

`HitlEvent` は §12B の `HumanChannel` メソッド（notify/communicate/reconnect）と一対一対応する。

### 12C.3 InteractionMode — OneWay / TwoWay

イベントの発行モードは二種類に大別される。

```rust
pub enum InteractionMode {
    OneWay,   // fire-and-forget。応答を待たない
    TwoWay,   // 応答を期待。interaction_id で追跡
}
```

| 特性 | OneWay | TwoWay |
|------|--------|--------|
| interaction_id | 不要 | 必須（UUIDv4） |
| 応答 | なし | 期待（outcome で解決） |
| 永続化 | 省略可能 | 必須（InteractionStore） |
| タイムアウト | なし | あり（`HITL_DEFAULT_TIMEOUT_SECS`） |
| 再送回数 | 0 | `<= MAX_RECONNECT_RETRIES` |
| 使用例 | 通知、ログ、メトリクス | HITL 依頼、承認フロー |

### 12C.4 TwoWay 状態機械

TwoWay インタラクションは以下の7状態の状態機械として管理される。

```
                +---> Resolved
                |
    Pending --->+---> TimedOut
                |
                +---> Unreachable
                |
                +---> ChannelClosed
                |
                +---> Aborted       (v2.3-g 追加)
```

```rust
pub enum InteractionStatus {
    Pending,           // 作成直後。未応答
    AwaitingExternal,  // 外部チャネル送信済み。応答待ち
    Resolved,          // 正常解決（outcome 確定）
    TimedOut,          // タイムアウト期限切れ
    Unreachable,       // チャネル到達不能
    ChannelClosed,     // チャネル切断
    Aborted,           // アプリケーションによる中断 (v2.3-g)
}
```

**遷移則:**
- `Pending → {AwaitingExternal, Aborted}`: EventBus::open() 直後
- `AwaitingExternal → {Resolved, TimedOut, Unreachable, ChannelClosed}`: 外部応答 or タイムアウト or エラー
- `{TimedOut, Unreachable, ChannelClosed} → AwaitingExternal`: reconnect 成功
- 終端状態: `Resolved`, `Aborted`（遷移不可）
- 上記以外の遷移は禁止 (MUST NOT)

### 12C.5 DarviumEventBus Trait

`DarviumEventBus` は全イベントの publish/管理を司る中心トレイトである。VirtualClock の管理権限は EventBus に独占される (§8.5 参照)。

```rust
#[async_trait]
pub trait DarviumEventBus: Send + Sync {
    /// OneWay イベントを publish する。VirtualClock を 1 以上進める (MUST)。
    async fn publish(&self, event: DarviumEventKind, payload: Value) -> Result<EventId>;

    /// TwoWay インタラクションを開始する。InteractionHandle を返す。
    async fn open(&self, kind: DarviumEventKind, payload: Value,
                  timeout: Option<Duration>) -> Result<InteractionHandle>;

    /// TwoWay インタラクションを解決する（outcome 確定）。
    async fn resolve(&self, interaction_id: &str, outcome: Value) -> Result<()>;

    /// TwoWay インタラクションを再接続する。
    async fn reconnect(&self, interaction_id: &str) -> Result<InteractionHandle>;

    /// イベント種別を購読する。
    async fn subscribe(&self, kinds: &[DarviumEventKind]) -> Result<Subscription>;

    /// VirtualClock 範囲でイベントをリプレイする。
    /// replay は VirtualClock を進めてはならない (MUST NOT)。
    async fn replay(&self, clock_range: Range<u64>) -> Result<Vec<DarviumEvent>>;

    /// 現在の VirtualClock 値を取得する。
    fn current_clock(&self) -> u64;

    /// 永続化失敗イベントを隔離 (quarantine) し、後続の repair に備える。
    async fn quarantine_failed_events(&self) -> Result<Vec<DarviumEvent>>;
}

pub struct InteractionHandle {
    pub interaction_id: String,
    pub rx: tokio::sync::oneshot::Receiver<Result<Value>>,
}

impl InteractionHandle {
    /// ブロッキング待機。§12B InteractionHandle.wait() と同一の意味論。
    pub async fn wait(self) -> Result<Value> {
        self.rx.await.map_err(|_| DarviumError::ChannelClosed)?
    }
}
```

### 12C.6 VirtualClock Commit Protocol

DarviumEventBus による VirtualClock 管理には以下の不変条件が課される。

| # | 規則 | 種別 |
|---|------|------|
| 1 | EventBus は commit ごとに VirtualClock を 1 以上単調増加させなければならない | MUST |
| 2 | 同一 event に対して重複 commit を行ってはならない | MUST NOT |
| 3 | replay は既存 event を再利用し、VirtualClock を再増加させてはならない | MUST NOT |
| 4 | advance_virtual_clock は EventBus 内部実装のみが呼び出せる | MUST NOT |
| 5 | VirtualClock の初期値は 0 とする | MUST |
| 6 | clock 値は commit の全順序を表現する（部分順序は認めない） | MUST |
| 7 | domain projection は event.virtual_clock (metadata.clock) を時系列の source of truth としなければならない | MUST |
| 8 | last_virtual_seen・ReciprocityEvent.virtual_clock・virtual freshness 依存ロジックは EventBus 由来の値を使用しなければならない (MUST) | MUST |

**MUST #6 の根拠:** 全順序性により、リプレイ時のイベント列が常に一意に定まる。これにより分散環境での決定論的再現が保証される。

### 12C.7 InteractionStore Trait

`InteractionStore` は TwoWay インタラクションの永続化を司るジェネリックトレイトである。InteractionRecord<TPayload> の完全な CRUD を提供する (§12B.2 参照)。

```rust
#[async_trait]
pub trait InteractionStore: Send + Sync {
    /// インタラクションを永続化する（新規作成または更新）。
    async fn store_interaction<I: InteractionPayload>(
        &self, record: &InteractionRecord<I>,
    ) -> Result<()>;

    /// interaction_id でインタラクションを読み込む。
    async fn load_interaction<I: InteractionPayload>(
        &self, interaction_id: &str,
    ) -> Result<Option<InteractionRecord<I>>>;

    /// 指定したステータスのインタラクション一覧を取得する。
    async fn list_interactions<I: InteractionPayload>(
        &self, status: Option<InteractionStatus>,
    ) -> Result<Vec<InteractionRecord<I>>>;

    /// インタラクションを Resolved として解決する。
    async fn resolve_interaction(
        &self, interaction_id: &str, outcome: Value,
    ) -> Result<()>;

    /// インタラクションを Aborted として中断する。
    async fn abort_interaction(&self, interaction_id: &str) -> Result<()>;

    /// インタラクションの再接続ステータスを更新する。
    async fn reconnect_interaction(
        &self, interaction_id: &str, new_channel_id: &str,
    ) -> Result<()>;
}
```

`InteractionRecord<TPayload>` の定義は §12B.2 を参照。`MetadataStore` は本トレイトの具象実装の一つとして統合される (§12B.7d 参照)。

### 12C.8 DarviumEventBus の MetadataStore 統合

DarviumEventBus の具象実装 `ConcreteEventBus` は MetadataStore と連携し、以下の責務を負う。

1. **イベント永続化**: commit された全 DarviumEvent を MetadataStore に追記する
2. **InteractionStore 委譲**: TwoWay インタラクションの永続化を InteractionStore（MetadataStore 実装）に委譲する
3. **クラッシュリカバリ**: 起動時に全未解決インタラクションを list_interactions(Pending | AwaitingExternal) で取得し、reconnect を試行する（§18.2 Repair Worker との連携）
4. **replay 保証**: MetadataStore に記録された DarviumEvent 列から VirtualClock 範囲でリプレイを構成する
5. **quarantine/repair**: 永続化失敗イベントを隔離し、Repair Worker（§18.2）による再試行または tombstone 化を可能にする

```rust
pub struct ConcreteEventBus<S: InteractionStore> {
    clock: Arc<Mutex<VirtualClockState>>,
    store: Arc<S>,
    subscriptions: Arc<Mutex<Vec<Subscription>>>,
}
```

### 12C.9 不変条件 (Event Architecture 健全性, 保証#11)

保証#11 は以下の3条件を定義する (§1 参照)。

- **EventBus 単一性**: 任意の時点でただ一つの DarviumEventBus インスタンスが VirtualClock を管理する (MUST)
- **全イベント通過**: すべてのドメイン状態遷移は EventBus を通じて行われなければならない (MUST)。直接的な状態変更は禁止 (MUST NOT)
- **replay 分離**: replay によるイベント再発行は VirtualClock を進めてはならず、副作用を伴ってはならない (MUST NOT)

### 12C.10 FakeEventBus (テスト用)

```rust
pub struct FakeEventBus {
    events: Arc<Mutex<Vec<DarviumEvent>>>,
    clock: Arc<Mutex<u64>>,
    interactions: Arc<Mutex<HashMap<String, InteractionStatus>>>,
}

impl FakeEventBus {
    pub fn new() -> Self;
    pub fn published_events(&self) -> Vec<DarviumEvent>;
    pub fn current_clock(&self) -> u64;
    pub fn quarantine_failed_events(&self) -> Vec<DarviumEvent>;  // fake: 常に空を返す
    pub fn reset(&self);
}
```

`FakeEventBus` は全イベントをメモリ上に記録し、外部依存なしで EventBus の動作検証を可能にする。`println!` + `--nocapture` による観測テストは本実装を介して行われる。

---
## 12D External Event Subscription (v2.3-g)

### 12D.1 EventChannel Trait

`EventChannel` は外部プロセスとのイベント送受信を抽象化するトレイトである。

```rust
#[async_trait]
pub trait EventChannel: Send + Sync {
    /// チャネルに接続する。
    async fn connect(&self) -> Result<()>;

    /// チャネルを切断する。
    async fn disconnect(&self) -> Result<()>;

    /// イベントをチャネル経由で送信する。
    async fn send(&self, event: &DarviumEvent) -> Result<()>;

    /// イベント種別を購読する。
    async fn subscribe(&self, kinds: &[DarviumEventKind]) -> Result<Subscription>;
}
```

### 12D.2 StdinoutEventChannel

`StdinoutEventChannel` は標準入出力を介した EventChannel の具象実装である。canonical JSON Lines プロトコルの詳細は §12B.9a を参照。

```rust
pub struct StdinoutEventChannel<R, W> {
    reader: Arc<Mutex<R>>,
    writer: Mutex<W>,
    compat: CompatMode,
}

pub enum CompatMode {
    Enabled,   // 旧 HITL プロトコル互換 (§12B.9)
    Disabled,  // canonical protocol のみ
}
```

**動作概要:**
- 入力行は `serde_json::from_str` でパースされ、`type` フィールドに基づいて適切な EventBus メソッドにルーティングされる
- 互換モード有効時は §12B.9a の変換マッピングに従い旧プロトコルを解釈する
- パースエラー時は `{"type":"error","code":"PARSE_ERROR","message":"..."}` を出力する

### 12D.3 WebSocketEventChannel

`WebSocketEventChannel` は WebSocket を介した EventChannel の具象実装である。標準入出力が利用できないリモートプロセスとの通信に使用する。

```rust
pub struct WebSocketEventChannel {
    url: String,
    subscription: Option<Subscription>,
}
```

**アーキテクチャ上の制約:**
- WebSocket 接続の管理は外部クレート（`tokio_tungstenite` 等）に委譲する
- 再接続ロジックは EventChannel 実装の内部で指数バックオフにより行う
- メッセージ形式は §12B.9a の canonical JSON Lines と同一とする

### 12D.4 Subscription Management

`Subscription` は購読状態を表現する構造体である。

```rust
pub struct Subscription {
    pub id: String,                      // UUIDv4
    pub kinds: Vec<DarviumEventKind>,    // 購読対象種別
    pub channel: Option<String>,         // 購読元チャネル識別子
}
```

**購読解除:** `Subscription` がドロップされた時点で暗黙的に購読が解除される。明示的な解除は `DarviumEventBus::subscribe()` に空リストを渡すことで行う。

### 12D.5 チャネル健全性

| 指標 | 計測方法 | 目的 |
|------|---------|------|
| メッセージスループット | 単位時間あたりの send 成功数 | チャネル容量監視 |
| エラー率 | `error` レスポンス数 / 総メッセージ数 | プロトコル健全性 |
| 再接続回数 | `connect()` 呼び出し頻度 | ネットワーク安定性評価 |
| 購読継続時間 | `subscription` 作成〜削除の経過時間 | 購読ライフサイクル異常検出 |

---
## 12E Event Projection Framework (v2.3-g)

### 12E.1 EventProjection Trait

`EventProjection` は DarviumEvent のストリームからドメイン固有の投影ビューを構築するためのトレイトである。

```rust
#[async_trait]
pub trait EventProjection: Send + Sync {
    /// 投影の名前（一意識別子）。
    fn name(&self) -> &'static str;

    /// 対象とする DarviumEventKind のリスト。
    /// 該当するイベントのみが project() に渡される。
    fn interested_kinds(&self) -> Vec<DarviumEventKind>;

    /// 一つのイベントを投影に取り込む。
    /// エラーは分離され、他の projection に影響を与えない (MUST)。
    async fn project(&self, event: &DarviumEvent) -> Result<(), ProjectionError>;

    /// 現在の投影状態をスナップショットとして出力する。
    async fn snapshot(&self) -> Result<serde_json::Value>;
}

pub struct ProjectionError {
    pub kind: ProjectionErrorKind,
    pub message: String,
}

pub enum ProjectionErrorKind {
    SchemaViolation,   // ペイロードスキーマ不一致
    StateConflict,     // 投影状態の不整合
    TransientIo,       // 一時的な IO エラー（リトライ可能）
    Fatal,             // 回復不能エラー（投影中断）
}
```

### 12E.2 Projection Catalog

以下は Darvium 標準で定義される投影である。

| 投影名 | 対象イベント種別 | 出力 | 用途 |
|--------|-----------------|------|------|
| `SearchTrace` | `DarviumEventKind::Search` | 検索連鎖の時系列ビュー | デバッグ・分析 |
| `ReciprocityProjection` | `DarviumEventKind::Reciprocity` | 信頼伝播の状態スナップショット | 監査 (§15.10.6) |
| `FusionTrace` | `DarviumEventKind::Fusion` | Fusion 実行履歴 | パフォーマンス分析 |
| `LifecycleLog` | `DarviumEventKind::Lifecycle` | ライフサイクルイベント一覧 | 運用監視 |
| `CacheEvictionLog` (v2.3-k) | `DarviumEventKind::Gc` + 内部 eviction trigger | cache eviction 履歴 | eviction 分析・capacity planning |

`ReciprocityProjection` は §15.10.6 の ReciprocityEvent を EventBus 経由で駆動する投影として再構成する。

### 12E.3 エラー分離原則

複数の投影が同時に稼働する際、一つの投影のエラーが他に波及してはならない (MUST NOT)。

```rust
pub struct ProjectionEngine {
    projections: Vec<Box<dyn EventProjection>>,
}

impl ProjectionEngine {
    pub async fn dispatch(&self, event: &DarviumEvent) -> Vec<ProjectionResult> {
        let mut results = Vec::new();
        for proj in &self.projections {
            if !proj.interested_kinds().contains(&event.kind) {
                continue;
            }
            match proj.project(event).await {
                Ok(()) => results.push(ProjectionResult::Ok { name: proj.name() }),
                Err(e) => {
                    // エラーを記録するが、他の投影は継続
                    results.push(ProjectionResult::Err { name: proj.name(), error: e });
                }
            }
        }
        results
    }
}
```

### 12E.4 将来拡張性

投影は以下の特性を備えて設計される：
- **追加的**: 新投影の追加は既存投影に影響を与えない (MUST)
- **リプレイ可能**: 同一イベント列からの再構築が常に可能 (MUST)
- **疎結合**: 投影間の依存関係は存在しない (MUST)

---
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
    /// Human-in-the-loop 待機が必要な場合の終端状態。
    /// 上位レイヤー（Orchestrator 等）はこの outcome を受け取り、
    /// HumanChannel::communicate() を呼び出して人間の判断を仰ぐ。
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

**§12B との層構造:** Training Orchestrator は上記の各 HITL 段階において `HumanChannel` トレイト (§12B) を下層通信抽象として利用する。`HumanChannel::communicate()` が各 review/feedback/report の双方向通信を提供し、`InteractionHandle::wait()` がブロッキング待機を実現する。`HumanDecision` 列挙子の各値は本条項の human feedback 5値と次のように対応する：Approved ↔ Good、Rejected ↔ Bad、NeedsRevision ↔ NeedsRevision、Irrelevant ↔ Irrelevant、Unsafe ↔ Unsafe。HumanChannel は transport のみを担当し、各 Training Orchestrator 段階の formal object (`TrainingMission`, `TrainingFeedback`, `PromotionCandidate`) への変換は Training Orchestrator 自身の責務である。

### 13B. Human Communication Patterns (v1.9)

v1.9 は、人間向け自然言語インタラクションを formal object に対応づけることを規範的に重視する。少なくとも次の prompt pattern を想定する。

- 「自主トレーニングとして以下のミッションを試したい。不要なミッションを削除してください。」
- 「必要であれば追加ミッションも入力してください。」
- 「優先度を変更したいものがあれば調整してください。」
- 「以下の training run を完了した。Good/Bad/NeedsRevision/Irrelevant/Unsafe を選んでください。」
- 「改善してほしい観点があれば短く追記してください。」
- 「production に昇格させたい候補があれば選んでください。」

これらは UX 表現であるが、その背後では `TrainingMission`、`TrainingFeedback`、`PromotionCandidate` などの formal object に必ず変換されなければならない (MUST)。

**HumanChannel データ型との対応:**

| 上記 prompt pattern | HumanChannel データ型 | 変換方向 |
|---|---|---|
| ミッション確認・編集・優先度調整 | `HumanRequest` (subject/body/context) + `HumanDecision::Approved/Rejected/NeedsRevision` | UX → `HumanRequest` → `HumanOutcome::Responded` → Orchestrator が解釈 |
| 訓練結果報告 | `HumanRequest` (result_report 内容) + `notify()` または `communicate()` | Orchestrator が `HumanRequest` に結果を格納 → channel 送出 |
| Good/Bad/NeedsRevision/Irrelevant/Unsafe | `HumanDecision` (5値) | `HumanResponse.decision` として受信 → `TrainingFeedback` への変換は Orchestrator 責務 |
| production 昇格候補選択 | `HumanRequest` + `HumanDecision::Approved` 等 | Orchestrator が `PromotionCandidate` を `HumanRequest.context` に格納 → channel 送出 |

これらは全て `HumanChannel` トレイトの `communicate()` または `notify()` を経由し、`InteractionHandle::wait()` によるブロッキング待機で完了する。Orchestrator は `HumanOutcome` を受け取り、対応する formal object に変換して後続処理を進める。

#### 13B.2 DarviumEventBus 変換マッピング (v2.3-g)

v2.3-g では HumanChannel の各メソッドが DarviumEventBus 上の操作に変換される。以下のマッピングは adapter 層の責務として実装される（§12C.8 参照）。

| HumanChannel メソッド | DarviumEventBus 操作 | DarviumEventKind | InteractionMode |
|-----------------------|---------------------|------------------|-----------------|
| `notify(request)` | `publish()` | `Hitl(HitlEvent::NotificationRequested)` | OneWay |
| `communicate(request)` | `open()` | `Hitl(HitlEvent::InteractionRequested)` | TwoWay |
| `reconnect(id, request)` | `reconnect()` | — | TwoWay (再接続) |
| — | `resolve()` | `Hitl(HitlEvent::InteractionResolved)` | TwoWay (完了) |

この変換により、HITL インタラクションの全ライフサイクルが EventBus を通過し、監査可能性・再現可能性が保証される。Orchestrator コードは HumanChannel トレイトに対する変更なしにこの恩恵を受ける（§16A.1 参照）。

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
    cache: &WorkflowCache,
    pair: &RepositoryPair,
    gold_id: WorkflowGraphId,
    patch: &GraphPatch,
    parent_trust: &TrustProfile,
    patch_conf: f32,
) -> Result<WorkflowGraphId, CacheError> {
    // 1. gold を読み取り、バージョンを記録 (cache → miss 時は RepositoryPair から load)
    let (gold_graph, gold_version) = cache.read_with_version(gold_id, pair).await?;
    // 2. atomic パッチ適用 (pure computation; ロック不要)
    let new_graph = apply_patch_atomic(&gold_graph, patch)
        .map_err(|_| CacheError::NotFound(gold_id))?;
    // 3. CAS 更新 (バージョン不一致なら CasConflict)
    // Gnew は新規 ID で登録するため、競合は gold への直接更新時のみ発生
    let new_id = WorkflowGraphId::new_v4();
    cache.insert_derived(new_id, new_graph, pair,
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

v1.7 では、WorkflowCache と Repository Pair の組み合わせを、単なる保存箱ではなく再利用可能資産の生態系として扱う。特に AbstractableSubgraph から生成された SubWorkflow は、局所最適化の副産物ではなく共有資産であり、検索・合成・継承・淘汰の対象として Lifecycle 管理を受けなければならない (MUST)。WorkflowCache はこの生態系への runtime access point であり、Repository Pair が資産の永続性と整合性を担保する。

GC は単純削除処理ではなく、自然淘汰として定義する。平時の長期選別と、resource pressure 下の淘汰加速を同一状態機械で扱い、瞬間的ノイズで消えないよう連続低スコア条件を持たせなければならない (MUST)。 また、SubWorkflow 資産化は無制限に行ってはならず、environment policy は 1 mission あたりの抽象化上限、最小再利用予兆、ANN index 増分上限の少なくとも 1 つを持つべきである (SHOULD)。

**v2.3-k 補足 — GcState と Cache Residency の連動:**

GcState は persistence lifecycle だけでなく、cache residency eligibility にも影響する (P-18, P-19)。`Protected` は cache eviction 完全除外とする。`SoftDeleted` / `HardDeleteCandidate` / `Tombstoned` は cache residency を縮退方向にしか遷移させてはならない (MUST NOT)。WorkflowCache は GcEvent を購読してこれらの遷移を検知し、適切な cache eviction を実行する (§8.4 `handle_gc_state_transition` 参照)。

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
R_{exp}(G)=\frac{\alpha \cdot (1-e^{-k(\alpha+\beta)})}{\alpha+\beta}
\]

ここで `α` は有益な再利用・有益 compose・正の間接寄与、`β` は有害再利用・失敗伝播・負の寄与を表す。`α + β = 0` の場合は `REPUTATION_COLD_START` を返すこと (MUST)。

### 15.4 Experience / Grace Period

各資産は `experience_count` を持つ。これは少なくとも成功実行、失敗実行、他 workflow からの再利用、Compose への寄与、Patch 親としての寄与により増加させなければならない (MUST)。

`experience_count < MIN_SURVIVAL_EXPERIENCE` の間、当該資産を `SoftDeleted` または `HardDeleteCandidate` へ遷移させてはならない (MUST NOT)。ただしセキュリティ事故・不可逆副作用・明白な破損グラフに対する緊急隔離は別扱いとし、通常 GC と混同してはならない (MUST NOT)。

**v2.3-k 補足 — Grace Period と Cache Residency の区別:**

`experience_count < MIN_SURVIVAL_EXPERIENCE` は **persistence GC 保護** であって、WorkflowCache 上の cache residency 永久保証ではない。Grace period 中の entry は `SoftDeleted` や `HardDeleteCandidate` へ遷移しない一方、cache memory pressure 時の eviction 候補から完全除外する必要はない (MAY)。すなわち、cache eviction により grace period 中の graph が WorkflowCache から消えても、Repository Pair 上には残存するため、次回の `get_or_load` で再ロード可能である。

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

- `Protected` (v2.3-i 追加)
- `Active`
- `SoftDeleted`
- `HardDeleteCandidate`
- `Tombstoned`

遷移規則は次を基準とする。

```text
Protected -- (いかなる条件でも) --X--> Active または削除状態
Protected は LifecycleScore 評価自体をスキップし、常に GC 対象外とする
Active -- L(G) < THETA_SOFT and grace-exited and consecutive_low >= N --> SoftDeleted
SoftDeleted -- L(G) >= THETA_RESTORE --> Active
SoftDeleted -- L(G) < THETA_HARD and retention_elapsed and refcount == 0 --> HardDeleteCandidate
HardDeleteCandidate -- delete/tombstone transaction success --> Tombstoned or physical delete
```

`Protected` は root preset (SystemPresetRoot) 等の GC 完全除外対象に割り当てられる。Protected への遷移は起動時検証 (§8.7) の baked registry 登録時にのみ行われ、runtime での動的遷移は認められない (MUST NOT)。`SoftDeleted` は検索候補集合から除外されるが、Repository Pair 内には残す。`HardDeleteCandidate` は lineage・SearchTrace・TrustAuditLog・SubWorkflow 参照整合性を満たすまでは物理削除してはならない (MUST NOT)。歴史参照が必要な環境では tombstone を残すことを推奨する (SHOULD)。

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

**v2.3-k 補足 — WorkflowCache Resource Pressure 観測:**

`ResourcePressure` の観測値として以下を追加しなければならない (MUST):

- `workflowcache_resident_entries`: WorkflowCache の現在エントリ数
- `workflowcache_estimated_bytes`: WorkflowCache の推定メモリ使用量
- `ann_hot_index_bytes`: AnnHotIndex の推定メモリ使用量

各 `PressureMode` における cache eviction 動作方針:

- `PressureMode::Normal`: 通常の periodic eviction を継続する。TTL ベース eviction は有効だが、通常ペースで動作する。
- `PressureMode::Constrained`: 非 protected で TTL 失効した entry の periodic cache eviction を推奨ではなく運用上の標準動作として実施する (SHOULD → 実質 MUST)。
- `PressureMode::Emergency`: protected 以外の全 TTL 失効 entry と低価値 entry (最終アクセスが長期前・experience_count が低い等) の eviction を即時実行するべきである (SHOULD)。

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

#### 15.9.1 Kind World 成立条件 (Kind World Conditions) — 5 因子最小値ゲート + レガシー診断

v2.3-f の Reciprocity-Aware Survival 拡張において、エコシステムが「協力的な生態系 (Kind World)」として成立しているかを評価するため、**5 因子最小値ゲート（§15.9.2）** を主条件とする。

\[
\text{is\_kind\_world} = (J_{kw} > 0.8) \land (\min(s_{growth}, s_{density}, s_{topology}, s_{search}, s_{fairness}) > 0.6)
\]

下記の **8 測定閾値** は旧 6 成分加重和モデルにおける Kind World 条件であり、現在は `legacy_flags` 診断用出力の計算にのみ使用される（§15.9.2 参照）。新 5 因子モデルでは代わりに 20 下位成分のノルム範囲較正（M5.1 で定義）が Kind World 条件を構成する。

**レガシー診断用 8 測定閾値 (Safety Invariants)**:

| 定数名 | 値 | 対応条件 |
|--------|-----|---------|
| `KW_MIN_POPULATION_GROWTH_RATE` | 0.01 | 最低人口成長率（1 tick あたり 1%） |
| `KW_MIN_CAPABILITY_COVERAGE_SHANNON` | 0.5 | 最小 Shannon 多様性指数 |
| `KW_MIN_REUSE_RATIO` | 0.3 | 最低再利用比率 |
| `KW_MAX_COST_EFFICIENCY_DECAY` | 0.95 | コスト効率改善比の上限（1.0 未満で単調減少） |
| `KW_MIN_VILLAGE_FORMATION_SCORE` | 0.3 | 最低村形成スコア |
| `KW_VILLAGE_CHURN_LOWER` | 0.05 | 適切な村流動性下限 |
| `KW_VILLAGE_CHURN_UPPER` | 0.30 | 適切な村流動性上限 |
| `KW_CROSS_VILLAGE_INTERACTION_MIN` | 0.1 | 最小村間相互作用率 |

**村クラスタリング定数 (Calibration Candidates)**:

| 定数名 | デフォルト値 | 感度分析推奨範囲 | 説明 |
|--------|-------------|-----------------|------|
| `VILLAGE_DISTANCE_THRESHOLD` | 0.2 | [0.1, 0.5] | 村所属判定の距離閾値 |
| `VILLAGE_MIN_SIZE` | 3 | — (Safety Invariant) | 最小村サイズ、3 未満はクラスタとみなさない |

**MagnificentSevenParams — 較正ループ sweep 対象 7 パラメータ**:

| パラメータ名 | デフォルト値 | Sweep 範囲 | 説明 |
|-------------|------------|-----------|------|
| `gamma_benevolence` | 0.15 | [0.0, 0.5] | 慈悲スコア重み |
| `lambda_gc_base` | 1.0 | [0.1, 2.0] | GC ベースハザード |
| `direct_reciprocity_weight` | 0.4 | [0.1, 0.8] | 直接互恵性重み |
| `indirect_reciprocity_weight` | 0.3 | [0.1, 0.8] | 間接互恵性重み |
| `softmax_temperature` | 0.5 | [0.1, 1.0] | ヘルパ選択のランダム性 |
| `gc_interval` | 3 | [1, 10] | GC 実行間隔（tick） |
| `child_ratio` | 0.3 | [0.1, 0.5] | 子ワークフロー比率 |

#### 15.9.2 Kind World 目的関数 (J_kw / J_kw_social) — 6 因子乗算結合モデル（状態の質 × 速度）

Kind World の成立度合いを定量化する目的関数 $J_{kw}(\theta)$ を定義する。これは Phase 3 較正の目的関数として使用される。

**$J_{kw}^{social}$ — 社会加速度目的関数（状態の質 × 時間効率）**: $J_{kw}^{social}$ は $J_{kw}$（状態の質の 5 因子乗算結合）に速度因子 $s_{speed}$ を乗じた 6 因子乗算結合として定義する。「どのような状態に到達したか」と「どれだけ速く到達したか」の両者を単一の目的関数で同時評価する。

\[
J_{kw}^{social}(\theta) = J_{kw}(\theta) \times s_{speed} = s_{growth} \times s_{density} \times s_{topology} \times s_{search} \times s_{fairness} \times s_{speed}
\]

速度因子 $s_{speed}$ は収束速度を $[0, 1]$ に正規化した値:

\[
s_{speed} = 1.0 - \frac{t_{converge}}{T_{max}}
\]

ここで $t_{converge}$ は `tick_to_convergence`（$s_{growth} \times s_{density}$ が初めて 0.8 を超えた tick 数）、$T_{max}$ は `KW4_SIMULATION_TICKS`。閾値未到達の場合は $t_{converge} = T_{max}$ となり $s_{speed} = 0$、$J_{kw}^{social} = 0$ となる。両因子の弾力性はともに 1（乗算結合の性質）であり、特定方向への偏りは生じない。

**基本構造**: 5 因子の乗算結合に速度因子 $s_{speed}$ を乗じた 6 因子乗算結合。加重和（$\sum \alpha_i x_i$）ではある因子の劣化を別の因子がマスクできるが、乗算結合では全因子が $J_{kw}^{social}$ に multiplicative に寄与する。1 因子でも 0 に近づけば全体が強く減衰するため、全 10 セクション・57 機構の捕捉が数学的に強制される。

\[
J_{kw}(\theta) = s_{growth} \times s_{density} \times s_{topology} \times s_{search} \times s_{fairness}
\]

各因子 $S \in [0, 1]$ は下位成分の算術平均。算術平均を用いることで各因子内部では平滑な勾配が得られ、Nelder-Mead 等の勾配なし最適化器が効率的に探索できる。5 因子の乗算結合であるため重み係数は不要（全因子等価）。

各因子の構成は**社会加速度定義**に対応する:

- $s_{growth}$: 「VirtualClock の進行に対して人口が増える」— 4 成分の算術平均
- $s_{density}$: 「個々のワークフローの密度が多層的（サブワークフローのネスト含む）に高くなる」— 5 成分の算術平均（密度・ネスト深度）
- $s_{topology}$: 「Darvium 空間の中で構造的クラスター係数および局所密度が増大する」— 6 成分の算術平均（協調・空間クラスタリング）
- $s_{search}$: 「新規タスクの実行に必要な探索半径と最上階推論ステップ数が減少する」— 4 成分の算術平均（効率・探索半径・推論ステップ）
- $s_{fairness}$: 構造的公平性 — 慈悲的優位のペナルティ逆数

\[
\begin{aligned}
s_{growth} &= \frac{1}{4}(j_{pop\_growth} + j_{lifecycle} + j_{child\_survival} + j_{freshness}) \\
s_{density} &= \frac{1}{5}(j_{cov} + j_{diffusion} + j_{reuse} + j_{nest\_depth} + j_{node\_density}) \\
s_{topology} &= \frac{1}{6}(j_{benevolence} + j_{reciprocity} + j_{help} + j_{trust} + j_{clustering} + j_{local\_density}) \\
s_{search} &= \frac{1}{4}(j_{cost} + j_{execution} + j_{search\_radius\_inv} + j_{reasoning\_steps\_inv}) \\
s_{fairness} &= 1.0 - j_{penalty}
\end{aligned}
\]

各下位成分の定義（全 20 成分）:

- $j_{pop\_growth} = \min(\text{population\_growth\_rate}, 1.0)$ — 旧 $j_{pop}$ から名称変更
- $j_{lifecycle} = \min(\text{mean\_lifecycle\_score}, 1.0)$ — 全個人の LifecycleScore $L(G)$ の集団平均
- $j_{child\_survival} = \min(\text{child\_survival\_rate}, 1.0)$ — 子供（経験不足個人）が生存する割合
- $j_{freshness} = \min(\text{mean\_freshness}, 1.0)$ — 全個人の BlendedFreshness $F_{time}$ の平均
- $j_{cov} = \min(\text{capability\_coverage}, 1.0)$ — Shannon 多様性指数で測った能力カバー率
- $j_{diffusion} = \min(\text{knowledge\_diffusion\_rate}, 1.0)$ — 村間知識拡散率
- $j_{reuse} = \min(\text{reuse\_ratio}, 1.0)$
- $j_{nest\_depth} = \min(\text{mean\_nest\_depth}, 1.0)$ — サブワークフローネスト深度の平均（社会加速度定義②に対応）
- $j_{node\_density} = \min(\text{mean\_node\_density}, 1.0)$ — グラフノード密度（KW_ACCEL_NODE_DENSITY_MAX で正規化、社会加速度定義②に対応）
- $j_{benevolence} = \min(\text{mean\_benevolence\_aggregate}, 1.0)$ — 全個人の慈悲総和 $B_i$ (F-3) の平均
- $j_{reciprocity} = \min(\text{mean\_reciprocity\_score}, 1.0)$ — 全個人の $(R^{dir} + R^{ind}) / 2$ の平均
- $j_{help} = \min(\text{help\_success\_rate}, 1.0)$ — 成功 HELP / 全 HELP
- $j_{trust} = \min(\text{trust\_inheritance\_fidelity}, 1.0)$ — 世代間信頼継承の忠実度（下記計算式）
- $j_{clustering} = \min(\text{cluster\_coefficient}, 1.0)$ — Watts-Strogatz 型大域クラスター係数（社会加速度定義③に対応）
- $j_{local\_density} = \min(\text{local\_density}, 1.0)$ — KW_ACCEL_DENSITY_RADIUS 内の近傍割合（社会加速度定義③に対応）
- $j_{cost} = \min(\text{cost\_efficiency}, 1.0)$ — cost_efficiency をそのまま正の向きで使用（高いほど効率的）。旧加重和モデルでは逆数（1.0 - cost_efficiency）として定義されていたが、乗算モデルでは全下位成分を [0,1] 正の向きに統一する。$s_{search}$ の入力。
- $j_{execution} = \min(\text{execution\_success\_rate}, 1.0)$ — 成功 step / 全 step
- $j_{search\_radius\_inv} = \min(\text{search\_radius\_inverse}, 1.0)$ — HELP セッションの探索距離の逆数（社会加速度定義④に対応）。`parse_workflow_id()` により全 ID 形式（"adult-N", "child-N", "wf-adult-N", "wf-child-N", "session-N", "nN"）からノード番号を抽出し、実 L2 距離を計算する。
- $j_{reasoning\_steps\_inv} = \min(\text{reasoning\_steps\_inverse}, 1.0)$ — compile_to_steps の出力長の逆数 $1/(1+\text{steps})$（社会加速度定義④に対応）
- $j_{penalty} = \max(0, 1.0 - \text{benevolent\_vs\_non\_benevolent\_coverage\_ratio})$ — 従来と同一の非対称ペナルティ。$s_{fairness} = 1.0 - j_{penalty}$ として s_fairness 因子に内包。

**Kind World 達成条件（旧 8 二値フラグから 5 因子最小値ゲート + 6 因子積閾値に変更）**:

\[
\text{is\_kind\_world} = (J_{kw}^{social} > 0.64) \land (\min(s_{growth}, s_{density}, s_{topology}, s_{search}, s_{fairness}) > 0.6)
\]

ここで $J_{kw}^{social} > 0.64$ は $J_{kw} > 0.8$ と $s_{speed} > 0.8$ の積に相当する。

**旧 8 二値フラグ**は較正条件からは排除し、診断用出力に格下げする。代わりに 5 因子の最小値ゲートを使用することで、全因子が最低水準を満たすことを保証する。また全下位成分（20 成分すべて）の値を診断情報として `KindWorldAssessment` に含める。

**J_trust の計算式**:

\[
J_{trust} = \frac{1}{|\mathcal{S}|} \sum_{(p,c) \in \mathcal{S}} \min\left(\frac{T_c}{T_p \cdot \gamma_{decay}}, 1.0\right)
\]

ここで $\mathcal{S}$ は全 spawn イベントの集合、$T_p$ は親の初期信頼値（operational/semantic/temporal の平均）、$T_c$ は子の初期信頼値、$\gamma_{decay}$ は `TRUST_INHERIT_DECAY` 定数（較正対象、default 0.70）。$J_{trust} \to 0$ は「信頼継承が完全に機能していない」、$J_{trust} \to 1$ は「全 spawn で完璧な継承」を示す。

#### 15.9.3 Kind World エコシステム成長指標 (Ecosystem Growth Metrics)

SocialAcceleration の下位指標として、エコシステムの成長を以下の 20 次元で計測する。5 因子乗算結合モデル（§15.9.2）の全下位成分はこのセクションで定義される測定値から計算される。これらのメトリクスは慈悲的集団と非慈悲的集団で層別集計され、比較可能でなければならない (MUST)。

**継続指標（旧 §15.9.3 から継続）:**

- `population_growth_rate`: (現在人口 - 前回人口) / max(前回人口, 1)。減少時負値、増加時正値。
- `capability_coverage`: ワークフローの能力空間 (position/experience) を 10×10 グリッドに量子化し Shannon 多様性指数 $H = -\sum p_i \log p_i$ を計算。$H_{\max} = \log(100)$ で除算し $[0, 1]$ 正規化。
- `reuse_ratio`: 同一 workflow が複数回ヘルプ提供または依頼を受けている割合 = 再利用回数 / 全インタラクション数。
- `cost_efficiency`: 1.0 - (失敗セッション数 + 放棄セッション数) / 全セッション数。1.0 に近いほど効率的。
- `benevolent_vs_non_benevolent_coverage_ratio`: 慈悲的集団 (上位 20%) の能力カバー率 / 非慈悲的集団 (下位 20%) の能力カバー率。> 1.0 で慈悲的優位を示す。$S_{fair}$ の入力。
- `knowledge_diffusion_rate`: 村間の知識 (experience) 分散の時間変化率。各村の平均 experience の標準偏差が時間とともに減少する速度。

**新規指標（5 因子モデル充足のため追加）:**

- `mean_lifecycle_score`: 全個人の LifecycleScore $L(G)$ の算術平均。$L(G)$ は freshness/success/trust/usage/reputation の幾何平均（RFC §15.3）。$s_{growth}$ の $j_{lifecycle}$ 成分の入力。
- `child_survival_rate`: 子供（`experience_count < MIN_SURVIVAL_EXPERIENCE`）の生存割合 = 生存子供数 / 全子供数（シミュレーション終了時）。$s_{growth}$ の $j_{child\_survival}$ 成分の入力。
- `mean_freshness`: 全個人の BlendedFreshness $F_{time}$ の算術平均。$F_{time} = w_H \cdot F_H + w_V \cdot F_V$（RFC §4A.9 機構 50）。$s_{growth}$ の $j_{freshness}$ 成分の入力。
- `mean_benevolence_aggregate`: 全個人の慈悲総和 $B_i$（F-3）の算術平均。$B_i = w_{dir} \cdot R^{dir} + w_{ind} \cdot R^{ind} + w_{rep} \cdot Rep_i$。$s_{topology}$ の $j_{benevolence}$ 成分の入力。
- `mean_reciprocity_score`: 全個人の平均互恵性スコア $(R^{dir} + R^{ind}) / 2$ の算術平均。$s_{topology}$ の $j_{reciprocity}$ 成分の入力。
- `help_success_rate`: 成功 HELP / 全 HELP セッション数。$s_{topology}$ の $j_{help}$ 成分の入力。
- `trust_inheritance_fidelity`: 世代間信頼継承の忠実度。§15.9.2 の $j_{trust}$ 計算式に従う。$[0, 1]$ 正規化。$s_{topology}$ の $j_{trust}$ 成分の入力。
- `execution_success_rate`: 成功実行 step / 全実行 step 数。$s_{search}$ の $j_{execution}$ 成分の入力。

**追加指標（社会加速度定義②③④充足のため新設）:**

- `mean_nest_depth`: サブワークフローのネスト深度の平均。単一グラフの SubWorkflow ノード比率で計測。$s_{density}$ の $j_{nest\_depth}$ 成分の入力。
- `mean_node_density`: グラフノード密度。KW_ACCEL_NODE_DENSITY_MAX で正規化。$s_{density}$ の $j_{node\_density}$ 成分の入力。
- `cluster_coefficient`: Watts-Strogatz 型大域クラスター係数。k-最近傍 (KW_ACCEL_K_NEAREST) 内の三角形割合。$s_{topology}$ の $j_{clustering}$ 成分の入力。
- `local_density`: KW_ACCEL_DENSITY_RADIUS 内の近傍ノード数の平均割合。$s_{topology}$ の $j_{local\_density}$ 成分の入力。
- `search_radius_inverse`: HELP セッションの探索距離の逆数 $1/(1+\text{mean\_distance})$。`parse_workflow_id()` により全 ID 形式からノード番号を抽出し、実 L2 距離から計算する。$s_{search}$ の $j_{search\_radius\_inv}$ 成分の入力。
- `reasoning_steps_inverse`: compile_to_steps の出力長の逆数 $1/(1+\text{steps})$。$s_{search}$ の $j_{reasoning\_steps\_inv}$ 成分の入力。

**全 20 指標が $[0, 1]$ 範囲に正規化されなければならない (MUST)。** NaN または Inf が発生した場合、該当指標は 0.0 として扱う（安全側への倒し込み）。

#### 15.9.4 村間相互作用指標 (Village Interaction Metrics)

村は「空間的近接性に基づく自律的クラスタ」として形成され (`assign_village_ids`、DBSCAN 類似の空間クラスタリング)、村間の適切な相互作用と知識拡散がエコシステム全体の健全性の指標となる。

- `cross_village_interaction_rate`: 異なる村 ID 間で発生したヘルプセッションの割合 = 村間セッション数 / 全セッション数。
- `village_formation_strength`: silhouette 類似スコア。各ワークフローの position と所属村の重心との距離の逆数平均。$[0, 1]$ 正規化。
- `knowledge_diffusion_rate`: 村間の知識 (experience) 分散の時間変化率。各村の平均 experience の標準偏差が時間とともに減少する速度。
- `village_flow_balance`: 村 churn 率 = 村間移動ワークフロー数 / 全生存ワークフロー数。適正範囲 $[KW\_VILLAGE\_CHURN\_LOWER, KW\_VILLAGE\_CHURN\_UPPER]$。範囲外はペナルティ対象。
- `compute_village_health_score(formation_strength, flow_balance, cross_rate, diffusion_rate) -> f64`: 4 指標を合成して $[0, 1]$ の総合健全性スコア = (formation_strength + flow_balance_health + cross_rate + diffusion_rate) / 4。flow_balance_health は churn が適正範囲内なら 1.0、範囲外なら 0.0。この出力は診断用メトリクスとして記録される（J_kw の直接成分ではない。村指標は J_diffusion($s_{density}$) を通じて間接的に J_kw に寄与する）。

### 15.10 Reciprocity-Aware Survival (v2.3-f)

v2.3-f は v2.3-e の LifecycleScore L(G) と GC 状態遷移を保持したまま、互恵性と協力行動が生存確率に正の影響を与える数理モデルを追加する。本拡張は既存の L(G) 定義、GC 遷移規則、Grace Period、Resource Pressure を変更せず、拡張項を additive に追加する (MUST NOT modify existing definitions)。

#### 15.10.1 Design principle

Darvium は単なる性能淘汰系ではなく、**協力的な ecosystem を選好する人工生態系**である。workflow の生存は、成功率・鮮度・使用度のみならず、**他者への貢献・直接互恵・間接互恵・支援実績・優しさの評判**に依存しなければならない (MUST)。child support village における HELP 成功、他者の成熟促進、支援の受諾率、裏切りの少なさは、将来の再利用・評判・生存保護へ接続される。本 RFC の normative intent は **benevolent cooperation is evolutionarily rewarded** である。

#### 15.10.2 Reciprocity contribution decomposition

##### Direct Reciprocity score

workflow i の直接互恵性スコアを次で定義する。

\[
R_i^{\mathrm{dir}} = \sigma\left(
\sum_{j \neq i}
\omega_{ij}^{\mathrm{dir}}
\left(
\alpha_h H_{ij}
+ \alpha_{hs} HS_{ij}
- \alpha_r RJ_{ij}
- \alpha_d DMG_{ij}
\right)
\exp(-\rho_{dir} \Delta t_{ij})
\right) \tag{F-1}
\]

ここで:
- \(H_{ij}\): workflow i が j に対して help offer / execution を行った回数または強度。
- \(HS_{ij}\): その支援が HelpSuccess に至った回数または強度。
- \(RJ_{ij}\): 一度 accepted した支援を途中で破綻させた、または期待された協力を返さなかった回数。
- \(DMG_{ij}\): 他者に負担や失敗を押し付けた harmful interaction の強度。
- \(\Delta t_{ij}\): 最終相互作用からの Human Time または Virtual Time に基づく経過量。
- \(\rho_{dir}\): 直接互恵性の時間減衰係数 (Calibration Candidate)。
- \(\sigma\): 値域を \([0,1]\) に押し込む logistic または calibrated sigmoid。

**Normative constraint**: \(\alpha_h, \alpha_{hs} > 0\)、\(\alpha_r, \alpha_d > 0\)。協力行為は \(R_i^{\mathrm{dir}}\) を非減少にし、裏切り・害は非増加にしなければならない (MUST)。

##### Indirect Reciprocity score

workflow i の間接互恵性スコアは、HELP network 上の global benevolence として次で定義する。

\[
R_i^{\mathrm{ind}} = \sigma\left(
\beta_1 C_i^{\mathrm{help}}
+ \beta_2 A_i^{\mathrm{village}}
+ \beta_3 U_i^{\mathrm{accepted}}
+ \beta_4 Q_i^{\mathrm{success}}
- \beta_5 B_i^{\mathrm{harm}}
\right) \tag{F-2}
\]

ここで:
- \(C_i^{\mathrm{help}}\): helper network 上の中心性。PageRank、eigenvector centrality、または weighted in/out degree を採用してよい。
- \(A_i^{\mathrm{village}}\): local village 内で child support に安定参加した回数・重み。
- \(U_i^{\mathrm{accepted}}\): offer が child に accept された率。
- \(Q_i^{\mathrm{success}}\): 実支援が child の mission success に寄与した率。
- \(B_i^{\mathrm{harm}}\): rejection / abandonment / harmful mismatch による負評価。

**Intent**: direct reciprocity は「相手と自分の関係」、indirect reciprocity は「社会全体から見た善良さ」を表す。v2.3-f では両者を分離したまま保持し、最終評判へ統合する。

##### Benevolence aggregate

互恵性と評判と優しさの合成量として BenevolenceScore B_i を定義する。

\[
B_i = w_{dir} R_i^{\mathrm{dir}} + w_{ind} R_i^{\mathrm{ind}} + w_{rep} \operatorname{Rep}_i \tag{F-3}
\]

ここで \(\operatorname{Rep}_i\) は ReputationProfile.final_score である。

BenevolenceScore は独立フィールドとして保存してよい。保存しない場合でも SearchTrace / Lifecycle recompute / TrainingRunLog の中間値として再現可能でなければならない (SHOULD)。

#### 15.10.3 ReputationProfile recompute with reciprocity

\[
\operatorname{Rep}_i = \operatorname{clip}_{[0,1]}\Big(
\theta_{dir} R_i^{\mathrm{dir}}
+ \theta_{ind} R_i^{\mathrm{ind}}
+ \theta_{exp} E_i^{\mathrm{norm}}
+ \theta_{inh} I_i
\Big) \tag{F-4}
\]

ここで:
- \(E_i^{\mathrm{norm}}\): experience_count を飽和正規化した値。
- \(I_i\): inherited score。
- 係数は非負であり、\(\theta_{dir} + \theta_{ind} + \theta_{exp} + \theta_{inh} = 1\) を推奨する。

Experience 正規化 (古参固定化防止):

\[
E_i^{\mathrm{norm}} = 1 - \exp(-\kappa_E \cdot \operatorname{experiencecount}(i)) \tag{F-5}
\]

**Required constraints**:
- `direct_score` と `indirect_score` の寄与は 0 であってはならない (MUST NOT) unless environment policy が明示的に village-help を無効化している場合。
- `final_score` は direct / indirect reciprocity が増加したとき、他条件一定なら非減少でなければならない (MUST)。

**Extended ReputationProfile**: v2.3-f では既存の ReputationProfile を拡張し、互恵性再計算の根拠となる観測量を保持することを推奨する。

```rust
struct ReputationProfile {
    // 既存フィールド (v2.3-e)
    direct_score:       f32,
    indirect_score:     f32,
    experience_score:   f32,
    inherited_score:    f32,
    final_score:        f32,
    alpha_positive:     u32,
    beta_negative:      u32,
    last_recomputed_at: SystemTime,
    // v2.3-f 追加フィールド
    direct_help_count:   u32,
    direct_success_count: u32,
    direct_reject_count: u32,
    harm_event_count:    u32,
    accepted_offer_rate: f32,
    help_success_rate:   f32,
    village_centrality:  f32,
    benevolence_score:   f32,
}
```

v2.3-f 追加フィールドを永続カラムとして保存しない場合でも、ReciprocityEvent から recompute 時に導出可能な event source が存在しなければならない (MUST)。

#### 15.10.4 LifecycleScore extension with benevolence

本 RFC では LifecycleScore を変更しない推奨案 B を採用する（F-6 は推奨案 A に該当する式のため欠番）。既存 LifecycleScore L(G) はそのまま維持し、GC hazard 側で benevolence を効かせる。

\[
\lambda_i^{GC} = \operatorname{softplus}\left(
\lambda_0
- \gamma_L L_i
- \gamma_B B_i
- \gamma_C C_i^{protect}
\right) \tag{F-7}
\]

ここで:
- \(\lambda_i^{GC}\): workflow i の淘汰ハザード。
- \(C_i^{protect}\): child protection / grace / support-protected term。
- softplus を使うことで常に非負。

GC 判定に使う離散確率:

\[
p_{GC}(i;\Delta t) = 1 - \exp(-\lambda_i^{GC} \Delta t) \tag{F-8}
\]

生存確率:

\[
P_{survive}(i;\Delta t)=\exp(-\lambda_i^{GC}\Delta t) \tag{F-9}
\]

**Normative monotonicity constraints**:
- \(\frac{\partial \lambda_i^{GC}}{\partial R_i^{dir}} \le 0\): 直接互恵性が高いほど淘汰ハザードは非増加。
- \(\frac{\partial \lambda_i^{GC}}{\partial R_i^{ind}} \le 0\): 間接互恵性が高いほど淘汰ハザードは非増加。
- \(\frac{\partial \lambda_i^{GC}}{\partial \operatorname{Rep}_i} \le 0\): 評判が高いほど淘汰ハザードは非増加。
- すなわち、\(R_i^{dir}, R_i^{ind}, \operatorname{Rep}_i\) の増加は \(P_{survive}\) を非減少にしなければならない (MUST)。これが Darvium の「優しい宇宙」の最も直接的な数理表現である。

#### 15.10.5 Child protection integration

既存の Grace Period (experience_count < MIN_SURVIVAL_EXPERIENCE) を保持し、benevolence を child 保護に接続する。

\[
C_i^{protect} = \eta_1 \mathbf{1}[\operatorname{Child}(i)] + \eta_2 H_i^{received} + \eta_3 G_i^{growth} \tag{F-10}
\]

- \(H_i^{received}\): child として有効支援を受けた量。
- \(G_i^{growth}\): child が maturation に向けて改善している量。

これにより「今は弱いが、助けられ、育っている child」は消されにくくなる。本項は既存の Grace Period を弱めず、補強する (MUST NOT weaken)。

#### 15.10.6 Reciprocity event log

互恵性再計算のため、Training Plane または runtime metadata に help interaction log を導入することを推奨する。

```rust
struct ReciprocityEvent {
    event_id: String,
    mission_id: String,
    source_graph_id: WorkflowGraphId,
    target_graph_id: WorkflowGraphId,
    event_kind: ReciprocityEventKind,
    weight: f32,
    created_at: SystemTime,
    virtual_clock: u64,
    trace_ref: Option<String>,
}

enum ReciprocityEventKind {
    HelpOffered,
    HelpAccepted,
    HelpRejected,
    HelpExecuted,
    HelpSucceeded,
    HelpAbandoned,
    HarmfulMismatch,
    ReturnedFavor,
}
```

**v2.3-g 補足:** ReciprocityEvent は §12E の EventProjection として再構成される。具体的には `ReciprocityProjection`（§12E.2 参照）が DarviumEventBus 上の `DarviumEventKind::Reciprocity` イベント列から ReciprocityEvent の状態を materialize する。これにより互恵性イベントの永続化・リプレイ・監査が EventBus の保証（VirtualClock 全順序、replay 分離）に統合される。従来の手動永続化（Training Plane ログ相当）は投影の一実装として存続する。

#### 15.10.7 Lifecycle calibration parameter object

```rust
struct ReciprocityLifecyclePolicy {
    theta_dir: f32,
    theta_ind: f32,
    theta_exp: f32,
    theta_inherit: f32,
    lambda_gc_base: f32,
    gamma_lifecycle: f32,
    gamma_benevolence: f32,
    gamma_child_protect: f32,
    rho_direct_decay: f32,
    tau_helper_softmax: f32,
    epsilon_remote_base: f32,
    epsilon_remote_max: f32,
    adult_experience_threshold: u32,
    adult_trust_threshold: f32,
    adult_reputation_threshold: f32,
}
```

これらは EnvironmentPolicy 参照下に置いてよい。重要なのは、versioned policy object として記録されることである。

#### 15.10.8 Multi-objective calibration objective

協力的な世界を operational にするための最終目的は単一指標ではない。multi-objective calibration を採用する。

主目的:
- 協力的・評判良好な workflow の \(P_{survive}\) を上げる。
- child support success rate を上げる。
- village churn を抑える。
- false-new rate を悪化させない。
- review-load を暴騰させない。

推奨 objective 関数:

\[
\mathcal{J}(\theta) =
\lambda_1 \cdot \operatorname{AUC}_{benevolent>nonbenevolent}
+ \lambda_2 \cdot \operatorname{HelpSuccessRate}
- \lambda_3 \cdot \operatorname{VillageChurnP95}
- \lambda_4 \cdot \operatorname{FalseNewRate}
- \lambda_5 \cdot \operatorname{ReviewLoad}
- \lambda_6 \cdot \operatorname{InstabilityPenalty} \tag{F-16}
\]

ここで \(\operatorname{AUC}_{benevolent>nonbenevolent}\) は「善良な workflow が非善良 workflow より survival ranking 上位に来る確率」を表す ranking 指標である。

#### 15.10.9 Calibration phases

v2.3-e の calibration candidate discipline、Training Plane、deterministic replay、property-based test を踏まえ、較正ループは **観測 → replay → perturbation → parameter update → regression gate** の閉ループとして定義すべきである。

**Phase 0: Pure function validation**

まず純粋関数層だけで数式 family を固定する。実装対象:
- compute_direct_reciprocity(events, now) -> f32
- compute_indirect_reciprocity(graph_metrics) -> f32
- recompute_reputation(profile_inputs) -> ReputationProfile
- compute_gc_hazard(memo, policy, now, clock) -> f32
- compute_survival_probability(hazard, delta_t) -> f32
- compute_helper_score(helper, child, mission, policy) -> f32

この段階では外部依存を一切持ち込まず、Fake-first で unit test を書く。

**Phase 1: Deterministic replay calibration**

既存の TrainingRunLog、SearchTrace、HelpOffer / HelpExecution / HelpSuccess を元に replay dataset を構成し、同一履歴で同一 score / hazard / helper ranking が出ることを保証する。手順: (1) 過去ログから reciprocity event stream を抽出、(2) policy version を固定、(3) recompute を replay、(4) ReputationProfile.final_score、BenevolenceScore、GC hazard、helper ranking をスナップショット比較。

**Phase 2: Small perturbation calibration**

v2.3 の ranking stability と oscillation risk の replay / property-based test に従い、benevolence integration も small perturbation に耐えなければならない。摂動例: help success 1 件追加、trust を 0.01 微増減、locality distance を微小変更、accepted offer を 1 件 rejected に置換、1 helper の reputation を微調整。観測: helper ranking flip rate、village churn、GC hazard drift、survival probability drift。小摂動で unbounded oscillation を起こしてはならない (MUST NOT)。

**Phase 3: Synthetic ecosystem simulation**

Training Plane の safe sandbox scope で synthetic population を走らせ、優しい世界 (Kind World, §15.9) が emergent に成立するかを検証する。目的関数として 5 因子乗算結合モデル $J_{kw}(\theta) = s_{growth} \times s_{density} \times s_{topology} \times s_{search} \times s_{fairness}$ (§15.9.2) を用い、MagnificentSevenParams (§15.9.1) を sweep 対象とする。必要な simulator: child / adult population generator、mission stream generator、locality position updater、help interaction simulator、trust / reputation recompute loop、lifecycle / gc loop。この simulator は production path を汚染せず、Training Plane または fake execution path に限定する。Kind World 較正は Phase 3 の拡張として、OFAT → grid sweep → seed 変更確認 → 統計的比較の 4 段階ループで実施する (§41C.3 M5.x)。J_kw の 5 因子各値および 20 下位成分すべてを diagnostics として記録する。

**Phase 4: Human-reviewed calibration**

最終的な係数更新は human-reviewed でなければならない。RFC の human-centered training / review queue 原則に従い、auto-update を production へ即時反映してはならない (MUST NOT)。(1) 候補係数セットを生成、(2) replay / simulation で評価、(3) 差分レポートを human review queue に送る、(4) approve 後に policy_version を更新。Kind World 較正結果 ($J_{kw}$ 5 因子内訳、5 因子最小値ゲート値、20 下位成分値、慈悲的 vs 非慈悲的比較の t 検定結果) も human review queue へのレポートに含めなければならない (MUST)。

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

### 16.4 知識認識候補評価 (v1.8)

SearchWorkflow が知識プリミティブを含む候補を評価する際、`EvaluateCandidatesStep` はワークフロー適用可能性の評価後かつ最終結果選択の前に、以下の追加サブステップを実行しなければならない (SHALL):

1. 候補に参加している各知識プリミティブノードから、正規化された `KnowledgeEvidenceBundle` を収集する。
2. 集約された証拠セットから `F_knowledge`、`V_knowledge`、`D_knowledge` を計算する。
3. §11.5 に従い `K` と `A_final` を計算する。
4. 知識ハードゲートを適用する。
5. `A_workflow`、`K`、`A_final`、エビデンス ID、バージョンコンテキスト、鮮度サマリー、発信元トレース ID を SearchTrace に永続化する。

この拡張は v1.6/v1.7 で導入された正当な SearchState 遷移を変更してはならない (SHALL NOT)。既存の `Evaluate` 状態内での候補評価を精緻化するのみである。したがって、v1.7 の有界探索、再帰ガード、安全でない遷移の拒否、決定論的リプレイに関する不変条件は引き続き有効である。

複数の候補が `A_final` で同点の場合、ランタイムはまず宣言された `evidence_strictness` のもとでエビデンス完全性が高い候補を、次に発信元トレース完全性が強い候補を、最後に低コストの候補を優先するべきである (SHOULD)。このタイブレーク順序は v1.8 の知識認識選択における規範である。

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

**HumanChannel との接続:** `HumanReviewQueuePolicy.review_timeout_secs` は `HumanRequest.timeout` の既定値として利用される。Training Orchestrator はキューから mission を取り出した際に、`HumanRequest { timeout: Some(Duration::from_secs(review_timeout_secs)), ... }` を `HumanChannel::communicate()` に渡す。これにより `InteractionHandle::wait(Some(timeout))` が `HumanOutcome::TimedOut` を返し、`escalation_timeout_secs` 経過後に再通知またはエスカレーションが発動する。

`HUMAN_REVIEW_TIMEOUT_SECS`、`HUMAN_REVIEW_ESCALATION_SECS`、`HUMAN_REVIEW_MAX_BATCH_SIZE` の推奨初期値は付録 A に記載されており、運用条件に応じて Annex E の方針に従い再キャリブレーションしてよい。

**v2.3-c 補足:** Conversational ingestion MAY be a target of the safe sandbox scope Auto-Approval Exception Policy, provided that:
- The ingested artifact remains within sandbox namespace (MUST).
- No conversational event, fragment, or candidate knowledge object may directly mutate production canonical knowledge (MUST NOT).
- Promotion auto-approval for conversational origin knowledge is prohibited (MUST NOT).
- The existing promotion discipline, trust review, origin-trace requirements, and dual-store consistency protocol apply without modification (MUST).

**v2.3-g 補足:** 本節の Training Orchestrator は HumanChannel トレイトのシグネチャ変更なしに DarviumEventBus の恩恵を受ける。具体的には、以下の透過性が保証される：

- **adapter 透過性**: Orchestrator は `HumanChannel` トレイトをそのまま使用し、内部で `ConcreteEventBus` への変換（§12C.8）が行われる。Orchestrator 側のコード変更は不要 (MUST)。
- **監査自動化**: 全 HITL インタラクションが EventBus を通過するため、Orchestrator による明示的なログ記録なしに監査証跡が生成される。
- **クラッシュリカバリ透過性**: EventBus の Repair Worker（§18.2）が未解決インタラクションを自動検出し reconnect を試行する。Orchestrator 側の回復ロジックは不要。

**v2.3-i 補足 (root preset 保護):** StructMem / Corpus2Skill の baked root preset (§8.5) は training によって変更してはならない (MUST NOT)。Training Plane での知識変異やワークフロー変異は sandbox namespace に留める。root preset から派生した descendant candidate workflow / candidate knowledge document のみが promotion 対象となる。training trust と production trust の分離は root preset に対しても維持される。root preset の lineage を祖先に持つ descendant artifact は通常の lifecycle (§15) および GC 対象に従うが、root preset 自体は GcState::Protected により保護される。

## 16B. 会話ナレッジパス (v2.3-c)

改訂 v2.3-c は、会話ナレッジパスを形式化することにより4層論理アーキテクチャを拡張する。これはポリシーに管理されたパイプラインであり、Darvium との人間の会話が、明示的な決定論的ゲート制御のもとで sandbox スコープの CandidateKnowledgeDocuments を生成し、昇格ゲートを通過した後に CanonicalDocuments を生成することを可能にする。

この拡張は厳密に追加的である。既存の §12A レジストリに新しい知識プリミティブを追加するものではなく、既存の `memorygetrecentevents`、`memorygetconcepts`、`memorygetconcepthistory`、`memorytraceorigin`、`memorypromotetodocument` プリミティブを検索および昇格のための計装として使用する。Training Plane の人間レビュー、sandbox 分離、昇格規律、デュアルストア一貫性、フュージョンセマンティクスを再定義するものではない。

会話ナレッジパスは、主要な受付機構としてトリガーフレーズに依存してはならない (SHALL NOT)。LLM ベースのポリシー条件付き分類が標準的な提案機構であり、決定論的ゲートが標準的な実施機構である。

#### アーキテクチャ概要

会話ナレッジパスは、すべての既存プレーンにまたがる垂直取り込み層を追加することにより、4層アーキテクチャを拡張する。

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

**v2.3-i StructMem 対応関係:** 会話ナレッジパスは StructMem 理論 (MemoryEvent → Fragment → MemoryConcept → CanonicalDocument) の具体化の一形態である。Conversational Ingestion Layer は StructMem の MemoryEvent 生成をポリシー駆動で自動化し、ConsolidationCandidateSet は MemoryConcept 形成の前段階として機能する。会話由来の CanonicalDocument は StructMem 理論における正準知識文書として LadybugDB に永続化される。ConversationalEvent は MemoryEvent の特殊化 (ConversationalEventKind により区別)、Fragment は両理論に共通、CandidateKnowledgeDocument は MemoryConcept 相当の中間表現として対応づけられる。

### 16B.1 会話ナレッジ取り込み (Conversational Knowledge Ingestion)

この節は会話ナレッジ取り込みのエントリポイントを形式化する。

#### 必要な型 (Required types)

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

#### 規範文書 (Normative text)

会話取り込みは、主要な受付機構としてトリガーフレーズに依存してはならない (MUST NOT)。実装は、ポリシー条件付き分類提案プロセスを通じて会話イベントを評価しなければならない (SHALL)。このプロセスでは、LLM または同等の意味推論器が、明示的な取り込みポリシーに基づいて、長期的再利用価値、カテゴリ、スコープ、時間性、プライバシーリスク、昇格適格性を評価する。

`proposed_category` が `Noise` または `Unsafe` の場合、イベントは知識変異に進んではならない (MUST NOT)。

`contains_pii` が真の場合、システムは `PiiHandlingPolicy` に従わなければならない (SHALL): `Reject` はイベントを破棄する; `RedactBeforePersist` は永続化前に正規化されたファクトをマスクすることを要求する; `AllowSandboxOnly` は sandbox スコープ内に限りマスクなしの保存を許可する。

`allow_auto_sandbox_ingest` が真の場合、その効果は安全な sandbox スコープに限定される。プロダクション正準知識への即時昇格は許可されない。

**v2.3-g 補足:** ConversationalEvent は §12C の DarviumEventBus を経由してルーティングされる。具体的な経路は以下の通り：

1. ConversationalEvent の受信時に `DarviumEventBus::publish(DarviumEventKind::Conversational, payload)` が呼ばれる（OneWay publish）。
2. EventBus は VirtualClock を進めてイベントを MetadataStore に永続化する。
3. §16B.2 以降の LLM classification / Deterministic Gate / ingestion は、EventBus 上の Conversational イベント列を入力として駆動される。
4. このルーティングにより全会話イベントに VirtualClock 順序が付与され、replay による再現が保証される。

### 16B.2 LLM駆動分類と決定論的ゲート (LLM-driven Classification and Deterministic Gate)

この節は、LLM提案と決定論的ゲートの間の責務分離を形式化する。

#### 必要な型 (Required types)

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

#### 決定手続き (Decision procedure)

以下の擬似コードは、決定論的取り込みゲートの規範的な決定手続きとして機能しなければならない (SHALL):

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

#### 編集上の要件 (Editorial requirement)

分類提案は非決定論的であってもよいが (MAY)、永続化、状態遷移、名前空間割り当て、昇格適格性、正準公開は、決定論的ゲート、監査可能な状態遷移、および既存の訓練-プロダクション分離不変条件によって統制されなければならない (SHALL)。

以下の図は、LLM の非決定論的提案役割と決定論的ゲートの実施役割の境界を示す:

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

会話から派生した知識変異は、すべて sandbox 優先でなければならない (MUST)。会話イベント、フラグメント、候補知識オブジェクトは、既存の昇格規律、信頼レビュー、発信元トレース要件、デュアルストア一貫性プロトコル (§25.x, §18.2) を通過することなく、プロダクション正準知識を直接変異させてはならない。これはハード不変条件である: 会話ナレッジパス全体は決定論的ゲートによって統制され、ゲート外のアドホックな変異経路は一切許可されない (MUST NOT)。

### 16B.3 会話 TrainingMission 構築 (Conversational TrainingMission Construction)

この節は、会話イベントから生成される TrainingMission の完全な形状を規定する。

#### 必要な型 (Required types)

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

#### 規範要件 (Normative requirements)

`MissionSource::HumanSubmitted` は会話取り込みミッションの標準ソースでなければならない (SHALL)。

会話イベントから TrainingMission を作成する行為自体は、CandidateKnowledgeDocument または CanonicalDocument を生成しない。これは会話エビデンスを Training Plane の統治下に置くだけである。

#### mission_text 生成規則 (mission_text generation convention)

以下のテンプレートは規範的でなければならない (SHALL):

```text
Consolidate the provided conversational evidence into a sandbox-scoped candidate knowledge object.
Preserve origin trace.
Do not infer beyond stated evidence.
Mark unresolved ambiguity explicitly.
Target namespace: {namespace}.
Target category: {category}.
```

#### success_criteria 要件 (success_criteria requirements)

最低限、以下の成功基準が自動設定されなければならない (SHALL):

- source_event_ids がすべて発信元トレースに保存されている。
- 各正規化ファクトがソースイベントにエビデンスアンカリングを持っている。
- あいまいさはすべて未解決として明示的にマークされている。
- 出力が sandbox 名前空間内にのみ出現する。

### 16B.4 フラグメントと候補作成 (Fragment and Candidate Creation)

この節は、会話フラグメントが Fragment および CandidateKnowledgeDocument としてどのように保存されるかを規定する。

#### ポリシー原則 (Policy principles)

- 生のトランスクリプト全文永続化はオプションである。`allow_raw_transcript_persistence` が偽の場合、正規化されたファクトとマスク済みサマリーのみが保存されなければならない (SHALL)。
- sandbox 名前空間のもとでは、会話フラグメントは LadybugDB に `Fragment` または `MemoryEvent` として保存してもよい (MAY)。
- CandidateKnowledgeDocument は sandbox 名前空間内の訓練文書として保持されなければならない (SHALL)。

#### 必要な型 (Required types)

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

#### 永続化規則 (Persistence rules)

`ConversationalFragmentMeta` は LadybugDB Fragment / MemoryEvent と結合可能でなければならない (MUST)。

`source_event_ids` は `origin_trace_ids` への昇格に適格な安定 ID として維持されなければならない (MUST)。

CandidateKnowledgeDocument が作成される際、以下のフィールドは既存の v1.9 定義 (§26 D.4) に従って設定されなければならない (SHALL): `knowledge_id`、`source_run_id`、`namespace`、`evidence_summary`、`origin_trace_ids`、`completeness_score`、`promotion_status`、`created_at`。

### 16B.5 マルチターン / マルチデイ統合ポリシー (Multi-turn / Multi-day Consolidation Policy)

この節は中核的な統合規則である。散在する会話フラグメントが単一の CandidateKnowledgeDocument にバンドルされる厳密な条件を定義する。

#### 必要な型 (Required types)

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

#### 規範的デフォルト閾値 (Normative default thresholds)

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

#### semantic_coherence 定義 (semantic_coherence definition)

`semantic_coherence` は、一連の会話フラグメントが同一の長命ファクト、プリファレンス、制約、またはプロジェクトコンテキストに属する程度 (0.0–1.0) として定義されなければならない (SHALL)。実装はこのスコアの計算に LLM 判断を使用してもよいが (MAY)、スコアの受理または却下は、ポリシー宣言された `min_semantic_coherence` に対する決定論的閾値によって決定されなければならない (SHALL)。

#### contradiction_score 安全規則 (contradiction_score safe rule)

`contradiction_score` が `max_contradiction_score` を超える候補セットは、自動的に正準化されてはならない (MUST NOT)。デフォルトの安全なアクションは以下のいずれかである:
- CandidateKnowledgeDocuments を別個の共存候補として保持する、または
- 矛盾セットを `SUPERSEDES` / `CONSOLIDATES` 候補として人間レビューキューに送信する。

破壊的マージは実行してはならない (SHALL NOT)。

以下の決定表は矛盾処理マトリックスを形式化する:

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

#### 規範的統合条件 (Normative consolidation condition)

マルチターンまたはマルチデイの会話フラグメントは、候補セットが意味的結束性、トレース完全性、時間的安定性、矛盾許容度についてポリシー宣言された閾値 (§16B.5 閾値表) を満たす場合にのみ、CandidateKnowledgeDocument に統合してもよい (MAY)。CanonicalDocument への昇格は ConversationalPromotionGate (§16B.7) を通じて別途ゲートされ続けなければならず (SHALL)、統合適格性によって暗黙的に示されるものではない。

#### ライブラリ化段階規則 (Libraryfication stage convention)

以下の4段階とそれらの段階間リネージ関係は規範的でなければならない (SHALL):

1. **ConversationalEvent** — 生の会話入力
2. **Fragment / MemoryEvent** — sandbox 名前空間下の正規化フラグメント
3. **CandidateKnowledgeDocument** — sandbox 名前空間下のバンドル候補
4. **CanonicalDocument** — 昇格された正準知識

リネージ関係:
- Event/Fragment → CandidateKnowledgeDocument: `DERIVEDFROM`
- Fragment bundle → CandidateKnowledgeDocument: `CONSOLIDATES`
- CandidateKnowledgeDocument → CanonicalDocument: `MATERIALIZEDAS`
- 置換された正準 / プリファレンス更新: `SUPERSEDES`

以下の状態遷移図は4段階パイプラインを示す:

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

### 16B.6 パーソナライゼーション名前空間規則 (Personalization Namespace Convention)

この節は、会話を通じて学習された個人知識の名前空間規則を標準化する。

#### 規範的命名規則 (Normative naming convention)

以下の形式が標準でなければならない (SHALL):

- `user/{user_id}/profile`
- `user/{user_id}/preferences`
- `user/{user_id}/projects/{project_id}`
- `user/{user_id}/history`
- `user/{user_id}/scratch`

#### 使用規則 (Usage convention)

| 名前空間 | 目的 | 昇格許可 |
|---|---|---|
| `profile` | 長期的個人属性、安定した自己記述 | 条件付き |
| `preferences` | 安定した嗜好、好み、コミュニケーション傾向 | 条件付き |
| `projects/{project_id}` | 長期間存続するプロジェクトコンテキスト、制約、ポリシー | 条件付き |
| `history` | 過去の事実記録、歴史的参照 | 通常は sandbox / レビュー必須 |
| `scratch` | 一時的なメモ、短期作業コンテキスト | 許可されない |

#### エキスパート名前空間との整合 (Expert Namespace alignment)

- ユーザー名前空間は v2.0 Expert Namespace として抽出およびフュージョン可能でなければならない (SHALL)。
- `scratch` および tombstone 化されたアーティファクトは、デフォルトでは必要な依存関係クロージャに含めてはならない (SHALL NOT)。

### 16B.7 正準文書への昇格 (Promotion to Canonical Document)

この節は、ライブラリ化の最終段階を形式化する: 会話由来の CandidateKnowledgeDocument から CanonicalDocument への昇格である。

#### ポリシー原則 (Policy principles)

- 会話由来の知識は、まず CandidateKnowledgeDocument 段階を通過しなければ CanonicalDocument になってはならない (MUST NOT)。
- `memorypromotetodocument` はこの遷移のための唯一の変異プリミティブである。昇格ゲートが満たされた後にのみ使用可能でなければならない (SHALL)。
- デュアルストア一貫性プロトコル (§25.x) は修正なしに適用される。

#### PromotionGate 型

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

#### 規範的条件 (Normative conditions)

会話由来の CandidateKnowledgeDocument は、以下のすべてが満たされた場合にのみ CanonicalDocument に昇格してもよい (MAY):

- `promotion_status = Approved`
- `completeness_score >= 0.80`
- `trace_completeness >= 0.80`
- `contradiction_score <= 0.20`
- `distinct_day_count >= 2`
- `training_good_ratio >= TRAINING_PROMOTION_MIN_GOOD_RATIO`
- `sandbox_success_rate >= TRAINING_PROMOTION_MIN_SUCCESS_RATE`
- `requires_human_review = false` または人間による承認が記録されている
- 単一の `op_id` を共有するデュアルストアコミットインテントが生成されている

既存の訓練定数 (`TRAINING_PROMOTION_MIN_GOOD_RATIO`、`TRAINING_PROMOTION_MIN_SUCCESS_RATE`) は較正候補であり、その値は会話由来の昇格に修正なしで適用される。

### 16B.8 プライバシー、保持、トゥームストーン、修復 (Privacy, Retention, Tombstone, and Repair)

この節は、会話メモリに固有の運用ルールを形式化する。

#### 必須規定 (Required provisions)

- 生の会話イベントは `RetentionPolicy` で宣言された TTL に従って期限切れになってもよい (MAY)。
- 拒否された CandidateKnowledgeDocument は、既存のトゥームストーン猶予期間 (§15 GcState) を継承しなければならない (SHALL)。
- ユーザー削除リクエストの対象となったアーティファクトは、最低限名前空間ローカルのトゥームストーンと監査ログエントリを保持しなければならず (SHALL)、通常の検索パスから除外されなければならない (MUST)。
- デュアルストア不整合に遭遇した会話アーティファクトは `NeedsRepair` または `Quarantined` に遷移しなければならず (SHALL)、通常の REUSE / PATCH / COMPOSE パスに出現してはならない (MUST NOT)。

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

### v2.3-i 補足: preset validation phase

v2.3-i では、startup repair scan に **preset validation phase** が前置される。起動時の実行順序は以下の通り:

1. **Preset validation phase (v2.3-i 新設)**: BakedPresetRegistry の展開・検証 (boot-fatal) → MutablePresetRegistry のスキャン・検証 (graceful degradation) → ResolvedWorkflowRegistry の構築 (§8.7 12段階手順)
2. **Startup repair scan (既存)**: 従来の dual-store commit intent の recovery
3. **Normal operation**: 通常の WorkflowCache + Repository Pair / PresetWorkflow 利用

Preset validation phase で検出された PresetValidationFailure は、DarviumEventKind::PresetRegistry (PresetRegistryEvent::PresetQuarantined) として Event Bus に発行されなければならない (MUST)。Boot-fatal エラー時はプロセス終了前に診断ログを標準エラー出力に出力し、DarviumEvent の発行は行われない。

**ConsistencyState との関係 (v2.3-i):** dual-store consistency で導入された `Committed / Pending / NeedsRepair / Quarantined` の運用概念は、preset ingestion の validation failure にも準用される。ただし、preset source file は LadybugDB / SQLite の repository pair object ではないため、ConsistencyState の状態機械を厳密に共有するのではなく、**運用意味論の準用**にとどめる。具体的には、rejected / quarantined な PresetValidationFailure は診断情報として記録されるが、ConsistencyState の状態遷移（Pending → NeedsRepair 等）は発生せず、recovery 対象にはならない。修復はユーザーによる preset file の修正と再起動によって行われる。

### 18.2 デュアルストア一貫性拡張 (v1.8)

リビジョン v1.8 は、知識変更経路に対するデュアルストアコミット契約を完全に規範的にする。ワークフローオーケストレーションメタデータと状態は Repository Pair (SQLite 側) に引き続き権威があり、LadybugDB は永続化された知識オブジェクトに対して権威を持ち続ける。両方のドメインを変更する操作は、共有 `opid` の下で実行されなければならず (SHALL)、以下の順序に従わなければならない (SHALL):

1. ワークフロー側の intent を書き込み、`ConsistencyState::Pending { opid, phase = MetaPrepared }` とマークする。
2. 同じ `opid` の下で知識側の intent を書き込む。
3. ワークフロー側の変更と知識側の変更を実行する。
4. 両方のコミットが成功した場合、`ConsistencyState::Committed` とマークする。
5. いずれかの準備または書き込みが発生した後に一方の側が失敗した場合、`ConsistencyState::NeedsRepair { opid, reason }` とマークし、`RepairLog` を追加し、修復のために操作をキューに入れる。

`Pending`、`NeedsRepair`、または `Quarantined` 状態のワークフローは、通常の REUSE、PATCH、または COMPOSE のために選択されてはならない (MUST NOT)。そのようなワークフローは、監査、修復、またはリプレイツールによってのみ検査されてよい (MAY)。修復ワーカーは、既存の v1.7 修復モデルに従って、retry-commit、補償的 tombstone、または quarantine を試みてもよい (MAY) が、成功したリカバリは元の `opid`、系統参照、および SearchTrace リンケージを保存しなければならない (MUST)。

ランタイムは、デュアルストアプロトコルをデータベースネイティブの XA 保証ではなく、アプリケーションレベルのコミットインテントプロトコルとして扱わなければならない (MUST)。したがって、実装は開始時修復スキャン中に中断された操作を決定論的に完了、quarantine、または tombstone するのに十分な intent と監査メタデータを保持しなければならない (MUST)。


### 18.x 異種ストア整合性とフェイルセーフ (v1.7 追補)

LadybugDB に保持される graph / embedding 系データと、SQLite に保持される Trust / Lifecycle / lineage / audit 系メタデータは、単一 ACID トランザクションではなく**論理コミット単位**として扱う。`apply_patch_atomic`、SubWorkflow 資産登録、GC 状態遷移、tombstone 化の各処理は、少なくとも `op_id` を持つ commit intent を先に生成し、両ストア更新後にのみ `consistency_state = Committed` へ遷移させなければならない (MUST)。

いずれか片側の書き込みが失敗した場合、当該資産を `NeedsRepair` または `Quarantined` に遷移させ、SearchWorkflow / RetrievalPrimitive の通常候補集合から除外しなければならない (MUST)。この隔離は runtime safety のための措置であり、通常 GC や trust 低下と混同してはならない (MUST NOT)。

```rust
/// RepositoryPair::commit_dual_store_update (§8) に委譲する。
/// デュアルストアコミットの責務主体は RepositoryPair であり、
/// WorkflowCache は commit 結果に基づいて cached MemoizedGraph の
/// consistency_state を更新する。
async fn commit_dual_store_update(
    pair: &RepositoryPair,
    graph: &mut MemoizedGraph,
    op_id: String,
) -> Result<(), PersistenceError> {
    pair.commit_dual_store_update(op_id, graph).await
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

### v2.3-k 補足 — WorkflowCache Eviction エラーハンドリング

cache eviction 関連のエラーとその扱いを以下に規定する:

- **protected graph への eviction 要求**: `CacheError::ProtectedEvictionForbidden` を hard error として返さなければならない (MUST)。このエラーは無視してはならず、発行元は即座に処理を中断し、呼び出し元にエラーを伝播しなければならない (MUST)。
- **Tombstoned graph の cache 残存**: `GcState::Tombstoned` の graph が WorkflowCache に発見された場合は invariant violation とみなし、警告出力に留めず `CacheError::EvictionInvariantViolation` を送出しなければならない (MUST)。実装は repair / panic policy の対象とするか事前に明文化すること。
- **cache eviction failure**: それ自体は persistence corruption ではない。しかし、capacity guard failure (capacity 超過時に protected のみが残り eviction 不可) により search path を degrade してよい (MAY)。degrade 時は `CacheError::CapacityExceeded` をエラーレベルで監査ログに記録しなければならない (MUST)。

## 19. 性能目標

### v2.3 補助観測指標

本節の主要性能目標に加え、RFC 準拠実装は運用品質の補助指標として、reuse quality、false-new rate、compose/new fallback frequency、repair rate、quarantine rate、rollback rate、human review queue depth、review latency、ranking stability under small patch を観測対象に含めることが望ましい (SHOULD)。これらは現時点では calibration candidate と operational metric であり、一律の固定閾値を意味しない。


| 指標 | 目標値 | 達成マイルストーン |
|------|--------|-----------------|
| LLM 呼び出し削減率 | ≥ 20% (vs ベースライン) | M2 |
| レイテンシ削減率 | ≥ 15% | M2 |
| ApplicabilityScore 適合率 (再利用後の成功率) | ≥ 95% | M2 |
| trustscore (成熟グラフ) | ≥ 0.70 | M3 |
| **cache hit rate (v2.3-k)** | ≥ 80% under normal load | M2.5 |
| **median reload latency (v2.3-k)** | ≤ 10 ms | M2.5 |
| **eviction count per hour (v2.3-k)** | 監視対象、固定閾値なし | M2.5 |
| **protected-entry eviction attempts (v2.3-k)** | 0 (ゼロ必須) | M2.5 |
| **tombstoned-entry residency duration p95 (v2.3-k)** | 0 または near-zero | M2.5 |
| **pressure-triggered eviction completion latency (v2.3-k)** | ≤ 100 ms (p95) | M3 |

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
| M -0.5 | Fake WorkflowCache / Repository Pair / embeddings | task/design dual retrieval、union rerank、ranking drift 検査、embedding version mismatch 移行テスト |
| M -0.5-4 | HITL HumanChannel 基盤 | HumanChannel トレイト定義、InteractionHandle、FakeHumanChannel、StdinoutChannel、MetadataStore HITL 永続化 4 メソッド、クラッシュリカバリプロトコル。人間との双方向通信をワークフローの命として抽象化する基盤層 (§12B) |
| M -0.5-5 | DarviumEvent canonical envelope | DarviumEvent/DarviumEventKind 型定義、InteractionMode (OneWay/TwoWay)、TwoWay 7状態機械。Event Architecture のデータモデル基盤 (§12C.1–4) |
| M -0.5-6 | DarviumEventBus + InteractionStore | DarviumEventBus トレイト (publish/open/resolve/reconnect/subscribe/replay)、InteractionStore トレイト (store/load/list/resolve/abort/reconnect)、VirtualClock commit protocol。Event Architecture の実行基盤 (§12C.5–7) |
| M -0.5-7 | External Event Channel | EventChannel トレイト、StdinoutEventChannel (canonical JSON Lines)、WebSocketEventChannel、CompatMode。Event Architecture の外部連携基盤 (§12D) |
| M -0.5-8 | Event Projection Framework | EventProjection トレイト、ProjectionEngine、標準投影 4 種 (SearchTrace/ReciprocityProjection/FusionTrace/LifecycleLog)。Event Architecture の投影基盤 (§12E) |
| M0 | Composition / New proposal 基盤 | ComposeExisting / GenerateNew proposal、lineage / invalidation / proposal validity テスト |
| M0.5 | Fake LLM adapter | scripted fake LLM、JSON schema parser、malformed output recovery、same-input same-output replay |
| M1 | Human-in-the-loop review | NeedsHumanReview、SearchTrace と TrustAuditLog / SearchRunLog の整合性、manual override |
| M1.5 | Real embedding provider | 実 embedding provider 接続、ANN recall と ranking drift 検証 |
| M2 | Limited real LLM | BuildQueryStep / RefineSearchPolicyStep のみ実 LLM 接続、schema conformance と budget overrun protection |
| M2.5 | Real query-policy evaluation | nondeterminism envelope 計測、provider latency と replay baseline 比較 |
| M3 | Real proposal generation | Compose / New / Patch proposal を実 LLM で生成し、review-gated validity を評価 |
| M4 | Real executor end-to-end | OpenFang / 実 executor を含む end-to-end。ただし unsafe side-effect path は review-gated を維持 |

**v2.3-k 補足 — WorkflowCache Eviction マイルストーン:**

WorkflowCache eviction semantics の実装は、M-0.5-7-P (WorkflowCache + RepositoryPair 型定義基盤) の完了を前提として、以下の独立タスクとして追加する。詳細は Darvium-Tickets-v2.3.md の対応チケットを参照。

| ID | 目的 | 主な内容 | 依存 |
|----|------|---------|------|
| E1 | Protected eviction guard | GcState::Protected / PresetSystem / RootPinned の eviction 除外 | M-0.5-7-P |
| E2 | Periodic eviction worker | バックグラウンド periodic worker、eviction_interval ごとの expired/pressure/capacity 評価 | E1 |
| E3 | TTL eviction semantics | Human Time + VirtualClock 二軸 TTL、preset-safe guard | E1 |
| E4 | Pressure-driven eviction | ResourcePressure + PressureMode による aggressiveness 切替 | E1 |
| E5 | GcEvent-driven eviction | GcEvent 購読、SoftDeleted/HardDeleteCandidate/Tombstoned 連動 | E1 + EventBus |
| E6 | Eviction invariants and tests | property-based test: protected never evicted, tombstoned never resident, committed reloadable | E1–E5 |

### 19.1 Legacy マイルストーン互換メモ

以下の旧マイルストーン表現は履歴参照用であり、v1.6 の正規実装計画ではない。

### 19.1 マイルストーン一覧

| ID | 名称 | 成果物 |
|----|------|--------|
| **M -1** | **ダミー層・ポート抽象化** | PortTrait 定義 + FakeImpl。OpenFang・LLM に未接続の状態でコアロジック全域をテスト可能にする |
| M0 | MVP | WorkflowGraph + compile_to_steps + WorkflowCache + Repository Pair (埋め込みなし、cold-start trust) |
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
├── workflow_cache.rs   ← WorkflowCache + CAS + cold-start + lazy load
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
    let cache = WorkflowCache::in_memory();
    let id   = cache.insert(build_simple_graph(), TrustProfile::cold_start_new())
                   .await.unwrap();
    // バージョン 0 を読み取り
    let (_, v0) = cache.read_with_version(id).await.unwrap();
    // 同バージョンで 2 回更新 → 2 回目は CasConflict
    cache.update_graph_cas(id, build_simple_graph(), v0).await.unwrap();
    let err = cache.update_graph_cas(id, build_simple_graph(), v0).await.unwrap_err();
    assert!(matches!(err, CacheError::CasConflict { .. }));
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

**スコープ**: WorkflowGraph + `compile_to_steps` + WorkflowCache + Repository Pair の実接続版。埋め込みなし・cold-start trust のみ。

#### 実装ステップ

1. **`OpenFangClient` 実装** (`src/openFang_client.rs`)  
   `WorkflowExecutor` トレイトを実装。`POST /v1/workflows` に `Vec<OpenFangStep>` を送信し、`ExecutionResult` を返す。タイムアウト・リトライは `ErrorMode` に従う。

2. **統合テスト追加** (`tests/m0/`)  
   ローカル OpenFang インスタンス（Docker）に対して `compile_to_steps → execute` の疎通を確認。

3. **cold-start trust 登録確認**  
   `WorkflowCache::insert` + `RepositoryPair::store` で `Trust::cold_start_new()` が正しく設定されることを統合テストで検証。

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
| OQ-12 | DarviumEvent canonical envelope の拡張性。新種別追加時の既存 projection への影響評価 | §12C | Medium |
| OQ-13 | StdinoutEventChannel の CompatMode 実装方針。旧プロトコル終了条件の定義 | §12D | Low |
| OQ-14 | WebSocketEventChannel の再接続バックオフ戦略。指数バックオフ vs 固定間隔 | §12D | Low |
| OQ-15 | EventProjection のスナップショット永続化戦略。全投影の一括スナップショット vs 個別 | §12E | Medium |
| OQ-16 | VirtualClock のリセット / ロールバック方針。clock overflow 対策 | §12C.6 | Low |
| OQ-17 | 複数 EventBus インスタンスの調整プロトコル。分散環境での clock 同期 | §12C | Low |
| OQ-11 | `TRUST_DEBOUNCE_DELTA = 0.05` の妥当性。Human フィードバックのバッチ更新パターンに依存する。非同期フィードバックの想定頻度によっては 0.02 や 0.10 が適切な可能性 | §9.5 | Low |
| OQ-18 | 下位 DAG retrieval の将来 RFC 分離。v2.3-h は最上階 WorkflowGraph の 4 層検索に限定し、sub-DAG レベルの構造検索は将来 RFC に分離する。分離時期・API 境界・Trace 連携方式を定義する必要がある | §12 | Low |

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
| `HUMAN_REVIEW_TIMEOUT_SECS` | 3600 | mission review デフォルトタイムアウト（秒）。この値を超えて未処理の mission は再通知または reviewer escalation が推奨される |
| `HUMAN_REVIEW_ESCALATION_SECS` | 14400 | エスカレーションタイムアウト（秒）。TIMEOUT 後も未解決の場合により上位の reviewer へ通知 |
| `HUMAN_REVIEW_MAX_BATCH_SIZE` | 20 | 同一種類の滞留 mission に対する一括承認/却下の最大件数 |
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


### A.x v2.3-d HumanChannel 追加定数

v2.3-d では、HumanChannel 通信基盤に関する以下の定数を追加する。

| 定数 | 既定値 | 意図 | 調整ガイド |
|---|---|---|---|
| `HITL_COMMUNICATE_COST_MULTIPLIER` | 3.0 | DeterminismScore における双方向 HITL のコスト係数 | **上げると** HITL を含むワークフローの決定論性スコアが低下し再利用候補から外れやすくなる。**下げると** HITL のコスト影響が軽減されるが人間待機が頻発する |
| `HITL_DEFAULT_TIMEOUT_SECS` | 3600 | communicate() のデフォルトタイムアウト秒数 | **小さくすると** 未応答インタラクションが早期に TimedOut になりエスカレーションが促進される。**大きくすると** 人間の応答をより長く待つが滞留インタラクションが増加する |
| `HITL_RECONNECT_BACKOFF_SECS` | 5.0 | reconnect 失敗時の再試行間隔 | **小さくすると** 再試行が頻繁になり負荷が増す。**大きくすると** 回復が遅延する |

### A.x v2.3-g Event Architecture 追加定数

v2.3-g では、Darvium Event Architecture に関する以下の定数を追加する。

| 定数 | 既定値 | 意図 | 調整ガイド |
|---|---|---|---|
| `EVENTBUS_CLOCK_INITIAL` | 0 | VirtualClock 初期値 | **変更不可 (Safety Invariant)** |
| `EVENTBUS_MAX_RECONNECT_RETRIES` | 3 | 再接続最大試行回数 | **上げると** 回復確率が向上するが無限再試行リスク増。**下げると** 早期断念でリソース節約 |
| `EVENTBUS_SUBSCRIPTION_MAX_KINDS` | 32 | 単一購読で指定可能な最大種別数 | **上げると** 柔軟性向上。**下げると** 購読設定の誤用防止 |
| `EVENTBUS_REPLAY_BATCH_SIZE` | 100 | replay 時の一括取得件数 | **上げると** スループット向上。**下げると** メモリ使用量抑制 |
| `EVENTBUS_CHANNEL_RECONNECT_BASE_DELAY_MS` | 1000 | チャネル再接続バックオフ初期値 (ms) | **上げると** 再試行間隔拡大でネットワーク負荷軽減。**下げると** 早期再接続が可能 |
| `EVENTBUS_CHANNEL_RECONNECT_MAX_DELAY_MS` | 30000 | チャネル再接続バックオフ最大値 (ms) | **上げると** 長時間断の再試行間隔が拡大。**下げると** 再試行頻度増加 |
| `EVENTBUS_PROJECTION_ERROR_BACKOFF_MS` | 5000 | projection エラー時再試行間隔 (ms) | **上げると** エラー発生時の負荷軽減。**下げると** 早期復旧機会増加 |

### A.x v2.3-i Preset Registry 追加定数

v2.3-i では、Preset Registry に関する以下の定数を追加する。

| 定数 | 既定値 | 意図 | 調整ガイド |
|---|---|---|---|
| `PRESET_MAX_WORKFLOW_COUNT` | 256 | 単一 registry が保持可能な最大 PresetWorkflow 数 | **変更不可 (Safety Invariant)**。メモリ安全上限 |
| `PRESET_MAX_DEPENDENCY_DEPTH` | 8 | PresetWorkflow 依存関係の最大深さ | **変更不可 (Safety Invariant)**。循環依存・過剰ネスト防止 |
| `PRESET_NAMESPACE_RESERVED` | `["platform", "builtin", "system"]` | BakedPresetRegistry 予約名前空間一覧 | **変更不可 (Safety Invariant)**。Mutable からの予約名使用を禁止 |
| `PRESET_BAKED_VALIDATION_TIMEOUT_MS` | 5000 | BakedPresetRegistry 検証タイムアウト (ms) | **上げると** 大規模 preset の検証余裕増。**下げると** startup 高速化 |
| `PRESET_MUTABLE_VALIDATION_TIMEOUT_MS` | 10000 | MutablePresetRegistry 検証タイムアウト (ms) | **上げると** 複雑な preset 検証余裕増。**下げると** startup 高速化 |

### A.x v2.3-k WorkflowCache Eviction 追加定数

v2.3-k では、WorkflowCache eviction 機構に関する以下の定数を追加する。

| 定数 | 既定値 | 意図 | 調整ガイド |
|---|---|---|---|
| `WORKFLOWCACHE_MAX_ENTRIES` | 1_000 | キャッシュが保持する最大エントリ数 | **変更不可 (Safety Invariant)**。メモリ安全上限 |
| `WORKFLOWCACHE_MAX_BYTES` | 500_000_000 (500MB) | キャッシュの推定メモリ使用量上限（バイト） | **上げると** より多くのグラフをキャッシュ可能。**下げると** メモリ消費抑制 |
| `WORKFLOWCACHE_TTL_HUMAN_MS` | 600_000 (10min) | 人間時間ベースのデフォルトTTL (ms) | **上げると** キャッシュヒット率向上。**下げると** 古いエントリの早期追い出し |
| `WORKFLOWCACHE_TTL_VIRTUAL_TICKS` | 1_000 | 仮想時間ベースのデフォルトTTL (ticks) | **上げると** 長い仮想時間生存。**下げると** 早期追い出し |
| `WORKFLOWCACHE_EVICTION_INTERVAL_MS` | 60_000 (1min) | 定期eviction実行間隔 (ms) | **上げると** eviction負荷軽減。**下げると** メモリ使用量の精密制御 |
| `WORKFLOWCACHE_PRESSURE_HIGH_WATERMARK` | 0.80 | 容量超過判定の高水位ライン (ratio) | **上げると** eviction頻度低下。**下げると** 早期eviction開始 |
| `WORKFLOWCACHE_PRESSURE_EMERGENCY_WATERMARK` | 0.95 | 緊急eviction発動の水位ライン (ratio) | **変更不可 (Safety Invariant)**。メモリ不足防止 |
| `WORKFLOWCACHE_PROTECTED_EVICTION_ALLOWED` | false | Protected エントリの eviction 許可フラグ | **変更不可 (Safety Invariant)**。P-18 遵守 |

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
| `ANN_TOP_K` | 10 | Stage 2のANN検索で取得する候補数。**上げると**（例：20）より多くの候補からStage 3/4で精密選択できるためヒット率が上がるが、GED計算コストがk倍増える。**下げると**（例：5）高速だが最良候補を見逃すリスクがある。Repository Pair が1万件を超えた段階で引き上げ検討が推奨される |
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

v1.9 では既存の `ValidationError`、`CompileError`、`CacheError`、`PersistenceError`、`SearchValidationError` 等に加え、`TrainingError` を追加する。training plane 導入によって既存エラー型の意味論を再定義してはならない。


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
// Layer 3: キャッシュエラー (§8.4 参照) — WorkflowCache 層の CAS 競合・不在
#[derive(Debug, thiserror::Error)]
enum CacheError {
    #[error("Version conflict: expected {expected}, found {actual}")]
    CasConflict { expected: u64, actual: u64 },
    #[error("Graph not found in cache: {0:?}")]
    NotFound(WorkflowGraphId),
    #[error("Lazy load from Repository Pair failed: {0}")]
    LoadFailed(String),
    // v2.3-k: Cache Residency / Eviction エラー
    #[error("Capacity exceeded: max_entries={max_entries}, max_bytes={max_bytes}")]
    CapacityExceeded { max_entries: usize, max_bytes: usize },
    #[error("Protected entry eviction forbidden: {0:?}")]
    ProtectedEvictionForbidden(WorkflowGraphId),
    #[error("Eviction invariant violation: {0}")]
    EvictionInvariantViolation(String),
}

// Repository Pair 永続化層エラー — デュアルストア一貫性・ストア操作失敗
#[derive(Debug, thiserror::Error)]
enum PersistenceError {
    #[error("Cross-store inconsistency detected: {0}")]
    CrossStoreInconsistency(String),
    #[error("SQLite operation failed: {0}")]
    SqliteError(String),
    #[error("LadybugDB operation failed: {0}")]
    LadybugError(String),
    #[error("Repository Pair not found: {0}")]
    PairNotFound(String),
}

// DarviumError (HumanChannel 関連バリアント, v2.3-d)
#[derive(Debug, thiserror::Error)]
enum DarviumError {
    #[error("Human channel I/O error: {0}")]
    HumanChannelIo(String),
    #[error("Human channel disconnected")]
    HumanChannelClosed,
    // v2.3-g: DarviumEventBus / InteractionStore エラー
    #[error("EventBus: clock conflict — expected monotonic increment")]
    EventBusClockConflict,
    #[error("EventBus: duplicate event — event_id {0} already committed")]
    EventBusDuplicateEvent(EventId),
    #[error("EventBus: interaction {0} not found")]
    EventBusInteractionNotFound(String),
    #[error("EventBus: interaction {0} is in terminal state, cannot mutate")]
    EventBusInteractionTerminal(String),
    #[error("EventBus: subscription capacity exceeded (max {0})")]
    EventBusSubscriptionCapacityExceeded(usize),
    #[error("EventBus: replay range invalid — start {start} > end {end}")]
    EventBusReplayRangeInvalid { start: u64, end: u64 },
    #[error("InteractionStore: record {0} not found")]
    InteractionStoreNotFound(String),
    #[error("InteractionStore: version conflict for {0}")]
    InteractionStoreVersionConflict(String),
    #[error("EventChannel: {0}")]
    EventChannel(String),
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
| (12) | HumanChannel state machine: S = {Idle, Pending, Resolved, TimedOut, Unreachable, ChannelClosed} | §12B.5 |
| (13) | D(G) HITL Communicate cost: w_communicate = base × 3.0 | §10.2 |

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

これらは Repository Pair (SQLite + LadybugDB) の graph blob source-of-truth を置き換えるものではなく、join / audit / queue / review / promotion を支える補助ストアである。WorkflowCache はこれらの永続データの in-memory working set として動作する。

v2.3-c では、会話メタデータについても同様に以下の workflow-side 推奨テーブルを追加する。

- ConversationalEventLog table
- ConversationalProposalLog table
- ConsolidationRunLog table

v2.3-d では、HITL インタラクション永続化のために以下のワークフローサイド推奨テーブルを追加する。

- HumanInteractions table (DDL定義は §12B.7 参照)

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

**v2.3-i 補足: StructMem / Corpus2Skill オブジェクト対応表**

| 知識形成理論 | オブジェクト | 一次ストア | 補足 |
|-------------|------------|-----------|------|
| StructMem | MemoryEvent | LadybugDB | 会話・内部観測由来の断片的記憶。ConversationalEvent は MemoryEvent の特殊化 |
| StructMem | Fragment | LadybugDB | 複数 MemoryEvent の集約 |
| StructMem | MemoryConcept | LadybugDB | 複数 Fragment から形成される抽象概念 |
| StructMem | CanonicalDocument | LadybugDB | 検証・昇格を経た正準知識文書 |
| Corpus2Skill | Chunk | LadybugDB | 構造化/非構造化文書の分割単位 |
| Corpus2Skill | Entity | LadybugDB | Chunk から抽出されるドメイン知識単位 |
| Corpus2Skill | SkillNode | LadybugDB | Entity からコンパイルされる実行可能ワークフロー表現 |
| 共通 | lineage | SQLite | DERIVEDFROM / CONSOLIDATES / MATERIALIZEDAS 等の系統関係 |
| 共通 | audit log | SQLite | 知識操作の監査ログ |

上記オブジェクトの source-of-truth は一貫して LadybugDB であり、SQLite は lineage・監査・メタデータの補助ストアとして機能する。StructMem / Corpus2Skill 専用の SQL テーブルは付録 D (§26 D.6) に定義する。

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
-- 影響行数が0ならCacheError::CasConflictエラーを返す
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

### D.6 v2.3-i Preset Registry データ型

v2.3-i で追加された Preset Registry 関連のデータ型は §8 (WorkflowCache と MemoizedGraph) に疑似コード定義がある。本節では主要 enum の値を列挙する:

**ArtifactOriginKind**: `PresetSystem` / `PresetUser` / `SearchGenerated` / `TrainingDerived` / `FusionDerived` / `Conversational` / `Manual`

**RegistrySource**: `BakedPlatform` / `MutableUser` / `MutableWorkspace`

**CapabilityFamily**: `StructMem` / `Corpus2Skill` / `Search` / `Training` / `General`

**PresetRootPolicy**: `RootPinned` / `RootUnpinned` / `RootAncestorPinned`

**PresetValidationReason**: `InvalidPresetSchema` / `DuplicateWorkflowId` / `ReservedNamespaceViolation` / `WorkflowNotFound` / `CrossRegistryDependencyViolation` / `CircularReference` / `InvalidInputMapping` / `OutputBindingMismatch` / `BootCriticalPresetMissing` / `BootCriticalPresetInvalid` / `MutableOverrideForbidden` / `PresetPolicyViolation`

**PresetRegistryEvent (EventKind)**: `StartupValidationStarted` / `StartupValidationCompleted` / `PresetAccepted` / `PresetQuarantined` / `CollisionResolved`

上記の完全な構造体定義・メソッド・検証手順については §8.5–§8.9 を参照すること。

## 27A. 付録 G — v2.3-h 4 層検索実験計画

この付録は、v2.3-h で導入する 4 層トップレベル GED 検索（§12）の実験的評価計画を定義する。本計画は将来の実験実施のための枠組みであり、現バージョンでの実装完了や全指標の達成を要求するものではない。

### G.1 評価問題定義

以下の 3 軸で評価問題を定義する：

- **検索精度**: Retrieval pipeline が上位ワークフローグラフ集合から正しい候補を選別できるか。WorkflowGraph 間の構造類似性が意味的類似性と独立して機能することを確認する。
- **計算効率**: Full GED の呼び出し回数が Cheap GED フィルタにより削減されるか。K_CHEAP と K_FULL の関係において、PruneGain（G.4）が正となることを確認する。
- **劣化耐性**: 入力クエリのノイズ（task_embedding の摂動、metadata の欠損、グラフ構造の軽微な変化）に対するランキング安定性を評価する。

### G.2 監督信号データセット

以下の 5 種のデータセットを構築し、評価の監督信号とする。各データセットは query WorkflowGraph 1 件に対して正解 candidate 1 件または複数件からなる (query, positive_candidate, negative_candidate_set) の組で構成される。

- **Gold Reuse**: 実ワークフロー実行履歴から抽出された「同一ミッション種別で正常再利用された」WorkflowGraph ペア。最も基本的な正解信号。目標サンプルサイズ n >= 500。
- **Gold Patch**: 「類似タスクだが一部修正が必要だった」WorkflowGraph ペア。GED が中程度（部分編集が必要）のケースでのランキング品質を評価。目標 n >= 300。
- **Gold Compose**: 「複数の既存ワークフローを合成して新しいワークフローが作られた」ケース。複数正解候補が存在する multi-positive 設定での nDCG 評価に使用。目標 n >= 200（クエリ数）。
- **Hard Negative**: 意味的類似度は高い（task_embedding cosine > 0.85）が構造的に異なる WorkflowGraph ペア。4 層検索の構造分離能力を評価するための最重要データセット。目標 n >= 200。
- **Perturbation**: 既存 WorkflowGraph に軽微な構造変更（ノード追加/削除、エッジ組換え、ラベル変更）を施したペア。GED の連続性とコストモデルの健全性を評価。目標 n >= 400。

### G.3 主要評価指標

以下を主要評価指標として採用する：

- **Recall@K**: 各 stage 後の候補集合に正解が含まれる割合。K_SEM=20, K_META=50, K_CHEAP=20, K_FULL=10 の各閾値で計測する。
- **nDCG@K**: 特に Gold Compose データセットでの multi-positive ランキング品質。
- **MRR (Mean Reciprocal Rank)**: Hard Negative データセットでの最初の正解候補の出現位置。
- **F1 (Hard Gate)**: Stage 0 ハードゲート通過率と正当な棄却率の調和平均。
- **Latency P50/P95/P99**: Pipeline 各 stage のレイテンシ（ミリ秒）。特に Full GED Stage 4 の P99 は FULLGED_TIMEOUT_MS を超えないことを確認する。
- **Stability**: task_embedding にガウスノイズ N(0, 0.01) を加えた 100 回の試行におけるランキング順位の変動係数。

### G.4 Cheap GED 有効性検証（PruneGain／MissRate）

Cheap GED（Stage 3）の追加価値を以下の指標で定量評価する：

- **PruneGain** = 1 − (K_CHEAP通過件数) / (K_META通過件数)。値が大きいほど Cheap GED が多くの候補を枝刈りしていることを示す。目標 PruneGain >= 0.40（つまり K_META=50 から K_CHEAP=20 への削減率 60% 以上）。
- **MissRate** = Cheap GED により誤って棄却された正解候補の割合。目標 MissRate < 0.02（2% 未満）。
- **Cheap GED 有効性曲線**: CHEAPGED_ENABLE_THRESHOLD を変化させたときの PruneGain と MissRate のトレードオフを計測。最適閾値を同定する。

Cheap GED を経由せず直接 Full GED に移行した場合（バイパス）とのレイテンシ比較も併せて実施する。

### G.5 Full GED Cost Model 較正

Full GED（Stage 4）のエディットコストパラメータ（GED_NODE_DELETE_COST, GED_NODE_INSERT_COST, GED_EDGE_DELETE_COST, GED_EDGE_INSERT_COST, GED_SIDEEFFECT_PENALTY, GED_KIND_MISMATCH_PENALTY, GED_AGENTSET_WEIGHT, GED_IO_WEIGHT, GED_DETERMINISM_WEIGHT）を以下の目的関数で較正する：

- **Pairwise Loss**: 正解ペア (q, g⁺) の GED が負例ペア (q, g⁻) の GED より常に小さくなるようパラメータを学習する。損失関数は合計 GED のマージンランキング損失 L = max(0, GED(q,g⁺) − GED(q,g⁻) + margin)。
- **Decision-Aware Objective**: 最終適用判断（Applicability の classify 結果）と正解ラベルの一致率を直接最適化する。GED 単独最適化ではなく下流タスク指向の較正を行う。
- **Ablation**: 各コストパラメータを個別に 0 に設定したときのランキング品質変化を観測し、影響が最大のパラメータを同定する。

### G.6 Blend 係数較正 + Ablation 実験計画

SIMILARITY_ALPHA（S_sem と S_struct のブレンド係数）および STRUCT_GED_LAMBDA（GED 指数類似度のスケーリング）を以下の実験計画で較正する：

- **A0（ベースライン）**: α = 0.45, λ = 4.0（§11 推奨初期値）。
- **A1（セマンティック優位）**: α = 0.70, λ = 4.0。意味的類似度を重視する設定。Hard Negative データセットでの性能低下が予想される。
- **A2（構造優位）**: α = 0.20, λ = 4.0。構造的類似度を重視する設定。Gold Reuse での Recall@K 低下リスクを観測する。
- **A3（スケーリング調整）**: α = 0.45, λ = 8.0。GED 指数の減衰を急峻にし、小さな GED 差を増幅する。
- **A4（スケーリング緩和）**: α = 0.45, λ = 2.0。GED 指数の減衰を緩やかにし、広い範囲の GED 値を差別化する。

各 ablation 条件で全データセット（G.2）に対する Recall@K / nDCG / MRR を計測し、A0 との相対差を報告する。

### G.7 OOD／Drift 監視

プロダクション投入後の経時劣化を検出するための OOD（Out-of-Distribution）監視指標を定義する：

- **Embedding Drift**: task_embedding の分布変化を MMD（Maximum Mean Discrepancy）で定量化。ベースライン分布（known good queries）との有意差を 95% 信頼水準で検定する。
- **Metadata Distribution Shift**: TopLevelGraphMetadata の各項目（node_count, edge_count, longest_path_len 等）の marginal distribution 変化を Kolmogorov-Smirnov 検定で監視する。
- **GED Score Drift**: Full GED スコアの分布変化。特に上位 K_FULL 候補の GED 値が経時的に増加傾向にある場合、Repository Pair 構造の全体的変化を示唆する。
- **Latency Regime Change**: Pipeline 各 stage のレイテンシ分布が事前定義された SLO を逸脱した場合に警告を発する。特に Full GED の P99 レイテンシが FULLGED_TIMEOUT_MS の 80% を超えた場合に注意喚起する。

各 drift 指標は週次集計とし、ダッシュボードで可視化することを推奨する。ドリフトが検出された場合は、該当期間の query 分布と Repository Pair 変更履歴を突き合わせて原因分析を行う。

## 27. 付録 E — v1.8 / v1.9 Calibration Candidates

以下の定数は v1.8 では規範的であるが、将来の較正候補として明示的に指定される: §11.5 の知識適用性指数、変更安全閾値 `K >= 0.50`、監査グレードハードゲート `K < 0.30`、および §16.4 のエビデンス完全性タイブレークポリシー。実装は v1.8 デプロイメント内でこれらの値を黙示的に変更してはならない (MUST NOT)。変更には明示的なバージョン管理、移行ノート、およびリプレイ/評価エビデンスが必要である。

さらに、v1.9 は以下をトレーニング関連の較正候補として指定する: トレーニング信頼から本番信頼への継承比率、サンドボックス成功/Good 率閾値、候補 tombstone 猶予期間、カリキュラム重み減衰、AI 生成ミッションの自動承認例外範囲、および昇格ロールバック粒度。さらに、人間レビュー SLA（レビュータイムアウト、エスカレーションタイムアウト、最大バッチサイズ）の推奨初期値が付録 A で提供され、ハード保証ではなく較正候補として扱われなければならない (SHALL)。これらのパラメータは、実装ローカルのドリフトではなく、明示的なバージョン管理された改訂を通じてのみ進化してよい (MAY)。

本付録は v1.7 で確立された設計規律を維持するために存在する: パラメータは進化してよい (MAY) が、アドホックな実装ドリフトではなく、明示的な RFC レベルの改訂を通じてのみ進化する。

さらに、v2.3-c は以下を会話型較正候補として指定する: 統合閾値（min_distinct_events、min_distinct_days、min_semantic_coherence、min_trace_completeness、min_temporal_stability、max_contradiction_score）、自動サンドボックス取り込みの LLM 信頼閾値、昇格完全性閾値、および矛盾共存ポリシー。これらのパラメータは、実装ローカルのドリフトではなく、明示的なバージョン管理されたデフォルトを持つ較正候補として扱われなければならない (SHALL)。初期規範的デフォルトは §16B.5 および §22（v2.3-c 追加定数）で提供される。

さらに、v2.3-f は以下を相互利益関連の較正候補として指定する:

- `RECIPROCITY_ALPHA_HELP`
- `RECIPROCITY_ALPHA_SUCCESS`
- `RECIPROCITY_ALPHA_REJECT`
- `RECIPROCITY_ALPHA_HARM`
- `RECIPROCITY_DIRECT_DECAY_RHO`
- `REPUTATION_WEIGHT_DIRECT`
- `REPUTATION_WEIGHT_INDIRECT`
- `LIFECYCLE_WEIGHT_BENEVOLENCE`
- `GC_HAZARD_GAMMA_BENEVOLENCE`
- `GC_HAZARD_GAMMA_CHILD_PROTECT`
- `HELP_WEIGHT_BENEVOLENCE`
- `HELP_SOFTMAX_TAU`
- `REMOTE_EXPLORATION_BASE`
- `REMOTE_EXPLORATION_MAX`
- `CHILD_GROWTH_WEIGHT_HELP_SUCCESS`
- `CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS`

加えて v2.3-g は以下を Event Architecture の calibration candidates として指定する:

- `EVENTBUS_MAX_RECONNECT_RETRIES` — 既定値 3, 範囲 1–10。大きな値は回復確率を向上させるが無限再試行リスクを伴う。
- `EVENTBUS_SUBSCRIPTION_MAX_KINDS` — 既定値 32, 範囲 1–128。大きな値は購読設定の柔軟性を高めるがルーティング複雑度が増す。
- `EVENTBUS_REPLAY_BATCH_SIZE` — 既定値 100, 範囲 10–1000。大きな値は大規模リプレイのスループットを改善するがメモリ圧力が増加する。
- `EVENTBUS_CHANNEL_RECONNECT_BASE_DELAY_MS` — 既定値 1000ms, 範囲 100ms–10000ms。チャネル再接続の初期バックオフ遅延を制御する。
- `EVENTBUS_CHANNEL_RECONNECT_MAX_DELAY_MS` — 既定値 30000ms, 範囲 5000ms–120000ms。指数バックオフの成長上限を設定する。
- `EVENTBUS_PROJECTION_ERROR_BACKOFF_MS` — 既定値 5000ms, 範囲 1000ms–60000ms。投影失敗後の再試行間隔。

加えて v2.3-h は以下を 4 層検索の calibration candidates として指定する:

- `TOPLEVELONLYRETRIEVAL = true` — 最上階 DAG のみ検索対象とする不変条件。
- `K_SEM` — Stage 1 semantic retrieval の上限候補数。既定値 20, 範囲 5–100。
- `K_META` — Stage 2 metadata filter 通過後の上限候補数。既定値 50, 範囲 10–200。
- `K_CHEAP` — Stage 3 cheap GED filter 通過後の上限候補数。既定値 20, 範囲 5–100。
- `K_FULL` — Stage 4 full GED rerank の最終上位候補数。既定値 10, 範囲 3–50。
- `METAFILTER_THRESHOLD` または `METAFILTER_TOPK` — metadata scored filter の閾値方式。既定値 top-K, 範囲 top-K / threshold。
- `CHEAPGED_ENABLE_THRESHOLD` — 候補数がこの値を超えると cheap GED が MUST となる閾値。既定値 30, 範囲 10–100。
- `CHEAPGED_LB_VERSION` — cheap GED lower bound 計算方式のバージョン識別子。
- `FULLGED_COST_MODEL_VERSION` — full GED cost model のバージョン識別子。
- `FULLGED_TIMEOUT_MS` — full GED 計算のタイムアウト。既定値 5000ms, 範囲 1000ms–30000ms。
- `SIMILARITY_ALPHA` — S_total の α 係数 (§11.3 式(8))。既定値 0.45, 範囲 0.0–1.0。
- `STRUCT_GED_LAMBDA` — S_struct の λ 係数 (§11.3 式(7))。既定値 4.0, 範囲 0.5–10.0。
- `APPLICABILITY_BETA` — A_final の β 係数 (§11.3 式(10))。既定値 0.70, 範囲 0.0–1.0。
- `GED_NODE_DELETE_COST` — ノード削除コスト δ₀。既定値 1.0, 範囲 0.5–5.0。
- `GED_NODE_INSERT_COST` — ノード挿入コスト ι₀。既定値 1.0, 範囲 0.5–5.0。
- `GED_EDGE_DELETE_COST` — エッジ削除コスト。既定値 0.5, 範囲 0.25–2.0。
- `GED_EDGE_INSERT_COST` — エッジ挿入コスト。既定値 0.5, 範囲 0.25–2.0。
- `GED_SIDEEFFECT_PENALTY` — side effect ペナルティ δₛₑ/ιₛₑ。既定値 3.0, 範囲 1.0–10.0。
- `GED_KIND_MISMATCH_PENALTY` — ノード種別不一致ペナルティ ηₖ。既定値 2.0, 範囲 0.5–5.0。
- `GED_AGENTSET_WEIGHT` — agent/tag set Jaccard 重み ηₐ。既定値 1.0, 範囲 0.0–3.0。
- `GED_IO_WEIGHT` — I/O type Jaccard 重み ηᵢ/ηₒ。既定値 0.5, 範囲 0.0–2.0。
- `GED_DETERMINISM_WEIGHT` — determinism 差重み η_d。既定値 0.5, 範囲 0.0–2.0。


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

## 28. リポジトリペア / エキスパートフュージョン統合仕様 (v2.0-final)

改訂 v2.0 は、v1.9 の完全な規範的内容を、事前の保証、正本境界、信頼ルール、ライフサイクルルール、トレーニング不変条件、適用可能性方程式、パッチルール、監査要件、修復規律を弱めたり、削除したり、再定義することなく維持する。v2.0 は厳密に追加的な改訂であり、既に確立された4平面論理アーキテクチャ上での第一級操作としてリポジトリペアレベルの合成フュージョンを導入する。

この改訂の目的は、破壊的なデータベースマージを定義することではない。目的は、SQLite と LadybugDB で構成されるリポジトリペアの安全な誕生、選択的抽出、フュージョン、分割、再構成を定義することであり、完全な系統、貢献履歴、アクターのトレーサビリティ、トレーニング/プロダクションの分離、およびデュアルストアの操作上の完全性を維持することである。

v2.0 におけるリポジトリペアは、(a) SQLite 側のワークフロー、信頼、ライフサイクル、監査、トレーニング、ランタイムメタデータ、および (b) LadybugDB 側の知識オブジェクト、関係、起点を保持するエビデンス構造から構成される可搬な運用個体として解釈される SHALL。フュージョン操作は新しい出力ペアを作成する SHALL であり、既存の入力ペアをその場で破壊的に変更してはならない MUST NOT。

この改訂はさらに、抽出とフュージョンの主要な意味論的選択単位としてエキスパート名前空間の概念を導入する。エキスパート資産は、明示的なマニフェストとクロージャポリシーによって選択される SHALL。アドホックなファイルコピー、生のプレフィックス照合、または系統と許容性を非決定論的にする実装ローカルのヒューリスティックによって選択されてはならない。

あいまいさを避けるため、v2.0-final は異なるソースペアからインポートされた概念的に類似した知識オブジェクト間の自動意味論的マージ、自動真実調整、または信頼度加重勝者選択を定義しない。2つ以上の知識オブジェクトが意味的に重複しているように見える場合、デフォルト安全ルールは再生されたIDと `CONSOLIDATES` や `SUPERSEDES` などの明示的な系統関係の下での共存であり、単一の正準オブジェクトへの破壊的な統合はこの改訂の範囲外である。

## 29. フュージョンコア用語集 (v2.0)

| 用語 | 定義 |
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

## 30. リポジトリペアモデル

v2.0 は、v1.9 の既存の所有権境界を変更することなく、リポジトリペアを第一級のリポジトリレベルオブジェクトとして定義する。SQLite は、ワークフローグラフ、WorkflowLineage、TrustProfile、ライフサイクル状態、SearchTrace、TrainingMission、TrainingRunLog、TrainingFeedback、PromotionCandidate、TrainingAuditLog、ランタイムキュー、修復状態、フュージョン側メタデータに対して権威であり続ける SHALL。LadybugDB は、知識オブジェクト、知識関係、起点トレース構造、エビデンス系統、および知識レベルの継承/統合関係に対して権威であり続ける SHALL。

リポジトリペアは、したがって、モノリシックなデータベースイメージではなく、論理的に結合された正本のペアとして扱われる SHALL。これらの境界を実装ローカルのマージストアに collapsing する任意の v2.0 抽出またはフュージョン実装は、一時的な実行の詳細として存在してもよい MAY が、出力ペア誕生後の規範的な所有権境界は v1.9 と同一でなければならない MUST。

v2.0 で導入されるリポジトリレベル操作は以下の通りである：

1. `ExtractExpert`
2. `FuseExperts`
3. `SplitPairByExpert`
4. `RecomposePair`

それぞれの操作は、1つ以上の新しい出力ペアを生成する SHALL。既存の入力ペアは、正準な永続化内容に関して変更不可能でなければならない MUST NOT。ただし、フュージョン操作の系統祖先として使用されたことを記録するオプションの追加専用監査証跡は例外とする。

## 31. エキスパート境界モデル

v2.0 におけるエキスパートは、主に名前空間によって識別され、副次的にマニフェストで宣言されたルートとクロージャポリシーによって識別される、意味論的に一貫した資産バンドルとして定義される SHALL。名前空間だけでは、マニフェストまたは移行ルールが最初にルートワークフロー、ルート知識、およびクロージャ意味論を確立していない限り、安全な抽出に十分であると扱われてはならない MUST NOT。

規範的なエキスパート境界モデルは、少なくとも以下の3つの層を含む SHALL：

1. **Primary Membership** — 名前空間に直接所属する資産。
2. **Required Dependency Closure** — 正しい実行または説明のために必要な外部サブワークフロー、共有知識オブジェクト、系統に重要な関係。
3. **Optional Contextual Closure** — 再現性または説明可能性のために含められる監査ログ、検索トレース、洗練ログ、トレーニングログ、および類似の資料。

実装は、抽出またはフュージョンされた任意のエキスパートについて、どの資産が一次所属で、どの資産が必須依存関係で、どの資産がオプションコンテキストであるかを回答できなければならない MUST。この分類を保持しないことは、トレーサビリティ欠陥として扱われる SHALL。

### 31.1 推奨される ExpertManifest の形状

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

上記のスキーマは例示であるが、規範的な代替手段は同等の表現力を保持する SHALL。

## 32. フュージョン / 抽出操作

フュージョンと抽出は、正式な計画オブジェクトによって駆動される宣言的なリポジトリ変換として定義される SHALL。ファイルコピー手順、SQLダンプアンドロード手順、または実装ローカルのバッチスクリプトとしてのみ指定されてはならない SHALL NOT。

### 32.1 ExtractExpert

`ExtractExpert` は、単一の入力ペアと1つ以上のエキスパート名前空間の選択を受け入れ、宣言された依存関係およびコンテキストポリシーの下で、選択されたエキスパートのクロージャである資産セットを持つ新しいペアを生成する SHALL。

### 32.2 FuseExperts

`FuseExperts` は、2つ以上の入力ペアと、各ペアからの1つ以上のエキスパート名前空間の選択を受け入れ、その内容が宣言された計画、許容性制約、再写像ポリシー、系統ポリシー、トレーニングポリシーのみによって決定される新しいペアを生成する SHALL。

### 32.3 SplitPairByExpert

`SplitPairByExpert` は、マニフェストで定義されたエキスパート境界によって入力ペアを分割し、1つ以上の新しい出力ペアを作成する SHALL。共有依存関係資産は、クロージャに必要な場合には複数の子ペアにコピーされてもよい MAY が、そのような重複は系統参照と再写像テーブルを通じて祖先性を保持しなければならない MUST。

### 32.4 RecomposePair

`RecomposePair` は、単一の入力ペア内での名前空間書き換え、ルート再編成、またはポリシークリーンアップを許可するが、それでも非破壊的な出力ペアの誕生、ID再写像、系統保持、許容性検証を要求する SHALL。

## 33. 許容性と安全ゲート

フュージョンの許容性モデルは、通常の v1.9 候補選択経路と少なくとも同等に厳格でなければならない SHALL。`Pending`、`NeedsRepair`、または `Quarantined` の整合性状態にある資産は、通常のプロダクションフュージョン結果に黙って含まれてはならない MUST NOT。

推奨されるデフォルト許容性ルールは、監査と人間によるレビューを伴う高リスク操作モードによって明示的に上書きされない限り、規範的である：

| 条件 | デフォルト動作 | 許可される例外 |
|-----------|------------------|-------------------|
| `ConsistencyState = Pending` | 拒否 | なし |
| `ConsistencyState = NeedsRepair` | 拒否 | 修復モード抽出のみ |
| `ConsistencyState = Quarantined` | 拒否 | 監査モードのみ |
| `TrainingArtifactState = TrainingOnly` | 拒否 | サンドボックスフュージョンのみ |
| `GcState = SoftDeleted` | 任意 | 明示的なオプトイン |
| `GcState = Tombstoned` | アクティブ資産として拒否 | 祖先参照のみ |
| `PromotionStatus = Candidate / Rejected / RolledBack` がプロダクションペアに含まれる場合 | 人間によるレビュー必須 | なし |

フュージョン実装は、許容性拒否を `FusionAuditRecord` に明示的に出力しなければならない MUST。また、その省略が要求されるクロージャまたは系統の完全性を破壊する場合には、そのような拒否を黙示の省略に格下げしてはならない MUST NOT。

## 34. ID 再写像

出力ペアの主要オブジェクトIDは、ペア間の衝突と暗黙のエイリアシングを避けるため、デフォルトで再生成される SHOULD。ソースIDの部分的な再利用は将来の改訂で許可されてもよい MAY が、v2.0 は明示的なトレーステーブルによる完全な再生成をデフォルト安全ポリシーとして扱う SHALL。

`IdentityRemapTable` は、最低限以下のフィールドを保持する SHALL：

| フィールド | 意味 |
|-------|---------|
| `source_pair_id` | 元ペア |
| `source_store` | `sqlite` または `ladybug` |
| `source_object_type` | workflow / knowledge / runlog / audit / relation / training object 等 |
| `source_id` | 元 ID |
| `target_pair_id` | 新ペア |
| `target_id` | 新 ID |
| `preserved_namespace` | 元名前空間 |
| `remap_reason` | `extract` / `fuse` / `split` / `recompose` |

具体化された任意のオブジェクトが (a) 再写像エントリ、または (b) オブジェクトがターゲットペアで新たに生まれしたがってソース祖先を持たないという明示的な宣言のいずれかを欠く場合、出力ペアはトレーサビリティ不完全とみなされる SHALL。

### 34.1 例示的な再写像の例

```text
pair_A.workflow:wf_001   -> pair_C.workflow:wf_c_9001
pair_A.knowledge:kb_1001 -> pair_C.knowledge:kb_c_2001
pair_B.workflow:wf_777   -> pair_C.workflow:wf_c_9002
pair_B.audit:trainlog_55 -> pair_C.audit:trainlog_c_801
```

## 35. 系統とトレーサビリティ要件

完全なトレーサビリティは、v2.0 の中心的な規範的目標の1つである。暗号学的不変性は依然として範囲外であるが、祖先への論理的および構造的な到達可能性は保持される SHALL。

このRFCの目的において、系統保持とは、出力ペアが、現在のオブジェクトをソースペア、ソースオブジェクト、ソースアクター、ソースラン、ソースフィードバックまで遡ることができるロスレスの手続きを保持することを意味する SHALL。ただし、そのような祖先が入力に存在する場合に限る。

### 35.1 ワークフロー側の系統

最低限、以下のワークフロー側構造は、再写像転送または祖先参照によって保持される SHALL：

- `WorkflowLineage`
- `ContributionRecord`
- `Provenance`
- `PatchHistory`
- `TrustAuditLog`
- `LifecycleAuditLog`
- `SearchTrace` / `SearchRunLog`
- `RefinementRunLog`

### 35.2 知識側の系統

最低限、以下の知識側の系統は保持される SHALL：

- 知識オブジェクト自体
- 起点トレース識別子
- エビデンスの要約および完全性フィールド
- `DERIVEDFROM`、`CONSOLIDATES`、`SUPERSEDES`、`MATERIALIZEDAS` などの系統関係
- トレーニング由来の知識フィールド（名前空間、起点トレースID、候補知識アーティファクトとして表現された場合のプロモーション状態など）

### 35.3 トレーニング側の系統

最低限、以下のトレーニング側構造は、計画ポリシーによって含まれる場合に保持される SHALL：

- `TrainingMission`
- `TrainingRunLog`
- `TrainingFeedback`
- `PromotionCandidate`
- `TrainingAuditLog`
- `CurriculumPolicy` またはカリキュラム関係メタデータ

### 35.4 アクター ID 拡張

v2.0 は、v1.9 の最小限の `actor_id: String` を超えて、アクターのトレーサビリティを強化する SHALL。実装は、以下の規範的戦略のうち少なくとも1つを選択する SHALL：

1. `actor_id` を、公開鍵と表示情報を生成する外部 ID レジストリに対して解決可能な安定した参照として定義する。
2. 安定したアクター参照、公開鍵参照、表示名スナップショットを保持する `ActorRef` 構造体を導入する。

推奨される形状：

```rust
struct ActorRef {
    actor_id: String,
    public_key_ref: String,
    display_name_snapshot: Option<String>,
    identity_provider: String,
}
```

### 35.5 貢献定量化の拡張

v2.0 は、アクターレベルおよびユーザー影響レベルのトレーサビリティをサポートするために、貢献会計を拡張する SHOULD。

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

## 36. フュージョンにおけるトレーニング/プロダクション分離

v1.9 のトレーニング分離不変条件、プロモーション規律不変条件、信頼分離不変条件、知識プロモーション不変条件は、v2.0 フュージョン意味論の下でも完全に有効であり続ける SHALL。プロモーションゲートを黙ってバイパスするようにフュージョン操作が定義されてはならない。

`FusionPlan` または `ExtractionPlan` は、明示的なトレーニングポリシーを含む SHALL。推奨値は以下の通り：

- `exclude_training_only`
- `include_promoted_only`
- `include_candidates_with_human_gate`
- `sandbox_all_training`

プロダクション向けの `FuseExperts` の場合、デフォルトは `exclude_training_only` である SHOULD。研究用またはサンドボックスフュージョンの場合、`sandbox_all_training` が明示的に選択されてもよい MAY が、結果として得られる出力ペアは、プロモーション要件が満たされるまで通常のプロダクション選択経路の外側に留まらなければならない MUST。

## 37. フュージョンオーケストレーターと誕生コミット

以下のオーケストレーター形状は、規範的な分解境界として強く推奨される。

```rust
struct FusionPlan { /* 正式計画オブジェクト */ }
struct ExtractionPlan { /* 正式計画オブジェクト */ }
struct ExpertManifest { /* エキスパート境界 */ }
struct IdentityRemapTable { /* 旧 -> 新 */ }
struct FusionAuditRecord { /* 操作ログ */ }

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

推奨される実行順序：

1. 計画検証
2. 許容性フィルタリング
3. エキスパートクロージャ計算
4. 競合スキャン
5. 完全な ID 再写像生成
6. ワークフロー側の具体化
7. 知識側の具体化
8. 系統/監査/トレーニングリンケージの具体化
9. 出力ペア整合性検証
10. 誕生ファイナライズ
11. フュージョン監査追記

出力ペアは、誕生ファイナライズと具体化後整合性検証が成功するまで、プロダクション選択経路に入ってはならない SHALL NOT。

### 37.1 誕生コミット規律

フュージョン誕生コミットは、v1.9 のデュアルストアインテントプロトコルと同じアプリケーションレベルの整合性哲学に従う SHALL。正確なコミットフェーズは知識変更コミットフェーズと異なってもよい MAY が、実装は、中断されたペア誕生を決定論的に完了、隔離、または墓石化するために十分なインテント、再写像メタデータ、系統リンケージ、修復メタデータを永続化しなければならない MUST。

誕生操作は、最低限以下を記録する SHALL：

- 操作 ID
- 入力ペアセット
- 選択されたエキスパートセット
- 出力ペアターゲット ID
- 再写像ポリシー
- 系統ポリシー
- トレーニングポリシー
- 現在の誕生フェーズ
- 中断された場合の修復/隔離理由

## 38. フュージョンの障害処理、隔離、修復

失敗したまたは中断されたフュージョンは、リポジトリレベルの整合性イベントとして扱われる SHALL。無視可能なベストエフォートバッチ障害としては扱われない。SQLite 側または LadybugDB 側の具体化が部分的にしか完了しなかった場合、ランタイムは修復可能な障害状態を記録しなければならず MUST、部分的に生まれたペアが通常のプロダクション取得に入るのを防がなければならない MUST。

推奨される障害状態には、`BirthPending`、`BirthNeedsRepair`、`BirthQuarantined`、`BirthTombstoned` が含まれる。実装は、意味論的な区別が監査可能なままである限り、既存の整合性またはライフサイクル機構を使用してこれらをエンコードしてもよい MAY。

フュージョン用の修復ワーカーは、再試行、隔離、または補償用の墓石化を試みてもよい MAY。修復が成功した場合、元のフュージョン操作 ID、再写像テーブル、ソースペア参照、系統リンケージを保持しなければならない MUST。無関係な新しい ID の下での祖先継続性のない暗黙の再生は禁止される。

## 39. v2.0 の移行と後方互換性

v2.0 は、v1.9 のリポジトリ意味論と後方互換性を維持しなければならない MUST。明示的な Expert Manifest オブジェクトを宣言していない既存の v1.9 ペアは、有効なリポジトリペアであり続ける SHALL。

ただし、レガシーペアを含む抽出およびフュージョンには移行ルールが必要である。推奨される移行ポリシーは以下の通り：

1. 名前空間、ワークフロールート、知識ルートから暫定的なエキスパート境界を推論する。
2. 推論されたマニフェストを暫定としてマークする。
3. クロージャのあいまいさまたは所有権のあいまいさが残る場合、人間によるレビューを要求する。
4. 暫定マニフェストがトレーサビリティ要件を満たせない場合、不可逆的なプロダクションフュージョンを禁止する。

v1.9 の `actor_id` のみのログは、有効な履歴記録であり続ける SHALL。v2.0 実装は、実行可能な場合には移行時に外部レジストリ解決または `ActorRef` の拡張によってこれらを豊かにする SHOULD だが、履歴の意味論を書き換えてはならない MUST NOT。

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

1. 正本保存不変条件 (source-of-truth preservation invariant): ワークフロー/信頼/ライフサイクル/トレーニングメタデータの所有権は SQLite に残り、知識の所有権は LadybugDB に残る。
2. 非破壊的フュージョン不変条件 (non-destructive fusion invariant): 入力ペア MUST NOT 破壊的に変更されてはならない。
3. 完全トレーサビリティ不変条件 (full traceability invariant): ターゲットオブジェクト MUST 再写像または明示的な誕生宣言を通じて祖先に到達可能でなければならない。
4. トレーニング分離不変条件 (training separation invariant): トレーニング専用成果物 MUST NOT 本番ペアに黙示的に入り込んではならない。
5. 許容性不変条件 (admissibility invariant): quarantine/pending/needs-repair の資産 MUST NOT 通常の本番フュージョンに黙示的に入り込んではならない。
6. 誕生完全性不変条件 (birth integrity invariant): 部分的に materialize された出力ペア MUST 通常の本番選択パスの外側に留まらなければならない。
7. アクタートレーサビリティ不変条件 (actor traceability invariant): 監査と貢献履歴 MUST アクターへの到達可能性を少なくとも安定した外部参照まで保持しなければならない。

### 41.2 Open questions

- ペアレベルの評判が構成資産メトリクスから純粋に導出されるべきか、明示的なペアレベルの信頼を持つべきか (whether pair-level reputation should remain purely derived from constituent asset metrics or gain explicit pair-level trust)。
- 暫定マニフェスト推論がレガシー v1.9 リポジトリに対してさらに標準化されるべきか (whether provisional manifest inference should be standardized further for legacy v1.9 repositories)。
- 将来の改訂が、より厳格なエビデンスルールの下で知識オブジェクトの選択的セマンティック統合を許可すべきか (whether future revisions should allow selective semantic consolidation of knowledge objects under stricter evidence rules)。
- ペア誕生ライフサイクルが `ConsistencyState` を直接再利用すべきか、専用の誕生状態機械を導入すべきか (whether pair-birth lifecycle should reuse `ConsistencyState` directly or introduce a dedicated birth-state machine)。

### 41.3 Deferred annex / future RFC responsibilities

以下のトピックは意図的に認識されているが、v2.0-final の規範的閉包の外側にある:

- **形式保証付録 (Formal guarantees annex)** — 適用可能性安定性、信頼収束、ライフサイクル均衡、安全性、活性に関する証明責務。
- **脅威モデル付録 (Threat model annex)** — 悪意あるアクター、プロンプトインジェクション、知識ポイズニング、フュージョンポイズニング、トレーニング破壊。
- **分散アーキテクチャ付録 (Distributed architecture annex)** — マルチノードレプリケーション、コンセンサス、パーティション処理、リモート修復調整。
- **探索理論 RFC-0003 スコープ (Exploration theory RFC-0003 scope)** — 検索ポリシー最適化、MCTS/bandit/RL 選択理論、Pareto 信頼、ダーウィン進化。

これらの省略は偶発的なギャップではなく意図的なスコープ境界である。将来の形式化は、SHALL v2.0-final の正本、トレーサビリティ、トレーニング分離、および非破壊的フュージョンの不変条件を保存しなければならない。


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

committed で監査可能な状態に修復できない操作は、MUST committed として扱われるのではなく、`NeedsRepair` または `Quarantined` に遷移しなければならない。

### 41A.2 Ranking stability and replay discipline

GED / approximation boundary 付近の挙動は retrieval architecture 自体の変更理由ではなく、calibration / testing / replay discipline により監視・改善されるべき境界挙動である。v2.3 は graph embedding や新しい最適化器を導入しない。

RFC 準拠実装は、少なくとも pre-production calibration において、small structural perturbation、rename-only patch、edge-local modification、GED size threshold 近傍の candidate set に対する ranking drift を replayable trace と property-based test で観測できるようにすることが望ましい (SHOULD)。

Small structural perturbations SHOULD NOT cause unbounded ranking oscillation without being surfaced by calibration metrics, replay traces, or tests. ただし、この要求は retrieval architecture の normative shape を v1.5–v2.2 から変更するものではなく、あくまで calibration discipline を補強するものである。

### 41A.3 Training review load optional policy

Human-in-the-loop は v1.9 以降の中心規範であり、v2.3 においても変わらない。したがって、human review queue の負荷軽減は human review の否定ではなく、安全に限定された運用補助としてのみ許容される。v2.3-d では下層通信基盤として §12B HumanChannel 抽象が追加され、human review queue の各インタラクションは `HumanChannel::communicate()` + `InteractionHandle::wait()` により実装される。

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

## 41C. v2.3 Milestone and Calibration Addenda

### 41C.1 Milestone addendum

M-1, M0, M1 の testing plan は、可能であれば次を含むよう補強されるべきである。

- `GED_GRAPH_SIZE_LIMIT` 前後の candidate に対する replayable ranking drift test
- rename-only patch、single-edge patch、small compose perturbation に対する property-based ranking stability test
- startup repair scan の deterministic recovery test (`Pending -> retry -> Committed`, `Pending -> NeedsRepair`, `NeedsRepair -> Quarantined`)
- safe sandbox scope policy の audit completeness test
- M-0.5-4 HumanChannel の reconnect 回復可能性テスト (MetadataStore Pending 生存からの recover 完全サイクル検証)
- M-0.5-4 全 54 ユニットテスト + 2 観測テストの通過
- M-0.5-4 StdinoutChannel JSON Lines プロトコルの deterministic replay test

### 41C.2 Calibration addendum

付録 E の calibration candidate には、必要に応じて次を追加してよい。

- GED 境界付近でのランキング安定性スコア (ranking stability score near GED boundary)
- 反復的な refine/requery ループ下での発振感度 (oscillation sensitivity under repeated refine/requery loops)
- 成功した compose/reuse リカバリに対する false-new 率 (false-new rate vs successful compose/reuse recovery)
- 修復収束時間と quarantine エスカレーション率 (repair convergence time and quarantine escalation rate)
- レビュー負荷指標と安全スコープ内の auto-approval 利用率 (review-load indicators and safe-scope auto-approval utilization)
- HITL 通信レイテンシ分布（インタラクション解決時間の P50/P90/P99）(HITL communication latency distribution)
- HITL クラッシュリカバリ成功率（reconnect 成功数 / 総リカバリ試行数）(HITL crash recovery success rate)
- チャネルタイプ別 HITL インタラクション完了率（Resolved / 総数）(HITL interaction completion ratio per channel type)

### 41C.3 v2.3-f milestone addendum

v2.3-f の実装マイルストーンとして以下を追加する。

- **M0.x**: reciprocity pure function + unit tests (compute_direct_reciprocity, compute_indirect_reciprocity, recompute_reputation, compute_gc_hazard, compute_survival_probability, compute_helper_score)
- **M1.x**: replayable reputation/hazard recompute (ReciprocityEvent ingestion, policy-versioned recompute, snapshot comparison)
- **M2.x**: perturbation suite + ranking stability gate (small perturbation tests, village oscillation detection, flip rate bounds)
- **M3.x**: synthetic village simulator (child/adult population generator, mission stream generator, help interaction simulator, trust/reputation recompute loop, lifecycle/gc loop)
- **M4.x**: human-reviewed calibration rollout (candidate coefficient set generation, replay/simulation evaluation, diff report to human review queue, policy version update on approve)
- **M5.x**: Kind World calibration (KW1-KW4, §15.9.1-§15.9.4, §41B.20.9)
  - **M5.1**: Kind World condition constants definition (5 factor minimum gate threshold 0.6 + 8 threshold Safety Invariants + 2 village clustering constants + MagnificentSevenParams sweep ranges + 20 J_kw sub-component normative ranges)
  - **M5.2**: Ecosystem growth metrics computation (20 metrics: population_growth_rate, capability_coverage, reuse_ratio, cost_efficiency, benevolent_vs_non_benevolent_coverage_ratio, knowledge_diffusion_rate, mean_lifecycle_score, child_survival_rate, mean_freshness, mean_benevolence_aggregate, mean_reciprocity_score, help_success_rate, trust_inheritance_fidelity, execution_success_rate, mean_nest_depth, mean_node_density, cluster_coefficient, local_density, search_radius_inverse, reasoning_steps_inverse) + EcosystemGrowthObserver
  - **M5.3**: Village interaction metrics (cross_village_interaction_rate, village_formation_strength, knowledge_diffusion_rate, village_flow_balance, compute_village_health_score) + VillageInteractionObserver + assign_village_ids
  - **M5.4**: Kind World calibration runner (OFAT → grid sweep → confirmation n=5 seed change → t-test n=5, objectives: J_kw maximization, human-reviewed final update)

### 41C.4 v2.3-i milestone addendum

v2.3-i の実装マイルストーンとして、新設二重 Preset Registry アーキテクチャに対応する **M-0.65 Preset Registry 基盤** フェーズを追加する。M-0.65 は M-0.5 (HumanChannel) と M-1 (FakeImpl 基盤) の間に位置し、preset registry のデータ構造・検証手順・Event Architecture 拡張・起動時統合を担当する。

| フェーズ | 責務 | テスト観点 |
|---------|------|-----------|
| **M-0.65-a** | ArtifactOriginKind, RegistrySource, CapabilityFamily, PresetRootPolicy, PresetMetadata, PresetValidationReason, PresetValidationFailure の型定義 | 全 enum variant の網羅テスト、JSON シリアライズ/デシリアライズ |
| **M-0.65-b** | MemoizedGraph への5新規フィールド追加、GcState::Protected 追加 | 新規フィールド付き cold-start 初期化、Protected 状態の GC 除外検証 |
| **M-0.65-c** | BakedPresetRegistry + MutablePresetRegistry データ構造と基本操作（load/validate/get） | baked boot-fatal 条件テスト、mutable graceful degradation テスト |
| **M-0.65-d** | 12段階起動時検証手順の実装と逐次実行 | 各段階の失敗シナリオテスト (baked fatal 3種 + mutable quarantine 6種) |
| **M-0.65-e** | ResolvedWorkflowRegistry + 依存方向制約 + 名前空間予約 | baked→mutable 依存禁止検証、予約名衝突検出、baked 優先解決の確認 |
| **M-0.65-f** | DarviumEventKind::PresetRegistry + 5種 PresetRegistryEvent | 各イベント発行確認、quarantine 時の PresetQuarantined イベント確認 |
| **M-0.65-g** | startup repair scan への preset validation phase 前置統合 | 起動時処理順序の確定、diagnostic log 出力確認 |
| **M-0.65-h** | 新規定数 5 種の constants 定義 | 定数値の意味論検証 (Safety Invariant 3種 + Calibration Candidate 2種) |
| **M-0.65-i** | StructMem / Corpus2Skill root preset の BakedPresetRegistry 登録 (stub) | root preset 検出、GcState::Protected 設定、RegistrySource 紐付け |

**依存関係**: M-0.65 は M-1 (型定義基盤・FakeImpl) の上に構築されるが、M-0.5 (HumanChannel) とは独立である。M-0.65-f (EventBus 統合) は §12C Event Architecture の実装完了を前提とする。M-0.65-i は M-0.65-c (BakedPresetRegistry) 完了が前提。


## 42. 参照文献

- 既存 v1.9 の参照文献をそのまま継承する。
- 追加で、repository transformation / provenance-preserving merge / lineage-preserving knowledge integration に関する文献を将来補充してよい。


---
TITLE: Darvium RFC-0001 Unified Edition v2.3-c - 41B. チャイルドサポートビレッジと HELP 合意拡張 (Child Support Villages and HELP Consensus Extension v2.3-e)

## 41B. チャイルドサポートビレッジと HELP 合意拡張 (Child Support Villages and HELP Consensus Extension v2.3-e)

リビジョン v2.3-e は v2.3-c に対する厳密に追加的な拡張である。本拡張は、ワークフロー集団における動的局所性 (dynamic locality)、チャイルドサポートビレッジ (child-support village) 形成、アダルトからチャイルドへの HELP オファーと同意セマンティクス、および関連する安定性と動特性の較正規律を形式化する。本リビジョン SHALL NOT、v1.6 から v2.3-c で既に定義された WorkflowGraph、GraphVersion、TrustProfile、ライフサイクル状態 (Lifecycle state)、SearchTrace、正準知識 (canonical knowledge)、学習-本番分離 (training-production separation)、デュアルストア一貫性 (dual-store consistency)、または正当な SearchState 遷移の源泉真実 (source-of-truth) 所有権を再定義してはならない。デプロイメント (deployment) MAY 本拡張を完全に省略し、v2.3-c に準拠したままとしてもよい。実装される場合、本セクションの不変条件 (invariants) は規範的 (normative) である。

### 41B.1 適用範囲と不変条件 (Scope and invariants)

本拡張の目的は、未成熟なワークフローが成熟したワークフローから構造化された監査可能な支援を受けることを、既存の安全性または完全性保証を弱めることなく可能にすることである。本拡張は主に Training Plane のミッション生成、サンドボックス実行 (sandbox execution)、リプレイ解析 (replay analysis)、および較正 (calibration) を意図している。MAY 実行時ローカルの検索整形 (runtime-local retrieval shaping) に情報を提供してもよいが、SHALL NOT 規範的な適用可能性計算 (applicability computation)、トラストハードゲート (trust hard gates)、ナレッジハードゲート (knowledge hard gates)、デュアルストア許容性 (dual-store admissibility)、または本番プロモーション要件 (production promotion requirements) を黙示的に変更してはならない。

以下の不変条件は規範的 (normative) である。

1. `ConsistencyState::Pending`、`ConsistencyState::NeedsRepair`、または `ConsistencyState::Quarantined` にあるワークフロー MUST NOT 通常の REUSE、PATCH、COMPOSE、またはチャイルドサポート実行経路において実行可能なヘルパー (executable helper) として参加してはならない。
2. HELP によって誘発されるワークフローまたは知識の変更 (mutation) MUST 既に Training Plane アクティビティと知識変更プリミティブを統治しているのと同じサンドボックス、トラスト、適用可能性、監査、起点トレース (origin-trace)、プロモーション、修復の各規律に従わなければならない。
3. ビレッジおよび HELP メカニズム SHALL 決定論的入力下でリプレイ可能でなければならない。実装 MUST NOT リプレイトレース、ログ、またはシード付きポリシー状態で表面化できない隠れたランダム性に依存してはならない。
4. 本拡張 SHALL NOT 静的なクラスタ識別子を規範的なライフサイクル概念として導入してはならない。局所性 (locality) は継続的に更新される位置と導出される近傍 (neighborhood) によって定義され、固定されたクラスメンバーシップによって定義されるものではない。
5. 本拡張 SHALL NOT 単に候補が存在するという理由だけで HELP 実行を強制してはならない。候補選択、アダルトオファー (adult offer)、およびチャイルド決定 (child decision) は異なる段階である。

### 41B.2 空間位置埋め込み (Space position embedding)

各 `MemoizedGraph` MAY 低次元連続空間における現在の生態学的位置 (ecological position) を表す追加フィールドを持つことができる。このフィールドの目的は、局所性形成 (locality formation)、近傍観測 (neighborhood observation)、チャイルドサポートルーティング (child-support routing)、および較正である。これはタスク埋め込み (task embeddings)、ワークフロー設計埋め込み (workflow design embeddings)、グラフ構造比較 (graph-structural comparison)、または適用可能性スコアリング (applicability scoring) の代替ではない。

推奨される追加フィールドは以下の通りである:

```rust
struct MemoizedGraph {
    id: WorkflowGraphId,
    graph: WorkflowGraph,
    taskembedding: Vec<f32>,
    workflowdesigntext: String,
    workflowdesignembedding: Vec<f32>,
    agentsethash: u64,
    trust: TrustProfile,
    performance: Metrics,
    provenance: Provenance,
    lineage: WorkflowLineage,
    contributions: Vec<ContributionRecord>,
    lastvirtualseen: u64,
    experiencecount: u32,
    timedecay: TimeDecayProfile,
    reputation: ReputationProfile,
    gcstate: GcState,
    tombstoneref: Option<TombstoneRef>,
    consistencystate: ConsistencyState,
    repairepoch: u64,
    spacepositionembedding: Option<[f32; 3]>,
    spacepositionupdatedat: Option<SystemTime>,
}
```

上記のオプショナルな表現は、レガシーグラフおよびリプレイに対する後方互換性を維持するため推奨される。実装 MAY リプレイ可能性とグラフ ID への監査可能なリンクが維持される限り、このフィールドをランタイムメタデータの別の場所に具体化してもよい。

`spacepositionembedding` が存在しない場合、実装 MAY そのワークフローを局所性不明 (locality-unknown) として扱い、中立的な局所性動作にフォールバックしてもよい。隠れた非リプレイ可能な位置を捏造 (fabricate) してはならない MUST NOT。

推奨される更新則は、観測位置に対する指数平滑化 (exponential smoothing) である:

\[
x_{t+1}(G) = (1-\alpha)x_t(G) + \alpha p_t(G) \tag{41B-1}
\]

ここで \(x_t(G)\) は現在位置、\(p_t(G)\) はミッションコンテキストおよび相互作用履歴から導出される観測位置、\(\alpha\) は \(0 < \alpha \le 1\) のバージョン管理された較正候補 (versioned calibration candidate) である。

推奨される観測位置の分解は以下の通りである:

\[
p_t(G) = \lambda_q q_t(G) + \lambda_h h_t(G) + \lambda_k k_t(G), \qquad \lambda_q + \lambda_h + \lambda_k = 1 \tag{41B-2}
\]

ここで \(q_t(G)\) はミッションまたはクエリコンテキストの局所性、\(h_t(G)\) は相互作用から導出されるヘルパーまたはコラボレーターの局所性、\(k_t(G)\) は知識パターンの局所性である。正確な推定器は実装固有であるが、使用されるコンポーネント MUST 永続化されたミッション、トレース、監査状態、または決定論的再構成ルールからリプレイ可能でなければならない。

### 41B.3 チャイルド、アダルト、ローカルビレッジ (Child, adult, and local village)

本拡張の目的において、ワークフローが既存の経験値猶予期間 (Experience Grace Period) 規律で使用される設定された経験値フロア (configured experience floor) を下回っている場合、そのワークフローは**チャイルド (child)** である:

\[
\operatorname{Child}(G) \iff \operatorname{experiencecount}(G) < \operatorname{MINSURVIVALEXPERIENCE} \tag{41B-3}
\]

この定義は意図的に既存の v1.7 ライフサイクル保護ルールと整合しており、SHALL NOT 本 RFC の他の箇所で既に定義されている猶予期間中の GC 禁止を弱めてはならない。

ワークフローが経験値、トラスト、およびレピュテーションに関する実装宣言された成熟度しきい値 (maturity thresholds) を満たす場合、そのワークフローは**アダルト (adult)** である:

\[
\operatorname{Adult}(G) \iff
\bigl(E(G) \ge E_{adult}\bigr)
\land \bigl(T(G) \ge T_{adult}\bigr)
\land \bigl(R(G) \ge R_{adult}\bigr) \tag{41B-4}
\]

ここで \(E(G)\) は `experiencecount`、\(T(G)\) は `TrustProfile.composite`、\(R(G)\) は `ReputationProfile.finalscore` である。しきい値 \(E_{adult}, T_{adult}, R_{adult}\) は較正候補 (calibration candidates) であり、MUST NOT デプロイメント内で黙示的に変動してはならない。

2つのワークフロー \(G_i\) と \(G_j\) について、推奨される局所性距離は生態学的位置空間におけるユークリッドノルム (Euclidean norm) である:

\[
d_t(G_i,G_j)=\|x_t(G_i)-x_t(G_j)\|_2 \tag{41B-5}
\]

チャイルドワークフロー \(c\) の**ローカルビレッジ (local village)** は、静的なクラスではなく、アダルトの導出近傍 (derived neighborhood) である。規範的なデフォルト SHOULD 上位 k 件のアダルト近傍 (top-k adult neighbors) とする:

\[
N_t(c)=\operatorname{TopKAdultsByDistance}(c,k) \tag{41B-6}
\]

実装 MAY 代替として半径形式 (radius form) をサポートしてもよい:

\[
N_t(c)=\{G\mid \operatorname{Adult}(G) \land d_t(G,c) \le d_{max}\} \tag{41B-7}
\]

選択されたルールを満たすアダルトがいない場合、ランタイム MAY 検索境界を拡大するか非ローカルなサポート経路にフォールバックしてもよいが、MUST そのフォールバックをトレースまたはトレーニング監査メタデータに表面化しなければならない。

### 41B.4 5段階プロトコルとしての HELP (HELP as a five-stage protocol)

HELP は汎用的な再利用 (reuse)、パッチ適用 (patching)、または合成 (composition) と同義ではない。本拡張において HELP は、識別可能な5つの段階を持つ支援プロトコルである。子ワークフローが支援の主要な受益者である（子ワークフローがレシピエントとして選択される確率にバイアスが適用される）が、提案条件は特定の年齢方向に制限されない。

1. `HelpProposal`: システムがミッションに対する候補ヘルパー \(h\) とレシピエント \(r\) のペアを識別する。レシピエント選択時、子ワークフローには支援ニーズバイアス \(\beta_{child}\) が適用される（式 41B-9a）。
2. `HelpOffer`: ヘルパーが現在のポリシー制約下で支援意思を表明する。
3. `HelpDecision`: レシピエントがオファーされた支援を受け入れるか拒否する。
4. `HelpExecution`: 受け入れられたヘルパーが実際に実現された計画に参加する。
5. `HelpSuccess`: ミッションが成功し、測定可能な利益が得られる。

推奨される正式なオブジェクトは以下の通りである:

```rust
#[derive(Debug, Clone)]
struct HelpOffer {
    helpofferid: String,
    missionid: String,
    childgraphid: WorkflowGraphId,
    adultgraphid: WorkflowGraphId,
    createdat: SystemTime,
    offerstate: HelpOfferState,
    decidedat: Option<SystemTime>,
    similarityscore: f32,
    spatialdistance: f32,
    adulttrust: f32,
    adultreputation: f32,
    childneedscore: f32,
    proposedmode: HelpMode,
    rationale: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpOfferState {
    Pending,
    Accepted,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpMode {
    ReuseAsSubWorkflow,
    ComposeWithChild,
    PatchChild,
    DemonstrationOnly,
}
```

このオブジェクト MAY SQLite ランタイムメタデータ、Training Plane メタデータ、または別の監査可能なストアに永続化してもよい。既存のメタデータテーブル外に永続化する場合、実装 SHOULD 専用テーブルを定義し、ミッションリンク、アクターのトレーサビリティ、タイムスタンプ、およびリプレイ可視性を保持すべきである。

### 41B.5 HelpProposal（ヘルプ提案）

ヘルパー候補品質スコア MAY 次のように定義してもよい:

\[
Q(h,c,M)=w_s S(h,c)+w_t T(h)+w_r R(h)+w_n N(c) \tag{41B-8}
\]

ここで \(S(h,c)\) は局所性または適合性類似度、\(T(h)\) はトラスト、\(R(h)\) はレピュテーション、\(N(c)\) は支援ニーズである。

推奨される提案条件 (proposal condition) は以下の通りである。v2.3-f までの
\(\operatorname{Child}(c) \land \operatorname{Adult}(h)\) 制約は撤廃され、
任意の生存ペア間で HELP が発生し得る:

\[
\operatorname{HelpProposal}(h \to r \mid M)
\iff h \in N_t(r)
\land Q(h,r,M) \ge \theta_{proposal} \tag{41B-9}
\]

ここで \(r\) はレシピエント（支援受領者）であり、子ワークフローに限定されない。
\(h \in N_t(r)\) は \(h\) が \(r\) の近傍に存在することを示す。

提案生成時のレシピエント選択には、支援ニーズに基づく重み付き確率選択が適用される:

\[
P(r \text{ が選択される}) = \frac{\mathbb{1}[\operatorname{Child}(r)] \cdot \beta_{child} + \mathbb{1}[\operatorname{Adult}(r)] \cdot 1.0}
{\sum_{a \in alive} \left( \mathbb{1}[\operatorname{Child}(a)] \cdot \beta_{child} + \mathbb{1}[\operatorname{Adult}(a)] \cdot 1.0 \right)} \tag{41B-9a}
\]

ここで \(\beta_{child} \ge 1.0\) は子ワークフローの支援優先度を制御するバイアス係数
（デフォルト: \(\beta_{child}=2.0\)、Calibration Candidate）。
\(\beta_{child}=1.0\) の場合は人口構成比に等しい選択確率となる。
自己提案（\(h = r\)）は禁止される。

この段階は候補識別段階のみである。SHALL NOT 実行が許可されることを示唆してはならない。

### 41B.6 HelpOffer（ヘルプオファー）

ヘルパー MAY ランタイムが提案しても支援オファーを辞退できる。これにより、負荷、適合性、およびリスクに対するヘルパー側のポリシー制御が維持される。

推奨されるオファーポリシー (offer policy) は以下の通りである:

\[
O(h,r,M)=\mathbf{1}\{a_1Q(h,r,M)-a_2L_{load}(h)-a_3P_{risk}(M) \ge \theta_{offer}\} \tag{41B-10}
\]

`HelpOffer` は `HelpProposal` が真であり、かつヘルパー側のオファーポリシーが正を返す場合にのみ存在する:

\[
\operatorname{HelpOffer}(h \to r \mid M)
\iff \operatorname{HelpProposal}(h \to r \mid M)
\land O(h,r,M)=1 \tag{41B-11}
\]

オファーポリシーにおけるミッションリスク MUST SearchWorkflow、Training Plane、および知識変更経路について既に定義されている既存の安全性およびサンドボックスポリシーに従属し続けなければならない。

### 41B.7 HelpDecision（ヘルプ決定）: レシピエント同意 (recipient consent)

レシピエントワークフロー MAY オファーされた支援を受け入れるか拒否できる。この同意層は、HELP が一方的なヘルパー注入に陥るのを防ぐため、本拡張において規範的 (normative) である。子ワークフローがレシピエントとなる場合が最も典型的であるが、任意のワークフローがレシピエントとなり得る。

推奨される支援ニーズスコアは以下の通りである:

\[
N(c)=\gamma_1\bigl(1-\tilde{E}(c)\bigr)+\gamma_2\bigl(1-T(c)\bigr)+\gamma_3\bigl(1-L(c)\bigr) \tag{41B-12}
\]

ここで \(\tilde{E}(c)\) は正規化された経験値、\(T(c)\) は複合トラスト、\(L(c)\) はライフサイクルスコアである。

推奨される受入ポリシー (acceptance policy) は以下の通りである:

\[
\operatorname{Accept}(c,h,M)=\mathbf{1}\{b_1Q(h,c,M)+b_2U(c,M)-b_3A(c,h) \ge \theta_{accept}\} \tag{41B-13}
\]

ここで \(U(c,M)\) はミッション固有のニーズまたは不確実性、\(A(c,h)\) は自律性または不一致項であり、過去の拒否履歴、非互換性、または自己解決選好を組み込んでもよい。

拒否されたオファー MUST 実装が他の HELP 決定を永続化する場合、監査またはリプレイメタデータ内で可視性を維持しなければならない。拒否されたオファーの黙示的消失は、較正エビデンスを曖昧にするため推奨されない。

### 41B.8 HelpExecution（ヘルプ実行）

ヘルパーは、オファーが存在し、レシピエントがそれを受け入れ、最終的に実現された計画にヘルパーが含まれる場合にのみ、実際の支援実行に参加する。

\[
\operatorname{HelpExecution}(h \to r \mid M)
\iff \operatorname{HelpOffer}(h \to r \mid M)
\land \operatorname{Accept}(r,h,M)=1
\land h \in \Pi(M,c) \tag{41B-14}
\]

ここで \(\Pi(M,c)\) はチャイルド中心実行計画における最終ヘルパー集合である。ヘルパーは、実装ポリシーに従って、再利用サブワークフロー、合成参加者、パッチ提供元、またはデモンストレーション計画ソースとして出現してもよい。

本拡張 SHALL NOT 新たな正当な SearchState 遷移を生成してはならない。HELP 実行は、既存の検索 (retrieval)、評価 (evaluation)、合成 (composition)、および最終化 (finalization) 機構内部の許容性および計画整形層 (admissibility and plan-shaping layer) である。

### 41B.9 HelpSuccess（ヘルプ成功）と成長 (growth)

ヘルプイベントが成功するのは、実行が発生し、ミッションが成功し、チャイルドが測定可能な利益を得た場合のみである。

\[
\operatorname{HelpSuccess}(h \to c \mid M)
\iff \operatorname{HelpExecution}(h \to c \mid M)
\land \operatorname{MissionSuccess}(M)
\land \Delta G(c,M) \ge \theta_{growth} \tag{41B-15}
\]

推奨されるチャイルド成長集計量 (child-growth aggregate) は以下の通りである:

\[
\Delta G(c,M)=u_1\Delta \tilde{E}(c)+u_2\Delta T(c)+u_3\Delta L(c)+u_4\Delta S(c) \tag{41B-16}
\]

ここで各項はそれぞれ、正規化された経験値の増加、トラストの増加、ライフサイクルの改善、およびミッション品質または運用成功の増加に対応する。

HelpSuccess MAY チャイルド経験値、トレーニングフィードバック集計、ヘルパー互恵性エッジ、またはサポート局所性状態を更新してもよい。本番向けトラスト、プロモーションステータス、または正準知識への変更はすべて、既存のレビューおよびプロモーションルールに拘束されたままである。

### 41B.10 互恵性とレピュテーションの統合 (Reciprocity and reputation integration)

既存の `ReciprocityEdge` には既に `usefulcalls`、`harmfulcalls`、`composecount`、および `patchhelpcount` が含まれている。本拡張の実装 SHOULD 可能な限りその構造を再利用し、別個の非互換な互恵性モデルを導入しないようにすべきである。

成功したヘルプイベント SHOULD アダルトからチャイルドへの少なくとも1つの互恵性関連カウンターをインクリメントすべきである。パッチ提供 (patch donation) が有効なヘルプモードであった場合、`patchhelpcount` のインクリメントが推奨される。アダルトがパッチ提供なしで運用上有用な支援を提供した場合、実現されたモードに従って `usefulcalls` または `composecount` のインクリメントが推奨される。

推奨されるアダルトヘルプ品質スコア (adult help quality score) は以下の通りである:

\[
HScore(h)=\rho_1\operatorname{successrate}_{help}(h)+\rho_2\operatorname{acceptancerate}(h)+\rho_3\operatorname{childgrowthgain}(h) \tag{41B-17}
\]

実装 MAY そのようなスコアを `ReputationProfile.indirectscore` または `experiencescore` の再計算に供給してもよいが、SHALL マッピングを文書化し、リプレイ可能に保たなければならない。

### 41B.11 チャイルドサポート TrainingMission 特化 (Child-support TrainingMission specialization)

本拡張は主に既存の Training Plane 内に配置されるように設計されている。チャイルドサポートミッションは、追加のサポートメタデータを持つ通常の `TrainingMission` であり、独立した実行宇宙ではない。

推奨される追加ポリシーオブジェクトは以下の通りである:

```rust
#[derive(Debug, Clone)]
struct ChildSupportPolicy {
    enabled: bool,
    maxhelpers: u32,
    minadulttrust: f32,
    minadultreputation: f32,
    spatialtopk: u32,
    spatialmaxdistance: Option<f32>,
    offerrequired: bool,
    childacceptrequired: bool,
    allowremoteexplorationfraction: f32,
    helpgrowththreshold: f32,
    positionupdatealpha: f32,
}
```

推奨される追加ミッション拡張は以下の通りである:

```rust
#[derive(Debug, Clone)]
struct TrainingMission {
    missionid: String,
    missiontext: String,
    successcriteria: Vec<String>,
    sandboxpolicy: SandboxPolicy,
    source: MissionSource,
    childtarget: Option<WorkflowGraphId>,
    childsupportpolicy: Option<ChildSupportPolicy>,
}
```

この特殊化が使用される場合、ミッションは Training Plane に対して既に定義されているすべての既存の人間レビュー、自動承認例外ポリシー、安全なサンドボックス範囲、プロモーション規律、監査要件、およびリプレイ期待値に従うものとする。

### 41B.12 ヘルパー重み付けと制御された探索 (Helper weighting and controlled exploration)

チャイルドが1つ以上の受け入れ済みヘルパーを持った後、ランタイム MAY それらを局所性、トラスト、およびレピュテーションに従って重み付けしてもよい。推奨される正規化ヘルパー重み (normalized helper weight) は以下の通りである:

\[
w_t(h\mid c)=
\frac{\exp(-\beta d_t(h,c))\,T(h)^{\mu}R(h)^{\nu}}
{\sum_{g\in H_t(c)} \exp(-\beta d_t(g,c))\,T(g)^{\mu}R(g)^{\nu}} \tag{41B-18}
\]

ここで \(H_t(c)\) は受け入れ済みヘルパー集合である。

局所性ロックイン (locality lock-in) を避けるため、実装 MAY 制限付き遠隔探索コンポーネント (bounded remote exploration component) を混合してもよい:

\[
\tilde{w}_t(h\mid c)=(1-\varepsilon)w_t(h\mid c)+\varepsilon w^{remote}_t(h\mid c) \tag{41B-19}
\]

ここで \(\varepsilon\) はバージョン管理された較正候補である。遠隔探索が有効な場合、遠隔候補のソース MUST 依然としてトラスト、一貫性、サンドボックス、および許容性制約を尊重しなければならない。

### 41B.13 成功ヘルプ後の位置適応 (Position adaptation after successful help)

実装 MAY 成功した支援がチャイルドをその成功ヘルパーの局所性に向けて緩やかにシフトさせることを許可してもよい。推奨される更新は以下の通りである:

\[
x_{t+1}(c) \leftarrow (1-\eta)x_{t+1}(c) + \eta\sum_{h\in H_t(c)} w_t(h\mid c)x_t(h) \tag{41B-20}
\]

この項はオプションであり、SHOULD 小さく保つべきである。意図は緩やかな生態学的適応 (gradual ecological adaptation) であり、急激な局所性崩壊ではない。

### 41B.14 安定性と動特性の規律 (Stability and dynamicity discipline)

ビレッジの振る舞いは安定性だけで判断されてはならない。実行可能な生態系は、小さな摂動に対して局所的に安定し、より長い時間軸では地球規模で適応的であることが期待される。

推奨される位置ドリフト指標 (position-drift metric) は以下の通りである:

\[
\Delta_x(G,t)=\|x_{t+1}(G)-x_t(G)\|_2 \tag{41B-21}
\]

チャイルド \(c\) に対する推奨される短期ビレッジ重複指標 (short-horizon village overlap metric) は以下の通りである:

\[
J(c,t)=\frac{|N_t(c)\cap N_{t+1}(c)|}{|N_t(c)\cup N_{t+1}(c)|} \tag{41B-22}
\]

対応するビレッジチャーン指標 (village churn metric) は以下の通りである:

\[
V(c,t)=1-J(c,t) \tag{41B-23}
\]

実装 SHOULD ヘルパー重み付けがアクティブな場合、Jensen-Shannon ダイバージェンスなどのヘルパー重み上の分布ドリフト指標も追跡すべきである。

動特性 SHOULD より長い時間軸で評価されるべきである。チャイルド \(c\) に対する推奨されるトラスト成長勾配 (trust growth slope) は以下の通りである:

\[
g_T(c,t)=\frac{T_c(t+\Delta t)-T_c(t)}{\Delta t} \tag{41B-24}
\]

推奨される長期ビレッジ重複指標 (long-horizon village-overlap metric) は以下の通りである:

\[
J_{\tau}(c,t)=\frac{|N_t(c)\cap N_{t+\tau}(c)|}{|N_t(c)\cup N_{t+\tau}(c)|} \tag{41B-25}
\]

健全な動作は通常、高い短期重複と無視できない長期更新を示す。完全に凍結したビレッジは、短期的なチャーンが低くても望ましくない。

期間 \(H\) に対する推奨される最小成長条件 (minimum-growth condition) は以下の通りである:

\[
T_c(t_0+H)-T_c(t_0) \ge \delta_T,
\qquad
E_c(t_0+H)-E_c(t_0) \ge \delta_E \tag{41B-26}
\]

本セクションで使用される定数は較正候補である。MUST 黙示的に変動するのではなく、バージョン管理されなければならない。

### 41B.15 運用指標と較正候補 (Operational metrics and calibration candidates)

本拡張が実装される場合、以下の指標が測定、較正、またはシャドウ評価 (shadow evaluation) のために推奨される。

- `spacepositiondriftp50`, `spacepositiondriftp95`
- `villagechurnp50`, `villagechurnp95`
- `helperweightjsdp50`, `helperweightjsdp95`
- `helpofferacceptancerate`
- `helpexecutionrate`
- `helpsuccessrate`
- `childmaturationtimep50`, `childmaturationtimep95`
- `childtrustgrowthslopemean`, `childtrustgrowthslopep95`
- `longhorizonvillagejaccardp50`, `longhorizonvillagejaccardp95`
- `adultsupportentropy`
- `localitygain`
- 既存の `falsenewrate`、`composefallbackfrequency`、`reviewqueuedepth`、および `reviewlatency`（前後比較用）

推奨される較正候補 (calibration candidates) は以下を含む:

- `SPACEPOSITIONUPDATEALPHA`
- `VILLAGETOPK`
- `VILLAGEMAXDISTANCE`
- `ADULTEXPERIENCETHRESHOLD`
- `ADULTTRUSTTHRESHOLD`
- `ADULTREPUTATIONTHRESHOLD`
- `HELPPROPOSALTHRESHOLD`
- `HELPOFFERTHRESHOLD`
- `HELPACCEPTTHRESHOLD`
- `HELPGROWTHTHRESHOLD`
- `HELPREMOTEEXPLORATIONFRACTION`
- `VILLAGESTABILITYMAXCHURNP95`
- `VILLAGEDYNAMICITYMINLONGHORIZONCHANGE`

これらの値 SHALL 本 RFC の他の箇所で既に使用されているものと同じバージョン管理された較正規律に従って扱われなければならない。実装ローカルな黙示的チューニングは禁止される。

### 41B.16 リプレイ、摂動、プロパティベーステスト (Replay, perturbation, and property-based testing)

本拡張は v2.3 のテスト規律に従うものであり、それを置き換えるものではない。

準拠する実装 SHOULD 同一のシード入力および同一の永続化トレースの下で、少なくとも以下の出力に対する決定論的リプレイカバレッジを追加すべきである。

- 空間位置更新 (space position updates)
- ローカルビレッジメンバーシップ (local village membership)
- ヘルプ提案集合 (help proposal sets)
- アダルトオファー決定 (adult offer decisions)
- チャイルド受入/拒否決定 (child accept or reject decisions)
- 実現ヘルパー集合 (realized helper sets)
- ヘルプ成功結果 (help success outcomes)
- チャイルド成長指標 (child growth metrics)

準拠する実装 SHOULD 少なくとも以下の摂動に対する小摂動テスト (small-perturbation tests) を追加すべきである。

- 小さなミッション埋め込みノイズ (small mission-embedding noise)
- 小さなアダルトトラスト変動 (small adult trust variation)
- チャイルドに対する単一経験値インクリメント (single experience increment on a child)
- ヘルパーの名前のみ変更またはエッジローカルな構造変更 (helper rename-only or edge-local structural change)
- 一時的なヘルパー利用不可または隔離 (temporary helper unavailability or quarantine)

これらのテストの目的は、意味的に小さな変更の下での無制限なビレッジ振動または脆いヘルパー交代を検出することである。

準拠する実装 SHOULD 生成されたワークフロー集団、局所性分布、およびミッションストリームに対するプロパティベーステストも追加すべきである。推奨される性質 (properties) は以下を含む:

1. 非コミット整合性状態 (non-committed consistency state) のヘルパーがヘルプ経路で実行可能になることは決してない。
2. 猶予期間 (grace conditions) にあるチャイルドは、ヘルプ試行が失敗したという理由だけで GC されてはならない。
3. 短期摂動は、較正エビデンスを表面化することなく設定された境界を超えた無制約チャーンを生成してはならない。
4. 実行可能なアダルトが存在する場合、長期チャイルド成長が生成されたほぼすべての実行でゼロに崩壊してはならない。
5. 生成された生態系が縮退 (degenerate) していない限り、長期ビレッジ更新が生成されたほぼすべての実行で恒久的同一性に崩壊してはならない。

### 41B.17 推奨実装分割 (Recommended implementation decomposition)

以下の分割は推奨されるが必須ではない。

- `src/spaceposition.rs`: 局所性距離と位置更新 (locality distance and position updates)
- `src/village.rs`: チャイルド/アダルト分類、近傍選択、チャーン指標 (child and adult classification, neighborhood selection, churn metrics)
- `src/help.rs`: `HelpOffer`、状態遷移、オファー/受入ポリシー (state transitions, offer and accept policy)
- `src/childsupport.rs`: チャイルドサポートミッションオーケストレーションとヘルパー重み付け (child-support mission orchestration and helper weighting)
- `src/stability.rs`: 短期および長期の局所性指標 (short- and long-horizon locality metrics)
- `tests/village_replay.rs`: 決定論的リプレイテスト (deterministic replay tests)
- `tests/village_perturbation.rs`: 摂動テスト (perturbation tests)
- `tests/village_proptest.rs`: プロパティベーステスト (property-based tests)

### 41B.18 非目標と禁止事項 (Non-goals and prohibitions)

曖昧さを避けるため、本拡張は以下のいずれも定義または許可しない。

- ビレッジの規範的なライフサイクル識別子としての静的クラスタ ID (static cluster IDs)
- ポリシーでオファーが有効な場合の、アダルトオファーなしでの一方的ヘルパー実行 (unilateral helper execution without adult offer)
- ポリシーでチャイルド同意が有効な場合の、チャイルド同意なしでの一方的ヘルパー実行 (unilateral helper execution without child consent)
- 非コミットヘルパーの実行可能サポート経路への黙示的含込み (silent inclusion of non-committed helpers)
- 生のビレッジ近接性による規範的適用可能性方程式の直接書き換え (direct rewriting of the normative applicability equation)
- Training Plane のレビュー、サンドボックス、監査、プロモーション、またはデュアルストア修復ルールの迂回 (bypass)

### 41B.19 付属書の取扱い (Annex treatment)

本セクションで導入されるすべてのデフォルト数値 SHOULD 不変の定数としてハードコードされるのではなく、較正候補付属書 (calibration-candidate annex) に配置されるべきである。これには平滑化率、近傍サイズ、受入しきい値、成長しきい値、および安定性または動特性の境界が含まれる。本 RFC の以前の較正候補と同様に、これらのデフォルトに対する将来の変更 MUST 明示的かつバージョン管理され、リプレイまたは評価エビデンスを伴わなければならない。

### 41B.20 Reciprocity-Enhanced Helper Selection and Child Growth (v2.3-f)

v2.3-f は v2.3-e の HELP プロトコルを保持したまま、helper weighting への benevolence 明示的追加、child growth / maturation の数式化、helper proposal への reciprocal bias 導入を strictly additive に行う。既存の式 (41B-8) から (41B-26) を一切変更せず、新たな構成要素を追加する。

#### 41B.20.1 Helper weighting with benevolence

既存の helper quality score Q(h,c,M) (41B-8) に対し、v2.3-f では benevolence 項を追加する:

\[
Q(h,c,M)=w_s S(h,c,M)+w_t T(h)+w_r \operatorname{Rep}(h)+w_b B(h)+w_n N(c)-w_d d(h,c) \tag{F-11}
\]

ここで:
- S(h,r,M): mission 適合性 / locality suitability (既存 41B-8)。
- T(h): trust (既存)。
- \(\operatorname{Rep}(h)\): final reputation (既存)。
- B(h): benevolence score (v2.3-f 追加、式 F-3)。
- N(r): レシピエントの支援ニーズ (既存 41B-12、子ワークフローに限定されない)。
- d(h,r): local village 距離 (既存 41B-5)。

**意味**: 同程度に有能なヘルパーが複数いるなら、より協力的で評判の良いヘルパーを選ぶ。この項は既存の helper weighting (41B-18) を置き換えず、quality score の構成要素を拡張する。

#### 41B.20.2 Softmax helper selection

proposal 候補集合上の weighted softmax selection:

\[
\pi(h \mid c, M)=
\frac{\exp(\tau_Q Q(h,c,M))}{\sum_{g\in N_t(c)}\exp(\tau_Q Q(g,c,M))} \tag{F-12}
\]

- \(\tau_Q\): 選好の鋭さ (Calibration Candidate)。
- 高すぎると helper 固定化、低すぎると benevolence bias が薄まるため calibration candidate とする。

#### 41B.20.3 Benevolence-aware remote exploration

v2.3-e の bounded remote exploration (41B-19) を保持しつつ、local adults の benevolence が十分高い場合は remote exploration を下げ、local shortage 時にのみ上げる:

\[
\varepsilon_{remote}(c)=\operatorname{clip}_{[0,\varepsilon_{max}]}
\left(
\varepsilon_0 + a_1 \cdot \operatorname{need}(c) - a_2 \cdot \overline{B}_{local}(c)
\right) \tag{F-13}
\]

これにより「近くに優しい大人がいるなら、まず近所で助け合う」という世界観を operational に実現する。

#### 41B.20.4 Child growth increment

child workflow c の成長量:

\[
\Delta G_c = \mu_1 \cdot \operatorname{MissionSuccess}_c
+ \mu_2 \sum_h \operatorname{HelpSuccess}(h \to c)
+ \mu_3 \cdot \overline{B}_{helpers(c)}
- \mu_4 \cdot \operatorname{FailureBurden}_c \tag{F-14}
\]

これを experience_count や maturation score に反映してよい。

#### 41B.20.5 Maturation probability

child から adult への成熟判断が存在する場合、benevolence-rich village で成長しやすくする:

\[
P_{mature}(c)=\sigma\left(
\nu_0 + \nu_1 E_c^{norm} + \nu_2 T_c + \nu_3 \operatorname{Rep}_c + \nu_4 \overline{B}_{helpers(c)}
\right) \tag{F-15}
\]

**Intent**: 優しい大人に囲まれた child は成熟しやすい。Darvium の世界観を child support village の生態に直結させる。

#### 41B.20.6 Calibration guidance

v2.3-f が導入する較正パラメータ群に対する推奨初期値設定と調整ルールを以下に示す。Kind World 較正ループ (§15.9.1) では、これらのパラメータのうち 7 つを **MagnificentSevenParams** (gamma_benevolence, lambda_gc_base, direct_reciprocity_weight, indirect_reciprocity_weight, softmax_temperature, gc_interval, child_ratio) としてグループ化し、OFAT / grid sweep の sweep 対象とする。

**推奨初期値思想**:
- `theta_dir`, `theta_ind`: 0 より十分大きくし、互恵性が評判に有意に寄与することを確保する。
- `theta_exp`: 中程度に抑え、経験値だけで古参が固定的に有利にならないようにする。
- `gamma_benevolence`: 明確に正とし、benevolence が GC hazard を下げる方向を確実にする。
- `tau_helper_softmax`: 中程度に設定し、helper の固定化と benevolence bias のバランスを取る。
- `rho_direct_decay`: 緩やかに設定し、過去の善行がすぐに消失しないようにする。

**If-then calibration guide**:
- HelpSuccessRate が低い → w_b, theta_dir, theta_ind をやや増加し、benevolent helper を選びやすくする。
- VillageChurnP95 が高い → tau_helper_softmax を下げる、または locality smoothing を強める。
- 善良 workflow の survival 優位が弱い → gamma_benevolence を増やす。
- 古参固定化が起こる → theta_exp を下げ、kappa_E を小さくする。
- child が育たない → mu_2, mu_3, nu_4, gamma_child_protect を増やす。
- harmful helper が残る → alpha_d, beta_5 を増やし、harm penalty を強める。
- review-load が急増する → village-help proposal のしきい値を上げる、bounded remote exploration を抑える。

**Silent drift prevention**:
- policy object に version を付ける。
- SearchTrace / TrainingRunLog / RepairLog 相当の audit object に `policy_version` を残す。
- production での係数変更は audit log 必須とする。
- rollout は canary environment policy から始める。

#### 41B.20.7 Additional operational metrics

v2.3-e §41B.15 の metrics に加え、以下を監視する:

- `benevolence_score_p50/p95`
- `direct_reciprocity_p50/p95`
- `indirect_reciprocity_p50/p95`
- `reputation_final_p50/p95`
- `benevolent_survival_advantage`: benevolence 上位群と下位群の survival ratio 差
- `harmful_gc_rate`: harmful score 上位群がどれだけ早く GC されるか
- `helper_accept_rate`
- `help_abandon_rate`
- `child_survival_rate`
- `ranking_flip_rate_under_small_patch`
- `gc_hazard_drift_under_small_patch`

#### 41B.20.8 Testing discipline for reciprocity integration

本拡張は v2.3-e の testing discipline (§41B.16) を保持し、以下を追加する。

**Monotonicity tests (MUST)**:
- 他条件一定で `direct_score` が増加したら `survival_probability` は減少してはならない。
- 他条件一定で `indirect_score` が増加したら `GC hazard` は増加してはならない。
- 同能力の helper 間で benevolence が高い helper は proposal ranking で不利になってはならない。

**Replay test (MUST)**:
- 同一 event stream、同一 policy version、同一 VirtualClock なら `ReputationProfile` と `GC hazard` の再計算結果は一致すること。

**Perturbation test (SHOULD)**:
- 1 件の help success 追加で village 全体が崩壊的に並び替わらないこと。
- 1 helper の微小な trust change で helper set が全入れ替えしないこと。

**Property-based test (SHOULD)**:
生成対象: workflow population size、child/adult ratio、distance matrix、help event stream、harm/reject noise、policy coefficients。
検証性質: benevolence monotonicity、hazard non-negativity、probability boundedness、no negative reputation、no silent overflow/NaN、child in grace period is not GC'd regardless of temporary low reputation。

#### 41B.20.9 Kind World ecosystem calibration metrics

Kind World 較正ループ (§15.9.1, §41C.3 M5.x) の目的関数 $J_{kw}(\theta)$ の入力を提供するため、以下の追加メトリクスを定義する。

**EcosystemGrowthMetrics** — エコシステムの成長を 20 次元で計測（5 因子乗算結合モデルの全下位成分、RFC §15.9.2-§15.9.3）:

- `population_growth_rate`: (現在人口 - 前回人口) / max(前回人口, 1)
- `capability_coverage`: 能力空間の 10×10 グリッド量子化による Shannon 多様性指数、$H_{\max} = \log(100)$ で正規化
- `reuse_ratio`: 同一 workflow の再利用回数 / 全インタラクション数
- `cost_efficiency`: 1.0 - (失敗 + 放棄セッション数) / 全セッション数
- `benevolent_vs_non_benevolent_coverage_ratio`: 慈悲的集団 (上位 20%) / 非慈悲的集団 (下位 20%) の能力カバー率比
- `knowledge_diffusion_rate`: 村間 experience 分散の時間変化率（VillageInteractionMetrics から移動、s_density の入力）
- `mean_lifecycle_score`: 全個人の LifecycleScore $L(G)$ の算術平均（s_growth の入力）
- `child_survival_rate`: 子供の生存割合（s_growth の入力）
- `mean_freshness`: 全個人の BlendedFreshness $F_{time}$ の算術平均（s_growth の入力）
- `mean_benevolence_aggregate`: 全個人の慈悲総和 $B_i$ の算術平均（s_topology の入力）
- `mean_reciprocity_score`: 全個人の平均互恵性スコア（s_topology の入力）
- `help_success_rate`: 成功 HELP / 全 HELP セッション数（s_topology の入力）
- `trust_inheritance_fidelity`: 世代間信頼継承忠実度（s_topology の入力）
- `execution_success_rate`: 成功実行 step / 全実行 step 数（s_search の入力）
- `mean_nest_depth`: サブワークフローネスト深度の平均（s_density の入力、社会加速度定義②）
- `mean_node_density`: グラフノード密度（KW_ACCEL_NODE_DENSITY_MAX で正規化、s_density の入力、社会加速度定義②）
- `cluster_coefficient`: Watts-Strogatz 型大域クラスター係数（s_topology の入力、社会加速度定義③）
- `local_density`: 閾値半径内近傍割合（s_topology の入力、社会加速度定義③）
- `search_radius_inverse`: HELP 探索距離の逆数（s_search の入力、社会加速度定義④）
- `reasoning_steps_inverse`: 推論ステップ数の逆数 $1/(1+\text{steps})$（s_search の入力、社会加速度定義④）

**VillageInteractionMetrics** — 村形成と相互作用の健全性を計測:

- `village_count`: 生存村数
- `cross_village_interaction_rate`: 村間ヘルプセッション数 / 全セッション数
- `village_formation_strength`: silhouette 類似スコア、$[0, 1]$ 正規化
- `knowledge_diffusion_rate`: 村間 experience 分散の時間変化率
- `village_flow_balance`: 村 churn 率 (村間移動数 / 全生存数)
- `mean_village_size`: 平均村サイズ
- `village_size_variance`: 村サイズの分散
- `compute_village_health_score(formation_strength, flow_balance, cross_rate, diffusion_rate) -> f64`: (formation_strength + flow_balance_health + cross_rate + diffusion_rate) / 4。flow_balance_health は churn が $[KW\_VILLAGE\_CHURN\_LOWER, KW\_VILLAGE\_CHURN\_UPPER]$ 内なら 1.0、範囲外なら 0.0。

**KindWorldAssessment** 構造体 — $J_{kw}$ の評価結果を保持（5 因子乗算結合モデル RFC §15.9.2）:

```
struct KindWorldAssessment {
    is_kind_world: bool,          // J_kw > 0.8 && min(S_i) > 0.6
    j_kw: f64,                    // s_growth * s_density * s_topology * s_search * s_fairness
    // 5 因子値（社会加速度定義に基づく再構成）
    s_growth: f64,                // 社会加速度①: 人口増加
    s_density: f64,               // 社会加速度②: ワークフロー多層密度
    s_topology: f64,              // 社会加速度③: 空間クラスター
    s_search: f64,                // 社会加速度④: 探索効率
    s_fairness: f64,              // 構造的公平性
    // 20 下位成分 (diagnostics)
    j_pop_growth: f64,            // 旧 j_pop から名称変更
    j_lifecycle: f64,
    j_child_survival: f64,
    j_freshness: f64,
    j_cov: f64,
    j_diffusion: f64,
    j_reuse: f64,
    j_nest_depth: f64,            // 社会加速度② 対応 新規
    j_node_density: f64,          // 社会加速度② 対応 新規
    j_benevolence: f64,
    j_reciprocity: f64,
    j_help: f64,
    j_trust: f64,
    j_clustering: f64,            // 社会加速度③ 対応 新規
    j_local_density: f64,         // 社会加速度③ 対応 新規
    j_cost: f64,
    j_execution: f64,
    j_search_radius_inv: f64,     // 社会加速度④ 対応 新規
    j_reasoning_steps_inv: f64,   // 社会加速度④ 対応 新規
    j_penalty: f64,
    // 旧 8 二値フラグ (diagnostics, 較正条件からは廃止)
    legacy_flags: [bool; 8],
}
```

$J_{kw} > 0.8$ かつ $\min(s_{growth}, s_{density}, s_{topology}, s_{search}, s_{fairness}) > 0.6$ をもって Kind World 成立と判定する。旧来の全 8 条件フラグ方式は 5 因子最小値ゲートに置き換えられた（§15.9.2 参照）。この閾値は較正フェーズの目的関数として使用され、最終的な係数更新は human-reviewed でなければならない (MUST NOT auto-update to production)。

**$J_{kw}^{social}$ における時間効率の統合**:

$J_{kw}^{social}(\theta) = J_{kw}(\theta) \times s_{speed}$ （§15.9.2 定義）において、速度因子 $s_{speed}$ は tick_to_convergence から算出される:

- `tick_to_convergence`: $s_{growth} \times s_{density}$ の積が初めて 0.8 を超えた tick 数。閾値未到達の場合は `KW4_SIMULATION_TICKS` を記録する。
- $s_{speed} = 1.0 - \text{tick\_to\_convergence} / \text{KW4\_SIMULATION\_TICKS}$: 収束速度を $[0, 1]$ に正規化した値。$J_{kw}^{social}$ の第 6 因子として乗算結合に内包される。

tick_to_convergence の計装は KW4 較正ループ内で行われ、$s_{speed}$ への変換を経て目的関数の一部となる。

