# 開発フェーズ24：多言語対応（英語/日本語）の完全実装

## 目的
Absorb, Query, Memify の各操作において、リクエストパラメータ `is_en` (boolean) によって英語モードと日本語モードを完全に切り替えられるようにします。これに伴い、ハードコードされている言語設定、言語ごとのプロンプトの分離、および言語別に分かれてしまっているクエリタイプの統合を行います。

## 概要
1.  **リクエストパラメータの追加**: `Absorb`, `Query`, `Memify` のリクエストに `is_en` を追加。
2.  **プロンプトの多言語化**: `prompts.go` 内の日本語専用プロンプトに対し、対となる英語用プロンプトを作成し、`IsEn` フラグで切り替えられるようにする。
3.  **タスク/ツールの改修**: 各パイプラインタスク (`GraphExtraction`, `Summarization`, `Memify`) およびクエリツールにおいて、`IsEn` フラグを受け取り、適切なプロンプトを選択するように変更。
4.  **クエリタイプの統合**: 英語/日本語で別IDになっていたクエリタイプを統合し、出力言語は `is_en` フラグで制御する。

---

## 重要な実装指針 (Critical Guidelines)

### 1. プロンプト言語の原則
- **命令は常に英語**: 日本語モードのプロンプトであっても、LLMへの**指示文（System Prompt等）は全て英語で記述**しなければなりません。「ユーザーへの最終出力のみを日本語にする」という指示を英語で記述してください。
- **思考プロセスの英語化**: 多段のLLM処理を行うパイプラインにおいて、中間生成物や思考プロセス（Reasoning）は**全て英語**で行わせてください。最終的なユーザー向け出力を行う最後のステップでのみ、日本語での出力を指示してください。

### 2. 知識グラフ生成における言語保持 (Cognify)
- **入力言語の尊重**: `Absorb` -> `Cognify` プロセスにおける知識グラフ生成（エンティティ抽出）では、**入力されたテキストの言語をそのままノードIDやプロパティに使用**してください。
- **翻訳の禁止**: 日本語の入力に対して、勝手に英語に翻訳してグラフ化することは**厳禁**です。同様に、英語入力は英語のままグラフ化してください。言語間の不必要な変換は情報の劣化を招くため避けてください。

### 3. 計算されたプロンプトの品質保持
- **既存プロンプトの尊重**: `Absorb` および `Query` で現在使用されているプロンプトは、既に高度にチューニングされています。多言語化の際、これらのプロンプトの指示内容やニュアンスを勝手に簡略化したり削除したりしないでください。
- **複製と調整**: 日本語用プロンプトを英語用に移植する際は、チューニングされたロジックを維持したまま、出力言語の指定部分のみを慎重に変更してください。`Memify` についてはチューニング途上ですが、`Absorb`/`Query` は特に注意が必要です。

---

## 実装詳細

### 1. リクエストパラメータの更新

`src/mode/rt/rtreq/cubes_req.go` の各リクエスト構造体に `IsEn` フィールドを追加します。

#### [MODIFY] [cubes_req.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rtreq/cubes_req.go)

```go
type AbsorbCubeReq struct {
    // ... 既存フィールド ...
    IsEn bool `json:"is_en"` // デフォルト false (日本語)
}

type QueryCubeReq struct {
    // ... 既存フィールド ...
    IsEn bool `json:"is_en"` // デフォルト false (日本語)
}

type MemifyCubeReq struct {
    // ... 既存フィールド ...
    IsEn bool `json:"is_en"` // デフォルト false (日本語)
}
```
※ `binding` タグは不要（boolのfalseがゼロ値のため、指定なしでfalseになる挙動で問題ないため）。

---

### 2. ハンドラーとBLの接続

ハンドラーからBLへ `IsEn` を渡すように変更し、BL内のダミー実装 `isEn := false` を削除します。

#### [MODIFY] [cubes_handler.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rthandler/hv1/cubes_handler.go)
- リクエストバインド後、BL呼び出し時に `req` がそのまま渡されるので変更なし（reqの中身が変わるだけ）。

