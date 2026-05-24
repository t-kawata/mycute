# Darvium RFC-0001 改訂指示書 for v2.3-i
## 知識関連メカニズム全面改訂指示（StructMem / Corpus2Skill / Preset Registry / Knowledge Primitive 実装化）

本指示書は、`Darvium-RFC-0001-Unified-v2.3-final.md` を対象に、今回の設計議論で確定した知識関連メカニズムを RFC 本文へ矛盾なく、欠落なく、運用可能な仕様として反映させるための**改訂実施指示書**である。[file:2]

本指示書の目的は、別担当の改訂者が元会話を参照しなくても、Darvium における StructMem / Corpus2Skill の位置づけ、LadybugDB と SQLite を用いた内部実装方針、知識プリセットのレジストリ体系、起動時検査、信頼境界、ライフサイクル、検索・適用との関係までを完全に理解し、RFC を編集できる状態にすることである。[file:2]

## 1. 改訂の基本方針

現行 RFC は v1.8-final 以降で LadybugDB, StructMem, Corpus2Skill, Knowledge Primitive Registry, dual-store consistency, Conversational Knowledge Path を導入済みであり、知識生態系はすでに主要な設計対象として扱われている。[file:2]

しかし現行本文では、StructMem / Corpus2Skill は主として**理論上の知識機構・概念的追加項目**として語られており、それらを Darvium 内でどのような不変 root artifact として持ち、どのような workflow preset として配布し、どのように LadybugDB / SQLite / WorkflowRegistry に落とし込むかが十分に規定されていない。[file:2]

したがって改訂では、StructMem / Corpus2Skill を「概念参照」から「Darvium が内部で保持・検査・実行・継承・派生させる実装対象」へ昇格させることを最重要方針とする。[file:2]

## 2. 今回追加・明文化すべき設計結論

改訂者は、以下の設計結論を RFC の規範文に落とし込むこと。[file:2]

- StructMem / Corpus2Skill は md ファイル群を単なる自由記述で読む仕組みとしては実装しない。[file:2]
- StructMem / Corpus2Skill は LadybugDB の知識オブジェクトと SQLite の管理メタデータを用いた**内部知識機構**として実現する。[file:2]
- StructMem / Corpus2Skill の root capability は、通常のユーザー編集 artifact ではなく、Darvium platform 側が供給する**preset workflow / preset knowledge root**として扱う。[file:2]
- preset workflow には、ユーザーが編集可能な mutable registry と、ビルド時に焼き付けられ変更不能な baked registry の二層が必要である。[file:2]
- runtime が参照するのは単一の `WorkflowRegistry` だが、その供給源は baked + mutable の統合結果でなければならない。[file:2]
- 起動時には mutable preset directory を全走査し、JSON workflow を parse・schema validate・graph validate・cross-reference validate し、**完全合格したものだけ** IR として registry に採用しなければならない。[file:2]
- baked preset は必須であり、特に StructMem / Corpus2Skill などの platform root preset は起動成功条件に含めるべきである。[file:2]
- mutable preset はユーザー追加・編集を許可するが、invalid なものは registry に採用せず、reject / quarantine 相当の診断状態として扱う。[file:2]
- baked preset は mutable preset に依存してはならず、dependency direction は baked→baked, mutable→baked, mutable→mutable を許可し、baked→mutable を禁止する。[file:2]
- platform preset namespace は予約され、mutable 側から同一 `workflowid` による上書きは許可してはならない。[file:2]

## 3. RFC 全体での編集方針

改訂は局所追記ではなく、以下の 4 層にまたがる**横断改訂**として行うこと。[file:2]

| 改訂層 | 対象 | 改訂目的 |
|---|---|---|
| 概念層 | Abstract, Scope, Layer summary, revision history | StructMem / Corpus2Skill の実装上の位置づけを明文化する。[file:2] |
| 型・IR 層 | WorkflowGraph, WorkflowRegistry, validation, compiler, repository metadata | preset registry と validation, provenance, immutability を型として追加する。[file:2] |
| 知識層 | LadybugDB, Knowledge Primitive Registry, Conversational / Training / Fusion | StructMem / Corpus2Skill を知識 object と skill object の materialization ルールとして規定する。[file:2] |
| 運用層 | startup scan, consistency, repair, lifecycle, testing | 起動時検査・隔離・fatal 条件・監査ログ・回帰試験を明文化する。[file:2] |