#### [MODIFY] [cubes_bl.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rtbl/cubes_bl.go)
- `AbsorbCube`, `QueryCube`, `MemifyCube` 内の `isEn := false` を削除。
- `u.CuberService.Absorb(...)` などの呼び出し時に `req.IsEn` を渡す。
- `summary` 生成ロジック（"Knowledge absorption... completed" 等）の `if isEn` 分岐は `req.IsEn` を使用するように変更。

---

### 2.5. Service層 (cuber.go) の改修

`src/pkg/cuber/cuber.go` において、受け取った `isEn` フラグを各タスクやツールに確実に伝播させるための変更を行います。

#### [MODIFY] [cuber.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/cuber.go)

1.  **Absorb / Cognify**:
    - `cognify` メソッドから `summarization.NewSummarizationTask` を呼び出す際、`isEn` を渡すように変更します（後述のタスク改修でファクトリ関数のシグネチャ変更が必要）。
    - 既存の `graph.NewGraphExtractionTask` への `isEn` 渡しは維持します。

2.  **Query**:
    - `Query` メソッド内で `queryConfig` に `isEn` をセットしてから `queryTool.Query` を呼び出します。
    - `query.NewGraphCompletionTool` は `isEn` を受け取らないため、実行時の `queryConfig` 経由で渡す形になります（後述の `types.QueryConfig` 改修を参照）。

    ```go
    // cuber.go Query method
    // ...
    queryConfig.IsEn = isEn // queryConfig に IsEn を注入
    answer, chunks, summaries, graph, embedding, qusage, err = queryTool.Query(txCtx, text, queryConfig)
    // ...
    ```

3.  **Memify**:
    - `Memify` メソッドは既に `isEn` を受け取っていますが、内部で呼び出すサブメソッド群へ `isEn` を伝播させる必要があります。
    - 以下のメソッドチェーン全てに引数 `isEn` を追加し、最終的にタスクのファクトリ関数へ渡します。
        - `attemptToResolveUnknown` -> `metacognition.NewSelfReflectionTask` へ `isEn` を渡す。
        - `executeMemifyCore` -> `memifyBulkProcess` / `memifyBatchProcess`
        - `memifyBulkProcess` -> `memify.NewRuleExtractionTask` へ `isEn` を渡す。
        - `memifyBatchProcess` -> `memify.NewRuleExtractionTask` へ `isEn` を渡す。

---

### 3. プロンプトの整備

`src/pkg/cuber/prompts/prompts.go` におけるプロンプト定義を根本的に見直し、言語ごとの明確な分離と、厳格な品質基準に基づいた再定義を行います。既存の `_JA` プロンプトは維持し、対となる `_EN` プロンプトを慎重に作成してください。

#### [MODIFY] [prompts.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/prompts/prompts.go)

**重要な定義ルール（厳守すること）:**

1.  **指示言語の一貫性 (All Instructions in English)**:
    -   `_JA` (日本語モード用) のプロンプトであっても、LLMへの**システム指示、タスク説明、制約事項は全て英語で記述**しなければなりません。
    -   日本語モードと英語モードの違いは、「**ユーザーへの最終出力（Final Output）の言語**」のみです。
    -   例: `_JA` 版では "Your final output MUST be in JAPANESE." と指示し、`_EN` 版では "Your final output MUST be in ENGLISH." と指示します。

2.  **思考プロセスの言語 (English Reasoning)**:
    -   CoT (Chain of Thought) や多段パイプラインの中間ステップでは、**思考（Thinking/Reasoning）および中間生成物は全て英語**で行わせてください。
    -   日本語モードであっても、内部的なロジック処理や推論を日本語で行わせないでください（一般にLLMは英語での論理推論能力が最も高いため）。
    -   **最終的なユーザー向けの回答生成ステップにおいてのみ**、日本語への変換（または日本語での出力）を要求してください。