この改訂は、既存仕様の否定ではなく、現行 RFC の v1.8-final 以降で導入された知識生態系・dual-store・Training Plane・Conversational Path を**実装可能な閉じた体系へ収束させる補完改訂**として記述すること。[file:2]

## 4. 新設すべき概念定義

RFC 内に少なくとも次の概念を新設すること。[file:2]

### 4.1 Preset Workflow

`Preset Workflow` を、配布時点で Darvium が既知の workflow graph artifact として定義すること。[file:2]

定義文では以下を明記すること。[file:2]

- preset workflow は `WorkflowGraph` の一種である。[file:2]
- preset workflow は `WorkflowRepository` の通常 artifact と異なり、**registry bootstrap source** でもある。[file:2]
- preset workflow は root workflow として他 workflow の `SubWorkflow` 参照先になりうる。[file:2]
- preset workflow は knowledge-oriented capability の提供単位でもありうる。[file:2]

### 4.2 BakedPresetRegistry

`BakedPresetRegistry` を、ビルド時にバイナリへ焼き付けられた immutable preset source として定義すること。[file:2]

定義文には以下を含めること。[file:2]

- ユーザーは変更できない。[file:2]
- platform-critical preset を保持する。[file:2]
- StructMem / Corpus2Skill root preset は原則ここに属する。[file:2]
- 起動時 validation 失敗は fatal である。[file:2]

### 4.3 MutablePresetRegistry

`MutablePresetRegistry` を、ファイルシステム上の preset directory からロードされる user-extensible preset source として定義すること。[file:2]

定義文には以下を含めること。[file:2]

- ユーザーは追加・編集・削除できる。[file:2]
- 起動時に全件検査される。[file:2]
- 合格 artifact のみ runtime registry に昇格する。[file:2]
- 不合格 artifact は reject / quarantine 診断対象であり、実行 lookup 面には出現しない。[file:2]

### 4.4 ResolvedWorkflowRegistry

runtime が compile / retrieval / composition / patch 評価で参照する最終 registry として `ResolvedWorkflowRegistry` を定義すること。[file:2]

これは既存 `WorkflowRegistry` の拡張または置換として導入してよいが、少なくとも以下の性質を持たせること。[file:2]

- baked と mutable を統合した単一 lookup 面である。[file:2]
- 各 `workflowid` について source provenance を保持する。[file:2]
- name collision policy を適用済みである。[file:2]
- validation 済み graph のみ含む。[file:2]

### 4.5 System Preset Root / Immutable Root / RootPinned

知識基盤 capability の root artifact を表す概念として、少なくとも次を導入すること。[file:2]

- `SystemPresetRoot`: platform が供給する root preset。[file:2]
- `ImmutableRoot`: 直接改変不能で lineage 上の基準点となる root。[file:2]
- `RootPinned`: GC や accidental replacement から保護される root。[file:2]

この 3 概念は別名でもよいが、**意味上の差異**は RFC 内で明確に書き分けること。[file:2]

## 5. StructMem / Corpus2Skill の位置づけ改訂

### 5.1 単なる理論参照から実装理論へ昇格

現行 RFC は LadybugDB の知識 object と primitive 群を列挙しているが、StructMem / Corpus2Skill をどう実装に落とすかは不十分である。[file:2]

改訂では、StructMem と Corpus2Skill を以下のように位置づけること。[file:2]

- StructMem は、会話・観測・文書・断片から durable memory object を形成し、`Fragment → MemoryEvent → MemoryConcept → CanonicalDocument` へと統合を進める**知識構造化理論**である。[file:2]
- Corpus2Skill は、corpus 上の chunk / entity / relation / procedure 記述から `SkillNode` とその下位構造を導出し、検索・展開・再利用可能な skill graph を構成する**技能化理論**である。[file:2]
- Darvium はこれらをテキスト解説としてではなく、LadybugDB object model と Workflow preset 群として内部実装する。[file:2]