3.  **既存プロンプトの品質維持 (Preserve Optimized Logic)**:
    -   現在 `Absorb` / `Query` で使用されているプロンプトは、特定の指示セットや制約（Schema Discovery Guidelines等）によって高度にチューニングされています。
    -   `_EN` 版を作成する際は、これらの「ロジック部分」を削除・簡略化せず、忠実に維持してください。単なる「翻訳よろしく」といった単純なプロンプトへの置き換えは品質低下を招くため厳禁です。

4.  **知識グラフ生成における言語保持 (Language Preservation in Cognify)**:
    -   `GENERATE_GRAPH_PROMPT` 系（知識抽出）においては、「**入力テキストの言語をそのまま保持する**」ことが最重要です。
    -   日本語のテキストが入力された場合 -> 日本語のノードID/プロパティを生成（勝手に英語に翻訳しない）。
    -   英語のテキストが入力された場合 -> 英語のノードID/プロパティを生成。
    -   `IsEn` フラグは「グラフ構造の抽出ロジック（文法解析のヒント等）」を切り替えるために使用しますが、「出力言語を強制的に変換する」ために使用してはいけません。

**具体的な定数定義の変更リスト:**

以下のプロンプトについて、`_JA` と `_EN` のペアを定義します。

**A. グラフ抽出 (Graph Extraction)**
*   `GENERATE_GRAPH_JA_PROMPT`: 既存の `GENERATE_GRAPH_PROMPT` をリネーム。日本語特有の文法ガイドライン（助詞、ゼロ代名詞処理など）が含まれています。
*   `GENERATE_GRAPH_EN_PROMPT`: 新規作成。
    *   ベースは `_JA` 版の Core Concepts や Schema Discovery Guidelines を使用。
    *   "Japanese-Specific Extraction Guidelines" セクションを、英語テキストおよび一般的な抽出ガイドライン（"General Extraction Guidelines"）に置き換えるか、あるいは削除して汎用的なルールにする。
    *   Few-Shot Examples を日本語の例から英語の例に差し替える。
    *   **重要**: "Preserve original language" のルールは維持し、入力が英語なら英語のまま出力させることを徹底する。

**B. 要約 (Summarization)**
*   `SUMMARIZE_CONTENT_JA_PROMPT`: 既存の `SUMMARIZE_CONTENT_PROMPT` をリネーム。"Analyze and reason... in English", "Output... in Japanese" を確認。
*   `SUMMARIZE_CONTENT_EN_PROMPT`: 新規作成。出力指示を "Output... in English" に変更。

**C. 検索・回答 (Query & Answer)**
*   `ANSWER_QUERY_WITH_HYBRID_RAG_PROMPT` -> `_JA` (既存) / `_EN` (新規)
*   `SUMMARIZE_GRAPH_ITSELF_PROMPT` -> `_JA` (既存) / `_EN` (新規)
*   `SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_PROMPT` -> `_JA` (既存) / `_EN` (新規)
*   `ANSWER_SIMPLE_QUESTION_PROMPT` -> `_JA` (既存) / `_EN` (新規)
    *   これらは全て「思考は英語、出力は指定言語」のルールを適用。

**D. 自己強化 (Memify) / メタ認知 (Metacognition)**
*   `RuleExtractionSystemPrompt` -> `RULE_EXTRACTION_SYSTEM_PROMPT_JA` / `_EN`
*   `UnknownDetectionSystemPrompt` -> `UNKNOWN_DETECTION_SYSTEM_PROMPT_JA` / `_EN`
*   `CapabilityGenerationSystemPrompt` -> `CAPABILITY_GENERATION_SYSTEM_PROMPT_JA` / `_EN`
*   `QuestionGenerationSystemPrompt` -> `QUESTION_GENERATION_SYSTEM_PROMPT_JA` / `_EN`
*   `KnowledgeCrystallizationSystemPrompt` -> `KNOWLEDGE_CRYSTALLIZATION_SYSTEM_PROMPT_JA` / `_EN`
*   `EdgeEvaluationSystemPrompt` -> `EDGE_EVALUATION_SYSTEM_PROMPT_JA` / `_EN`
    *   これらはまだ実験段階ですが、出力フィールド（"text"など）の言語を指定言語に合わせるよう指示を変更してください。

実装手順:
1.  新しい定数ペア (`_JA`, `_EN`) を `prompts.go` に全て定義する。
2.  `GENERATE_GRAPH_EN_PROMPT` の中身（英語用ガイドラインとFew-Shot）を作成する。
3.  ビルドエラーを防ぐため、一時的に旧定数名（`GENERATE_GRAPH_PROMPT`等）を `_JA` へのエイリアスとして残すか、一括置換で対応する。

---

### 4. タスク/ツールの改修

多段パイプライン処理において、「思考プロセスは英語」「最終出力のみ指定言語」という原則を徹底するための実装を各コンポーネントに行います。これには、**タスク構造体への `IsEn` フィールド追加と、ファクトリ関数（New...）の引数変更**が含まれます。

**共通実装パターン**:
すべての LLM 呼び出しにおいて、単純に `IsEn` フラグでプロンプトを切り替えるだけでなく、**プロンプト自体が「English for Reasoning, Target Language for Output」の構造になっていること**を前提として実装します。

#### [MODIFY] [graph_extraction_task.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/tasks/graph/graph_extraction_task.go)

**変更内容**:
- 構造体 `GraphExtractionTask` とファクトリ関数 `NewGraphExtractionTask` は既に `isEn` をサポートしている可能性がありますが、未実装であれば追加してください。
- `IsEn` フラグは「ロジック切り替え」には使用せず、**プロンプトの選択のみ**に使用します。
- グラフ抽出における「出力」は、**入力テキストの言語に依存**します（日本語入力なら日本語グラフ）。
- したがって、`GENERATE_GRAPH_EN_PROMPT` であっても、"Output must preserve original language" という指示を含めることで、英語入力なら英語、日本語入力なら日本語のグラフが出力されるように制御します。
- `IsEn` が true の場合は、英語テキストを前提とした最適化（英語の言語的特徴に基づいた抽出ロジック）が含まれる `_EN` プロンプトを使用し、false の場合は日本語特化の `_JA` を使用します。

```go
// Run メソッド内のLLM呼び出し部分の修正
// ...
var promptTemplate string
if t.IsEn {
    // 英語モード: 英語テキストに最適化された抽出ロジックを持つ英語プロンプト
    promptTemplate = prompts.GENERATE_GRAPH_EN_PROMPT
} else {
    // 日本語モード: 日本語テキストに最適化された抽出ロジック（助詞解析など）を持つ英語プロンプト
    // ※ 思考は英語だが、対象言語の特性を理解させるための指示が含まれる
    promptTemplate = prompts.GENERATE_GRAPH_JA_PROMPT
}

// プロンプト自体は入力テキストの言語を保持するよう指示されているため、
// 日本語テキストが入力されれば日本語のグラフが生成される。
content, chunkUsage, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, promptTemplate, prompt)
// ...
```

#### [MODIFY] [summarization_task.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/tasks/summarization/summarization_task.go)

**変更内容**:
- `SummarizationTask` 構造体に `IsEn bool` フィールドを追加します。
- `NewSummarizationTask` ファクトリ関数の引数に `isEn bool` を追加し、構造体に設定するように修正します。
- `SummarizationTask` は「アーティファクト作成」のステップであり、ユーザーが直接利用する成果物（要約）を生成するため、**最終出力**とみなします。
- したがって、出力言語は `IsEn` フラグに**厳密に従います**。
- `Run` メソッド内で正しいプロンプトを選択します。

```go
// Run メソッド内のLLM呼び出し部分の修正
// ...
var promptTemplate string
if t.IsEn {
    // 英語モード: Output MUST be in English
    promptTemplate = prompts.SUMMARIZE_CONTENT_EN_PROMPT
} else {
    // 日本語モード: Output MUST be in Japanese
    promptTemplate = prompts.SUMMARIZE_CONTENT_JA_PROMPT
}

// System Promptには "Think in English" が常に含まれている。
// 最終出力言語のみがプロンプトによって異なる。
summaryText, chunkUsage, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, promptTemplate, prompt)
// ...
```