### 5.2 StructMem の object 対応

StructMem を次の object 対応で規範化すること。[file:2]

| StructMem 機能 | LadybugDB object / relation | 補助メタデータ |
|---|---|---|
| raw observation / utterance memory | `Fragment` | conversational origin, trace, privacy, confidence。[file:2] |
| event-level consolidation | `MemoryEvent` | temporal stability, evidence count, contradiction score。[file:2] |
| concept formation | `MemoryConcept` | concept namespace, aliases, applicability, trace completeness。[file:2] |
| canonicalized memory artifact | `CanonicalDocument` | promotion state, provenance, review gate, supersession lineage。[file:2] |
| provenance tracing | `DERIVED_FROM`, `CONSOLIDATES`, `ABOUT_CONCEPT`, `SUPERSEDES` | SQLite-side consistency / audit / trust metadata。[file:2] |

改訂者は、現行の conversational ingestion / consolidation / promotion の節に、上記対応が StructMem の実装であると明示すること。[file:2]

### 5.3 Corpus2Skill の object 対応

Corpus2Skill を次の object 対応で規範化すること。[file:2]

| Corpus2Skill 機能 | LadybugDB object / relation | 補助メタデータ |
|---|---|---|
| corpus segmentation | `Chunk` | source span, canonical source id, extraction confidence。[file:2] |
| entity grounding | `Entity` | aliases, source trace, namespace。[file:2] |
| skill abstraction | `SkillNode` | skill kind, parent-child relation, applicability scope, evidence set。[file:2] |
| corpus to skill materialization | `COMPILED_TO_SKILL`, `MATERIALIZED_AS` | compile policy, revision, trust, review state。[file:2] |
| skill expansion / backtracking | skill primitives | retrieval and trace audit in SQLite。[file:2] |

Corpus2Skill は「文書を読む」機構ではなく、**corpus から reusable skill structure を抽出・固定化する Darvium 内部機構**として書き換えること。[file:2]

## 6. md ファイル依存の否定を明文化

改訂者は、本文の適切な箇所に次の規範を明記すること。[file:2]

- StructMem / Corpus2Skill の normative implementation は markdown file parsing ではない。[file:2]
- markdown や JSON はあくまで authoring / interchange / distribution form として利用しうるが、runtime の正本は LadybugDB / SQLite / validated Workflow IR である。[file:2]
- knowledge object の意味論はファイルレイアウトではなく object relation と policy により定義される。[file:2]

これは source-of-truth の節、および LadybugDB / SQLite の annex に追記すること。[file:2]

## 7. Registry アーキテクチャ改訂

### 7.1 二重 registry 構造を正式仕様化

`WorkflowRegistry` の節を改訂し、単純な `HashMap<WorkflowId, Arc<WorkflowGraph>>` の説明だけで終わらせず、その供給源が baked preset と mutable preset の合成であると明記すること。[file:2]

推奨記述内容は次の通り。[file:2]

- `BakedPresetRegistry` は immutable source である。[file:2]
- `MutablePresetRegistry` は startup-loaded source である。[file:2]
- `ResolvedWorkflowRegistry` はその両者を統合した runtime registry である。[file:2]
- compiler の `registry.get(workflowid)` は必ず resolved registry に対して行われる。[file:2]

### 7.2 依存方向制約

本文に明示的な dependency rule を追加すること。[file:2]

- baked preset は baked preset のみ参照してよい。[file:2]
- mutable preset は baked preset および mutable preset を参照してよい。[file:2]
- baked preset が mutable preset を参照することは MUST NOT とする。[file:2]

この規則は compile-time validation rule としても記載すること。[file:2]

### 7.3 namespace 予約

以下のいずれかの予約 namespace 制度を導入すること。[file:2]

- `platform.*` / `builtin.*` / `system.*` のいずれかを予約し、baked preset 専用 namespace とする。[file:2]
- mutable preset は `user.*`, `workspace.*`, `org.*` などの別 namespace を使う。[file:2]
- mutable source から予約 namespace の `workflowid` を提出した場合は validation failure とする。[file:2]