#### [MODIFY] [types/query_config.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/types/query_config.go) (想定)

- `QueryConfig` 構造体に `IsEn bool` フィールドを追加します。これにより `queryTool.Query` メソッド内で言語設定を参照可能にします。

#### [MODIFY] [graph_completion_tool.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/tools/query/graph_completion_tool.go) (および `AnswerQueryWithHybridRAG` メソッド)

**変更内容**:
- Queryパイプラインは、中間生成物（思考材料）は全て英語で作成し、最後の回答生成だけを指定言語で行う A -> B -> C -> Final 構造にします。
- **グラフ説明文の生成 (Intermediate Step D)**:
    - ハイブリッドRAGのコンテキストとして使用される「グラフ説明文」は、**常に英語**で生成します。
    - 最終回答生成を行うLLM（Step E）は「英語で思考」するため、入力コンテキストも英語である必要があります。日本語モードであっても、ここを日本語にしてはいけません。
- **最終回答生成 (Final Step E)**:
    - 英語のコンテキスト（Chunks + Graph Explanation）を受け取り、指定された言語で回答を生成します。

```go
// AnswerQueryWithHybridRAG メソッド内のロジック

// 1. コンテキスト用のグラフ説明文生成 (Intermediate Step - MUST BE ENGLISH)
// ユーザーの言語設定(IsEn)に関わらず、LLMへの入力ソースとなるグラフ説明は「常に英語」で生成する。
// これにより、LLMは英語で一貫した論理推論(Reasoning)を行うことができる。
graphTextForContext := &strings.Builder{}
graphTextForContext = GenerateNaturalEnglishGraphExplanationByTriples(triples, graphTextForContext)

// ※ 注意: QUERY_TYPE_GET_GRAPH_EXPLANATION (Type 7) のように
// 「グラフ説明文そのもの」をユーザーに返却する場合は、IsEn に従って言語を切り替える必要がある。
// しかし、Hybrid RAG の "コンテキスト" として使う場合は、上記のように常に英語とする。

// 2. 回答生成 (Final Step - Target Language Output)
var promptTemplate string
if t.IsEn {
    // 英語モード: Think in English -> Reply in English
    promptTemplate = prompts.ANSWER_QUERY_WITH_HYBRID_RAG_EN_PROMPT
} else {
    // 日本語モード: Think in English -> Reply in Japanese
    // 入力される graphTextForContext は英語だが、最終出力は日本語にするよう指示する。
    promptTemplate = prompts.ANSWER_QUERY_WITH_HYBRID_RAG_JA_PROMPT
}

// ここがパイプラインの最終出口。
// 入力: 英語のグラフ説明 + (原文の)チャンク
// 思考: 英語 (Prompt指示による)
// 出力: 指定言語 (Prompt指示による)
answer, usage, err := utils.GenerateWithUsage(ctx, t.chatModel, t.modelName, promptTemplate, userQuery, chunks, graphTextForContext.String())
// ...
```

// ...
```

#### [ADDITIONAL EXAMPLE 2] Multi-Hop Reasoning Query (Decomposition)

**シナリオ**:
ユーザーの質問が複雑で、そのままでは回答できないため、複数のサブ質問に分解して順次解決する場合。
`QUERY_TYPE_MULTI_HOP_REASONING` (Hypothetical / Future Type) 相当。

**パイプライン構造**:
A: Query Input (JP/EN)
B: Decomposition (Thinking Step: English) -> Sub-Questions (English)
C: Execution of Sub-Questions (Search/Reasoning: English)
D: Synthesize Answer (Thinking Step: English)
E: Final Answer Generation (Output: Target Language)

```go
// Step B: 分解 (Must be English)
// Input: "Why did Project X fail and how does it relate to User Y?" (JP input)
// Output: ["What were the causes of Project X failure?", "What represents User Y's relationship to Project X?"] (in English)
subQuestionsEn := decomposeQueryInEnglish(ctx, userQuery)

// Step C: 実行 (Must be English context)
var contexts []string
for _, sq := range subQuestionsEn {
    // 検索や小規模推論も全て英語で行う
    ctxEn := searchAndReasoningInEnglish(ctx, sq)
    contexts = append(contexts, ctxEn)
}

// Step D: 統合 (Must be English Thinking)
// Prompt: "Based on these contexts, synthesize a comprehensive answer. Think in English and plan the response structure."
draftValuesEn := synthesizeDraftLogicInEnglish(ctx, contexts)

// Step E: 最終出力 (Target Language Generation)
var promptTemplate string
if t.IsEn {
    promptTemplate = prompts.FINALIZE_ANSWER_EN_PROMPT // Polish English
} else {
    promptTemplate = prompts.FINALIZE_ANSWER_JA_PROMPT // Generate natural Japanese answer based on English logic
}
// Input: English Logic/Values + Original Contexts
// Output: Target Language Answer (Direct Generation, NOT Translation)
finalAnswer, _, _ := utils.GenerateWithUsage(ctx, t.chatModel, t.modelName, promptTemplate, draftValuesEn)
```

#### [ADDITIONAL EXAMPLE 3] Self-Correction Answer Generation

**シナリオ**:
回答のハルシネーションを防ぐため、一度ドラフトを作成してから自己批判し、修正した後にユーザーへ返す場合。
これは「直列の多段パイプライン」の典型例です。

※ 注意: 以下に記載の GENERATE_FINAL_ANSWER_JA という定数が存在するわけではありませんし、GENERATE_FINAL_ANSWER_JA を作らなければならないわけではありません。あくまで例として扱ってください。

**パイプライン構造**:
A: Query Input
B: Draft Generation (Thinking: English, Output: English)
C: Critique/Validation (Thinking: English, Output: English)
D: Refinement (Thinking: English, Output: English)
E: Final Answer Generation (Output: Target Language)

```go
// Step B: Draft (English)
// Prompt: "Draft a detailed answer based on the context. Think in English. Output in English."
draftEn := generateDraft(ctx, query, context)

// Step C: Critique (English)
// Prompt: "Review the draft for inaccuracies or missing info. Think in English. Output critique in English."
critiqueEn := critiqueDraft(ctx, draftEn, context)

// Step D: Refinement (English)
// Prompt: "Refine the logic and facts based on the critique. Output clean English logic/points."
refinedLogicEn := refineDraftLogic(ctx, draftEn, critiqueEn)

// Step E: Finalize (Target Language Generation)
// ここで初めてユーザーの言語設定(IsEn)が登場する
var promptTemplate string
if t.IsEn {
    promptTemplate = prompts.GENERATE_FINAL_ANSWER_EN // Generate English answer
} else {
    promptTemplate = prompts.GENERATE_FINAL_ANSWER_JA // Generate Japanese answer based on English logic
    // Prompt: "Based on the provided English logic and facts, write a natural and accurate answer in Japanese."
}