### 7.4 collision policy

同一 `workflowid` が複数 source から現れた場合の挙動を明文化すること。[file:2]

- baked 同士の重複は build defect であり fatal。[file:2]
- mutable 同士の重複は startup validation failure。[file:2]
- mutable が baked ID と衝突した場合は reject であり、silent override は MUST NOT。[file:2]

## 8. 起動時検査アルゴリズムを新設

起動時 preset load を規範手順として追加すること。[file:2]

### 8.1 手順全体

次の手順を numbered procedure として RFC に書くこと。[file:2]

1. baked preset source を展開する。[file:2]
2. baked preset を parse・validate する。[file:2]
3. baked critical preset が欠落または invalid なら boot failure とする。[file:2]
4. mutable preset directory を scan する。[file:2]
5. 各 JSON file を parse し candidate set を構成する。[file:2]
6. schema validation を行う。[file:2]
7. local graph validation を行う。[file:2]
8. cross-workflow reference validation を行う。[file:2]
9. registry policy validation を行う。[file:2]
10. 合格した mutable preset のみ accepted set に昇格する。[file:2]
11. baked + accepted mutable を統合して resolved registry を生成する。[file:2]
12. rejected preset は diagnostic log / event / projection に残す。[file:2]

### 8.2 validation taxonomy

ValidationError 節を拡張し、preset 専用エラー群を足すこと。[file:2]

少なくとも以下を含めること。[file:2]

- `InvalidPresetSchema`
- `DuplicateWorkflowId`
- `ReservedNamespaceViolation`
- `WorkflowNotFound`
- `CrossRegistryDependencyViolation`
- `CircularReference`
- `InvalidInputMapping`
- `OutputBindingMismatch`
- `BootCriticalPresetMissing`
- `BootCriticalPresetInvalid`
- `MutableOverrideForbidden`
- `PresetPolicyViolation`。[file:2]

既存 `CompileError` / `ValidationError` と関係づけ、creation-time validation と compile-time validation の責務分割も明記すること。[file:2]

### 8.3 rejected preset の扱い

rejected preset はただ捨てるのではなく、少なくとも以下を記録すること。[file:2]

- source path。[file:2]
- workflowid。[file:2]
- failure reason list。[file:2]
- detected time。[file:2]
- source class (`BakedPlatform`, `MutableUser`, `MutableWorkspace` など) 。[file:2]

これを `DarviumEvent` の `SystemEvent` または `KnowledgeEvent` 投影対象として残す指示を加えること。[file:2]

## 9. source-of-truth と dual-store の整理

改訂者は、Workflow artifact / Preset artifact / Knowledge artifact で正本の意味が異なることを整理して明記すること。[file:2]

### 9.1 workflow の正本

- validated runtime workflow は `ResolvedWorkflowRegistry` と `WorkflowRepository` により表現される。[file:2]
- mutable preset JSON は source artifact ではあるが、validation 前は正本ではない。[file:2]
- baked preset は distribution-time root source だが、runtime lookup は resolved registry を通す。[file:2]

### 9.2 knowledge の正本

- knowledge object の正本は LadybugDB relation graph + SQLite metadata である。[file:2]
- file artifact は import/export/authoring 形態にとどまる。[file:2]

### 9.3 consistency の接続

dual-store consistency で導入済みの `Committed / Pending / NeedsRepair / Quarantined` を、knowledge object と preset ingestion の両方に適用可能な運用概念として拡張する指示を書くこと。[file:2]

ただし preset source file 自体は repository pair object ではないので、**状態機械を厳密に共有するのではなく、運用意味論を準用する**と明記すること。[file:2]

## 10. StructMem / Corpus2Skill を preset root として扱う規範

StructMem / Corpus2Skill root preset の扱いを明文化する専用節を追加すること。[file:2]

### 10.1 root preset の性格

- platform knowledge capability の基底実装である。[file:2]
- baked registry に属する。[file:2]
- immutable である。[file:2]
- root-pinned である。[file:2]
- GC 対象外、または特別保護対象である。[file:2]

### 10.2 root からの派生