finalAnswer, _, _ := utils.GenerateWithUsage(ctx, t.chatModel, t.modelName, promptTemplate, refinedLogicEn)
```

**結論**:
Phase-24におけるすべての実装変更において、上記の **"Intermediates in English -> Final in Target"** パターンを厳守してください。

---

#### [MODIFY] Memify & Metacognition Tasks (Multi-Step Logic)

**変更内容**:
- 以下のタスクについて、構造体への `IsEn` 追加とファクトリ関数のシグネチャ変更（`isEn` 引数追加）を行います。
    - `src/pkg/cuber/tasks/metacognition/self_reflection_task.go` (`SelfReflectionTask`)
    - `src/pkg/cuber/tasks/memify/rule_extraction_task.go` (`RuleExtractionTask`)
- Memify (Metacognition) は多段推論プロセスです。
- **原則**: 中間生成物（Unknowns, Questions, Capability descriptions）は、**全て英語**で処理させることが理想的ですが、Memifyに関しては「自己強化」であり、強化された知識（ルール・洞察）がグラフ（入力言語準拠）にマージされる必要があります。
- そのため、現状の Memify 実装においては、**`IsEn` フラグが出力言語（＝知識グラフの言語）と一致していると仮定**して処理を進めます。
- 将来的には「英語で思考 -> グラフ言語に合わせて翻訳」というステップが必要になる可能性がありますが、Phase-24では以下のように振り分けます。

```go
// 例: RuleExtractionTask (Refinement)
var promptTemplate string
if t.IsEn {
    // 抽出されるルール自体が英語になる (English Graph向け)
    promptTemplate = prompts.RULE_EXTRACTION_SYSTEM_PROMPT_EN
} else {
    // 抽出されるルール自体が日本語になる (Japanese Graph向け)
    promptTemplate = prompts.RULE_EXTRACTION_SYSTEM_PROMPT_JA
}
```
※ Memify については、不整合（日本語グラフなのに英語モードで実行して英語ルールが混ざる等）のリスクがありますが、ユーザー責任（モード選択）として扱い、実装上は `IsEn` に従順に従う形とします。ただし、プロンプト内部では可能な限り「思考は英語」を守らせるように `_JA` プロンプトを設計します。

---

#### [ADDITIONAL EXAMPLE 2] Memify: Self-Correction Loop (Refinement Task)

**シナリオ**:
既存の知識グラフから「矛盾」や「不足」を見つけ出し、自分で自分に問いかけて補完する自己強化ループ。
A: 現状の知識ルールをロード (Input)
B: 矛盾・不足の検知 (Logic: English)
C: 補完のための「問い」の生成 (Logic: English)
D: 問いへの回答生成（自問自答） (Logic: English)
E: 最終的な新たな洞察・ルールとして保存 (Output: Target Language)

```go
// src/pkg/cuber/tasks/metacognition/self_reflection_task.go などのイメージ

// Step A: Input (JP rules) -> B: Detect Unknowns (Think en, Output en structure)
// Prompt: "Analyze these Japanese rules. Identify logical gaps. Think in English. Output gaps in clean English."
unknownsEn := detectUnknownsInEnglish(ctx, rulesJP) 

// Step C: Generate Questions (Think en, Output en)
// Prompt: "Based on these gaps (en), formulate search queries/questions. Think in English. Output in English."
questionsEn := generateQuestionsInEnglish(ctx, unknownsEn)

// Step D: Answer/Reasoning (Think en, Output en)
// Prompt: "Answer these questions using internal knowledge. Think rigorously in English. Output the insight in English."
insightEn := reasoningInternal(ctx, questionsEn)

// Step E: Final Conversion (Think en, Output JP)
// Prompt: "Based on this English insight, generate a formal Japanese rule for the knowledge graph. Output MUST be in Japanese."
finalRuleJP := generateAndFormat(ctx, insightEn, targetLang="Japanese")

// Save finalRuleJP to Graph
```
※ 現状の実装コードがここまで分離されていない場合は、Phase-24では **Step B~D を 1つの English Prompt で行い、最後の E だけ分離する**、あるいは **B~E を1つのプロンプトで行うが `Think in English` セクションを設ける** などの改修を行います。
最も重要なのは、「日本語のルールを入力したからといって、思考プロセスまで日本語で行ってはいけない」という点です。

---

#### [ADDITIONAL EXAMPLE 3] Knowledge Crystallization (Merging)

**シナリオ**:
複数の類似した小さな事実（チャンクやノード）を統合し、より抽象度の高い「結晶化された知識」を作成する。
A: 複数の事実を収集 (Input)
B: 共通項・パターンの分析 (Logic: English)
C: 抽象化・概念化 (Logic: English)
D: 結晶化された知識の記述 (Output: Target Language)

```go
// src/pkg/cuber/tasks/metacognition/crystallization_task.go

// 1. Analysis & Abstraction (Intermediate - English)
// Input: 複数の断片的な日本語の事実
// Prompt: "Analyze these facts found in Japanese text. Identify the underlying common principle or concept. Think and reason in English. Output the crystallized concept in English."
crystallizedEnglish, _ := utils.Generate(ctx, llm, "gpt-4", prompts.CRYSTALLIZE_ANALYSIS_EN_PROMPT, factsJP)