- root preset 自体は改変しない。[file:2]
- patch / compose / refine により生成された descendant workflow は通常の repository artifact として扱う。[file:2]
- descendant は lineage 上で root を祖先として保持する。[file:2]
- descendant の trust / lifecycle / GC は通常規則に従うが、必要に応じて inherited floor を持ちうる。[file:2]

### 10.3 知識 root と workflow root の関係

StructMem / Corpus2Skill では workflow root と knowledge root の両方が必要になりうるため、以下を規定すること。[file:2]

- workflow root: capability 実行 graph の root。[file:2]
- knowledge root: ontology / policy / skill taxonomy / consolidation rule などの知識 object root。[file:2]
- 両者は同一概念圏に属しても ID と lifecycle は分離しうる。[file:2]

## 11. JSON preset schema への追加指示

JSON authoring format を正式に許可する場合、preset metadata schema を新設すること。[file:2]

最低限必要なフィールド例を normative または strongly recommended として示すこと。[file:2]

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

上記は例示であり名称は調整してよいが、意味的には以下を必ず表現できるようにすること。[file:2]

- source class。[file:2]
- scope。[file:2]
- trust class。[file:2]
- boot criticality。[file:2]
- immutable / pinned 属性。[file:2]
- capability family (`StructMem`, `Corpus2Skill`, `General`, `TrainingOnly` など) 。[file:2]

## 12. WorkflowRepository / MemoizedGraph への追加メタデータ

現行 `MemoizedGraph` は trust, provenance, lineage, gcstate, consistencystate, topmetadata を持つ。[file:2]

改訂では以下の top-level metadata を追加すること。[file:2]

- `artifact_origin_kind` : `BakedPreset`, `MutablePreset`, `RepositoryDerived`, `TrainingDerived`, `ConversationalDerived` など。[file:2]
- `preset_source_info` : source path, embedded resource id, load epoch。[file:2]
- `root_policy` : immutable, pinned, protected, boot critical。[file:2]
- `capability_family` : StructMem / Corpus2Skill / Search / Training / General。[file:2]
- `registry_source` : resolved registry 内での source classification。[file:2]

さらに provenance に `sourceversion` だけでなく `presetlineage` または同等の識別情報を追加し、元 preset との関係を追跡可能にすること。[file:2]

## 13. Knowledge Primitive Registry の改訂指示

Knowledge Primitive Registry 節を改訂し、primitive 群が単なる API 群ではなく、StructMem / Corpus2Skill 実装の**基本操作集合**であることを明示すること。[file:2]

### 13.1 StructMem primitive 群

次の意味を持つ primitive として説明を強化すること。[file:2]

- `memorygetrecentevents`: recent memory event retrieval。[file:2]
- `memorygetconcepts`: concept lookup。[file:2]
- `memorygetconcepthistory`: concept lineage and supersession trace。[file:2]
- `memorytraceorigin`: fragment/event/document origin trace。[file:2]
- `memorypromotetodocument`: event/concept から canonical document への昇格。[file:2]

### 13.2 Corpus2Skill primitive 群

- `skilllistchildren`: skill hierarchy expansion。[file:2]
- `skillgetchunks`: supporting corpus chunk retrieval。[file:2]
- `skillexpandentities`: entity-based grounding expansion。[file:2]
- `skillbacktrack`: skill node から source evidence への逆追跡。[file:2]
- `kbhybridsearch`: graph + metadata + semantic hybrid retrieval。[file:2]

また、これらの primitive が baked root preset から呼び出される可能性があることを明記すること。[file:2]

## 14. Conversational Knowledge Path との整合

Conversational Knowledge Path の節に、今回の StructMem 実装化との接続を追記すること。[file:2]

### 14.1 Fragment 生成の位置づけ

会話から生成される `Fragment` は StructMem pipeline の最下層 object であることを明記すること。[file:2]

### 14.2 Consolidation の意味

`ConsolidationCandidateSet` と `ConsolidationPolicy` は、StructMem における `Fragment → MemoryEvent / MemoryConcept / CanonicalDocument` の昇格規則を担うことを明記すること。[file:2]

### 14.3 Promotion gate

Conversational promotion gate は training-only artifact を即 production memory にしないための gate であり、StructMem の durable memory 化における human / policy review の一部であると明記すること。[file:2]

## 15. Training Plane との整合

Training Plane 節にも以下を反映すること。[file:2]

- training から生じた knowledge mutation や workflow mutation は sandbox namespace に留める。[file:2]
- StructMem / Corpus2Skill の baked root preset 自体は training で変更してはならない。[file:2]
- root から派生した candidate workflow / candidate knowledge document のみが promotion 対象となる。[file:2]
- training trust と production trust の分離は root preset に対しても維持される。[file:2]

## 16. GMR Retrieval / SearchWorkflow との整合

改訂者は、knowledge-aware candidate evaluation の節に以下を追記すること。[file:2]

- StructMem / Corpus2Skill preset は retrieval candidate になりうるが、root preset は通常の ad hoc workflow と同じ扱いではない。[file:2]
- root preset は patch / compose の祖先テンプレートとして利用できる。[file:2]
- reusable descendant を選ぶ際、candidate evaluation は lineage 上の root family と capability family を参照してよい。[file:2]
- query design / applicability の評価で、knowledge capability family は top-level metadata filter の一部になりうる。[file:2]

## 17. Lifecycle / GC の改訂指示

root preset とその派生 artifact の扱いを区別するため、Lifecycle 節を改訂すること。[file:2]

### 17.1 root preset

- `GcState` の通常遷移対象から除外するか、特別な `Protected` 扱いを導入する。[file:2]
- 少なくとも immutable root / root-pinned artifact は soft delete / hard delete candidate に自動遷移してはならない。[file:2]

### 17.2 derived descendant

- descendant artifact は通常 GC の対象だが、root lineage のみを理由に保護してはならない。[file:2]
- 保護は lineage ではなく root policy / pin policy / current utility / trust によって決まると明記すること。[file:2]

## 18. Event / Audit / Repair の改訂指示

Event Architecture 節に、preset load / validation / rejection / boot failure をイベント化する指示を追加すること。[file:2]

少なくとも次を event kind または subtype として扱えるようにすること。[file:2]

- `PresetRegistryScanStarted`
- `PresetValidated`
- `PresetRejected`
- `PresetBootCriticalFailure`
- `PresetRegistryResolved`

さらに RepairLog と関連づけ、mutable preset の validation failure を repository pair の `NeedsRepair` と同列に扱うのではなく、**configuration-plane diagnostic** として区別することを明記すること。[file:2]

## 19. Source text 改訂箇所の具体指示

以下の章・節は必ず改訂すること。[file:2]

1. 冒頭 Abstract / revision history / in-scope summary。[file:2]
2. Layer overview の表。[file:2]
3. Layer 2 Workflow IR の `WorkflowRegistry`, validation, compiler 周辺。[file:2]
4. WorkflowRepository / MemoizedGraph の metadata 定義。[file:2]
5. Knowledge Primitive Registry。[file:2]
6. LadybugDB / SQLite annex。[file:2]
7. Training Plane。[file:2]
8. Conversational Knowledge Path。[file:2]
9. Lifecycle GC。[file:2]
10. Event Architecture / Repair / Operational Clarifications / Milestone addendum。[file:2]

## 20. 既存本文への具体的編集方法

改訂者は、単なる追記だけでなく、次の編集を行うこと。[file:2]

- 「StructMem / Corpus2Skill を追加」とだけ書いている revision note は、**preset root + knowledge object implementation + registry architecture**まで含む説明に差し替える。[file:2]
- `WorkflowRegistry` の単純定義は、resolved registry architecture を反映するよう書き換える。[file:2]
- `WorkflowNotFound` など既存 compile error は、preset startup validation と compile-time residual error の二段構えとして位置づけ直す。[file:2]
- source-of-truth の節に、authoring file と runtime truth の違いを追記する。[file:2]

## 21. 改訂後に RFC が満たすべき読了要件

改訂後の RFC は、読者が以下を迷わず理解できる状態でなければならない。[file:2]