// 2. Formatting (Final - Target Language)
var promptTemplate string
if isEn {
    promptTemplate = prompts.FORMAT_CRYSTAL_EN_PROMPT // Just format
} else {
    promptTemplate = prompts.FORMAT_CRYSTAL_JA_PROMPT // Generate & Format in JP
}

// Prompt: "Take this English concept: '{crystallizedEnglish}'. Express it as a concise, high-quality knowledge statement in {TargetLanguage}."
finalCrystal, _, _ := utils.GenerateWithUsage(ctx, llm, "gpt-4", promptTemplate, crystallizedEnglish)
```

**結論**:
Phase-24におけるすべての実装変更において、上記の **"Intermediates in English -> Final in Target"** パターンを厳守してください。

---

### 5. クエリタイプの統合

`src/pkg/cuber/types/query_types.go` およ `query_config.go` を修正し、言語別のタイプを廃止・統合します。

#### [MODIFY] [query_types.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/types/query_types.go)

**変更前 (イメージ):**
```go
const (
    // ...
    QUERY_TYPE_GET_GRAPH_EXPLANATION_EN = 7
    QUERY_TYPE_GET_GRAPH_EXPLANATION_JA = 8
    QUERY_TYPE_GET_GRAPH_SUMMARY_EN     = 9
    QUERY_TYPE_GET_GRAPH_SUMMARY_JA     = 10
    // ...
)
```

**変更後:**
```go
const (
    // ... 1-6 はそのまま ...
    QUERY_TYPE_GET_GRAPH_EXPLANATION                 = 7
    QUERY_TYPE_GET_GRAPH_SUMMARY                     = 8
    QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER           = 9
    QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH = 10
    QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH            = 11
)
// 12-16 は欠番とする（あるいは定数を削除）
```
※ `IsValidQueryType` 等のバリデーションロジックも、上限を `16` から `11` に変更する。

#### [MODIFY] [graph_completion_tool.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/tools/query/graph_completion_tool.go)
- `Query` メソッド内の switch 文を更新。
- 事例:
  ```go
  case types.QUERY_TYPE_GET_GRAPH_EXPLANATION:
      if t.IsEn {
          // 英語版ロジック/プロンプト
      } else {
          // 日本語版ロジック/プロンプト
      }
  ```
- 既存の `_EN`/`_JA` で分岐していたロジックを、1つの case 内での `if t.IsEn` 分岐に統合する。
- 統合対象:
    - 7, 8 -> 7
    - 9, 10 -> 8
    - 11, 12 -> 9
    - 13, 14 -> 10
    - 15, 16 -> 11

#### [MODIFY] [cubes_handler.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rthandler/hv1/cubes_handler.go)
- `QueryCube` の Swagger コメント (`@Description`) を更新し、統合された新しいID体系を反映させる。言語の区別がなくなったことを記述する。

---

## 期待される動作
- `IsEn=true` で `Absorb` すると、抽出されるグラフのノードIDなどは原文(英語)のまま、内部推論も英語で行われ、要約も英語で保存される。
- `IsEn=false` (デフォルト) で `Absorb` すると、日本語テキストとして扱われ、現状通りの挙動となる。
- `Query` において、`IsEn=true` であれば、回答やグラフの説明文が英語で返される。思考プロセス(CoT)は常に英語で行うが、ユーザーへの最終出力言語は `IsEn` に従う。
- `Memify` において、抽出されるルールや洞察が、`IsEn` に応じた言語で生成・保存される。

## 注意事項
- `prompts.go` の変更は慎重に行うこと。既存の日本語プロンプトの挙動を変えないよう、リファクタリング時はコピー＆ペーストで分離してから修正すること。
- `query_types.go` の変更に伴い、既存のテストコードや定数参照箇所でコンパイルエラーが出る可能性があるため、grep等で影響範囲を確認すること。