- StructMem / Corpus2Skill は何か。[file:2]
- それが LadybugDB / SQLite のどの object と relation で表現されるか。[file:2]
- それが workflow preset とどう接続されるか。[file:2]
- なぜ baked registry と mutable registry の二層が必要か。[file:2]
- startup validation は何を検査し、何が fatal で、何が reject か。[file:2]
- root preset はなぜ immutable / pinned なのか。[file:2]
- 派生 artifact はどこまで通常 lifecycle に従うのか。[file:2]
- markdown/JSON source はどの位置づけで、runtime の正本は何か。[file:2]

## 22. 追加すべき規範文の例

改訂者が RFC 本文に流用しやすいよう、以下の規範文の趣旨を明確に反映すること。[file:2]

- Darvium SHALL implement StructMem and Corpus2Skill as internal knowledge-capability mechanisms materialized through LadybugDB knowledge objects, SQLite-side metadata, and validated workflow presets, and SHALL NOT treat markdown authoring files as the normative runtime representation.[file:2]
- Darvium SHALL maintain both an immutable baked preset registry and a mutable startup-loaded preset registry, and SHALL construct the runtime workflow lookup surface from their validated union.[file:2]
- Platform-critical knowledge presets, including StructMem and Corpus2Skill root presets, MUST reside in the baked preset registry and MUST cause startup failure if absent or invalid.[file:2]
- Mutable user-supplied preset workflows MAY be added or modified by users, but MUST be admitted into the runtime registry only after complete startup validation succeeds.[file:2]
- A baked preset MUST NOT depend on a mutable preset.[file:2]
- A mutable preset MUST NOT override a reserved platform preset identifier.[file:2]
- Immutable root presets MUST NOT be mutated in place; all adaptation SHALL occur through derived descendants with preserved lineage.[file:2]

## 23. 推奨する新設データ型

RFC に例示コードを足すなら、少なくとも次のような型を追加することを推奨する。[file:2]

```rust
#[derive(Debug, Clone)]
enum RegistrySource {
    BakedPlatform,
    MutableUser,
    MutableWorkspace,
}

#[derive(Debug, Clone)]
struct PresetRootPolicy {
    immutable_root: bool,
    root_pinned: bool,
    boot_critical: bool,
    capability_family: CapabilityFamily,
}

#[derive(Debug, Clone)]
enum CapabilityFamily {
    StructMem,
    Corpus2Skill,
    Search,
    Training,
    General,
}

#[derive(Debug, Clone)]
struct PresetValidationFailure {
    workflowid: Option<WorkflowId>,
    source: RegistrySource,
    source_path: Option<String>,
    reasons: Vec<PresetValidationReason>,
    detected_at: SystemTime,
}

#[derive(Debug, Clone)]
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
```

名称は変更してよいが、意味論は維持すること。[file:2]

## 24. 改訂禁止事項

改訂者は以下を行ってはならない。[file:2]

- StructMem / Corpus2Skill を再び「将来検討」レベルへ後退させること。[file:2]
- baked / mutable の二層 registry を曖昧な実装メモに留めること。[file:2]
- root preset の immutability と lineage-based derivation を省略すること。[file:2]
- markdown source を runtime truth のように書くこと。[file:2]
- startup validation を optional best effort のように弱めること。[file:2]
- mutable preset による baked preset override を許容すること。[file:2]

## 25. 改訂完了条件

改訂作業は、少なくとも次の条件を満たしたときに完了と見なすこと。[file:2]

- RFC 内で StructMem / Corpus2Skill の内部実装位置づけが明確である。[file:2]
- LadybugDB / SQLite / WorkflowRegistry / Training / Conversational / Lifecycle の各章に整合的な記述が入っている。[file:2]
- baked preset registry と mutable preset registry の役割が明文化されている。[file:2]
- startup validation 手順と fatal / reject 条件が明文化されている。[file:2]
- root preset と descendant artifact のライフサイクル差が明文化されている。[file:2]
- source-of-truth と authoring format の違いが明文化されている。[file:2]
- 新設語彙が glossary 的に理解可能な粒度で説明されている。[file:2]

以上を満たす改訂により、Darvium RFC-0001 は知識関連メカニズムについて、概念記述から実装指向仕様へと遷移できる。[file:2]
