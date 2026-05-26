# Darvium v2.3 完全実装チケット分解設計書（科学的計装版・改）

> **v2.3 改訂注記**
>
> 本文書は、Darvium RFC-0001 v2.0-final を基礎に構築された既存の完成度を維持しつつ、RFC v2.3 で明文化・強化された規範のみを慎重に反映した改訂版である。
> 本改訂で特に強化したのは、**SearchWorkflow を mission-completion-oriented orchestration として解釈すること**、**WorkflowGraph / SearchWorkflowGraph の DAG 検証を作成時と使用時の双方で必須とすること**、**frontier-based parallel execution の安全条件を明示すること**、**dual-store consistency / startup repair / quarantine discipline を selection path 遮断まで含めて規範化すること**、および **ranking stability / false-new rate / review queue / repair convergence などの補助メトリクスを観測対象として明示すること** である。
> 既存版の設計思想、13フェーズ構成、オフライン・メモリ内完結、科学的計装、トレイト駆動、テストファーストの原則は変更しない。今回の改訂は、前版の情報を毀損せず、v2.3 の意味論・安全規律・観測規律をチケット単位で誤読なく伝えるための最小かつ必要十分な上書きである。

Darvium RFC-0001 v2.0-final に基づき、実生産コードの投入を限界まで引き剥がし、「1チケット・1不変条件」を徹底した全13フェーズの完全実装チケット分解設計書を作成しました。

すべてのチケットは、外部I/O（ネットワーク、実ストレージ、本物LLM）を排除し、Rustのメモリ内データ構造と固定シード疑似乱数生成器（PRNG: Pseudo-Random Number Generator）によるテストコードのみで100%シミュレート・検証・観察可能なクローズド単位に区切っています。

> **🔬 データベース非依存の原則（超重要）**
>
> 本設計書の全13フェーズ（M-2 〜 M4）は **SQLite も LadybugDB も一切使用しない**。
> すべての「ストア操作」は Rust の `Vec` / `HashMap` 等のメモリ内データ構造でエミュレーションされる。
>
> **なぜこれが重要か**：
> 1. 実データベースを導入すると、トランザクション分離レベル、HNSW近似誤差、ネットワーク障害、ディスク満杯、デッドロックなど、論理的正当性とは無関係なノイズが実験結果に混入する
> 2. 全チケットをオフライン・高速（ミリ秒単位）・決定論的に実行可能に保つことで、実験の再現性とフィードバック速度を最大化する
>
> **⚠️ 実装者が絶対に誤解してはいけないこと**：
> - M1.5-2 や M4-4 で「SQLite側」「LadybugDB側」という表現が出てくるが、これらは**メモリ内のエミュレーション**であり、実際のデータベース接続ではない
> - M2.5-1 の「テスト用SQLiteテーブル」は括弧書きの代替手段に過ぎず、主実装はメモリ内レジストリである
> - **実データベースの導入は本13フェーズの完了後**、別フェーズとして計画・実施すること
>
> **✅ 本13フェーズのスコープ内で実施すべき抽象化**：
> - M-2-1.5 で定義する `GraphStore` / `MetadataStore` トレイト階層は **本13フェーズのスコープ内** で実装する
> - M-2-1.6 で定義する `LLMClient` トレイト階層（`FakeLlmClient` / 将来の `RealLlmClient`）も同様
> - M-2-1.7 で定義する `EmbeddingProvider` トレイト階層（`FakeEmbeddingProvider` / 将来の `RealEmbeddingProvider`）も同様
> - M-2-1.8 で定義する `Clock` トレイト階層（`VirtualClock` / `SystemClock` / `FrozenClock`）も同様
> - M-0.5-4 で定義する `HumanChannel` トレイト階層（`FakeHumanChannel` / `StdinoutChannel` / 将来の `SlackChannel` 等）も同様
> - M4-2.5 で定義する `ExternalApiClient` トレイト階層（`FakeExternalApiClient` / 将来の `RealApiClient`）も同様
> - これにより全外部依存コードはトレイトに対するプログラミングとなり、将来の実I/O差し替えを準備する
> - トレイト＋メモリ内実装のペアを全13フェーズに先立って確立することで、各チケットの実装が直接 `Vec`/`HashMap` や API 直呼び出しをするのを防ぐ
>
> **🧪 実DB接続フェーズで初めて顕在化する検証対象**（本設計書ではカバーしない）：
> - トランザクション分離＋競合の現実的な挙動
> - HNSW近似誤差とポリシー評価の相互作用
> - 多様な障害モード（ネットワーク切断、ディスク満杯等）への耐性
> - レイテンシを考慮したタイムアウト較正
>
> これらの検証は、本設計書の全チケット完了後に計画する「永続化結合フェーズ」で行う。

> **実験計画の宣言**
> **本プロジェクトの全チケットは、複雑系科学の実験計画として設計される。**
> 各チケットの完了条件は「コードが動くこと」ではなく、「観測可能な振る舞いが特徴づけられ、実験系列として記録されていること」である。
> 「テスト」という語は慣習的な呼称であり、その実体は仮説駆動型の計算機実験である。
> アサーションは実験の安全装置（不変条件の監視）に限定し、本質的な検証は統計的観測で行う。
> 実装段階で追加の実験必要と判断された場合は、躊躇せずに追加実験を実施し、その結果を実験系列に記録しなければならない。
> チケットに書かれていないことを理由に実験を省略してはならない。
> 実験方法論の詳細は以下のルールファイルを参照すること：
> * `rules/darvium/observational-testing.md` — 観測テストの種類・統計的要求・出力形式
> * `rules/darvium/calibration-loop.md` — 較正ループ・目的関数 $J(\theta)$ の設計
> * `rules/darvium/experiment-reporting.md` — 実験レポート構造・系列追跡
> * `rules/darvium/simulation-runner.md` — SimulationRunner の設計・マイルストーン別設定
> 
> 

---

## 4層アーキテクザ・13段階テストファーストチケットマップ

### ── 第1段階：純粋ロジック・状態機械の完全隔離検証（M-2 〜 M-1） ──

外部からの揺らぎ（乱数、環境、LLM）を完全に遮断し、状態遷移マトリクスと言語規則の整合性のみを検証するフェーズです。

### 1. マイルストーン M-2：SearchWorkflow 仕様固定（v2.3 mission-completion semantics 対応）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。M-2-1.5 で抽象トレイト (`GraphStore` / `MetadataStore`) を定義し、将来の実DB差し替えに備える。

#### ✅ チケット M-2-1: `RetrievalPrimitive` 抽象インターフェース及びコアデータ型の定義

* **対象不変条件 / 規範:** §13.4 RetrievalPrimitive 契約
* **実装スコープ:** `RetrievalPrimitive` トレイト、`QueryRepresentation`、`RetrievalPolicy`、`CandidateSet`、`RankedCandidate` のピュア構造体の定義。
* **テストコードによる検証:** 具象実装を持たないダミートレイト境界が、コンパイル時点で型シグネチャを完全に充足することを確認する。
* **計装方法・観測対象:** 抽象インターフェースの型多重度変化に対する、コンパイル時における型シグネチャのマッチング網羅率（全射性）およびトレイト境界の結合強度変化の動的検証。型定義空間から生成される依存グラフにおいて、トレイト境界の不整合を誘発する変異コード（境界値ケース）を網羅的に自動生成した際の、コンパイルエラーのバリアント網羅率（包括性）、および型依存関係の直径 $d_{diam}$ が有界に制限されていることの静的型システム上の整合性証明。

#### ✅ チケット M-2-1.5: Dual-Store 抽象トレイト階層の定義 (GraphStore / MetadataStore)

* **対象不変条件 / 規範:** §12 4-Layer Retrieval、§25 データベース構成、§18.2 クロスストア書き込み規約、§37 Birth Commit Discipline
* **実装の背景と目的:** 全13フェーズはメモリ内完結だが、後段の実DB接続フェーズで SQLite / LadybugDB への差し替えを可能にするため、ストレージ操作を抽象化する2系統のトレイトを\*\*全ストレージ利用コードに先立って\*\*定義する。これにより各チケットの実装はトレイトに対するプログラミングとなり、テストは Mock 実装で完結、実DB接続フェーズでは具象実装（`SqliteMetadataStore` / `LadybugGraphStore`）を書くだけで置き換えが完了する。
* **実装スコープ:**
  - `GraphStore` トレイト（LadybugDB 責務）: ワークフローグラフの格納・読取、埋め込みベクトルの登録・近似最近傍探索、Knowledge object / relation の CRUD、origin trace の記録・参照
  - `MetadataStore` トレイト（SQLite 責務）: メタデータ・信頼スコア・系列監査ログ（`SearchTrace` / `TrustAuditLog` / `PatchHistory`）の永続化、Training / Fusion メタデータの CRUD
  - `InMemoryGraphStore`: `HashMap` / `Vec` による全操作のメモリ内実装（高速・決定論的）
  - `InMemoryMetadataStore`: `HashMap` / `Vec` による全操作のメモリ内実装
  - エラー型: 既存の `DarviumError` を拡充（`Storage` / `NotFound` 等のバリアント追加）
* **テストコードによる検証:**
  1. メモリ内実装（`InMemoryGraphStore` / `InMemoryMetadataStore`）が全トレイトメソッドを充足することのコンパイル時検証
  2. 基本的な CRUD 操作（登録→読取→更新→削除）の正常系テスト
  3. 存在しないキーへのアクセスが適切な `Err(DarviumError::NotFound)` を返す異常系テスト
  4. トレイト境界のオブジェクト安全性確認（`Box<dyn GraphStore>` / `Box<dyn MetadataStore>` として使用可能）
  5. `semantic_search` に登録済みベクトルと同一のクエリを入力した際、最高類似度 `1.0` が最上位に返ることを確認
  6. `store_search_trace` で書き込んだトレースが `load_search_trace` で完全に同一内容として読み出せることを確認
* **計装方法・観測対象:** 2系統のトレイト境界によって形成される代数的データ空間の直交分解特性。`GraphStore` 及び `MetadataStore` への操作命令列（ログ）を時系列に記録し、命令列の挿入・削除・変更に対するトレイト実装のバイナリ互換性の変化率。メモリ内実装における全操作の命令ステップ数が入力サイズ $n$ に対して $O(1)$ または $O(n)$ の範囲に有界であることのスケーリング検証、および二重実装（トレイト境界＋メモリ内実装）がコンパイル時に生成する型依存グラフの直径 $d_{diam}$ が、トレイト階層導入前と比較して一定の範囲内に維持されていることの静的メトリクス計測。

#### ✅ チケット M-2-1.6: LLMClient 抽象トレイトの定義

* **対象不変条件 / 規範:** §14.2 構造化出力要求契約、§13A LLM adapter interface
* **実装の背景と目的:** M0.5-1 で Fake LLM client を作り、M2〜M3 で本物の LLM API に差し替える計画だが、その間にトレイトが存在しない。本チケットでは LLM 呼び出しを抽象化する `LLMClient` トレイトと、決定論的な `FakeLlmClient` を定義する。これにより M2 で `RealLlmClient` を追加するだけでシームレスに置き換えが可能になる。併せて `DarviumError` に LLM エラーバリアントを追加する。
* **実装スコープ:**
  - `LLMClient` トレイト: `fn generate_structured(&self, prompt: &str, schema: &LlmSchema) -> Result<String, DarviumError>`
  - `LlmSchema` 列挙型: `QueryDesignText`, `PatchOperations`, `SelfScore`, `FreeText`
  - `FakeLlmClient`: コンストラクタで指定された固定文字列を返す決定論的モード、及び指定確率で不正フォーマットを返す乱数モードを持つ
  - エラー型: `DarviumError::Llm(String)` 及び `DarviumError::LlmMalformedJson(String)` 追加
* **テストコードによる検証:**
  1. `FakeLlmClient` がトレイト境界を充足することのコンパイル時検証
  2. 固定文字列モードで `generate_structured` を呼び出し、指定した文字列が正確に返ること
  3. `DarviumError::LlmMalformedJson` が JSON パース失敗を正しく表現すること
  4. トレイトのオブジェクト安全性確認（`Box<dyn LLMClient>`）
* **計装方法・観測対象:** トレイト境界を通過する LLM 呼び出しの全二重記録による完全監査可能性。`FakeLlmClient` における出力のシャノンエントロピーと注入された乱数ノイズのエントロピーの一致性。

#### ✅ チケット M-2-1.7: EmbeddingProvider 抽象トレイトの定義

* **対象不変条件 / 規範:** §12 4-Layer Retrieval、§9 WorkflowDesignText / QueryDesignText
* **実装の背景と目的:** M-0.5〜M1.5 ではメモリ内疑似埋め込みを、M1.5 以降では本物の埋め込み API を使用する。両者の抽象境界を定義しないと、全呼び出し箇所での修正が発生する。本チケットでは埋め込み生成を抽象化する `EmbeddingProvider` トレイトと、固定シード PRNG 駆動の `FakeEmbeddingProvider` を定義する。
* **実装スコープ:**
  - `EmbeddingProvider` トレイト: `fn embed(&self, text: &str) -> Result<Vec<f32>, DarviumError>`, `fn embed_dimension(&self) -> usize`
  - `FakeEmbeddingProvider`: 固定シード `StdRng::seed_from_u64(12345)` を使用し、テキストのハッシュをシードに疑似埋め込みベクトルを生成（次元数はコンストラクタ指定可能、デフォルト 384）
  - `ConstantEmbeddingProvider`: 常に同じベクトルを返す（テスト用）
  - エラー型: `DarviumError::Embedding(String)` 及び `DarviumError::EmbeddingDimensionMismatch { expected: usize, actual: usize }` 追加
* **テストコードによる検証:**
  1. `FakeEmbeddingProvider` / `ConstantEmbeddingProvider` がトレイト境界を充足することのコンパイル時検証
  2. 同じテキストを 2 回 embed すると同じベクトルが返る（決定論性）
  3. 異なるテキストを embed すると異なるベクトルが返る（衝突率 < 1e-6）
  4. `embed_dimension()` がコンストラクタ指定値と一致すること
  5. 空文字列 embed 時の挙動が定義通りであること
  6. トレイトのオブジェクト安全性確認（`Box<dyn EmbeddingProvider>`）
* **計装方法・観測対象:** 埋め込み生成の完全決定論性（同一ハッシュ入力に対する出力ベクトルのビットレベル完全一致）。`FakeEmbeddingProvider` の生成する疑似埋め込みベクトル空間におけるコサイン類似度の分布が、高次元超球面上の一様分布と統計的に区別できないこと（カイ二乗検定、$p > 0.05$）。

#### ✅ チケット M-2-1.8: Clock / VirtualClock 抽象トレイトの定義

* **対象不変条件 / 規範:** §v1.7 Human Time / Virtual Time 二軸モデル、§13.6 SearchBudget ガード条件、§18.2 タイムアウト処理
* **実装の背景と目的:** RFC §v1.7 は `VirtualClock` を「SystemTime とは独立した仮想時間軸」として明文化している。M-2-2 の `SearchBudget` は `wall_clock_ms_used` を持ち、実時間に依存するとテストが非決定論的になる。本チケットでは時間を抽象化する `Clock` トレイトと、手動進行可能な `VirtualClock`、実時間を使用する `SystemClock` を定義する。これにより全時間依存コードを抽象化し、deterministic replay (M2.5-2) を保証する。なお、全ての Human Time（`SystemTime` 経由の時間）は UTC を強制する (MUST)。
* **実装スコープ:**
  - `Clock` トレイト: `fn now_ms(&self) -> u64`, `fn advance(&mut self, delta_ms: u64)`（VirtualClock のみ意味を持つ; SystemClock では advance はパニックまたは no-op）
  - `VirtualClock`: 内部カウンタを持ち、`advance()` でのみ時間が進行する（完全決定論的）
  - `SystemClock`: `SystemTime::now()` をラップする
  - `FrozenClock`: 常に一定値を返す（テスト用、特定時刻の固定）
* **テストコードによる検証:**
  1. 全実装が `Clock` トレイト境界を充足することのコンパイル時検証
  2. `VirtualClock` が `advance(100)` で正確に 100ms 進行すること
  3. `SystemClock` の `now_ms()` が実時間と大きく乖離しないこと（誤差 < 1秒）
  4. `FrozenClock` が常に同じ値を返すこと
  5. トレイトのオブジェクト安全性確認（`Box<dyn Clock>`）
* **計装方法・観測対象:** `VirtualClock` の単調増加性（巻き戻し禁止）のアサーション。`Clock` トレイトを通して観測される時間の流れが、実時間または仮想時間のいずれかで一貫していることの検証。

#### ✅ チケット M-2-2: `SearchBudget` 及び `RecursionGuard` 初期化仕様の検証

* **対象不変条件 / 規範:** §13.3 データモデル制約、§13.6 ガード条件
* **実装スコープ:** `SearchBudget` と `RecursionGuard` の構造体定義、およびデフォルト値、サチュレーティング演算子によるカウントインクリメント関数の実装。
* **テストコードによる検証:** `current_depth` を手動でカウントアップし、`max_depth` を超えた瞬間に条件式が `false` を返す境界値アサーションを記述。
* **計装方法・観測対象:** 初期予算ベクトル $\mathbf{B} = (\text{tokens}, \text{calls}, \text{time})$ をシード固定乱数で変動させた $10^4$ 個 of 初期状態アンサンブルを生成。 サチュレーション演算子に突入する各アンサンブルの軌道が、上限境界という超曲面（不動点マニホールド）へ引き込まれるまでの平均緩和時間 $\tau_{relax}$ の計測。 境界接触後における状態ベクトルの時間変化率（マクロフラックス） $\Delta \mathbf{B}(t) = \mathbf{B}(t) - \mathbf{B}(t-1) = \mathbf{0}$ への即時収束、および不動点アトラクタ近傍における局所リアプノフ指数 $\lambda_{local} < 0$ （吸い込み安定性）の動的計測により、サチュレーション演算の完全結晶化特性を実証する。

#### ✅ チケット M-2-3: 決定論的空リターン用 Mock クライアントの実装

* **対象不変条件 / 規範:** §13.4 RetrievalPrimitive pure retrieval contract
* **実装スコープ:** 常に空の `CandidateSet`、あるいは常に特定の固定エラー（`RetrievalError::Timeout`）を即座に返す Mock 構造体の実装。
* **テストコードによる検証:** どのようなクエリを入力しても、Mock が 100% 決定論的に指定のエラーまたは空配列を返すことを検証。
* **計装方法・観測対象:** Wall-clock time（物理時間）の代わりに、コード内のインストルメンテーション・カウンタが消費する仮想命令ステップ数（あるいはアロケーションバイト数）をプローブで計測。 入力クエリのハッシュ多様体空間から出力空間への写像におけるKolmogorov複雑度の不変性。 クエリのシャノンエントロピーを $H(Q) \in [0, 8]$ ビットの範囲で可変させた際の、消費命令ステップ数 $S_{inst}$ の分散 $\sigma^2(S_{inst}) = 0$ （完全同型性）が維持されていることの決定論的検証。

---

### 2. マイルストーン M-1.5：Search state machine 検証

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### ✅ チケット M-1.5-1: `SearchState` 合法状態遷移マトリクスの実装

* **対象不変条件 / 規範:** §13.5 状態遷移規則（Init -> Retrieve -> Evaluate ... の遷移表）、および v2.3 において明文化された「SearchWorkflow は単一候補の失敗で即座に mission failure とみなしてはならず、残る候補・fallback path・bounded retry / refine / compose / new を評価し尽くした後にのみ終端へ至る」という mission-completion-oriented orchestration の意味論。
* **実装スコープ:** `SearchState` Enum、および現状態と次状態のペアが合法か否かを判定する純粋関数 `is_legal_transition(current, next) -> bool` の実装。加えて、候補単体の compile failure / patch failure / reuse rejection が発生した場合に、残候補が存在する限り SearchWorkflow が `Abort` ではなく `Evaluate` / `Retrieve` / `Refine` / `Compose` / `ProposeNew` 系の継続可能状態へ留まりうることを、状態系列の設計原理として明文化する。
* **テストコードによる検証:** 遷移表に定義されたすべての合法経路（例: `Init` から `Retrieve`）が `true` を返し、違法経路（例: `Init` から `Finalize`）が 100% `false` を返すことを総当たりマトリクステストでアサートする。加えて、候補Aが compile-time DAG failure により hard reject されても、候補Bが残っている場合には状態機械が `Abort` に落ちず、`Evaluate` / `Retrieve` 系へ継続可能であることを補助ケースとして確認する。
* **計装方法・観測対象:** 状態空間 $N$ に対し、マルコフ連鎖ストレステスト用シグナルとして、すべてのランダムな遷移試行（違法経路を含む）を発生させる。ガードロジック通過直後の有効遷移確率行列 $P_{actual}$ のスペクトル半径 $\rho(P_{actual})$ の動的計測。全遷移試行シグナルに対する有効遷移確率行列 $P_{actual}$ において、違法状態集合 $S_{illegal}$ への流入フラックスが厳密に $F(S_{illegal}) = 0$ であること（完全隔離トポロジー）を実証する。また、正当な状態遷移からなる部分行列の固有値スペクトル、および終端状態（`Finalize` / `Abort`）を吸収状態としたとき、任意の初期状態から終端状態へトラップされるまでの平均自由行程（ステップ数）の有限性を実証する。

#### ✅ チケット M-1.5-2: 終端状態（`Finalize` / `Abort`）非再入不変条件の強制

* **対象不変条件 / 規範:** §13.5 「Finalize と Abort は終端状態であり、終端後に再遷移してはならない (MUST NOT)」。加えて v2.3 では、`Finalize` / `Abort` は fallback path が尽き、または budget / recursion / explicit abort reason が成立した後にのみ到達可能な真の終端であり、単一候補 failure をもって早期終端してはならない。
* **実装スコープ:** 状態変更メソッド `transition_to(&mut self, next: SearchState)` 内で、現在状態が終端状態の場合に必ずエラーを返却するガードロジック。さらに、候補単体 failure を終端理由として誤って扱わないよう、終端遷移を許可する条件セットを明示する補助判定器を設計に含める。
* **テストコードによる検証:** 状態を一度 `Finalize` に設定した後、別の状態へ遷移を試みた場合に必ず `Err(SearchValidationError::TerminalStateViolation)` が発生することを検証する。加えて、候補単体 failure のみでは `Finalize` / `Abort` に遷移せず、予算超過・再帰超過・明示的 Abort 条件などの正当な終端理由でのみ終端に入ることを確認する。
* **計装方法・観測対象:** 終端状態に固定された `SearchState` に対し、マルチスレッド並行下で10万回の非同期状態遷移割り込みシグナル（パルス注入）を印加。 割り込みパルス印加時における、ガードロジックの例外ハンドリング処理時間 $\tau_{gate}$ の極値統計分布（Gumbel分布への適合度）。 外的シグナルの多重度（コンテンションレベル）を増大させた際の、終端状態維持率100%の不変境界条件、および状態フラグの物理メモリビット表現におけるハミングエントロピーが完全に $0$ で凍結していることの確認。

#### ✅ チケット M-1.5-3: `SearchPolicyOscillation`（無限往復暴走）検出エンジンの検証

* **対象不変条件 / 規範:** §13.5 「Refine -> Retrieve -> Refine が閾値回数を超えて往復する場合、SearchPolicyOscillation として検出すること」
* **実装スコープ:** 遷移履歴配列（あるいは特定の状態往復用カウンタ）と、閾値超過時に `SearchOutcome::AbortSearch` へ強制ダウングレードする判定器の実装。
* **テストコードによる検証:** ループ内で `Refine` と `Retrieve` を交互に手動でシグナル注入し、指定カウンタ（例: 3回）に達した瞬間に状態機械が自動的に強制 `Abort` 状態へ遷移することを確認。
* **計装方法・観測対象:** 状態遷移に人為的往復ノイズを注入し、時間発展に対する状態遷移履歴バッファの周期性を追跡。往復イテレーション回数 $t$ を進めた際の状態空間上のハミング距離の周期的な挙動（周期的なゼロ落ち特性）の自動検出。カウンタが臨界閾値 $N_c$ に達した瞬間に、システム状態が不連続に `AbortSearch` へ確定状態遷移する際の潜時（消費される仮想命令ステップ数）の計測、および閾値判定インターセプターが例外なく100%確実にループを遮断することの決定論的検証。

---

### 3. マイルストーン M-1 shadow-first：Fake policy evaluator

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### ✅ チケット M-1-1: 静的閾値による `EvaluateCandidatesStep` 決定エンジンの実装

* **対象不変条件 / 規範:** §13.4 & §13.5 評価ロジック・分岐境界。加えて v2.3 で補強された §12.2–§12.3 の自己評価割引（`SELF_CONF_DISCOUNT`）および validator 優先化の重み切り替え規則に、後段で自然に接続できる shadow-first evaluator であること。
* **実装スコープ:** 入力されたダミー候補のスコアが特定閾値（0.50）以上の場合に `REUSE`、未満の場合に `PATCH` を選択する純粋判定関数。ただし本関数は最終形そのものではなく、後段で自己評価割引・validator weight switch・patch confidence 計算へ滑らかに接続される簡約モデルとして位置づける。
* **テストコードによる検証:** スコア 0.51 のダミーデータを渡すと `SearchOutcome::ReuseExisting`、0.49 を渡すと `SearchOutcome::PatchExisting` というバリアントが返ることを確認する。加えて、後段接続の足場として、生自己評価 0.90 に discount を適用して通常側の分岐へ残るケース、および生自己評価 0.45 により validator weight switch 相当の低信頼分岐が必要になるケースを補助的に検証する。
* **計装方法・観測対象:** 判定エンジンに対し、スコア $S$ の周辺に平均 0 、分散 $\sigma^2_{noise}$ のガウスノイズを付加した入力を与える（不確実性の包含）。 ノイズ分散 $\sigma^2_{noise}$ を増大させた際の、決定境界近傍（ $S = 0.50 \pm \epsilon$ ）における `REUSE` / `PATCH` 選択確率のシグモイド（ロジスティック）曲線への変形特性。 この滑らかな相転移曲線における微細傾き（感受率）の最大値が、分散の逆数（温度の逆数 $\beta = 1/\sigma^2$ ）に比例して鋭くなるスケーリング則、および境界超曲面の平均幾何学的曲率の実測。

#### ✅ チケット M-1-2: `SearchBudgetExceeded` ハードガードの遮断アサーション

* **対象不変条件 / 規範:** §13.6 ガード条件「SearchBudgetの上限超過時は SearchBudgetExceeded を返し、Abort へ遷移すること」
* **実装スコープ:** ループ実行前に `budget.prompt_tokens_used > budget.max_prompt_tokens` などの条件を評価し、即座に状態を変更するインターセプタの実装。
* **テストコードによる検証:** トークン使用量に限界値以上の値を手動で代入したコンテキストを判定器に流し込み、即座に `Err(SearchBudgetExceeded)` が返ることを確認。
* **計装方法・観測対象:** 物理クロックや物理サイクルに依存せず、インストルメンテーション・プローブがカウントする『総仮想命令ステップ数 $S_{inst}$』を観測対象とする。超過量 $\Delta B$ の対数掃引に対し、ガード遮断に要する命令ステップ数の分散 $\sigma^2(S_{inst}) = 0$ （完全な最悪時間有界性）が維持されていることを決定論的に検証する。

#### ✅ チケット M-1-3: `SearchRecursionExceeded` 深さ制限ガードの強制

* **対象不変条件 / 規範:** §13.6 ガード条件「RecursionGuard の深さ超過時は SearchRecursionExceeded を返すこと」
* **実装スコープ:** サーチエンジン再入メソッド呼び出し時に `current_depth` を加算し、上限を検査するガード。
* **テストコードによる検証:** 限界深さを `3` に設定したテスト上で、4回目のダミー呼び出しを行うと、下層への進入をブロックして `SearchRecursionExceeded` を返すことを確認。
* **計装方法・観測対象:** `global_allocator` にフックする計装探針をインプラントし、再帰呼び出しメソッドが呼び出されるたびにアクティブなアロケーションバイト数 $A_{bytes}$ とスタックフレームポインタの変位 $\Delta SP$ を追跡。 深さ $d = d_{max}$ に到達し、`SearchRecursionExceeded` の遮断ロジックが連続して $10^4$ 回発動した状態における、アロケーション増分累積値 $\sum \Delta A_{bytes} = 0$ の完全なる静止定常状態、および呼び出しツリーのトポロジー階層の成長率が完全に不連続に 0 となるカットオフ境界の確認。

---

### 3A. マイルストーン M-0.75：v2.3-h 4 層検索データ基盤（構造的メタデータ）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### ✅ チケット M-0.75-1: v2.3-h 4 層検索用データ型定義（TopLevelGraphMetadata / CheapGedSignature / TopLevelQueryMetadata）

* **対象不変条件 / 規範:** §8 MemoizedGraph、§9 QueryRepresentation、§12 Stage 2-4（v2.3-h 改訂）
* **実装スコープ:**
  - `TopLevelGraphMetadata` 構造体（12 フィールド）の定義: `top_node_count: u16`, `top_edge_count: u16`, `top_source_count: u16`, `top_sink_count: u16`, `top_longest_path_len: u16`, `top_max_width: u16`, `top_label_histogram: Vec<(String, u16)>`, `top_edge_type_histogram: Vec<(String, u16)>`, `top_determinism_summary: f32`, `top_sideeffect_summary: SideEffectSummary`, `top_agentsethash: Vec<u64>`, `top_layer_signature: Vec<u64>`
  - `CheapGedSignature` 構造体（7 フィールド）の定義: `topo_rank_labels: Vec<u64>`, `indegree_histogram: Vec<u16>`, `outdegree_histogram: Vec<u16>`, `ancestor_bitset_sketch: Vec<u64>`, `descendant_bitset_sketch: Vec<u64>`, `path_hash_multiset: Vec<(u64, u16)>`, `signature_version: String`
  - `TopLevelQueryMetadata` 構造体の定義（クエリ側メタデータフィルタ入力）
  - `MemoizedGraph` に `top_metadata: TopLevelGraphMetadata` および `cheap_ged_signature: CheapGedSignature` フィールドを追加
  - `QueryRepresentation` に `top_query_metadata: TopLevelQueryMetadata` および `cheap_ged_signature: CheapGedSignature` フィールドを追加
  - `StructuralMatch` enum の v2.3-h 更新: `CheapGedScore(f32)`（Stage 3 結果）、`FullGedScore(f32)`（Stage 4 結果）、`GraphNeedsAbstraction`（例外パス）の 3 variant
* **テストコードによる検証:**
  1. 全構造体が `Debug`, `Clone`, `PartialEq` を derive することのコンパイル時検証
  2. `TopLevelGraphMetadata` の各フィールドがデフォルト値（ゼロ初期化）を持つことの確認
  3. `MemoizedGraph` のダミーインスタンス生成で新フィールドが初期化可能であることの確認
  4. `QueryRepresentation` のダミーインスタンス生成で新フィールドが初期化可能であることの確認
  5. 全フィールドのシリアライズ可能性（`serde` トレイト充足）のコンパイル時検証
* **計装方法・観測対象:** 型定義空間における v2.3-h 新設データ型の構造的完全性。新設 3 構造体のフィールド定義が RFC §8（MemoizedGraph）および §12（TopLevelGraphMetadata / CheapGedSignature）の定義と一致することの静的検証。

#### ✅ チケット M-0.75-2: 最上階 WorkflowGraph → TopLevelGraphMetadata 導出

* **対象不変条件 / 規範:** §12 Stage 2 Metadata Filter（v2.3-h）、§8 TopLevelGraphMetadata 定義
* **実装の背景と目的:** 4 層検索の Stage 2（SQLite metadata filter）は TopLevelGraphMetadata を入力として scored filter を実行する。本チケットは WorkflowGraph の最上階 DAG から 12 種のメタデータメトリクスを計算する純粋関数を実装する。すべての計算は決定論的で graph traversal のみに依存し、外部状態・乱数・不安定なハッシュに依存してはならない。
* **実装スコープ:**
  - `pub fn compute_top_level_metadata(graph: &WorkflowGraph) -> TopLevelGraphMetadata` 関数
  - DAG 走査によるメトリクス計算:
    - `top_node_count` / `top_edge_count`: 最上階ノード・エッジの総数
    - `top_source_count` / `top_sink_count`: ソースノード・シンクノードの数
    - `top_longest_path_len`: トポロジカルソート後の最長パス長
    - `top_max_width`: 同一トポロジカル階層の最大ノード数
    - `top_label_histogram`: ノードラベルの頻度分布（安定順序で Vec<(String, u16)> に集約）
    - `top_edge_type_histogram`: エッジ種別の頻度分布
    - `top_determinism_summary`: 全ノードの決定論性スコアの集約統計量（SoftMin 平均）
    - `top_sideeffect_summary`: 副作用種別ごとの有無を集約した SideEffectSummary
    - `top_agentsethash`: 全ノードの agent/tag set の安定ハッシュ（Vec<u64>）
    - `top_layer_signature`: トポロジカルレイヤーごとのノード数分布フィンガープリント（Vec<u64>）
  - 全メトリクスは同一入力に対して常に同一出力を返すこと（決定論性保証）
* **テストコードによる検証:**
  1. 既知の小規模 DAG（3 ノード、source→middle→sink）に対して手計算可能なメトリクス値との一致確認
  2. 同一グラフに対する 2 回の呼び出しが完全一致する決定論性テスト
  3. 空グラフ（ノード 0）に対するエッジケース（全カウント 0、空ヒストグラム）の確認
  4. 単一ノードグラフに対する source_count == sink_count == 1 の確認
  5. 分岐・合流を含む DAG に対する longest_path_len / max_width の正しさ確認
  6. ラベルカウントがノード種別ごとに正確に集計されることの確認
* **計装方法・観測対象:** グラフメトリクス計算関数の純粋性と決定論性。異なる構造パターン（直列・分岐・合流・並列分岐）の DAG に対して出力メトリクス分布を観測し、構造特徴の分離可能性を検証。同一グラフに対する 100 回の繰り返し計算で完全一致（分散 0）を確認。

#### ✅ チケット M-0.75-3: CheapGedSignature + TopLevelQueryMetadata 導出

* **対象不変条件 / 規範:** §8 CheapGedSignature、§9 TopLevelQueryMetadata（v2.3-h 改訂）
* **実装の背景と目的:** 4 層検索の Stage 3（Cheap GED Filter）は CheapGedSignature 間の lower-bound 近似で候補を枝刈りする。本チケットは WorkflowGraph から CheapGedSignature（7 成分）を、QueryDesignText から TopLevelQueryMetadata を導出する純粋関数を実装する。
* **実装スコープ:**
  - `pub fn compute_cheap_ged_signature(graph: &WorkflowGraph) -> CheapGedSignature` 関数:
    - `topo_rank_labels`: トポロジカル順序でソートされたノード種別ラベルの安定エンコード（Vec<u64>）
    - `indegree_histogram`: 入力次数の頻度分布
    - `outdegree_histogram`: 出力次数の頻度分布
    - `ancestor_bitset_sketch`: 各ノードの先祖到達可能性の bitset スケッチ（DAG 幅に応じて圧縮）
    - `descendant_bitset_sketch`: 各ノードの子孫到達可能性の bitset スケッチ
    - `path_hash_multiset`: 全 source→sink パスの経路ハッシュマルチセット（同一経路重複を含む）
    - `signature_version`: シグネチャ計算アルゴリズムのバージョン識別子
  - `pub fn compute_query_metadata(query_design_text: &str) -> TopLevelQueryMetadata` 関数:
    - QueryDesignText の構造スケッチからクエリ側メタデータを導出（§12 Stage 2 フィルタ入力用）
    - クエリの node_count 推定、edge_count 推定、label 分布推定、パス長推定
  - 全関数は決定論的かつ replayable であること（乱数・外部状態不使用）
* **テストコードによる検証:**
  1. 小規模 DAG に対する CheapGedSignature 各成分が手計算可能な値と一致することの確認
  2. indegree_histogram と outdegree_histogram の整合性（総 indegree == 総 outdegree == エッジ数）
  3. 同一グラフに対する 2 回の呼び出しが完全一致する決定論性テスト
  4. 異なるグラフ構造に対する signature の差異が構造的距離と相関することの確認
  5. `path_hash_multiset` が同一構造のグラフで同一値、異なる構造で異なる値を返すことの確認
  6. `signature_version` が固定文字列であることの確認
* **計装方法・観測対象:** シグネチャ計算の決定論性と構造弁別能力。同一グラフでの 100 回再計算による分散 0 の確認（決定論性）。異なるグラフ構造間でのシグネチャハミング距離分布を観測し、Cheap GED lower-bound としての識別可能性を評価。`path_hash_multiset` のエントロピーを計測し、異なる経路構造が十分に弁別可能なエントロピーを持つことを確認。

#### ✅ チケット M-0.75-4: v2.3-h 較正定数定義（4 層検索 pipeline 用）

* **対象不変条件 / 規範:** §27 付録 E Calibration Candidates（v2.3-h 追加分）
* **実装の背景と目的:** 4 層検索 pipeline（Stage 1-4）および Applicability Check（Stage 5）で使用される較正定数を `src/constants.rs` に追加する。これらの定数は後続の M-0.5-5/6/7 チケットで参照されるため、M-0.75 マイルストーン内で事前定義する。
* **実装スコープ:**
  - Pipeline stage 定数:
    - `K_SEM: usize = 20` — Stage 1 Semantic Retrieval top-k
    - `K_META: usize = 50` — Stage 2 Metadata Filter top-k
    - `K_CHEAP: usize = 20` — Stage 3 Cheap GED Filter top-k
    - `K_FULL: usize = 10` — Stage 4 Full GED Rerank top-k
    - `CHEAPGED_ENABLE_THRESHOLD: usize = 30` — cheap GED を有効化する最小候補数
    - `METAFILTER_THRESHOLD: f64 = 0.30` — metadata filter scored 閾値
  - Applicability blend 定数:
    - `SIMILARITY_ALPHA: f64 = 0.45` — S_total = α·S_sem + (1-α)·S_struct
    - `STRUCT_GED_LAMBDA: f64 = 4.0` — S_struct = exp(-λ·GED̃)
    - `APPLICABILITY_BETA: f64 = 0.70` — A_final = A_workflow^β · K^(1-β)
  - GED cost model 定数:
    - `GED_NODE_DELETE_COST: f64 = 1.0`, `GED_NODE_INSERT_COST: f64 = 1.0`
    - `GED_EDGE_DELETE_COST: f64 = 0.5`, `GED_EDGE_INSERT_COST: f64 = 0.5`
    - `GED_SIDEEFFECT_PENALTY: f64 = 3.0`, `GED_KIND_MISMATCH_PENALTY: f64 = 2.0`
    - `GED_AGENTSET_WEIGHT: f64 = 1.0`, `GED_IO_WEIGHT: f64 = 0.5`, `GED_DETERMINISM_WEIGHT: f64 = 1.0`
    - `FULLGED_TIMEOUT_MS: u64 = 5000`, `FULLGED_COST_MODEL_VERSION: &str = "v2.3-h-1"`
    - `CHEAPGED_LB_VERSION: &str = "v2.3-h-1"`
  - Metadata filter weight 定数:
    - `METAFILTER_W_V: f64 = 1.0`, `METAFILTER_W_E: f64 = 1.0`, `METAFILTER_W_L: f64 = 1.0`
    - `METAFILTER_W_P: f64 = 1.0`, `METAFILTER_W_S: f64 = 2.0`
  - 全定数に分類コメントを付与: Safety Invariant / Environment Policy Knob / Calibration Candidate
* **テストコードによる検証:**
  1. 全定数が `pub const` として定義され、コンパイル時に整数型・浮動小数点型として解決されることの確認
  2. 定数値が RFC 推奨初期値と一致することの確認
  3. 値の範囲が妥当範囲内であることのコンパイル時アサート（例: `SIMILARITY_ALPHA` ∈ [0,1], `K_SEM` ≥ 1）
* **計装方法・観測対象:** 定数定義の完全性と RFC との一致。全 20+ 定数が RFC §27 Calibration Candidates リストと一対一対応することの静的検証。各定数の分類（Safety Invariant / Calibration Candidate）がコメントとして正しく記述されていることの確認。

---

## ── 第2段階：擬似乱数・ノイズを投入した「制御された不確実性」検証（M-0.5 〜 M0.5） ──

> **DB**: この段階も引き続きメモリ内完結。PRNG ノイズは導入されるが、データベースは不要。

シード固定の疑似乱数生成器（PRNG）をインプラントし、システムに人為的なスコアの揺らぎやフォーマットの崩れを与え、安全弁の動作を精密観察するフェーズです。

### 4. マイルストーン M-0.5：Fake repository / embeddings

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### ✅ チケット M-0.5-1: メモリ内デュアルストア候補抽出及び統合・重複排除器（Stage 2c）の検証

* **対象不変条件 / 規範:** §12.2 Stage 2c union + dedupe
* **実装スコープ:** `Vec<RankedCandidate>` を受け取り、UUIDをキーとして一意にマージする純粋関数の実装。
* **テストコードによる検証:**

1. 2つのストア由来を模した重複あり・なしの混在リストを投入。
2. セマンティック側リストと構造側リストに同一UUIDで異なるスコアを持つデータを意図的に混入させ、統合後の配列が1件になり、かつ高い方のスコア（あるいは定義通りのブレンド値）が残ることをアサート。

* **計装方法・観測対象:** セマンティックリスト $K_{sem}$ 件、構造リスト $K_{struct}$ 件の要素を PRNG で大量に重複生成し、マージ関数に投入。 統合後のオブジェクトIDハッシュバケットへのインデックス再配置において、帰無仮説 $H_0: \text{スロット割り当ての確率は一様分布に従う}$ に対するカイ二乗検定統計量 $\chi^2 = \sum \frac{(O_i - E_i)^2}{E_i}$ の算出（自由度 $df = \text{バケット数} - 1$ ）。 マージイテレーションを繰り返した際の $p$値の分布が $[0.0, 1.0]$ 上で一様分布をなすこと（アルゴリズムにバイアスがないことの証明）、およびマージ多様体上での総アセットスコアの最大値保存則の検証。

#### ✅ チケット M-0.5-2: ランクドリフト頑健性シミュレーションテスト

* **対象不変条件 / 規範:** §12.2 類似度統合式及び順位付け、§21.1 OQ-04/08 近似挙動観察
* **実装スコープ:** 固定シード（`StdRng::seed_from_u64(12345)`）から生成されたガウスノイズを各候補の類似度スコア $S_{sem}, S_{struct}$ に確率的に加算するテスト用の FakeGraphStore / FakeMetadataStore（M-2-1.5）による検索エンジン実装。
* **テストコードによる検証:** 1,000回連続でダミー検索エンジンを回し、乱数ノイズによる順位の逆転（Drift）が頻発する環境下でも、上位選択アルゴリズムがインデックスアウトオブバウンズ等の致命的クラッシュを起こさず、期待値通り最高値の候補を引き当てられるかをアサート。
* **計装方法・観測対象:** 候補ベクトル空間のコサイン類似度に、固定シード由来の微小ホワイトノイズ $\xi(t)$ を定常注入。 イテレーション $t$ に伴う、真の最高ランク候補の順位変動軌道の平均二乗変位 $\langle |x(t) - x(0)|^2 \rangle \sim t^{2H}$ におけるハースト指数 $H$ の推定。 ノイズ強度が臨界値 $\sigma^2_c$ 未満において $H \le 0.5$（通常の拡散、またはサブディフュージョン：局所トラップ安定性）が維持され、異常なスーパーディフュージョン（ $H > 0.5$ ：カオス的順位崩壊）へ相転移しないための、最高候補の生存時間包廊線（カイザークラフ変換）の同定。

#### ✅ チケット M-0.5-3: 埋め込みモデル・テンプレートバージョン不整合（AG-06 / AG-07）ハードゲートの全弾ブロックテスト

* **対象不変条件 / 規範:** §11.1 AG-06、AG-07 ハードゲート仕様
* **実装スコープ:** 候補の `EmbeddingChannelVersion` を読み取り、クエリ側の宣言と不一致であれば即時 `false` を返すApplicabilityゲートの実装。
* **テストコードによる検証:** テストコード側で確率的に候補のバージョン文字列を `"v2.0-final"` から `"v1.8-legacy"` へ書き換えるループを構築。 不整合が発生した候補が、Stage 1 の段階で100%ここに漏れなく排除され、後段のロジックに一切進入しないことをアサート。
* **計装方法・観測対象:** クエリと候補のバージョン文字列間のハミング距離（不一致ビット数） $E$ を制御パラメータとして、Applicabilityハードゲートに $10^5$ 回走査投入。 距離 $E$ に対するゲート通過確率 $P_{pass}(E)$ の配置特性。 計装プローブにより $P_{pass}(E) = \frac{1}{1 + \exp(\beta(E - E_c))}$ （ただし化学ポテンシャルに相当する臨界距離 $E_c = +1$ ビット、逆温度 $\beta \to \infty$ ）の階段関数マッピングを実測。 $E \ge 1$ における False Positive フラックス（誤通過率）が理論限界有意水準 $\alpha = 0.00$（完全遮断）に完全に固定されていることの数理的・統計的検証。

#### ✅ チケット M-0.5-4: HITL HumanChannel 抽象トレイトの定義

* **対象不変条件 / 規範:** §12B HumanChannel Communication Abstraction（新設）
* **実装の背景と目的:** 本チケットは人間との双方向通信（Human-in-the-Loop）をワークフローの一級市民として抽象化する基盤層を定義する。M1-1 の人間レビューキューや §13A Training Orchestrator はこの下層基盤の上で動作する。単なる一方向通知（`notify`）に加え、応答を待つ双方向通信（`communicate`）とクラッシュ後の再接続（`reconnect`）を提供する `HumanChannel` トレイトを定義する。後段で WebSocket / Slack / Email / Tauri ダイアログ等の具象通信手段に差し替える際、トレイトの別実装を追加するだけで完了する。
* **実装スコープ:**
  - `HumanChannel` トレイト: `fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError>`、`fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>`、`fn reconnect(&self, interaction_id: Uuid, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>`
  - `InteractionHandle`: `interaction_id: Uuid` + `mpsc::Receiver` + `fn wait(timeout: Option<Duration>) -> Result<HumanOutcome, DarviumError>`
  - データ型: `HumanRequest` (subject/body/context/timeout)、`HumanOutcome` (Responded/TimedOut/Unreachable)、`HumanResponse` (decision/comment/revised_body)、`HumanDecision` (Approved/Rejected/NeedsRevision/Irrelevant/Unsafe)、`StoredInteraction` (interaction_id/request/outcome/status/created_at/updated_at)、`InteractionStatus` (Pending/Resolved)
  - `FakeHumanChannel`: プリロードされた `VecDeque<HumanOutcome>` + `HashMap<Uuid, InteractionRecord>` + アトミックカウンター。`export_interactions()` を提供
  - `StdinoutChannel<R: BufRead + Send, W: Write + Send>`: JSON Lines プロトコルによる参照実装。reader スレッド + mpsc パターンによる非同期読み取り
  - `MetadataStore` 4 メソッド追加: `store_human_interaction()`, `load_human_interaction()`, `list_pending_human_interactions()`, `resolve_human_interaction()`
  - エラー型: `DarviumError::HumanChannelIo(String)`, `DarviumError::HumanChannelClosed`
* **テストコードによる検証:**
  1. トレイト境界充足: `FakeHumanChannel` が `HumanChannel` トレイトを実装することのコンパイル時検証
  2. `notify()` 呼び出し後、`requests_sent` に内容が追跡されカウントがインクリメントされること
  3. `communicate()` → `InteractionHandle::wait(None)` でプリロード済み `HumanOutcome::Responded` が取得できること
  4. `communicate()` → `wait(Some(短期間))` でタイムアウトが正しく `HumanOutcome::TimedOut` として返ること
  5. `reconnect()` が既存 `Pending` 状態のインタラクションを正しく復旧し、復旧後に `wait()` で応答が取得できること
  6. トレイトのオブジェクト安全性確認（`Box<dyn HumanChannel>`）
  7. `FakeHumanChannel.export_interactions()` が全記録を `Vec<StoredInteraction>` として出力すること
  8. クラッシュリカバリプロトコルの検証: `reconnect()` による新インスタンス復旧（プリロードキュー空の場合に `Err(HumanChannelIo)` が返ること）
* **計装方法・観測対象:** §12B.11 に基づく 6 つの観測指標:
  - HITL 完了率: `communicate()` 呼び出し数に対する `HumanOutcome::Responded` の割合。FakeHumanChannel の AtomicU64 カウンターから構造化テキスト出力
  - HITL タイムアウト率: 全インタラクション中の `HumanOutcome::TimedOut` 割合
  - HITL 到達不能率: `HumanOutcome::Unreachable` の発生率
  - 応答レイテンシ分布: `StoredInteraction.created_at` から `resolved_at` までの経過時間の統計分布（中央値・P90・P99）。OTS で `println!` + `--nocapture` 経由で計測
  - クラッシュリカバリ成功率: `reconnect()` 成功数 / 再起動後総試行数
  - MetadataStore 整合性: `list_pending()` 全件に対する reconnect 試行の成否率
* **M1-4 への委譲事項:** 本チケットがカバーしない以下のギャップは M1-4（#48）で解決する: (a) 複数 Pending の一括回復, (b) StdinoutChannel クロスインスタンス回復, (c) TimedOut 状態からの再通知経路, (d) 回復中競合状態のテスト。RFC §12B.13 委譲テーブルを参照。

---

#### ✅ チケット M-0.5-5: 4 層検索 Stage 2 Metadata Filter（scored filter）の実装

* **対象不変条件 / 規範:** §12 Stage 2 Metadata Filter（v2.3-h）、式(12) scored filter
* **実装の背景と目的:** 4 層検索の Stage 2 は、TopLevelQueryMetadata と各候補 MemoizedGraph の TopLevelGraphMetadata を比較する scored filter で候補数を K_META まで削減する。グラフ本体をロードせずメタデータのみでフィルタリングする。
* **実装スコープ:**
  - `pub fn metadata_filter(query_meta: &TopLevelQueryMetadata, candidates: &[&MemoizedGraph], k_meta: usize) -> Vec<CandidateId>` 関数
  - 式(12) の scored filter: M(q,G) = w_vΔV + w_eΔE + w_lΔL + w_pΔP + w_sΔS
    - ΔV: node count 差分（正規化絶対差）
    - ΔE: edge count 差分（正規化絶対差）
    - ΔL: label histogram 間の余弦距離（Jensen-Shannon 発散代替）
    - ΔP: longest_path_len + layer_signature の加重距離
    - ΔS: side effect summary の不一致ペナルティ（不一致種別数 × 係数）
  - 重み w_v, w_e, w_l, w_p, w_s は constants として定義（較正候補）
  - TopK (最小 M 値) の K_META 件を選択、K_META 未満の場合は全件通過
  - 全演算はメモリ内の Vec/HashMap で実施（SQLite エミュレーション不要）
* **テストコードによる検証:**
  1. 完全一致する metadata ペアで M(q,G) == 0.0 となることの確認
  2. 大幅に異なる metadata（node_count 10 倍差）で M(q,G) が閾値を超えることの確認
  3. 候補数 > K_META の場合に正確に K_META 件が返ることの確認
  4. 候補数 <= K_META の場合に全件が通過することの確認
  5. 各 Δ 成分が単独で変化したときの M 値の単調性確認（重み符号方向に単調）
  6. side effect 不一致ペナルティが正しく加算されることの確認
* **計装方法・観測対象:** メタデータフィルタリングの選別精度と候補削減率。T_q 件の候補に対してフィルタ通過率（K_META / T_q）を測定し、Stage 2 の枝刈り効果を定量化。Δ 成分ごとの寄与率分布を観測し、各成分のフィルタリング有効性を評価。

#### ✅ チケット M-0.5-6: 4 層検索 Stage 3 Cheap GED Filter + Stage 4 Full GED Rerank

* **対象不変条件 / 規範:** §12 Stage 3 Cheap GED Filter（式13-15）、Stage 4 Full GED Rerank（式16-20）（v2.3-h）
* **実装の背景と目的:** Stage 3 は CheapGedSignature 間の lower-bound 近似 GED で候補を枝刈りする。Stage 4 は node alignment + edit cost model による full GED を計算し、最終上位 K_FULL 件を決定する。
* **実装スコープ（Cheap GED Filter — Stage 3）:**
  - `pub fn cheap_ged_filter(query_sig: &CheapGedSignature, candidates: &[(&MemoizedGraph, f32)], k_cheap: usize, enable_threshold: usize) -> Vec<(&MemoizedGraph, f32)>` 関数
  - 式(13) lower-bound: LB(q,G) ≤ GED(q,G) を満たす 5 成分の近似
    - node/edge count lower bound: |V_q - V_G| + |E_q - E_G|
    - label multiset mismatch: histogram cosine 距離
    - topological layer mismatch: topo_rank_labels の編集距離下限
    - reachability sketch mismatch: ancestor/descendant bitset の不一致率
    - path hash multiset mismatch: path_hash_multiset の積集合非一致率
  - 式(14) 閾値フィルタ: LB(q,G) ≤ τ_cheap(q) を満たす候補を通過
  - 式(15) TopK 方式: -LB(q,G) の上位 K_cheap 件選択
  - candidate count ≤ enable_threshold の場合、cheap GED を skip して全件通過
  - cheap GED skip 時はその理由（candidate count below threshold）を追跡
* **実装スコープ（Full GED Rerank — Stage 4）:**
  - `pub fn full_ged_rerank(q: &QueryRepresentation, candidates: &[&MemoizedGraph], k_full: usize) -> Vec<RankedCandidate>` 関数
  - 式(17) GED = min_π Σ c_V + Σ c_E + c_ins/del
  - 式(18) ノード置換コスト: η_k * 1[kind mismatch] + η_a * (1-J_A) + η_i/o * (1-J_I/O) + η_d * |det diff|
  - 式(19) エッジ置換コスト: η_t * 1[type mismatch] + η_b * 1[branch mismatch]
  - 式(20) ノード削除/挿入コスト: δ_0 + δ_se * risk（side effect 高コスト）
  - 式(16) TopK: -GED(q,G) の上位 K_full 件選択
  - cost model version を SearchTrace に記録（§12.3C）
* **テストコードによる検証:**
  1. LB(q,G) ≤ GED(q,G) が常に成立すること（lower-bound 健全性）の 100 件ランダムテスト
  2. 同一グラフペアで cheap GED == 0.0 かつ full GED == 0.0 となることの確認
  3. 大幅に異なるグラフ（ノード数 5 倍差）で LB が閾値を超えることの確認
  4. cheap GED skip 条件が正しく動作することの確認（count ≤ threshold → 全件通過）
  5. full GED のノード置換コストが kind mismatch で正しく加算されることの確認
  6. side effect penalty が正しく加算されることの確認
  7. full GED が決定論的であることの 100 回繰り返し確認
  8. K_FULL 件の TopK が正確であることの確認
* **計装方法・観測対象:** Cheap GED の PruneGain（1 - K_cheap / K_meta）と MissRate（正解候補の誤棄却率）の計測。Full GED の edit cost 成分ごとの寄与率分布。同一グラフペアに対する cheap GED と full GED の散布図観測による lower-bound 品質の可視化。Stage 3 通過前後の候補集合の重複率（Jaccard 係数）。

#### ✅ チケット M-0.5-7: 4 層検索パイプライン統合（Stage 1→2→3→4→5）+ Applicability 結合

* **対象不変条件 / 規範:** §12 Stage 1-5 全体（v2.3-h）、§11 Applicability Check（式6-10）、疑似コード §12.3D
* **実装の背景と目的:** 4 層検索パイプライン全体を統合する retrieve_top_level_candidates オーケストレータと、最終候補に対する evaluate_candidate 評価関数を実装する。各 stage 通過後の候補数単調減少不変条件を検証する。
* **実装スコープ:**
  - `pub fn retrieve_top_level_candidates(q: &QueryRepresentation, repo: &WorkflowRepository, k_sem: usize, k_meta: usize, k_cheap: usize, k_full: usize) -> Vec<RankedCandidate>` 関数
    - Stage 1（Semantic Retrieval）: `semantic_topk(&q.task_embedding, repo, K_SEM)` — task_embedding のみの余弦類似度 TopK
    - Stage 2（Metadata Filter）: M-0.5-5 の metadata_filter を呼び出し
    - Stage 3（Cheap GED Filter）: M-0.5-6 の cheap_ged_filter を呼び出し
    - Stage 4（Full GED Rerank）: M-0.5-6 の full_ged_rerank を呼び出し
    - Stage 5（Applicability Evaluation）: evaluate_candidate を呼び出し
  - 不変条件検証: N_sem ≥ N_meta ≥ N_cheap ≥ N_full（各 stage 通過後の候補数単調減少）
  - `pub fn evaluate_candidate(q: &QueryRepresentation, g: &MemoizedGraph, full_ged: f32) -> ApplicabilityOutcome` 関数
    - 式(6) S_sem = max(0, cosine(task_embedding))
    - 式(7) S_struct = exp(-λ * normalize_ged(full_ged, q, g))
    - 式(8) S_total = α * S_sem + (1-α) * S_struct
    - 式(9) A_workflow = max(S_total, f_S)^αS * max(D, f_D)^αD * max(T, f_T)^αT
    - 式(10) A_final = A_workflow^β * K^(1-β)（knowledge-aware 時）
    - 推奨初期値 α=0.45, λ=4.0, β=0.70
  - 各 stage の中間結果（候補集合・スコア）を SearchTrace 互換で記録
  - 全 stage の tie-break は WorkflowGraphId の安定順序で固定（§12.3C）
* **テストコードによる検証:**
  1. 合成クエリと既知の WorkflowGraph 集合に対するパイプライン全体の実行確認
  2. 候補数単調減少不変条件の検証（N_sem ≥ N_meta ≥ N_cheap ≥ N_full）
  3. Stage 1 で semantic 類似度が高い候補が上位にランクされることの確認
  4. Stage 5 で正当な REUSE/PATCH/NEW/ABORT 判定が出力されることの確認
  5. 各 stage スキップ時の動作確認（candidate count = 0 で後続 stage が空を返す）
  6. evaluate_candidate の決定論性検証（100 回繰り返しで同一結果）
  7. 同一 query に対する pipeline の 2 回実行が完全一致することの確認
* **計装方法・観測対象:** パイプライン全体の candidate 減少曲線（N_sem → N_meta → N_cheap → N_full）の観測。各 stage のレイテンシ分布（仮想命令ステップ数計測）。最終出力の決定論性（100 回繰り返しの分散 0 確認）。各 Applicability 判定（REUSE/PATCH/COMPOSE/NEW/ABORT）の出現率分布。

#### ✅ チケット M-0.5-8: AG-07 v2.3-h 更新（cheap_ged_signature_version + ged_cost_model_version ゲート）

* **対象不変条件 / 規範:** §11 AG-07（v2.3-h 版: cheap_ged_signature_version + ged_cost_model_version による互換性ゲート）
* **実装の背景と目的:** AG-07 は互換性ゲートとして cheap_ged_signature_version と ged_cost_model_version の一致チェックを行う。v2.3-g までは WorkflowDesignEmbedding の model_version を使用していたが、v2.3-h の 4 層検索移行に伴い現在の方式に変更された。
* **実装スコープ:**
  - AG-07 の互換性ゲート条件: `memoized_graph.cheap_ged_signature.signature_version == query.cheap_ged_signature.signature_version && memoized_graph.top_metadata.ged_cost_model_version == query.ged_cost_model_version`
  - チェック失敗時のエラー種別: `ApplicabilityError::SignatureVersionMismatch`
  - AG-07 の較正定数: `cheap_ged_signature_version` / `ged_cost_model_version`（§27 較正候補）
* **テストコードによる検証:**
  1. version 一致時の正常通過（AG-07 が Ok を返す）
  2. cheap_ged_signature_version 不一致時のブロック（AG-07 が Err を返す）
  3. ged_cost_model_version 不一致時のブロック（AG-07 が Err を返す）
* **計装方法・観測対象:** version 比較の PASS/FAIL 比率。

### 5. マイルストーン M0：Composition / New proposal 基盤

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### ✅ チケット M0-1: `CompositionPlan` データ整合性及び変数スコープ静的バリデータの実装

* **対象不変条件 / 規範:** §13.3 構造定義、§6.4 変数スコープ前向き走査（V-03, V-04）。加えて v2.3 における frontier-based parallel execution を前提として、変数スコープ検証は serial compile path だけでなく parallel ready frontier 上でも未解決変数と scope leakage を防がなければならない。
* **実装スコープ:** `CompositionPlan` 内の `composition_edges` を走査し、送信ノードの `output_var` が受信ノード of `inputs` に存在するかを確認する検証アルゴリズム。
* **テストコードによる検証:** データフローの変数名が一致しない不正な `CompositionPlan` をメモリ上で組み立ててバリデータに投入し、必ず `Err(ValidationError::UnresolvedVariable)` が返却されることを確認する。加えて、互いに独立な parallel-ready ノードが同名変数を扱うケースや、片方の出力を不正に先読みするケースを投入し、並列 frontier 上でも scope leakage / unresolved variable が確実に検出されることを確認する。
* **計装方法・観測対象:** ノード数 $n \in [10, MAX\_GRAPH\_NODES]$、エッジ生成確率 $p \in [0.0, 1.0]$ のランダム有向グラフ（Erdős–Rényiアンサンブル）を $10^4$ 個自動生成し、未解決変数を確率的に埋め込んでバリデータに投入。 グラフ構造の代数的複雑さ（隣接行列のシャノン・グラフトポロジーエントロピー $H_{graph}$ ）と、バリデータの異常検知ステップ数（命令数 $C_{validate}$ ）の相関関数。 グラフサイズが臨界点（巨大成分の出現境界 $p = 1/n$ ）を通過する際の、捕捉潜時のスケーリング指数 $\gamma$ （ $C_{validate} \sim n^\gamma$ ）の計測、および最大サイズ限界（512ノード）における最悪計算ステップ数が有界であることの実証。

#### ✅ チケット M0-2: `GenerateNew` 選択時のレビュー強制・安全ガードロジックの検証

* **対象不変条件 / 規範:** §13.6 ガード条件「side-effect safety invariant に反する SearchStep 遷移は UnsafeSearchTransition として拒否すること」。加えて v2.3 では、production plane では `writes_external_api` / `irreversible` / 永続状態変更を含む `GenerateNew` は必ず human review を要し、training plane の safe sandbox に限って明示的に許容された safe-scope new proposal にのみ限定的 auto-approval 例外を認めうる。
* **実装スコープ:** `GenerateNew` を選択した際、対象となるミッションの副作用プロファイルと plane 属性を確認し、production plane では即座に実環境へ投入せず人間承認待ち状態へバイパスするガードロジックを実装する。併せて training plane / safe sandbox の場合に限り、明示的に安全と分類された scope に対してのみ自動承認を許容する分岐を設計に含める。
* **テストコードによる検証:** `writes_external_api: true` を含む要求に対し、レビューを通さないダイレクトな採択経路を模倣したコンテキストを入力した際、システムが検知して `Err(SearchValidationError::UnsafeSearchTransition)` を投げることを検証する。加えて、同等の proposal であっても production plane では review へ送られ、training plane の safe sandbox では許可条件を満たす場合のみ auto-approval 例外が発動する対照ケースを確認する。
* **計装方法・観測対象:** 副作用ベクトル空間 $\mathbf{E} = (writes\_api, sends\_notif, modifies\_db)$ 上の全 $2^3 = 8$ パターン、および危険スコア $risk\_score \in [0.0, 1.0]$ を連続変化させた危険要求アンサンブルを大量注入。 ガード判定器の内部状態空間における、ダイレクト採択軌道の閉包特性。 `writes_external_api: true` または `irreversible: true` の条件下で、システム軌道が「本番実行可能状態」の位相的開集合（Open Set）へ進入する確率が厳密に 0 であり、すべての軌道が「人間レビュー待ち集合（閉包）」のコンパクト空間へと完全にホモトピー射影されることのトラッキング検証。

#### ✅ チケット M0-3: PRNG駆動型擬似提案スコア（Confidence）による結果多様性シミュレーション

* **対象不変条件 / 規範:** §13.3 & §16.1 Empirical Claimの検証足場
* **実装スコープ:** 擬似乱数を用いて、プランの `confidence` 値を `[0.30, 0.95]` の範囲でバラつかせるMock提案器の実装。
* **テストコードによる検証:** テストループを 500 回実行し、Confidence が低ければ `Refine` 状態へ、高ければ `Finalize` 状態へ状態機械が正しく、かつ確率分布を反映した挙動で分岐することを確認。
* **計装方法・観測対象:** 判定ロジックに入力されるプランの内部信頼度ベクトル $\mathbf{C} = (c_s, c_v, c_h)$ に対し、微小変化 $\delta \mathbf{C}(0) = 10^{-6}$ を加えたツイン軌道を実行。 探索状態機械のイテレーション進行に伴うツイン軌道間の最大距離変位の対数成長率（リアプノフ指数 $\lambda = \lim_{t \to \infty} \frac{1}{t} \ln \frac{|\delta \mathbf{C}(t)|}{|\delta \mathbf{C}(0)|}$ ）。 状態遷移が不連続にスイッチする境界線上での局所リアプノフ指数 $\lambda_{local}$ をプロットし、システム全体の期待値として $\lambda \le 0$ （非カオス・局所収束安定性）が満たされていること、および目的関数 $J(\theta)$ に対する較正ループの安定収束動態の実測。

---

### 6. マイルストーン M0.5：Fake LLM adapter（プロパティベースフェイルセーフ）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### ✅ チケット M0.5-1: スクリプト化された壊れたフォーマット出力 Fake LLM クライアントの実装

* **対象不変条件 / 規範:** §14.2 構造化出力・JSONパースエラーハンドリング
* **実装スコープ:** 文字列を返すMock LLMにおいて、確率的に「JSONの末尾カッコが欠落している」「指定キー名が異なる」といった不正データを生成するクライアントの実装。
* **テストコードによる検証:** 不正フォーマットが注入された際、システムがクラッシュせず、`LlmError::MalformedJson` として安全にキャッチし、上位のリカバリロジックにエラーをハンドリングできることを検証。
* **計装方法・観測対象:** LLM出力文字列のビット反転、ノードリストキー名の欠落、カッコのネスト破壊を誘発するランダム・ビット・ミュートエントロピー $p_m \in [0.0, 1.0]$ を連続制御してデシリアライザに入力。 不正構文によるシステムエラー率の相転移プロファイル。 構文破壊確率 $p_m$ が臨界値 $p_c$ を超えた際の、デシリアライザの例外発生フラックスの応答曲線、およびどれほどエントロピーが高まっても、Rustの安全なメモリ管理（パニックの抑止と `Result::Err` への制御されたシンク射影）が 100% 維持され、未定義動作（UD）によるシステムハングが完全ゼロであることの確認。

#### ✅ チケット M0.5-2: 確率的パッチ操作インジェクションによるバリデータ耐久テスト

* **対象不変条件 / 規範:** §14.4 `validate_patch_result`（DAG検証、スコープ検証不変条件）。加えて v2.3 では、DAG 検証は create / update 時だけでなく compile / execute 前にも要求され、patch-time validation はその二重防御の一部である。また 1 候補の DAG failure は hard reject reason ではあるが、SearchWorkflow 全体の即時終了理由ではない。
* **実装スコープ:** `apply_patch_atomic` のフェーズ3において、生成されたパッチがグラフのDAG性を破壊していないかを評価する `petgraph::algo::toposort` 連携部。さらに、この validation は patch-time 専用の局所チェックではなく、create / update 段階と compile / execute 前段階の双方に接続される DAG 健全性防御の一部として位置づける。
* **テストコードによる検証:** プロパティベーステスト（シード固定）により、既存の安全なDAGグラフに対し、ランダムなノード接続 edge操作（`PatchOperation::AddEdge`）を大量に生成・適用。 **意図的に巡回ループ（Cycle）を発生させたケースにおいて、バリデータが100%それを検知し、`PatchError::CycleCreated` としてパッチをアトミックに拒否できること**をアサート。
* **計装方法・観測対象:** 既存の正当なDAG（ノード数最大512）に対し、プロパティベーステスト（固定シード）によってランダムなバックエッジを挿入し、意図的に強連結成分（SCC：Strongly Connected Component）を形成させた変異グラフアンサンブルを大量投入。 ランダム・グラフ変異空間上の隣接行列のスペクトル半径 $\rho$ （最大固有値）の動的変化と、バリデータのループ検出フラックスの代数的完全性検証。 巡回ループ存在（ $\rho > 0$ ）時におけるアトミック拒否成功率の完全性アサーション（サンプル数 $10^4$ ）、および二項分布に基づく見落とし確率の上側 99% 信頼限界が $p_{miss} < 4.6 \times 10^{-4}$ 以下に封じ込められていることの数学的証明。

#### ✅ チケット M0.5-3: パッチ適用における未解決変数（VarScopeViolation）の確率的検出テスト

* **対象不変条件 / 規範:** §14.3 バリデータスコア $c_v$ 減算規則、§14.4 変数スコープ検証
* **実装スコープ:** パッチ適用後の仮グラフに対し、入力変数スコープの前向き整合性を走査するアルゴリズム。
* **テストコードによる検証:** ランダムに生成したパッチの変数宣言を壊し（存在しない変数からのDataFlow接続など）、バリデータに投入。 減算ルール通り、未解決変数1件につきバリデータスコア $c_v$ が `-0.15` 正確に引かれ、閾値を下回った場合は自動的に `PatchError::LowConfidence` へ落ちることをアサート。
* **計装方法・観測対象:** パッチ生成器の出力する未解決変数バグの件数 $E_v$ を 0 から 10 まで精密にインクリメント注入。 複合信頼度関数 $PatchConfidence = c_s^{w_s} \cdot c_v^{w_v} \cdot c_h^{w_h}$ の出力に対する、バリデータスコア $c_v(E_v) = 1.0 - 0.15 \cdot \min(E_v, 3)$ の偏微分感度。 $E_v \le 3$ 領域において、計装データから得られる微分係数の分散が $\sigma^2(\frac{\partial PatchConfidence}{\partial E_v}) = 0$ で完全に定数直線上に拘束されていること、および $c_s < 0.50$ における重みの動的切り替え点（ $w_v: 0.40 \to 0.50$ ）を通過した瞬間の、決定勾配ベクトル場の幾何学的不連続ジャンプ（相転移不連続性）の測定。

---

### ── 第3段階：エコシステム・人間介在の論理検証（M1 〜 M1.5） ──

> **DB**: メモリ内完結。SQLite / LadybugDB 不要（「異種データストア間のコミット整合性」の試験もメモリ内エミュレーション）。

人間フィードバックによる非同期シグナルや、異種データストア間のコミット整合性など、システム周辺の統合コンポーネントをメモリ内で結合・検証するフェーズです。

### 7. マイルストーン M1：Human-in-the-loop review

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### ✅ チケット M1-1: `NeedsHumanReview` 発生時の隔離レビューキューイングロジックの検証

* **対象不変条件 / 規範:** §13.3 SearchOutcome バリアント、§13A Training Orchestrator 連携
* **実装スコープ:** 検索結果が人間レビューを要求した場合に、対象のミッションとコンテキストを専用のメモリ内スタック（`HumanReviewQueue`）へプッシュし、状態を中断状態（Pending）で固定する機能。
* **テストコードによる検証:** レビュー待ちに入ったミッションが、人間の明示的な応答（`HumanDecision::Approved` / `HumanDecision::Rejected`）が `HumanChannel` 経由で到着するまで、通常の自動実行ラインに絶対に復帰しないことを確認。
* **計装方法・観測対象:** 探索エンジンから `NeedsHumanReview` シグナルを平均到着率 $\lambda$ でメモリ内キューへ連続注入し、人間処理フラックスを意図的に 0 （ $\mu = 0$ ）に固定。 時間発展に伴うキューの滞留長 $L_q(t) = \lambda t$ の完全なる線形成長ダイナミクス、および自動実行スレッド群からのアクセスに対するスレッドセーフ（ロック・コンテンション）時のセマフォ待機時間の確率分布。 自動実行スレッドへのアセット情報リーク率が確率空間上で厳密に $P_{leak} = 0$ の壁（無限大ポテンシャル障壁）を維持していることの一貫性検証。

#### ✅ チケット M1-2: 管理者 `AdminFastTrack` 発動時における信頼値強制更新と `TrustAuditLog` 生成不変条件の検証

* **対象不変条件 / 規範:** §8.2 「管理者 fast-track を適用した場合、その操作を TrustAuditLog に記録しなければならない (SHOULD/MUST)」
* **実装スコープ:** `apply_admin_fast_track` メソッドの実装。 `HumanTrustLogistic.score` を強制的に `0.80` に固定し、キャッシュ無効化フラグを立て、`TrustAuditLog` 配列にレコードを追加する一連のアトミック操作。
* **テストコードによる検証:** メソッド呼び出し後、対象グラフの `trust.human.score` が正確に `0.80` に書き換わっていること、および監査ログ配列の最後の要素の `event_type` が `TrustAuditEvent::AdminFastTrack` であることをアサート。
* **計装方法・観測対象:** メモリ上に $10^3$ 個の Applicability キャッシュ多様体（依存アセット）を配置した状態で、`apply_admin_fast_track` を一斉発動。 信頼値のデルタ関数的跳躍（ $T_{new} = 0.80$ 強制固定）がインプットされた瞬間の、周辺依存キャッシュの同期クエンチ（消滅）レイテンシのサンプリング。 キャッシュ無効化シグナルが依存トポロジーグラフ上を伝播する際、ノード間の最短経路距離（ホップ数）に関わらず、同期の遅れが仮想命令ステップ軸上で $\Delta t_{step} = 0$ （完全原子的・即時同期クエンチ）で完了することのアトミック性実証。

#### ✅ チケット M1-3: 人間フィードバック非同期連続注入に対する Debounce（キャッシュ無効化抑制）ロジック의 検証

* **対象不変条件 / 規範:** §10.5 TrustUpdate::Human 「スコア変動が閾値 TRUST_DEBOUNCE_DELTA (0.05) 未満の場合はキャッシュ無効化をスキップする」
* **実装スコープ:** 複合信頼スコアの差分絶対値を計算し、`0.05` 未満であれば `invalidate_applicability_cache()` の実行をバイパスする条件分岐ロジック。
* **テストコードによる検証:** 非常に微小なフィードバック（例: 複合スコアが `0.01` しか動かない thumbs-up）を連続で10回入力。 1回ごとに内部のキャッシュクリア関数が呼ばれたかどうかのフラグをアサートし、スキップ条件通りフラグが `false` のままである（無駄なキャッシュ破棄が発生しない）ことを確認。
* **計装方法・観測対象:** `TrustUpdate::Human` に対し、複合スコア変動デルタ $\Delta T$ を $0.000$ から $0.100$ まで $0.001$ 刻みで連続変化させたフィードバックパルスを大量注入。 キャッシュ無効化発動フラグの応答特性（ステップ関数 $\theta(\Delta T - TRUST\_DEBOUNCE\_DELTA)$  ）。 $\Delta T < 0.05$ の不感帯領域における無効化フラックス（キャッシュクリア発生率）が厳密に 0.00 、$\Delta T \ge 0.05$ に到達した瞬間にフラックスが 1.00 へと垂直に跳躍するヒステリシス曲線の曲率測定、および不感帯境界のシャープさの限界実測。

#### ✅ チケット M1-4: HITL 起動時回復ループ — 全Pendingインタラクションの確実な再開保証（#48）

* **対象不変条件 / 規範:** §12B.6 クラッシュリカバリプロトコル、§12B.5 インタラクション状態機械
* **実装スコープ:**
  - JsonMetadataStore 簡易ファイル永続化（起動時読込 + 変更時原子書込、依存追加不要）
  - MetadataStore 上の全 Pending HITL インタラクションに対する起動時回復ループ（`list_pending → reconnect × N → wait → resolve`）
  - 複数 Pending の一括回復テスト（N ≥ 10）
  - StdinoutChannel クロスインスタンス回復（プロセス再起動越え）
  - TimedOut 状態からの再通知経路の設計・実装
  - 回復中競合状態のテスト
* **テストコードによる検証:**
  1. JsonMetadataStore の永続化・復元
  2. 単一 Pending の Orchestrator 経由回復
  3. N≥10 一括回復（全件成功）
  4. 混合シナリオ（成功 + タイムアウト + 到達不能）
  5. StdinoutChannel クロスインスタンス回復
  6. TimedOut 状態からの再通知
  7. 競合状態テスト（旧プロセス応答直後クラッシュ）
  5. TimedOut 状態からの再通知
  6. 競合状態テスト（旧プロセス応答直後クラッシュ）
* **計装方法・観測対象:**
  - バッチ回復成功率: list_pending 件数に対する reconnect 成功件数の比率
  - 回復レイテンシ分布: 回復ループ開始から全件解決までの経過時間（中央値・P90・P99）
  - TimedOut 変換率: 回復ループ内でタイムアウトにより打ち切られたインタラクションの比率
  - 競合検出率: 競合状態テストでの不整合検出率（期待値: 0）

---

### 8. マイルストーン M1.5：Real embedding provider 擬似結合

> **DB**: メモリ内完結。SQLite / LadybugDB 不要（「SQLite側」「LadybugDB側」という表現はエミュレーション）。

#### ✅ チケット M1.5-1: 実フォーマット形状ベクトル（1536次元等）のメモリ内 HNSW インデックス検索（Stage 2a/2b）Mockの検証

* **対象不変条件 / 規範:** §12 4-Layer Retrieval、§25 データベース構成
* **実装スコープ:** ダミーの固定多次元配列を保持するメモリ内インデックス構造体を作り、入力されたクエリベクトルとの間で擬似的なコサイン類似度上位 $k$ 件を返す空間検索関数の実装。
* **テストコードによる検証:** あらかじめ特定の多次元配列を数件登録。 それと完全に一致するクエリを入力した際、Stage 2c の統合フェーズへ最高類似度 `1.0` として最上位にソートされて引き渡されることを確認。
* **計装方法・観測対象:** 1536次元の単位超球（Unit Hypersphere）上の HNSW 擬似グラフ空間において、ランダムサンプリングされた 3 つの埋め込みベクトル（クエリ $\mathbf{q}$、候補 $\mathbf{a}$、候補 $\mathbf{b}$）を生成。 コサイン計量から誘導される幾何学的三角不等式 $d(\mathbf{q}, \mathbf{b}) \le d(\mathbf{q}, \mathbf{a}) + d(\mathbf{a}, \mathbf{b})$ の充足率。 インデックス検索 Mock がソート結果を上位 $k$ 件として返す際のソートインバリアントの普遍性、および多次元ノイズを付加した際の検索結果集合の距離空間的コンパクト性・有界性の統計的実証。

#### ✅ チケット M1.5-2: 異種ストア論理一貫性コミット（ConsistencyState::Pending）プロトコルのシミュレーション

* **対象不変条件 / 規範:** §18.2 & §25.x クロスストア書き込み規約（論理コミット単位、不完全状態の排除）。加えて v2.3 では、`ConsistencyState::Pending` / `NeedsRepair` / `Quarantined` のいずれの状態にあるアセットも retrieval selection path に露出してはならない (MUST NOT) こと、および Repair 完了後にのみ安全な復帰可能性を評価しうることが強化された。
* **実装スコープ:** `commit_dual_store_update` 関数の実装。 SQLite側領域への書き込み（第一段階）に成功した時点で状態を `ConsistencyState::Pending { phase: CommitPhase::MetaPrepared }` に移行させる状態遷移ロジック。さらに、MetaPrepared 後に失敗が発生した場合は `NeedsRepair` へ確定遷移し、その時点から hard retrieval exclusion を維持する規律を含める。
* **テストコードによる検証:** SQLite側へのインテント書き込み完了直後、かつLadybugDB側への書き込みが完了する手前の段階において、当該アセットの外部からの retrieval がハードゲートで100%弾かれ、通常検索候補に絶対に浮上しないことをアサートする。加えて、`Pending`、`NeedsRepair`、`Quarantined` の各状態にあるレコードがいずれも検索候補に露出しないこと、および clean state へ明示的に復帰した後にのみ候補復帰が可能となることを確認する。
* **計装方法・観測対象:** 特定アセットが `ConsistencyState::Pending { phase: CommitPhase::MetaPrepared }` に拘束されている遅延時間窓 $\Delta t$ の間に、別スレッドの $10^4$ 個の並行サーチ要求命令（検索クエリ）を過剰集中注入。ハードゲートチェックを通過してセマンティック候補セットに不完全アセットが混入した確率（汚染読取フラックス） $P_{taint}$ の計測に加え、SQLite側のインテント書き込み完了直後かつLadybugDBへの書き込み手前の段階で、意図的にタイムアウトやI/Oエラーパルスを注入する動的破壊実験を走行。エラー注入から `NeedsRepair` 状態への中断完了（修復キューへのフォールバック軌道）に要する仮想命令ステップ数（潜時）の有界性、およびエラーパルス強度に対するストア間の不整合生存時間窓 $\Delta \tau_{unclean}$ の極値統計分布の実測。並行アクセススレッド数を 1 から 64 までスケールさせた際にも、競合状態をすり抜けることなく $P_{taint} = 0.00000$ を維持し続けることの一貫性遮断曲線をプロットする。さらに、Repair 完了後の clean state 復帰率、tombstone / quarantine への安全収束率、および repair convergence time を補助メトリクスとして記録し、v2.3 の repair discipline の運用観測基盤とする。

#### ✅ チケット M1.5-3: 起動時修復スキャン（Repair Worker）によるクラッシュリカバリの決定論的テスト

* **対象不変条件 / 規範:** §18.2 & §25.x 「起動時修復スキャンにより、片側成功状態の放置を避けること」。加えて v2.3 では、startup repair scan は dual-store の壊れた状態を selection path から隔離し、安全状態へ収束させる中核規律であり、片側成功状態の黙過を許してはならない。
* **実装スコープ:** データベースを模した構造体走査時に `Pending` または `NeedsRepair` を見つけた場合、`RepairAction::ConvertToTombstone` または再試行を実行するリカバリロジック。さらに、修復途中状態が retrieval selection path に露出しない hard exclusion を維持しつつ、最終的に clean / tombstone / quarantined のいずれかの安全状態へ収束させる。
* **テストコードによる検証:** メモリストア上に意図的に壊れた状態（`NeedsRepair`）のダミーレコードを配置した状態で、シミュレートされたシステム再起動関数（`startup_repair_scan()`）を実行。スキャン完了後、対象レコードの `consistency_state` が一貫して消去、または安全に隔離（`Quarantined`）された状態へと収束していることをアサートする。加えて、修復途中状態がスキャン中に検索候補へ一切露出しないことを確認する。
* **計装方法・観測対象:** メモリストア上に、意図的に不整合状態（`NeedsRepair`, `Pending` 等）を確率分布に沿って初期配置した 1万件のアセット母集団（損傷アンサンブル）を構築。修復ワーカーの起動スキャン回数（ステップカウント） $t$ に対する、ストア内の総不整合アセット残存数（不整合ノルム $\|\mathbf{E}(t)\|$）。スキャン進行に伴う残差ノルムの漸近減衰レートが、式 $\ln \|\mathbf{E}(t)\| \sim -\Gamma t$ （ただし修復減衰定数 $\Gamma > 0$ ）の指数関数的消滅軌道を描き、有限ステップ内で確実に一貫したクリーン状態またはマルコフ吸収状態（Tombstone / Quarantined）へ収束しきる定常動態を完全計測する。加えて、repair success rate、quarantine rate、repair convergence time を v2.3 補助メトリクスとして収集し、selection path 安全性との相関を追跡する。

---

### 8A. マイルストーン M1.5-R：v2.3-g Event Architecture 整合（追加的統合）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。
>
> **⚠️ このマイルストーンの位置づけ:** 本節は v2.3-g で RFC に追加された Darvium Event Architecture を既存の M1.5 実装（HumanChannel・InteractionHandle・StoredInteraction・MetadataStore・VirtualClock）と整合させるための追加的統合チケット群である。全ての変更は既存の HITL 実行意味論・`InteractionHandle.wait()` ブロッキング・MetadataStore crash recovery を厳格に保存した上で (MUST NOT)、strictly additive に Event Architecture を導入する。v2.3-g 改訂指令 Phase 1-7 に対応する。
>
> **下位互換性の保証:** 以下の8項目は本マイルストーンを通じて変更してはならない (MUST NOT):
> 1. HITL `notify` / `communicate` / `reconnect` の実行意味論（トレイトシグネチャ不変）
> 2. `InteractionHandle.wait()` ブロッキング機構
> 3. `StoredInteraction` 永続化（`InteractionRecord<HitlPayload>` 型エイリアスで存続）
> 4. MetadataStore crash recovery プロトコル
> 5. StdinoutChannel JSON Lines 旧プロトコル（互換モード保持）
> 6. Training Orchestrator の HumanChannel 依存（透過的 adapter）
> 7. `HumanDecision` / `HumanOutcome` の全バリアント
> 8. 既存 core invariant（trust / lifecycle / ApplicabilityScore / SearchState / DAG / fusion / Conversational gate）

#### ✅ チケット M1.5-R1: `InteractionRecord<TPayload>` ジェネリック型 + `InteractionStatus` 7状態列挙型の定義

* **対象不変条件 / 規範:** RFC §12C InteractionRecord / InteractionStatus。既存 `StoredInteraction` は `InteractionRecord<HitlPayload>` の型エイリアスとして存続し、全フィールドを保存すること (MUST NOT shrink)。InteractionStatus は RFC §12C で既に定義された6状態（Pending, AwaitingExternal, Resolved, TimedOut, Unreachable, ChannelClosed）に `Aborted` を加えた7状態で定義する。
* **実装スコープ:**
  - `InteractionPayload` トレイト: `Clone + Serialize + Deserialize` 境界、associated type `Outcome: Clone + Serialize + Deserialize`
  - `InteractionRecord<TPayload: InteractionPayload>` ジェネリック構造体（RFC §12C に従い）: `interaction_id`, `payload: TPayload`, `outcome: Option<TPayload::Outcome>`, `status: InteractionStatus`, `created_at: u64`, `updated_at: u64`
  - `HitlPayload { request: HumanRequest }` 構造体 + `impl InteractionPayload for HitlPayload { type Outcome = HumanOutcome }`
  - `InteractionStatus` 列挙型: `Pending`, `AwaitingExternal`, `Resolved`, `TimedOut`, `Unreachable`, `ChannelClosed`, `Aborted`
  - `pub type StoredInteraction = InteractionRecord<HitlPayload>;` 型エイリアス定義（既存コードの `StoredInteraction` 参照が透過的に解決されること）
  - 後方互換アクセサ: `impl StoredInteraction { fn request(&self) -> &HumanRequest; fn outcome(&self) -> &Option<HumanOutcome>; }`
  - 既存の `InteractionStatus::Pending` / `InteractionStatus::Resolved` パターンマッチが拡張後も変更なしにコンパイル通過すること
* **テストコードによる検証:**
  1. `InteractionRecord<HitlPayload>` が既存 `StoredInteraction` の全フィールドを保持することのフィールド単位確認
  2. `InteractionStatus::Aborted` への遷移が既存の5状態遷移機械に追加可能であることの状態遷移マトリクス確認
  3. 異種ペイロード型 (`InteractionRecord<HelpPayload>` 等) のインスタンス化がコンパイル可能であること
  4. 既存 `StoredInteraction` を参照するテストコードが型エイリアス変更後も変更なしにコンパイル・通過すること（下位互換性）
* **計装方法・観測対象:** ジェネリック型のフィールド構成を既存 `StoredInteraction` のフィールド一覧と人手照合し、全フィールドの完全保存を確認する。`Aborted` 状態を加えた7状態間の全遷移可能性行列 $T \in \{0,1\}^{7\times7}$ を列挙し、既存5状態の遷移が一切変更されていないことを行列差分 $\Delta T = T_{new} - T_{old}$ のゼロ確認により検証する。

#### ✅ チケット M1.5-R2: MetadataStore 汎用 Interaction API 拡張（store / load / list / resolve / abort / reconnect）

* **対象不変条件 / 規範:** RFC §12C MetadataStore Interaction API。既存の HITL 特化メソッドは維持しつつ (MUST NOT remove)、汎用 `InteractionRecord<TPayload>` を扱う6メソッドを追加する。crash recovery プロトコルは不変 (MUST NOT change)。
* **実装スコープ:**
  - `MetadataStore` トレイトへの6汎用メソッド追加:
    - `store_interaction(record: InteractionRecord<TPayload>) -> Result<()>`
    - `load_interaction(interaction_id) -> Result<Option<InteractionRecord<TPayload>>>`
    - `list_interactions(filter: InteractionFilter) -> Result<Vec<InteractionRecord<TPayload>>>`
    - `resolve_interaction(interaction_id, outcome) -> Result<()>`
    - `abort_interaction(interaction_id, reason) -> Result<()>`
    - `reconnect_interaction(interaction_id, new_channel_id) -> Result<()>`
  - 既存 HITL 特化メソッド（`store_hitl_interaction` 等）を上記汎用メソッドの HITL 特化ラッパーとして再実装
  - `InteractionFilter` 構造体: `status`, `channel_id`, `created_after`, `created_before`, `limit` フィールド
  - `FakeMetadataStore` への汎用 Interaction API 実装追加
* **テストコードによる検証:**
  1. 汎用 `store_interaction` + `load_interaction` の write-after-read 一貫性（`n = 100` ラウンドトリップ）
  2. `list_interactions` で `InteractionFilter` の各フィールドによるフィルタが正確に動作すること
  3. `resolve_interaction` 後の `load_interaction` で status が `Resolved` になっていること
  4. `abort_interaction` 後の status が `Aborted` になること
  5. 既存 HITL 特化メソッドが汎用 API 経由でも同一結果を返すこと（ラッパー検証）
  6. crash recovery テスト（M1.5-3）が本変更後も同一結果を返すこと（退行なし）
* **計装方法・観測対象:** 既存の HITL 特化メソッド呼び出しが本変更後も同一の内部状態を生成することを、`FakeMetadataStore` の内部 `HashMap` スナップショット比較で検証する。汎用 API の throughput を 6 メソッド × 1000 呼び出しで計測し、線形 O(1) パフォーマンスを確認する。

#### ✅ チケット M1.5-R3: `StoredInteraction` → `InteractionRecord<HitlPayload>` 型エイリアス移行

* **対象不変条件 / 規範:** RFC §12C backward compatibility。`StoredInteraction` は `InteractionRecord<HitlPayload>` の `type` エイリアスとして再定義し、全既存コードの変更をゼロに抑える (MUST)。移行後も既存のシリアライズ形式との互換性を維持する。
* **実装スコープ:**
  - `type StoredInteraction = InteractionRecord<HitlPayload>` エイリアス定義
  - 既存の `StoredInteraction` を参照する全箇所のコンパイル確認（エイリアス解決により透過）
  - シリアライズ/デシリアライズ互換性: JSON 表現が既存フォーマットと互換であることの確認
  - 既存コメントの更新（「StoredInteraction」の参照を必要に応じて更新）
* **テストコードによる検証:**
  1. `let s: StoredInteraction = InteractionRecord::<HitlPayload>::default();` がコンパイル可能であること
  2. 既存テストコードの `StoredInteraction` 参照が変更なしにコンパイル・通過すること
  3. JSON シリアライズ結果が既存フォーマットと互換であること（ラウンドトリップ）
  4. `InteractionRecord::<HitlPayload>` として作成したレコードが既存の `load_stored_interaction()` 関数で読み出せること
* **計装方法・観測対象:** コンパイル時の型解決追跡により、全 `StoredInteraction` 参照が `InteractionRecord<HitlPayload>` に透過的に置換されたことを確認する。シリアライズ互換性テストのラウンドトリップ成功率を $n = 1000$ で計測し、100% 互換であることを確認する。

#### ✅ チケット M1.5-R4: `DarviumEvent` canonical envelope + `DarviumEventKind` + `InteractionMode` 型定義

* **対象不変条件 / 規範:** RFC §12C DarviumEvent canonical envelope、DarviumEventKind 13 subtype、InteractionMode。
* **実装スコープ:**
  - `DarviumEvent` 構造体: `event_id: String`, `kind: DarviumEventKind`, `interaction_mode: InteractionMode`, `payload: serde_json::Value`, `causality: Option<Vec<String>>`, `metadata: HashMap<String, String>`, `transport_meta: Option<TransportMeta>`, `visibility: EventVisibility`, `retention: RetentionPolicy`, `privacy: PrivacyClass`
  - `DarviumEventKind` 列挙型: 13 variant（`System`, `Search`, `WorkflowExecution`, `Training`, `Knowledge`, `Conversational`, `Lifecycle`, `Gc`, `Repair`, `Reciprocity`, `Fusion`, `Hitl`, `Extension`）
  - `InteractionMode` 列挙型: `OneWay`（fire-and-forget）, `TwoWay { interaction_id: String }`
  - `EventVisibility`, `RetentionPolicy`, `PrivacyClass` 補助列挙型
  - `TransportMeta` 構造体: `channel_type`, `source`, `delivery_attempt` 等
  - 全型に `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` を付与
* **テストコードによる検証:**
  1. 全13 variant の `DarviumEventKind` が `Debug + Clone + PartialEq + Serialize + Deserialize` を実装可能であることのコンパイル時確認
  2. `DarviumEvent` の全フィールドを設定したインスタンスがコンパイル可能であり、全フィールドにアクセス可能であること
  3. `InteractionMode::OneWay` と `InteractionMode::TwoWay { interaction_id: "test".into() }` のパターンマッチが網羅的であること
  4. JSON シリアライズ/デシリアライズのラウンドトリップが全フィールドで一致すること
  5. `EventVisibility`, `RetentionPolicy`, `PrivacyClass` の各 variant が期待通りに動作すること
* **計装方法・観測対象:** 全型定義のフィールド一覧を RFC §12C の定義と人手照合し、過不足なく実装されていることを確認する。シリアライズラウンドトリップ成功率 $n = 1000$、JSON 表現の構造的一貫性（必須フィールドの欠落ゼロ）を検証する。

#### ✅ チケット M1.5-R5: `DarviumEventBus` トレイト + `FakeEventBus` 実装

* **対象不変条件 / 規範:** RFC §12C DarviumEventBus trait。Event Bus は全状態遷移の canonical 経路であり、VirtualClock の唯一の authority である。既存の直接的な `advance_virtual_clock` 呼び出しは禁止 (MUST NOT) されるが、FakeEventBus 内でのみ例外的に許容する。
* **実装スコープ:**
  - `DarviumEventBus` トレイト: 8メソッド
    - `publish(event: DarviumEvent) -> Result<EventId>`（OneWay publish）
    - `open(event: DarviumEvent) -> Result<InteractionId>`（TwoWay open）
    - `resolve(interaction_id, outcome) -> Result<()>`（TwoWay resolve）
    - `reconnect(interaction_id, new_channel) -> Result<()>`
    - `subscribe(filter: EventFilter) -> Box<dyn EventSubscription>`
    - `replay(since_vt: u64, filter: EventFilter) -> Result<Vec<DarviumEvent>>`
    - `current_clock() -> u64`
    - `quarantine_failed_events(interaction_id, reason) -> Result<()>`
  - `FakeEventBus`: 全メソッドを `Vec<DarviumEvent>` + `HashMap<InteractionId, InteractionRecord<serde_json::Value>>` で実装したメモリ内実装。`current_clock()` は内部イベントカウンタを返す。
  - `EventFilter` 構造体: `kind_filter: Option<Vec<DarviumEventKind>>`, `since_vt: Option<u64>`, `until_vt: Option<u64>`
  - `EventSubscription` トレイト: `poll() -> Option<DarviumEvent>`
  - `EventId`, `InteractionId` の newtype 定義（既存の interaction_id: String との相互変換）
* **テストコードによる検証:**
  1. `publish()` 後の `replay()` で同一イベントが取得できる read-after-write 一貫性
  2. `open()` 後の `resolve()` で TwoWay インタラクションが完了すること
  3. `subscribe()` でフィルタ条件に合致するイベントのみが届くこと
  4. `replay(since_vt=0)` で全イベントが時系列順に取得できること
  5. `current_clock()` が publish/open 呼び出し後に単調増加すること
  6. `quarantine_failed_events()` 後の該当インタラクションが replay 結果から除外されること
  7. `FakeEventBus` が `DarviumEventBus` トレイト境界を充足することのコンパイル時検証
* **計装方法・観測対象:** publish → replay の完全性を $n = 1000$ イベントの一括発行で検証し、イベント消失率 0% を確認する。`current_clock()` の単調増加性を並行アクセス下（$n = 64$ スレッド）で検証し、クロックの巻き戻りが一切発生しないことを確認する。

#### ✅ チケット M1.5-R6: `VirtualClock` 再定義 — EventBus commit clock への制限

* **対象不変条件 / 規範:** RFC §12C VirtualClock redefinition。VirtualClock は「commit 済み DarviumEvent 列の順序番号」として再定義される。EventBus がクロック進行の唯一の authority であり、外部からの直接 `advance_virtual_clock` 呼び出しは禁止される (MUST NOT)。
* **実装スコープ:**
  - `VirtualClock` トレイトの再定義: `fn now(&self) -> u64`（読み取りのみに制限）
  - `FakeEventBus` 内部でのみ `advance_virtual_clock` を保持（内部実装詳細として隠蔽）
  - `advance_virtual_clock` 関数の可視性をモジュール限定に縮小し、EventBus 実装以外からの直接呼び出しをコンパイル時に禁止
  - 既存コードで直接 `advance_virtual_clock` を呼び出している箇所を特定し、EventBus 経由に書き換え
  - `FakeClock` / `SystemClock` の VirtualClock トレイト実装との整合性確保
* **テストコードによる検証:**
  1. `VirtualClock::now()` が読み取り専用であること（`&self` で宣言）のコンパイル時確認
  2. `advance_virtual_clock` が EventBus 実装以外から呼び出せないことのコンパイルエラー確認
  3. EventBus 経由の publish/open 後に `now()` がインクリメントされること
  4. 既存の `VirtualClock` 利用コード（M-2-1.8, M1.75-1 の `should_update_position` 等）が変更なしでコンパイルを通ること
  5. `FrozenClock` / `SystemClock` が引き続き `VirtualClock` トレイトを実装可能であること
* **計装方法・観測対象:** EventBus 操作（publish/open/resolve）とクロック値の相関を $n = 1000$ 操作で計測し、操作ごとにクロックが単調増加することを確認する。直接 `advance_virtual_clock` 呼び出しの試行をコンパイル時に完全遮断できることを型検査で検証する。

#### ✅ チケット M1.5-R7: `HumanChannel` を EventBus / InteractionStore 上の HITL 特化 adapter へ再構成

* **対象不変条件 / 規範:** RFC §12C HumanChannel adapter。既存 HITL の完全な実行意味論（`notify` / `communicate` / `reconnect` のシグネチャとブロッキング動作）を一切変更せず (MUST NOT)、内部実装のみ EventBus + InteractionStore 経由に置き換える。
* **実装スコープ:**
  - 既存 `HumanChannel` トレイトメソッドの内部実装を EventBus + InteractionStore 経由に変更:
    - `notify` → EventBus への `DarviumEventKind::Hitl` の OneWay publish
    - `communicate` → EventBus への `InteractionMode::TwoWay` の open + InteractionStore での状態追跡
    - `reconnect` → InteractionStore の `reconnect_interaction` 呼び出し + EventBus の reopen
  - `HumanChannelConfig` に `event_bus: Arc<dyn DarviumEventBus>` と `interaction_store: Arc<dyn InteractionStore>` の参照を追加（optional、後方互換性のため）
  - `FakeHumanChannel` に EventBus adapter モードと従来モードの切り替えを追加（テスト既存コードの変更ゼロ）
  - `InteractionHandle` 内部で EventBus の `resolve()` / `reconnect()` を待機する adapter 実装
* **テストコードによる検証:**
  1. 既存の HITL テストコード（M-0.5-4, M1-4 等）が一切の変更なくコンパイル・通過すること（後方互換性 MUST）
  2. `notify` 呼び出し後に EventBus が同一内容の `DarviumEvent` を保有していること
  3. `communicate` 呼び出し後の `InteractionHandle` が InteractionStore に正しく記録されていること
  4. `reconnect` が InteractionStore の再接続レコードを正しく生成すること
  5. Training Orchestrator の HumanChannel 依存コードが透過的に動作すること
* **計装方法・観測対象:** 既存テストスイートの全テストが adapter 変更後も同一結果を返すことを確認する（退行検出率 100%）。EventBus adapter モードと従来モードの出力一致率を $n = 100$ のランダム操作系列で検証する。

#### ✅ チケット M1.5-R8: `EventChannel` トレイト + `StdinoutEventChannel` canonical JSON Lines プロトコル

* **対象不変条件 / 規範:** RFC §12D EventChannel trait、StdinoutEventChannel。既存の `StdinoutChannel` JSON Lines 旧プロトコルは互換モードとして保持し (MUST NOT remove)、新 `StdinoutEventChannel` は canonical Event JSON Lines プロトコルを実装する。
* **実装スコープ:**
  - `EventChannel` トレイト: `send(event: DarviumEvent) -> Result<()>`, `receive() -> Result<Option<DarviumEvent>>`, `flush() -> Result<()>`
  - `StdinoutEventChannel`: 標準入出力を介した canonical JSON Lines プロトコル実装（各行が1つの `DarviumEvent` の JSON 表現）
  - 既存 `StdinoutChannel` を互換モードラッパーとして維持（`EventChannel` トレイト実装として再公開）
  - `WebSocketEventChannel` の型定義のみ（実装は将来フェーズ）
* **テストコードによる検証:**
  1. `StdinoutEventChannel` の `send` → `receive` ラウンドトリップ（バッファ経由）
  2. 既存 `StdinoutChannel` の互換モードが旧 JSON Lines 形式を正しく読み書きできること
  3. canonical 形式と互換形式の相互変換が可能であること
  4. `EventChannel` トレイトがオブジェクト安全であること（`Box<dyn EventChannel>`）
* **計装方法・観測対象:** canonical JSON Lines 形式のパース成功率を $n = 1000$ イベントで計測し、100% のラウンドトリップ一貫性を確認する。互換モードでの旧形式との往復変換で情報損失がゼロであることを検証する。

#### ✅ チケット M1.5-R9: `EventProjection` フレームワーク + `ProjectionCatalog` 実装

* **対象不変条件 / 規範:** RFC §12E Event Projection Framework。ドメイン固有のビュー（SearchTrace・TrainingRunLog・ReciprocityEvent 等）は `EventProjection` として DarviumEvent ストリームから materialize される。Projection はイベントソーシングの読み取りモデルとして機能し、基盤の EventBus に影響を与えてはならない (MUST NOT)。
* **実装スコープ:**
  - `EventProjection` トレイト: `project(event: &DarviumEvent) -> Result<()>`, `snapshot() -> Result<serde_json::Value>`, `clear() -> Result<()>`
  - `ProjectionCatalog`: `register(name, projection)`, `get(name) -> Option<Arc<dyn EventProjection>>`, `project_all(event: &DarviumEvent) -> Result<()>`（全登録 projection にイベントを配送）
  - `ProjectionEventFilter`: どの event_kind をどの projection に配送するかのフィルタ定義
  - `FakeProjectionCatalog`: メモリ内実装
* **テストコードによる検証:**
  1. 単一 projection の `project()` + `snapshot()` ラウンドトリップ
  2. 複数 projection への同時配送（`project_all`）で全 projection が同一イベントを受け取ること
  3. `ProjectionEventFilter` でフィルタされた event_kind のみが配送されること
  4. projection の `clear()` 後は空の snapshot が返ること
  5. `ProjectionCatalog` が全 projection の状態を独立に保持すること（cross-projection contamination ゼロ）
* **計装方法・観測対象:** $n = 1000$ イベントの一括配送後、各 projection の snapshot が独立かつ完全であることを確認する。フィルタリング精度（配送イベントの kind 一致率 100%）を検証する。

#### ✅ チケット M1.5-R10: ドメイン統合 — SearchTrace・TrainingRunLog・TrainingOrchestrator の EventProjection 化

* **対象不変条件 / 規範:** RFC §12E Domain projections。検索・訓練・相互互恵性の各ドメイン状態は DarviumEvent ストリームから materialize される EventProjection として再定義される。既存のドメインインターフェース（`SearchWorkflow`・`TrainingOrchestrator` 等）は透過的に EventProjection を利用する。
* **実装スコープ:**
  - `SearchTraceProjection`: `DarviumEventKind::Search` イベントから SearchTrace を materialize
  - `TrainingRunLogProjection`: `DarviumEventKind::Training` イベントから TrainingRunLog を materialize
  - `ReciprocityEventProjection`: `DarviumEventKind::Reciprocity` イベントから ReciprocityEvent 系列を materialize
  - `SearchRunLogProjection`: 検索実行ログの Projection（`DarviumEventKind::Search` の subset）
  - 各 Projection を ProjectionCatalog に登録する初期化関数
  - 既存の `SearchTrace` 保存コードを EventBus publish + Projection materialize の2経路に変更（既存コードは互換性のため存続）
* **テストコードによる検証:**
  1. Search イベント publish → SearchTraceProjection で同一内容の SearchTrace が materialize されること
  2. Training イベント publish → TrainingRunLogProjection で同一内容が materialize されること
  3. Reciprocity イベント publish → ReciprocityEventProjection で同一内容が materialize されること
  4. 全ドメインイベントを混在させて publish しても、各 projection が自身の kind のみを正しく抽出すること
  5. 既存のドメインコード（SearchWorkflow・TrainingOrchestrator 等）が変更なしでコンパイルを通ること
* **計装方法・観測対象:** domain event → projection materialize の完全性（全フィールド一致率 100%）を $n = 1000$ イベントで検証する。Projection 間の分離（cross-domain contamination 0%）を確認する。

#### ✅ チケット M1.5-R11: Event Architecture 較正候補定数 + プロパティベース不変条件ファジング

* **対象不変条件 / 規範:** RFC §12C calibration candidates、v2.3-g Event Architecture 定数表。Event Bus のバッファサイズ・タイムアウト・リトライポリシー等は Calibration Candidates として管理される。不変条件: EventBus の publish 後のイベント消失禁止、TwoWay の状態遷移完全性、clock の単調増加性。
* **実装スコープ:**
  - `constants.rs` への Event Architecture 較正候補定数追加:
    - `EVENT_BUS_CHANNEL_CAPACITY: usize = 1024`（Safety Invariant, 変更禁止）
    - `EVENT_BUS_DEFAULT_TIMEOUT_MS: u64 = 5000`（Calibration Candidate）
    - `EVENT_BUS_MAX_RETRY_COUNT: u32 = 3`（Calibration Candidate）
    - `INTERACTION_CLEANUP_INTERVAL_TICKS: u64 = 100`（Calibration Candidate）
    - `EVENT_REPLAY_BATCH_SIZE: usize = 256`（Calibration Candidate）
    - `PROJECTION_INITIAL_CAPACITY: usize = 64`（Environment Policy Knob）
    - `QUARANTINE_MAX_EVENTS: usize = 10000`（Safety Invariant）
  - `proptest` 戦略群: `darvium_event_strategy()`, `event_kind_strategy()`, `interaction_mode_strategy()`
  - Event Architecture invariant suite:
    1. publish 後のイベントが replay で必ず取得可能
    2. TwoWay の状態遷移（open → resolve / abort）が finite ステップで完了
    3. clock の単調増加性
    4. quarantine 後のイベントが検索から除外される
    5. projection の独立性（cross-contamination 0）
  - failing seed export → replay fixture 昇格機構
* **テストコードによる検証:**
  1. ランダムイベント列 $n \ge 10^4$ の publish → replay でイベント消失率 0 であること
  2. TwoWay インタラクションの状態遷移が $n \ge 10^4$ のランダム操作で finite ステップ停止すること
  3. `clock` の単調増加性がマルチスレッド下で保たれること
  4. 各较正候補定数のデフォルト値で invariant が成立すること
  5. 極端な定数値（`EVENT_BUS_CHANNEL_CAPACITY = 1`, `EVENT_BUS_DEFAULT_TIMEOUT_MS = 0`）でもパニックしないこと
* **計装方法・観測対象:** fuzz ケース全体に対する invariant violation 率（期待値: 0）を記録する。パラメータ空間における violation clustering を検出し、脆弱なパラメータ領域の有無を観測する。失敗 seed は replay fixture に昇格した数をカウントし、発見されたエッジケースの蓄積を監視する。

---

### 8B. マイルストーン M1.75：Child Support Villages / HELP Consensus（v2.3-e）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。Village locality / HELP offer-consent / stability calibration / replay はすべてメモリ内データ構造と固定シード PRNG により完全決定論的に再現・観測する。
>
> **⚠️ このマイルストーンの位置づけ:** 本節は既存の M1（Human-in-the-loop review）、M1.5（擬似 dual-store / repair discipline）、および M1.5-R（v2.3-g Event Architecture）を一切毀損せず、その上に strictly additive に積み増される Child Support Villages and HELP Consensus Extension の実装群である。v2.3-g の Event Architecture との整合のため、全 HELP 状態遷移は `DarviumEventKind::Reciprocity` イベントとして EventBus へ publish され、位置更新は `DarviumEventKind::System` イベントとして記録される。ここで追加される要素は、Training Plane 上の child-support mission orchestration、space-position-based locality、adult HELP offer policy、child consent semantics、helper weighting、bounded remote exploration、stability / dynamicity calibration discipline を扱う。ApplicabilityScore、legal SearchState transitions、training-production separation、dual-store consistency、promotion / repair invariants は本マイルストーンで変更してはならない (MUST NOT)。

#### ✅ チケット M1.75-1: `SpacePositionEmbedding` / `VillagePosition` 型定義および位置更新ダイナミクスの実装

* **対象不変条件 / 規範:** RFC §41B `spacepositionembedding`、v2.3-g §12C EventBus VirtualClock に基づく局所性更新、ならびに small perturbation 下で位置軌道が unbounded oscillation を起こさない stability discipline。
* **実装スコープ:**
  - `SpacePositionEmbedding`、`VillagePosition`、`VillageObservation`、`PositionUpdatePolicy` のピュア構造体・列挙型定義
  - 純粋関数 `update_space_position(prev, obs, alpha) -> VillagePosition` の実装。基礎更新式は

    \[
    x_{t+1} = (1 - \alpha) x_t + \alpha \cdot \Delta_t
    \]

    とし、`alpha` は calibration candidate として扱う
  - `VirtualClock`（v2.3-g では EventBus の commit clock、`DarviumEventBus::current_clock()` 経由で取得）と結合する `should_update_position(last_updated_vt, now_vt, policy)` の実装
  - 位置更新発生時に `DarviumEventKind::System` イベント（`SpacePositionUpdated` ペイロード）を EventBus へ publish する機能
  - fixed-point 収束テストのための補助関数 `l2_distance(a, b)`
* **テストコードによる検証:**
  1. `alpha = 0.0` のとき更新後位置が常に `prev` と完全一致すること
  2. `alpha = 1.0` のとき更新後位置が観測 `obs.delta` に完全一致すること
  3. 同一観測を反復入力したとき、位置系列が指数的に固定点へ収束すること
  4. EventBus の `current_clock()` ポリシーで更新窓外にあるとき、位置更新が発火しないこと
  5. 位置更新後に EventBus が対応する `DarviumEvent` を保有していること（publish 検証）
* **計装方法・観測対象:** 固定シード PRNG で観測ノイズを注入した位置更新系列を $10^4$ 本生成し、平均二乗変位 $\langle \|x(t)-x(0)\|^2 \rangle$ の時間発展を計測する。緩和率 $\Gamma$ を位置更新率 $\alpha$ の関数として観測し、small perturbation regime において軌道が発散せず、有限分散に拘束されることを確認する。さらに位置更新イベントの発火密度と EventBus `current_clock()` の更新窓幅の関係を走査し、過剰更新による村構造のノイズ増幅が起きる臨界領域を同定する。位置更新イベントの EventBus publish 完全性（更新1件につきイベント1件の対応）を $n = 1000$ で検証する。

#### ✅ チケット M1.75-2: Child / Adult maturity 判定器および Local Village 構成ロジックの実装

* **対象不変条件 / 規範:** RFC §41B Child / Adult distinction、ExperienceCount・TrustProfile・ReputationProfile を用いた成熟判定、および local village が child の近傍 adult 集合としてのみ構成されるという locality 規範。
* **実装スコープ:**
  - `WorkflowMaturity::{Child, Adult}` の定義
  - `classify_maturity(exp, trust, reputation) -> WorkflowMaturity` の実装
  - `LocalVillage { child_id, adult_ids, centroid, radius }` の定義
  - 純粋関数 `build_local_village(child, adults, k, radius) -> LocalVillage`
  - `ConsistencyState != Committed`、`LifecycleState::Quarantined`、および adult maturity 未達 workflow を village adult 候補から除外するフィルタ
* **テストコードによる検証:**
  1. Experience / Trust / Reputation の境界値 ±1 ステップで maturity 判定が正しく切り替わること
  2. 人工配置した adult 群に対して、child 近傍から距離昇順で `k` 件が local village に選抜されること
  3. `Pending` / `NeedsRepair` / `Quarantined` な adult 候補が village へ一切混入しないこと
  4. adult 候補が 0 件のとき、empty village として安全に表現されること
* **計装方法・観測対象:** adult population 密度 $\rho_a$、village 半径 $r$、近傍数 $k$ を制御パラメータとして village 構成シミュレーションを大量に走らせる。平均 village サイズ、平均近傍距離、空 village 発生率、child ごとの nearest-adult 距離分布を観測し、疎密転移点における locality coverage の急減領域を同定する。さらに maturity 閾値の微小変更が adult population の実効密度に与える影響を測定し、child-support 供給能力のボトルネックを定量化する。

#### ✅ チケット M1.75-3: HELP プロトコル (`HelpProposal` / `HelpOffer` / `HelpDecision` / `HelpExecution` / `HelpSuccess`) 状態機械の実装

* **対象不変条件 / 規範:** RFC §41B HELP consensus protocol。v2.3-g では全 HELP 状態遷移を `DarviumEventKind::Reciprocity` イベントとして EventBus へ publish する。adult 側の申し出 (`Offer`) と child 側の受諾 / 拒否 (`Decision`) を明示的に分離し、終端状態からの再遷移を禁止すること。
* **実装スコープ:**
  - `HelpState::{Proposal, Offered, Accepted, Rejected, Executing, Succeeded, Failed}` の定義
  - `HelpProposal`, `HelpOffer`, `HelpDecision`, `HelpExecution`, `HelpSuccess`, `HelpFailure` の構造体定義
  - 純粋関数 `is_legal_help_transition(current, next) -> bool`
  - `HelpSession::transition_to(next)` のガード実装
  - HELP 状態遷移を `DarviumEventKind::Reciprocity` イベントとして EventBus へ publish する emit 機能（`emit_help_event(session, transition)`）
  - publish される DarviumEvent の payload には `help_id`, `from_workflow`, `to_workflow`, `transition_type`, `timestamp_vt` を含む
  - `HelpRejectionReason`, `HelpFailureReason` の列挙型定義
* **テストコードによる検証:**
  1. すべての合法遷移が `true`、違法遷移が `false` となる遷移行列総当たりテスト
  2. `Proposal -> Offered -> Accepted -> Executing -> Succeeded` の正常系列が完走すること
  3. `Offered -> Rejected` で終端し、その後の再遷移が厳格に拒否されること
  4. `Executing -> Failed` 後の再実行や `Succeeded` への飛び遷移が不可能であること
  5. 各状態遷移後に EventBus へ対応する `DarviumEvent` が publish されていること（遷移種別とイベント種別の一致検証）
  6. publish されたイベントの EventBus `replay()` による再取得完全性
* **計装方法・観測対象:** ランダム生成した HELP 遷移系列を大量投入し、違法遷移集合への流入フラックスが厳密に 0 であることを観測する。加えて、吸収状態（`Rejected`, `Succeeded`, `Failed`）までの平均到達長、終端分布、child 拒否率と adult offer 率の関係を測定し、HELP 状態機械が有限ステップで吸収されることを実証する。publish された HELP イベントの EventBus 上の一貫性（遷移系列とイベント系列の完全対応）を $n = 1000$ 遷移で検証する。

#### ✅ チケット M1.75-4: adult HELP offer policy と child consent policy の純粋判定器実装

* **対象不変条件 / 規範:** RFC §41B adult offer semantics / child acceptance semantics。adult は自動的に child を強制実行してはならず、offer は policy-governed proposal として発生し、child 側の consent を通過して初めて execution に進入しなければならない (MUST)。
* **実装スコープ:**
  - `AdultHelpOfferPolicy`、`ChildHelpAcceptancePolicy` の構造体定義
  - `should_offer_help(child, adult, context, policy) -> bool`
  - `decide_help_offer(child, offer, policy) -> HelpDecision`
  - `OfferScoreBreakdown { distance_term, maturity_term, reciprocity_term, reputation_term, urgency_term }` の記録構造
  - reject / abstain / accept の理由コード体系
* **テストコードによる検証:**
  1. adult policy が false の場合、提案が execution に直接進まず必ず offer 不成立で終わること
  2. child consent が reject の場合、execution path が完全遮断されること
  3. 近距離・高成熟・高信頼な adult が、遠距離・低成熟・低信頼 adult より高い offer score を持つこと
  4. `Unsafe`, `Irrelevant`, `Overloaded` 等の reject reason が期待どおりに出ること
* **計装方法・観測対象:** child-adult ペア空間上に距離・信頼・評判・緊急度のパラメータグリッドを形成し、offer 発火率と accept 率の相図を計測する。acceptance decision surface の等高線を追跡し、閾値境界近傍での decision jitter を測定することで、過度に鋭い閾値による不安定切替の有無を検知する。

#### ✅ チケット M1.75-5: child-support `TrainingMission` specialization および Training Orchestrator 統合

* **対象不変条件 / 規範:** RFC §16A Training Plane、§41B child-support mission specialization。child support は production path ではなく training / safe sandbox 文脈で orchestrate され、training-production separation を破ってはならない (MUST NOT)。
* **実装スコープ:**
  - `TrainingMissionKind::ChildSupport` 追加
  - `ChildSupportMissionPayload { child_id, helper_ids, village_snapshot, objective, safety_scope }`
  - `spawn_child_support_mission(child, village, policy) -> Option<TrainingMission>`
  - child-support mission の発行・進行・完了の各段階で `DarviumEventKind::Training` イベントを EventBus へ publish
  - `TrainingRunLog`（v2.3-g では `EventProjection` として materialize）への HELP execution / outcome / child growth delta 記録拡張
  - production mission と child-support mission を混線させない plane ガード
* **テストコードによる検証:**
  1. child workflow のみが `ChildSupport` mission を生成し、adult workflow には生成されないこと
  2. production plane では `ChildSupport` mission が直接実行されず、safe sandbox 条件下でのみ許容されること
  3. mission 発行時に helper snapshot と village snapshot がログへ完全記録されること
  4. empty village の child に対して mission を無理に生成せず fallback policy へ移行すること
* **計装方法・観測対象:** child-support mission 発行率、mission 実行完了率、実行後の ExperienceCount 増分、child maturity 到達時間分布を収集する。training load と child-support mission 量の相互作用を観測し、review queue depth や training latency を悪化させる過剰発行領域を同定する。

#### ✅ チケット M1.75-6: helper weighting、bounded remote exploration、および helper 候補フィルタの実装

* **対象不変条件 / 規範:** RFC §41B helper weighting / bounded exploration。helper 選定は locality を基本としつつ、探索的多様性のための bounded remote exploration を許容する。ただし non-committed asset、quarantined asset、unsafe asset を helper 候補へ入れてはならない (MUST NOT)。
* **実装スコープ:**
  - `HelperWeight`, `HelperSelectionPolicy`, `RemoteExplorationPolicy` の定義
  - helper 重み関数

    \[
    w_t(h \mid c) = \frac{\exp(-\beta d_t(h,c)) \cdot q_t(h)}{\sum_{g \in N_t(c)} \exp(-\beta d_t(g,c)) \cdot q_t(g)}
    \]

    の実装
  - `select_helpers(child, village, policy) -> Vec<HelperWeight>`
  - exploration 率 $\varepsilon$ に基づき、遠方 helper を少数だけ混入させるロジック
  - `ConsistencyState != Committed`、repair pending、quarantined、adult maturity 未達の候補を落とす hard filter
* **テストコードによる検証:**
  1. 同一 quality なら近距離 helper の weight が遠距離 helper より必ず高いこと
  2. quality が十分高ければ、適度に遠い helper が近距離低品質 helper を上回りうること
  3. `Pending` / `NeedsRepair` / `Quarantined` helper が 1 件も選ばれないこと
  4. $\varepsilon = 0$ で exploration が 0、$\varepsilon = 1$ で常に remote sampling が発火すること
* **計装方法・観測対象:** 距離減衰係数 $\beta$ と exploration 率 $\varepsilon$ を 2 次元グリッドで掃引し、helper 分布エントロピー、平均 helper 距離、remote helper 混入率、success rate の相図を計測する。過度な局所固定化（低エントロピー）と過度なランダム化（高 churn）の双方を避ける sweet spot を観測的に同定する。

#### ✅ チケット M1.75-7: village stability / dynamicity メトリクス定義および観測パイプラインの実装

* **対象不変条件 / 規範:** RFC §41B stability / dynamicity metrics。位置ドリフト、近傍集合 churn、helper 分布 divergence、child survival / maturation を可観測な系列として残すこと。
* **実装スコープ:**
  - `VillageMetrics`, `VillageMetricsWindow`, `VillageMetricsSnapshot` の定義
  - `compute_position_drift`, `compute_village_jaccard`, `compute_village_churn`, `compute_helper_jsd`, `compute_child_survival_rate`, `compute_child_maturation_time`
  - `SimulationRunner` へ metrics hook を追加し、tick ごとに観測値を記録
  - v2.3-g の EventProjection フレームワークとの統合: `SearchTrace`・`TrainingRunLog` は EventBus 上の `EventProjection` として materialize され、`VillageObservationLog` は新規 Projection として登録
  - `SearchTrace` / `TrainingRunLog` / `VillageObservationLog` 間のキー整合
* **テストコードによる検証:**
  1. 同一近傍集合のとき Jaccard = 1, churn = 0 になること
  2. 完全 disjoint な近傍集合のとき Jaccard = 0, churn = 1 になること
  3. 同一 helper weight 分布に対して JSD = 0 が返ること
  4. child 成長イベントの蓄積により maturation time が有限になるシナリオを正しく集計できること
* **計装方法・観測対象:** `position_drift_p50/p95`, `village_churn_p50/p95`, `helper_jsd_p50/p95`, `helper_count_mean`, `child_survival_rate`, `child_maturation_time`, `false_new_rate`, `compose_fallback_frequency`, `review_queue_depth`, `review_latency` を観測系列として収集する。village 導入前後の差分系列を同一 seed 条件で比較し、既存 operational metrics を悪化させずに child support を追加できているかを検証する。

#### ✅ チケット M-0.5-7-P: WorkflowCache + RepositoryPair + CacheError/PersistenceError 型定義基盤（v2.3-j RFC §8 追従）

* **対象不変条件 / 規範:** §8 WorkflowCache と MemoizedGraph、§18 デュアルストアエラー再配置
* **実装の背景と目的:** v2.3-j の用語是正により、`WorkflowRepository` は `WorkflowCache`（runtime cache）と `RepositoryPair`（永続化ペア）へ分割された。本チケットはこの新しい型体系をコード上に定義する。`DualStoreCoordinator`（既存 `src/store/coordinator.rs`）を `RepositoryPair` の具象実装として位置付け直す。
* **実装スコープ:**
  1. `WorkflowCache` 構造体の定義:
     ```rust
     struct WorkflowCache {
         working_set: Arc<RwLock<Vec<MemoizedGraph>>>,
         ann_hint:    Arc<RwLock<AnnHotIndex>>,
         policy:      CachePolicy,
     }
     ```
  2. `CachePolicy` 列挙型の定義: `Default`, `Pinned { workflow_ids }`, `Preload { workflow_ids }`
  3. `AnnHotIndex` 型エイリアス: `type AnnHotIndex = AnnIndex;`
  4. `CacheError` 列挙型の定義: `CasConflict { expected, actual }`, `NotFound(WorkflowGraphId)`, `LoadFailed(String)`
  5. `PersistenceError` 列挙型の定義: `CrossStoreInconsistency(String)`, `SqliteError(String)`, `LadybugError(String)`, `PairNotFound(String)`
  6. `WorkflowCache::get_or_load` メソッド: cache hit → 即時返却、cache miss → RepositoryPair から lazy load して cache に昇格
  7. `RepositoryPair` 型の整備: 既存の `DualStoreCoordinator` に `pub type RepositoryPair = DualStoreCoordinator` または同等の façade を追加
  8. エラー型の crate 公開 API への追加: `pub use` による再公開
* **検証項目:**
  1. 全ての新規型がコンパイルを通ること
  2. `WorkflowCache::get_or_load` が cache miss 時に RepositoryPair から正しくロードすること
  3. `CacheError` / `PersistenceError` が `DarviumError` との間で適切に変換されること
* **依存関係:** 本チケットの完了は M-0.5-7-R の前提条件である。M-2-1.5 の `InMemoryGraphStore` / `InMemoryMetadataStore` は `RepositoryPair::in_memory()` 内部で利用する。

#### ✅ ⚠️ 改修チケット M-0.5-7-R: `retrieve_top_level_candidates` の WorkflowCache + RepositoryPair 移行（v2.3-j 追従）

* **対象不変条件 / 規範:** §8 WorkflowCache と MemoizedGraph、§12 v2.3-j 補足（DB 主導 + cache 加速）、§18 デュアルストア責務再配置
* **背景:** v2.3-i で実装済みの ✅ M-0.5-7 は関数シグネチャに `repo: &WorkflowRepository` を使用していた。v2.3-j の用語是正により `WorkflowRepository` は `WorkflowCache` + `RepositoryPair` へ分割され、検索フローも「DB 主導 + cache 加速」へ明確化された。M-0.5-7 の実装コードはこの新しい責務区分に追従しなければならない (MUST)。
* **実装スコープ:**
  1. `retrieve_top_level_candidates` の関数シグネチャを `(q: &QueryRepresentation, repo: &WorkflowRepository, ...)` から `(q: &QueryRepresentation, cache: &WorkflowCache, pair: &RepositoryPair, ...)` へ変更する
  2. `semantic_topk` 呼び出しを `repo` 経由から `(cache, pair)` 経由へ変更し、cache hit → lazy load の順序でデータアクセスする
  3. 各 Stage（metadata_filter / cheap_ged_filter / full_ged_rerank）の呼び出しに渡す MemoizedGraph 参照を `cache.get_or_load` 経由に変更する
  4. テスト構築を `WorkflowRepository::in_memory()` から `(WorkflowCache::in_memory(), RepositoryPair::in_memory())` へ変更する
  5. `RepositoryPair::in_memory()` の内部では既存の `InMemoryGraphStore` + `InMemoryMetadataStore`（M-2-1.5）を利用する
  6. `evaluate_candidate` のシグネチャは `&MemoizedGraph` のままで変更不要
* **検証項目:**
  1. 改修後のパイプラインが改修前と等価な検索結果（同一候補集合・同一順位）を返すことの確認（固定 seed 条件下）
  2. 候補数単調減少不変条件（N_sem ≥ N_meta ≥ N_cheap ≥ N_full）が維持されること
  3. cache hit 時は `WorkflowCache` からの高速パス、cache miss 時は `RepositoryPair` からの lazy load が正動作すること
  4. `CacheError` / `PersistenceError` が適切なレイヤで送出されること
* **依存関係:** 本チケットは ✅ M-0.5-7 の改修版であり、完了後は M-0.5-7 の実装を置き換える。M-0.5-7-P（WorkflowCache + RepositoryPair 型定義基盤）が完了していることが前提条件である (MUST)。M-2-1.5 の `InMemoryGraphStore` / `InMemoryMetadataStore` は引き続き `RepositoryPair::in_memory()` 内部で利用するため、新規トレイトは不要。

#### ⬜ チケット M-0.5-7-E1: WorkflowCache protected eviction guard

* **対象不変条件 / 規範:** P-18（Protected エントリの eviction 禁止）、§8 WorkflowCache eviction API、§15.1 GcState ↔ Cache Residency
* **実装の背景と目的:** v2.3-k で導入された eviction semantics において、`GcState::Protected` の MemoizedGraph、`ArtifactOriginKind::PresetSystem` または `PresetRootPolicy::RootPinned | RootAncestorPinned` に該当する preset-derived graph は eviction 対象から除外しなければならない (MUST)。本チケットは `is_eviction_protected()` 判定関数を実装し、protected entry への eviction 要求が常に失敗することを保証する。
* **実装スコープ:**
  1. `WorkflowCache::is_eviction_protected(&self, graph: &MemoizedGraph) -> bool` の実装
     - `GcState::Protected` の場合は true
     - `ArtifactOriginKind::PresetSystem` の場合は true
     - `PresetRootPolicy::RootPinned | RootAncestorPinned` の場合は true
     - それ以外は false
  2. `evict_one()` の先頭で `is_eviction_protected()` をチェックし、protected な場合は `CacheError::ProtectedEvictionForbidden` を返す
  3. `GcState::Protected` と `PresetRootPolicy::RootPinned | RootAncestorPinned` の排他的一貫性をコメントで明記
* **検証項目:**
  1. protected entry への `evict_one()` が `CacheError::ProtectedEvictionForbidden` を返すこと
  2. `RootPinned` の preset root (`StructMem`, `Corpus2Skill`) が cache eviction されないことを replay で確認
  3. `RootUnpinned` の entry は通常通り eviction 可能であること
  4. `is_eviction_protected()` が `GcState::Protected` と `PresetRootPolicy` の両方を正しく判定すること
* **依存関係:** M-0.5-7-P（WorkflowCache 型定義基盤）が完了していることが前提条件。

#### ⬜ チケット M-0.5-7-E2: WorkflowCache periodic eviction worker

* **対象不変条件 / 規範:** P-19（定期 eviction の義務）、§8 WorkflowCache eviction API
* **実装の背景と目的:** WorkflowCache はバックグラウンドの periodic worker を持ち、`eviction_interval` ごとに expired / pressure / over-capacity を評価して適宜 eviction を実行する。EventBus 非依存でも最小構成で動作する Fake 実装を用意する。
* **実装スコープ:**
  1. `WorkflowCache::evict_expired(human_now: SystemTime, vt_now: u64) -> EvictionReport` の実装
     - `default_ttl_human` と `last_cache_hit_at` の差分超過を判定
     - `default_ttl_virtual` と `last_cache_hit_vt` の超過を判定
     - protected entry は TTL 評価対象外（スキップして `skipped_protected` に計上）
     - 非 Committed エントリは hot path から除外（`skipped_non_committed` に計上）
  2. `WorkflowCache::evict_for_pressure(pressure_mode: PressureMode) -> EvictionReport` の実装
     - `Constrained` / `Emergency` で candidate selection の強さを切替
  3. `WorkflowCache::evict_to_capacity() -> EvictionReport` の実装
     - `max_entries` 超過時は超過分の非 protected entry を追い出し
     - `max_bytes` 超過時は推定バイト数の大きい順に追い出し
  4. バックグラウンド periodic worker（tokio interval または equivalent）の追加
     - `eviction_interval` ごとに expired / pressure / over-capacity を順次評価
     - 各実行結果を `EvictionReport` として収集（metrics 連携用）
  5. Fake 実装による EventBus 非依存テスト
* **検証項目:**
  1. periodic worker が `eviction_interval` に従って定期的に実行されること
  2. `evict_expired` が TTL 超過エントリを正しく判定・削除すること
  3. protected entry が TTL 評価をスキップされること
  4. Fake 実装でも最小構成で動作すること
* **依存関係:** E1（protected eviction guard）が完了していることが前提条件。

#### ⬜ チケット M-0.5-7-E3: WorkflowCache TTL eviction semantics

* **対象不変条件 / 規範:** §8 CacheResidencyMeta、Cache TTL Policy、P-19
* **実装の背景と目的:** 二軸（Human Time / VirtualClock）TTL による eviction eligibility 判定を実装する。`Provenance.last_used_at` と `last_virtual_seen` に基づき、protected preset は TTL 対象外とする。
* **実装スコープ:**
  1. `CacheResidencyMeta` の初期化ロジック（`get_or_load` 内で設定）
  2. TTL eligibility 判定関数の実装:
     - `is_ttl_expired_human(meta: &CacheResidencyMeta, ttl: Duration, now: SystemTime) -> bool`
     - `is_ttl_expired_virtual(meta: &CacheResidencyMeta, ttl_ticks: u64, vt_now: u64) -> bool`
  3. `get_or_load` の cache hit 時に `last_cache_hit_at` / `last_cache_hit_vt` を更新
  4. protected entry は TTL 判定前に早期リターン（判定不要）
* **検証項目:**
  1. Human Time TTL 超過エントリが正しく eviction 対象となること
  2. VirtualClock TTL 超過エントリが正しく eviction 対象となること
  3. 同一エントリでどちらか一方のみ超過の場合も対象となること
  4. protected preset は二軸とも TTL 対象外であること
* **依存関係:** E1（protected guard）が完了していることが前提条件。

#### ⬜ チケット M-0.5-7-E4: WorkflowCache pressure-driven eviction

* **対象不変条件 / 規範:** §8 ResourcePressure、§15.8 ResourcePressure observations、P-19
* **実装の背景と目的:** `ResourcePressure` と `PressureMode` に応じて eviction aggressiveness を動的に切り替える。`Constrained` では通常の candidate selection、`Emergency` ではより強力な eviction を実行する。ANN hot index bytes を pressure signal に含める。
* **実装スコープ:**
  1. `PressureMode` に応じた eviction 強度の切替:
     - `Normal`: eviction は periodic worker の通常判定に委ねる
     - `Constrained`: eviction candidate 数を 2 倍に増加、TTL 閾値を 0.7 倍に短縮
     - `Emergency`: 非 protected 全エントリを強制 eviction 候補化、TTL 閾値を 0.3 倍に短縮
  2. `ann_hot_index_bytes` を `ResourcePressure` の signal として考慮
  3. `evict_for_pressure()` の実装（E2 から呼び出し）
* **検証項目:**
  1. `Constrained` 時は通常時より多くの eviction が実行されること
  2. `Emergency` 時は全非 protected エントリが eviction されること
  3. ANN hot index bytes 増大時に pressure 判定が適切に動作すること
  4. protected entry は PressureMode の如何にかかわらず eviction されないこと
* **依存関係:** E1（protected guard）、E2（periodic worker）が完了していることが前提条件。

#### ⬜ チケット M-0.5-7-E5: WorkflowCache GcEvent-driven eviction

* **対象不変条件 / 規範:** §12C GcEvent（GraphGcStateChanged）、§15.1 GcState ↔ Cache Residency、P-20
* **実装の背景と目的:** `DarviumEventKind::GcEvent` を購読し、GcState 遷移に応じて cache eviction を実行する。特に `SoftDeleted`, `HardDeleteCandidate`, `Tombstoned` への遷移時に対応する cache entry の residency を縮退させる。`Tombstoned` が cache に残存しないことを invariant test で保証する。
* **実装スコープ:**
  1. `WorkflowCache::handle_gc_state_transition(event: GraphGcStateChanged) -> Result<EvictionReport>` の実装
     - `SoftDeleted`: 該当エントリを eviction candidate に追加
     - `HardDeleteCandidate`: 該当エントリを eviction candidate に追加
     - `Tombstoned`: 該当エントリを直ちに eviction（P-20 違反防止）
     - `Active` / `Protected`: 特に eviction 不要（Protected は E1 で保護済み）
  2. GcEvent 購読のセットアップ（EventBus subscribe または polling）
  3. `Tombstoned` が cache に残存しないことの invariant test（property-based）
  4. `ConsistencyState::Committed` 以外のエントリが hot path（`get_or_load` の通常フロー）から除外されること
* **検証項目:**
  1. `SoftDeleted` / `HardDeleteCandidate` への遷移で cache からエントリが削除されること
  2. `Tombstoned` への遷移で直ちに eviction されること
  3. Tombstoned が cache に残存しない invariant が `proptest` で成立すること
  4. `ConsistencyState::Committed` 以外のエントリが `get_or_load` の hot path から除外されること（P-21）
* **依存関係:** E1（protected guard）が完了していることが前提条件。§12C GcEvent の型定義が利用可能であること。

#### ⬜ チケット M-0.5-7-E6: WorkflowCache eviction invariants and tests

* **対象不変条件 / 規範:** P-17（eviction ≠ persistence deletion）、P-18（protected 不変）、P-19（定期/容量 eviction 必須）、P-20（Tombstoned non-resident）、P-21（非 Committed 除外）
* **実装の背景と目的:** E1-E5 で実装された eviction semantics の総合的な不変条件テスト・property-based test・replay test・capacity test を追加する。各不変条件が成立することを deterministic に検証する。
* **実装スコープ:**
  1. Property-based test（proptest）:
     - protected entry が決して eviction されない不変条件
     - Tombstoned entry が cache に resident しない不変条件
     - Committed entry は再ロード可能である不変条件（P-17）
     - 非 Committed は通常の hot path から除外される不変条件（P-21）
  2. Replay test:
     - GC event stream を replay して cache residency が deterministic に変化することの検証
     - 固定 seed 条件下で eviction 結果が bit-level に再現されること
  3. Capacity test:
     - `max_entries` / `max_bytes` 超過時に非 protected のみが eviction されること
     - protected entry が容量圧迫時も保持されること
  4. Integration test:
     - Periodic worker + GcEvent-driven + pressure-driven の複合シナリオで不変条件が維持されること
* **検証項目:**
  1. 全 property-based test が n >= 1000 の試行で不変条件を満たすこと
  2. Replay test が同一 seed で同一結果を返すこと
  3. Capacity test で protected entry 維持が確認できること
* **依存関係:** E1-E5 が全て完了していることが前提条件。

#### ✅ チケット M1.75-8: deterministic replay シナリオによる village-help 再現性テスト

* **対象不変条件 / 規範:** RFC §41B replay discipline。固定 seed・固定 population・固定 mission stream・固定 VirtualClock 進行のもとで village 構造と HELP outcome が bit-level に再現されなければならない (MUST)。
* **実装スコープ:**
  - `VillageReplayScenario { seed, workflows, missions, clock_schedule, policy_bundle }`
  - `run_replay_scenario(scenario) -> ReplayTrace`
  - `ReplayTrace` に `space_positions`, `villages`, `helper_weights`, `help_sessions`, `child_growth_events` を格納
  - trace equality comparator の実装
* **テストコードによる検証:**
  1. 同一 scenario を 2 回実行して trace が完全一致すること
  2. policy bundle だけを変えた場合に、差分が期待された項目にのみ現れること
  3. seed を変更した場合、個別履歴は変動しても summary metrics が一定範囲に収まること
* **計装方法・観測対象:** replay trace を JSON Lines または構造化ログとして保存し、各 tick ごとの village membership と HELP 状態遷移を差分比較する。trace divergence がゼロであることを deterministic replay の完了条件とし、将来の回帰テスト種 (`golden trace`) として格納する。

#### ✅ チケット M1.75-9: small perturbation 実験スイート（ranking stability 相当の village stability 検証）

* **対象不変条件 / 規範:** RFC §41B small perturbation stability。微小な embedding ノイズ、trust 変動、single-edge patch、利用履歴 1 件追加などの小摂動に対し、village 構造と helper 分布が catastrophic oscillation を起こさないこと。
* **実装スコープ:**
  - perturbation generator 群（`EmbeddingNoise`, `TrustDelta`, `SingleEdgePatch`, `UsageIncrement`, `TemporaryHelperQuarantine`）
  - baseline / perturbed シナリオ比較器
  - `StabilityRegressionSummary` 出力
* **テストコードによる検証:**
  1. 微小ノイズ注入下で `village_churn` と `helper_jsd` が上限閾値を超えにくいこと
  2. helper 1 体の一時隔離で child-support 全体が全面崩壊しないこと
  3. patch による局所変化が global village structure の無制限振動へ波及しないこと
* **計装方法・観測対象:** 摂動強度 $\sigma$ を掃引し、`village_churn_p95(\sigma)` と `helper_jsd_p95(\sigma)` の応答曲線を記録する。臨界摂動強度 $\sigma_c$ を越えたときの相転移的悪化点を同定し、calibration candidate の初期値設定に引き渡す。さらに、false-new rate / compose fallback frequency / review-load など既存メトリクスへの副作用も同時観測し、局所性導入の負の外部性を検出する。

#### ✅ チケット M1.75-10: property-based village invariant fuzzing と failing seed の replay fixture 昇格

* **対象不変条件 / 規範:** RFC §41B village invariants。child には利用可能な adult が存在する限り helper が最低 1 体以上付与されること、non-committed helper が混入しないこと、終端した HELP session が再遷移しないこと、empty village が unsafe execution を誘発しないこと。
* **実装スコープ:**
  - `proptest` による workflow population generator
  - parameter generator (`k`, `alpha`, `beta`, `epsilon`, maturity thresholds)
  - invariant assertion suite
  - failing seed exporter と replay fixture writer
* **テストコードによる検証:**
  1. random population 全域にわたり helper 選定 invariants が破れないこと
  2. `ConsistencyState != Committed` helper 混入が 100% 検出・拒否されること
  3. HELP 終端状態の非再入性が fuzz 下でも維持されること
  4. empty village ケースで unsafe execution ではなく fallback policy が発火すること
* **計装方法・観測対象:** fuzz ケース全体に対する invariant violation 率、最小 failing population size、パラメータ空間における violation clustering を記録する。失敗 seed は replay fixture に昇格し、次回以降の deterministic regression suite へ恒久編入する。

#### ✅ チケット M1.75-11: village calibration loop harness と目的関数 \(J_{village}(\theta)\) の実装

* **対象不変条件 / 規範:** RFC §41B calibration candidates。village-help は fixed constant ではなく calibration candidate の束として管理され、versioned override と観測結果に基づいて調整されるべきである。
* **実装スコープ:**
  - `VillageCalibrationConfig`, `VillageCalibrationHarness`, `VillageCalibrationResult` の定義
  - 目的関数

    \[
    J_{village}(\theta) = a_1 (1 - churn_{p95}) + a_2 (1 - jsd_{p95}) + a_3 survival - a_4 false\_new - a_5 review\_load
    \]

    の実装
  - one-factor-at-a-time sweep、grid sweep、Latin hypercube sampling の 3 モード
  - 結果を `CalibrationReport` として保存する出力器
* **テストコードによる検証:**
  1. 同一パラメータ束で複数回評価したとき目的関数値が決定論的に一致すること
  2. 極端なパラメータ（高 `alpha`、高 `beta`、低 `k` など）が churn / jsd を悪化させ、目的関数が低下すること
  3. harmless な sweep 実行が invariant を壊さずに完走すること
* **計装方法・観測対象:** 目的関数地形 $J_{village}(\theta)$ を粗視化サンプリングし、安定高原（plateau）、鋭い崖、trivial optimum を観測する。感度ベクトル $\nabla J$ の近似を通じて、どのパラメータが stability を支配し、どのパラメータが dynamicity を維持するのかを分離する。結果は calibration-loop.md の形式に従って反復記録し、次段の実装最適化チケットへ渡す。

#### ✅ チケット M1.75-12: village-help 実験レポート生成と系列管理の統合

* **対象不変条件 / 規範:** 既存の observational-testing / experiment-reporting discipline。全チケットは「コードが動くこと」ではなく、「観測可能な振る舞いが特徴づけられ、実験系列として記録されること」を完了条件とする。
* **実装スコープ:**
  - `VillageExperimentReport` 構造体
  - `SimulationRunner` と `CalibrationHarness` の出力を統合する Markdown / JSON report writer
  - replay trace、metrics summary、failing seeds、best-known parameter bundle、open anomalies のセクション生成
  - `rules/darvium/experiment-reporting.md` に準拠した report skeleton 適用
* **テストコードによる検証:**
  1. replay・perturbation・fuzz・calibration の各実験結果が単一レポートへ欠落なく統合されること
  2. empty metrics や failure-only ケースでも壊れたレポートを出さず、必須フィールドを維持すること
  3. failing seed と golden trace 参照がレポート中で相互整合していること
* **計装方法・観測対象:** レポート生成自体の完全性を監視対象とし、各実験系列に対する missing field 率、未解決 anomaly の件数、best-known parameter bundle の更新履歴長を追跡する。実験系列の蓄積に伴う再現性、説明可能性、回帰検出感度の改善をメタ指標として観測し、village-help 実装が「導入された」だけでなく「観測と較正の対象として運用可能になった」ことを完了条件とする。

---

### 8C. マイルストーン M1.76：Reciprocity-Aware Survival and Benevolence Integration（v2.3-f/g）

このチケット分解により、開発チームは以下のステップで機械的に開発を進めることができます。
> **DB**: メモリ内完結。SQLite / LadybugDB 不要。全相互互恵性計算、評判再計算、GC hazard 拡張、HELP helper weighting 拡張、child growth / maturation はメモリ内データ構造と固定シード PRNG により完全決定論的に再現・観測する。
>
> **⚠️ このマイルストーンの位置づけ:** 本節は既存の M1（Human-in-the-loop review）、M1.5（擬似 dual-store / repair discipline）、M1.75（Child Support Villages and HELP）、および M1.5-R（v2.3-g Event Architecture）を一切毀損せず、その上に strictly additive に積み増される Reciprocity-Aware Survival and Benevolence Integration の実装群である。v2.3-g の Event Architecture との整合のため、ReciprocityEvent は `DarviumEventKind::Reciprocity` の DarviumEvent から materialize される EventProjection として動作する。ここで追加される要素は、直接互恵性 (F-1)・間接互恵性 (F-2)・BenevolenceScore 集約 (F-3)・評判再計算 (F-4, F-5)・benevolence-aware GC hazard (F-7〜F-9)・child protection (F-10)・HELP helper weighting への benevolence 追加 (F-11)・softmax selection (F-12)・remote exploration (F-13)・child growth (F-14)・maturation probability (F-15)・多目的較正目的関数 (F-16)・5 段階較正ループ (Phase 0-4) である。既存の L(G) 定義、GC 遷移規則、Grace Period、Resource Pressure、Training-Production Separation、ApplicabilityScore、legal SearchState transitions、dual-store consistency、promotion / repair invariants、village locality、HELP consent protocol、helper weighting ベース式を一切変更してはならない (MUST NOT)。RFC 上では欠番の F-6 は推奨案 A 相当の式であり、実装対象外とする。
>
> **RFC §41C.3 マイルストーン参照:** 本節のチケットは RFC §41C.3 で定義された M0.x〜M4.x の各較正フェーズに対応する。各チケットの対象不変条件に該当フェーズを明記する。

1. **チケットの順番通りに Rust の `tests/` ディレクトリに空のテスト関数（`#[test]`）を作成する。**
2. テストをパスさせるために必要な**最小限のデータ構造と純粋関数**を `src/` 側に記述する。
3. M-0.5 に達した段階で、`rand::rngs::StdRng` を用いたシード固定の確率的テストを導入し、ノイズに対するシステムの耐久性を高める。
4. M2 に到達するまでは、PCのネットワークを切断した状態（完全ローカル環境）であっても `cargo test` が100%グリーンかつミリ秒単位で高速作動する状態を維持する。

#### ✅ チケット M1.76-1: ReciprocityEvent / ReciprocityEventKind データ型定義

* **対象不変条件 / 規範:** RFC §15.10.6 Reciprocity event log、v2.3-g §12C DarviumEvent canonical envelope。ReciprocityEvent は `DarviumEventKind::Reciprocity` の DarviumEvent から materialize される EventProjection として再定義される。ReciprocityEvent の全フィールドが Rust の型システムで表現可能であり、event 系列から直接互恵性スコア・間接互恵性スコアが再現可能でなければならない (MUST)。本チケットは RFC §41C.3 の M0.x（pure function validation）に先行するデータ型基盤である。
* **実装スコープ:**
  - `ReciprocityEvent` 構造体: `event_id: String`, `mission_id: String`, `source_graph_id: WorkflowGraphId`, `target_graph_id: WorkflowGraphId`, `event_kind: ReciprocityEventKind`, `weight: f32`, `created_at: SystemTime`, `virtual_clock: u64`, `trace_ref: Option<String>`
  - `ReciprocityEventKind` 列挙型: `HelpOffered`, `HelpAccepted`, `HelpRejected`, `HelpExecuted`, `HelpSucceeded`, `HelpAbandoned`, `HarmfulMismatch`, `ReturnedFavor`
  - `DarviumEvent`（`DarviumEventKind::Reciprocity`）から `ReciprocityEvent` への変換: `TryFrom<DarviumEvent> for ReciprocityEvent`
  - 全型に `#[derive(Debug, Clone, PartialEq)]` を付与
  - `DarviumError` に `ReciprocityError(String)` バリアント追加
* **テストコードによる検証:**
  1. 全 8 バリアントの `ReciprocityEventKind` が `Debug + Clone + PartialEq` を実装可能であることのコンパイル時確認
  2. `ReciprocityEvent` の全フィールドを設定したインスタンスがコンパイル可能であり、各フィールドにアクセス可能であること
  3. `DarviumEvent`（kind=`DarviumEventKind::Reciprocity`）から `TryFrom` で `ReciprocityEvent` への変換が成功すること
  4. `DarviumEventKind` が `Reciprocity` 以外のイベントからは変換が失敗すること（型安全性）
  5. `event_kind` のパターンマッチングが網羅的であること（`_ =>` 代替がないことの確認）
* **計装方法・観測対象:** 型定義の完全性確認。全フィールドが RFC §15.10.6 の構造体定義と一致していることを人手照合可能な一覧として記録する。`DarviumEvent` ↔ `ReciprocityEvent` の往復変換完全性を $n = 1000$ で検証する。`ReciprocityEventKind` の各バリアントが M1.76-3 以降のスコア計算で参照されることを前提とした型安全性の静的検証。

#### ✅ チケット M1.76-2: ReciprocityLifecyclePolicy 構造体 + ReputationProfile 拡張フィールド定義

* **対象不変条件 / 規範:** RFC §15.10.7 Lifecycle calibration parameter object、§15.10.3 Extended ReputationProfile。全パラメータは versioned policy object として記録されなければならない (MUST)。v2.3-f 追加フィールドを永続カラムとして保存しない場合でも、ReciprocityEvent から recompute 時に導出可能でなければならない (MUST)。本チケットは RFC §41C.3 の M0.x に先行するデータ型基盤である。
* **実装スコープ:**
  - `ReciprocityLifecyclePolicy` 構造体: `theta_dir: f32`, `theta_ind: f32`, `theta_exp: f32`, `theta_inherit: f32`, `lambda_gc_base: f32`, `gamma_lifecycle: f32`, `gamma_benevolence: f32`, `gamma_child_protect: f32`, `rho_direct_decay: f32`, `tau_helper_softmax: f32`, `epsilon_remote_base: f32`, `epsilon_remote_max: f32`, `adult_experience_threshold: u32`, `adult_trust_threshold: f32`, `adult_reputation_threshold: f32` に加え、`policy_version: String` を保持
  - 既存 `ReputationProfile` 構造体への 8 フィールド追加: `direct_help_count: u32`, `direct_success_count: u32`, `direct_reject_count: u32`, `harm_event_count: u32`, `accepted_offer_rate: f32`, `help_success_rate: f32`, `village_centrality: f32`, `benevolence_score: f32`
  - 推奨初期値定数群（`RECIPROCITY_ALPHA_HELP`, `RECIPROCITY_ALPHA_SUCCESS`, `RECIPROCITY_ALPHA_REJECT`, `RECIPROCITY_ALPHA_HARM`, `RECIPROCITY_DIRECT_DECAY_RHO`, `REPUTATION_WEIGHT_DIRECT`, `REPUTATION_WEIGHT_INDIRECT`, `LIFECYCLE_WEIGHT_BENEVOLENCE`, `GC_HAZARD_GAMMA_BENEVOLENCE`, `GC_HAZARD_GAMMA_CHILD_PROTECT`, `HELP_WEIGHT_BENEVOLENCE`, `HELP_SOFTMAX_TAU`, `REMOTE_EXPLORATION_BASE`, `REMOTE_EXPLORATION_MAX`, `CHILD_GROWTH_WEIGHT_HELP_SUCCESS`, `CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS`）
* **テストコードによる検証:**
  1. `ReciprocityLifecyclePolicy` の全フィールドがデフォルト値で初期化可能であること
  2. `ReciprocityLifecyclePolicy` の `policy_version` が明示的に設定・更新可能であること
  3. 拡張後の `ReputationProfile` が既存の全フィールドを保持し、かつ 8 つの新規フィールドが追加されていること
  4. 全定数が `f32` または `u32` として定義され、`NaN` / 負値 / 異常値でないことのアサーション
* **計装方法・観測対象:** 構造体のメモリレイアウト（フィールド数・型サイズ）をコンパイル時に確認。定数群の命名一覧と RFC 付録 E の v2.3-f calibration candidates との対応をテーブル化し、過不足なく網羅されていることを照合する。既存 `ReputationProfile` の破壊的変更が発生していないこと（全既存フィールドが同一名・同一型で維持されていること）を型チェックで検証する。

#### ✅ チケット M1.76-3: 直接互恵性スコア compute_direct_reciprocity (F-1) 純粋関数実装

* **対象不変条件 / 規範:** RFC §15.10.2 式 F-1。`α_h, α_hs > 0`、`α_r, α_d > 0`。協力行為は `R_i^dir` を非減少にし、裏切り・害は非増加にしなければならない (MUST)。本チケットは RFC §41C.3 の **M0.x（pure function validation）** に対応する。
* **実装スコープ:**
  - `compute_direct_reciprocity(events: &[ReciprocityEvent], now: u64, policy: &ReciprocityLifecyclePolicy) -> f32` 純粋関数
  - 式 F-1: `σ( Σ_{j≠i} ω_ij^dir ( α_h H_ij + α_hs HS_ij - α_r RJ_ij - α_d DMG_ij ) exp(-ρ_dir Δt_ij) )`
  - 時間減衰 `exp(-ρ_dir Δt_ij)` の実装（`virtual_clock` または `created_at` に基づく経過量）
  - logistic sigmoid または calibrated sigmoid による `[0, 1]` への押し込み
  - 係数マッピング（`ReciprocityEventKind` → `(H, HS, RJ, DMG)` の重み割り当てテーブル）
* **テストコードによる検証:**
  1. 空イベントリスト `[]` に対して `0.5`（sigmoid(0)）が返ること
  2. `HelpSucceeded` イベントのみの系列で、イベント数増加に伴いスコアが単調増加すること
  3. `HarmfulMismatch` イベントのみの系列で、イベント数増加に伴いスコアが単調減少すること
  4. 同じ positive イベントでも `Δt_ij` が大きい（古い）ほどスコアが低くなること（時間減衰）
  5. 係数 `α_h = 0, α_hs = 0` のとき、他条件一定で正のスコア変化がゼロになること
* **計装方法・観測対象:** 固定シード PRNG で生成した ReciprocityEvent 系列 $n \ge 10^4$ を投入し、`R_i^dir` の値域が常に `[0, 1]` に拘束されること、および入力イベント種別比率と出力スコアの相関を散布図として観測する。時間減衰パラメータ `ρ_dir` を `[0.001, 0.1]` の範囲で sweep し、同一イベント系列に対する減衰曲線 $R_i^dir(t)$ の形状（半減期）を計測する。協力行為・裏切り行為それぞれに対するスコア変化の単調性を、`n = 1000` のランダム挿入系列で検証する。

#### ✅ チケット M1.76-4: 間接互恵性スコア compute_indirect_reciprocity (F-2) + BenevolenceScore 集約 (F-3)

* **対象不変条件 / 規範:** RFC §15.10.2 式 F-2、F-3。間接互恵性スコアは「社会全体から見た善良さ」を表し、直接互恵性と分離して保持される (MUST)。BenevolenceScore は評判・直接互恵性・間接互恵性の合成量として定義される。本チケットは RFC §41C.3 の **M0.x** に対応する。
* **実装スコープ:**
  - `compute_indirect_reciprocity(events: &[ReciprocityEvent], centrality: f32, village_metrics: &VillageMetrics) -> f32` 純粋関数
  - 式 F-2 の各項: `β_1 C_i^help`（中心性）、`β_2 A_i^village`（村参加度）、`β_3 U_i^accepted`（受諾率）、`β_4 Q_i^success`（成功貢献率）、`β_5 B_i^harm`（負評価）
  - `compute_benevolence_score(dir_score: f32, ind_score: f32, reputation: f32, policy: &ReciprocityLifecyclePolicy) -> f32`
  - 式 F-3: `B_i = w_dir · R_i^dir + w_ind · R_i^ind + w_rep · Rep_i`
  - 係数は非負、かつ推奨 `w_dir + w_ind + w_rep = 1`
* **テストコードによる検証:**
  1. `C_i^help = 0, A_i^village = 0, U_i^accepted = 0, Q_i^success = 0, B_i^harm = 0` のとき `R_i^ind = 0.5`（sigmoid(0)）が返ること
  2. 中心性 `C_i^help` を `[0, 1]` で sweep したとき、`R_i^ind` が単調増加すること
  3. `B_i^harm` 増加に伴い `R_i^ind` が単調減少すること
  4. `BenevolenceScore` が `[0, 1]` に bounded されること
  5. `w_dir = 1, w_ind = 0, w_rep = 0` のとき `B_i = R_i^dir` となること
  6. `w_dir = 0, w_ind = 0, w_rep = 1` のとき `B_i = Rep_i` となること
* **計装方法・観測対象:** `C_i^help`（中心性）と `B_i^harm`（負評価）の 2 次元パラメータ空間上で `R_i^ind` の応答曲面を観測する。β 係数を個別に sweep した際の各項の感度曲線 $\partial R_i^ind / \partial β_k$ を中心差分で推定する。BenevolenceScore の合成則について、`(w_dir, w_ind, w_rep)` の単体 simplex 上の目的関数等高線をプロットし、評判偏重・互恵性偏重の中間領域での挙動を特徴づける。

#### ✅ チケット M1.76-5: ReputationProfile 再計算 recompute_reputation (F-4, F-5)

* **対象不変条件 / 規範:** RFC §15.10.3 式 F-4、F-5。direct_score と indirect_score の寄与は 0 であってはならない (MUST NOT) unless environment policy が明示的に village-help を無効化している場合。final_score は direct / indirect reciprocity が増加したとき、他条件一定なら非減少でなければならない (MUST)。experience 正規化 (F-5) は古参固定化防止のために適用される。本チケットは RFC §41C.3 の **M0.x** に対応する。
* **実装スコープ:**
  - `recompute_reputation(inputs: ReputationInputs, policy: &ReciprocityLifecyclePolicy) -> ReputationProfile` 純粋関数
  - 式 F-4: `Rep_i = clip_{[0,1]}( θ_dir · R_i^dir + θ_ind · R_i^ind + θ_exp · E_i^norm + θ_inh · I_i )`
  - 式 F-5: `E_i^norm = 1 - exp(-κ_E · experience_count(i))`
  - `ReputationInputs` 構造体（既存 ReputationProfile のスコア成分 + 拡張フィールド）
  - 係数和 `θ_dir + θ_ind + θ_exp + θ_inh = 1` の推奨正規化アサーション
* **テストコードによる検証:**
  1. 全係数を正值に設定し、`R_i^dir`, `R_i^ind`, `E_i^norm`, `I_i` 各成分を個別に sweep したとき final_score が単調非減少であること
  2. `θ_dir = 0, θ_ind = 0` の設定で、village-help 無効化ケースを模した final_score 計算が warning または error を発すること（ただし 0 自体はコンパイルを通す）
  3. `experience_count = 0` のとき `E_i^norm = 0` となること
  4. `experience_count → ∞` のとき `E_i^norm → 1` に漸近すること
  5. 全成分 0 のとき `final_score = 0`、全成分 1 のとき `final_score = 1` となること
  6. 拡張フィールド（`direct_help_count` 等）が正確に反映されること
* **計装方法・観測対象:** 経験値飽和曲線 $E_i^{norm}(κ_E)$ を $κ_E \in [0.001, 0.1]$ で sweep し、$experience\_count = 10$ における正規化値の分布を観測する。係数ベクトル `(θ_dir, θ_ind, θ_exp, θ_inh)` を確率単体上でラテン方格サンプリングし、最終スコア $Rep_i$ の超平面応答を計測する。各成分の部分微分 ∂Rep_i/∂θ_k の感度分析により、どの互恵性成分が評判支配的であるかを同定する。

#### ✅ チケット M1.76-6: GC hazard with benevolence (F-7, F-8, F-9) — [`tickets/specs/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9.md`](tickets/specs/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9.md)

* **対象不変条件 / 規範:** RFC §15.10.4 式 F-7、F-8、F-9。`λ_i^GC = softplus( λ_0 - γ_L · L_i - γ_B · B_i - γ_C · C_i^protect )`。softplus により常に非負。∂λ_i^GC/∂R_i^dir ≤ 0、∂λ_i^GC/∂R_i^ind ≤ 0、∂λ_i^GC/∂Rep_i ≤ 0。直接・間接互恵性・評判の増加は P_survive を非減少にしなければならない (MUST)。本チケットは RFC §41C.3 の **M0.x** に対応する。
* **実装スコープ:**
  - `compute_gc_hazard(lifecycle_score: f32, benevolence_score: f32, child_protection: f32, policy: &ReciprocityLifecyclePolicy) -> f32` 純粋関数
  - 式 F-7: `softplus( λ_0 - γ_L · L_i - γ_B · B_i - γ_C · C_i^protect )`
  - `compute_gc_probability(hazard: f32, delta_t: u64) -> f64`
  - 式 F-8: `p_GC(i; Δt) = 1 - exp(-λ_i^GC · Δt)`
  - `compute_survival_probability(hazard: f32, delta_t: u64) -> f64`
  - 式 F-9: `P_survive(i; Δt) = exp(-λ_i^GC · Δt)`
  - 既存 GC 計算との和算・結合方法（既存 L(G) は変更せず、GC hazard 側で benevolence を効かせる design を維持）
* **テストコードによる検証:**
  1. `λ_0 = 1.0, γ_L = γ_B = γ_C = 0` のとき、`hazard = softplus(1.0) ≈ 1.1269` となること
  2. `benevolence_score` を `[0, 1]` で sweep したとき hazard が単調減少すること（単調性 MUST）
  3. `lifecycle_score` を `[0, 1]` で sweep したとき hazard が単調減少すること（既存と矛盾しない）
  4. `hazard = 0` のとき `P_survive = 1` であり、Δt を変えても不変であること
  5. `hazard > 0` のとき `P_survive` が `[0, 1)` に bounded され、Δt 増加に伴い単調減少すること
  6. `γ_B = 0` のとき benevolence が hazard に影響しないこと（既存挙動との一致）
  7. `γ_L = γ_B = γ_C = 0`、`λ_0 = 0` のとき hazard = 0 となること
* **計装方法・観測対象:** `(L_i, B_i)` の 2 次元パラメータグリッド上で `λ_i^GC` の応答曲面を観測する。`γ_B / γ_L` の比を sweep し、benevolence が lifecycle score と比較してどの程度の hazard 低減効果を持つかを感度比として計測する。softplus の非負性を $n = 10^6$ のランダム入力で検証し、浮動小数点例外（NaN/Inf）が発生しないことを確認する。$P_{survive}$ の値域が常に `[0, 1]` に bounded されることの統計的検証。

#### ✅ チケット M1.76-7: Child protection integration (F-10)

* **対象不変条件 / 規範:** RFC §15.10.5 式 F-10。本項は既存の Grace Period（`experience_count < MIN_SURVIVAL_EXPERIENCE`）を弱めず、補強する (MUST NOT weaken)。`C_i^protect = η_1 · 1[Child(i)] + η_2 · H_i^received + η_3 · G_i^growth`。本チケットは RFC §41C.3 の **M0.x** に対応する。
* **実装スコープ:**
  - `compute_child_protection(is_child: bool, help_received: f32, growth_improvement: f32, policy: &ReciprocityLifecyclePolicy) -> f32` 純粋関数
  - `is_child` 判定は既存 `classify_maturity()` の `WorkflowMaturity::Child` を流用
  - `help_received`: child として有効支援を受けた量（M1.75-3 の `HelpExecution`/`HelpSuccess` から派生）
  - `growth_improvement`: child が maturation に向けて改善している量（M1.76-10 の F-14 と接続）
  - 既存 Grace Period との併用アサーション（Grace Period 中かつ C_i^protect > 0 でも hazard 増加がないこと）
* **テストコードによる検証:**
  1. `is_child = false, help_received = 0, growth_improvement = 0` のとき `C_i^protect = 0` となること
  2. `is_child = true` のとき、`help_received` と `growth_improvement` によらず最低 `η_1` の保護が得られること
  3. `help_received` を `[0, 1]` で sweep したとき `C_i^protect` が単調増加すること
  4. `growth_improvement` を `[0, 1]` で sweep したとき `C_i^protect` が単調増加すること
  5. 既存 Grace Period の child 保護効果と本式の保護効果が独立に additive に効くこと（既存 GC hazard 計算に本項を加えても既存の Grace Period 条件が無効化されないこと）
* **計装方法・観測対象:** `(is_child, help_received, growth_improvement)` の 3 次元入力空間上で `C_i^protect` の応答を観測する。既存 Grace Period 下の child と Grace Period 超過後も本保護が継続する child の 2 群について、GC hazard の経時変化を追跡し、「育っている child」が保護される度合いを定量化する。`η_1, η_2, η_3` の比率を sweep し、child 保護の利得曲線を観測する。

#### ✅ チケット M1.76-8: Helper quality score with benevolence (F-11) + softmax selection (F-12)

* **対象不変条件 / 規範:** RFC §41B.20.1 式 F-11、§41B.20.2 式 F-12。同程度に有能な adult が複数いるなら、より協力的で評判の良い adult を helper に選ぶ (MUST)。softmax の温度 `τ_Q` は calibration candidate であり、高すぎると helper 固定化、低すぎると benevolence bias が薄まる。本チケットは RFC §41C.3 の **M0.x** に対応する。
* **実装スコープ:**
  - `compute_helper_quality_score(mission_suitability: f32, trust: f32, reputation: f32, benevolence: f32, child_need: f32, distance: f32, policy: &ReciprocityLifecyclePolicy) -> f32` 純粋関数
  - 式 F-11: `Q(h,c,M) = w_s · S + w_t · T + w_r · Rep + w_b · B + w_n · N - w_d · d`
  - `softmax_helper_selection(candidates: &[HelperCandidate], policy: &ReciprocityLifecyclePolicy) -> Vec<SoftmaxWeight>` 純粋関数
  - 式 F-12: `π(h|c,M) = exp(τ_Q · Q(h,c,M)) / Σ_g exp(τ_Q · Q(g,c,M))`
  - `SoftmaxWeight { helper_id: WorkflowGraphId, probability: f64, rank: usize, score_breakdown: QualityScoreBreakdown }`
  - 既存 helper weighting (41B-18) のベース式を変更せず、benevolence 項 `w_b · B(h)` を additive に追加
* **テストコードによる検証:**
  1. `w_b = 0` のとき既存 weight と一致すること（下位互換性）
  2. 同一 S, T, Rep, N, d の候補間で `benevolence` が高い candidate が常に高い softmax 確率を得ること
  3. `τ_Q → ∞` の limit で softmax が argmax に近づくこと
  4. `τ_Q → 0` の limit で softmax が一様分布に近づくこと
  5. 全 candidate の softmax 確率和が 1.0（±浮動小数点誤差）になること
  6. empty candidate list に対して空の `Vec` が返ること
* **計装方法・観測対象:** 同一スコア候補群に対する benevolence の単一要素感度を計測し、`w_b` が選好に与える影響度を定量化する。`τ_Q` を `[0.1, 10.0]` で sweep し、helper 分布エントロピー $H(π) = -Σ π_h log π_h$ の応答曲線を観測する。エントロピーが低すぎる（固定化）または高すぎる（ランダム化）領域を同定し、sweet spot の較正範囲を推定する。候補数 $K_{cand}$ を sweep し、softmax 計算の数値安定性（log-sum-exp trick の要否）を $K_{cand} \ge 1000$ まで検証する。

#### ✅ チケット M1.76-9: Benevolence-aware remote exploration (F-13)

* **対象不変条件 / 規範:** RFC §41B.20.3 式 F-13。v2.3-e の bounded remote exploration (41B-19) を保持しつつ、local adults の benevolence が十分高い場合は remote exploration を下げ、local shortage 時にのみ上げる。「近くに優しい大人がいるなら、まず近所で助け合う」を operational に実現する。本チケットは RFC §41C.3 の **M0.x** に対応する。
* **実装スコープ:**
  - `compute_benevolence_aware_remote_exploration(child_need: f32, local_benevolence_mean: f32, policy: &ReciprocityLifecyclePolicy) -> f32` 純粋関数
  - 式 F-13: `ε_remote(c) = clip_{[0, ε_max]}( ε_0 + a_1 · need(c) - a_2 · B_local_avg(c) )`
  - `local_benevolence_mean`: local village 内 adult の BenevolenceScore 平均
  - 既存 M1.75-6 の `select_helpers()` と接続（既存の exploration 率 ε を本関数で上書きする adapter）
* **テストコードによる検証:**
  1. `need = 0, B_local_avg = 1.0` のとき `ε_remote` が最小値（`clip` 下限）になること
  2. `need = 1.0, B_local_avg = 0` のとき `ε_remote` が最大値（`clip` 上限）になること
  3. `local_benevolence_mean` 増加に伴い `ε_remote` が単調非増加であること
  4. `a_2 = 0` のとき既存の exploration 式と一致すること（下位互換性）
  5. `clip` により `ε_remote` が常に `[0, ε_max]` に bounded されること
* **計装方法・観測対象:** `(need, B_local_avg)` の 2 次元パラメータ空間上で `ε_remote` の応答曲面を観測する。`a_1 / a_2` の比率を sweep し、need-driven exploration と benevolence-driven restraint のトレードオフ曲線を計測する。既存の exploration 率と本式による調整後の exploration 率の差分分布を `n = 10^4` の random village 状態で観測し、benevolence が remote exploration をどの程度抑制するかを定量化する。

#### ✅ チケット M1.76-10: Child growth increment (F-14) + Maturation probability (F-15)

* **対象不変条件 / 規範:** RFC §41B.20.4 式 F-14、§41B.20.5 式 F-15。benevolence-rich village で child は成長しやすく、成熟しやすい。「優しい大人に囲まれた child は成熟しやすい」を数理的に実現する。本チケットは RFC §41C.3 の **M0.x** に対応する。
* **実装スコープ:**
  - `compute_child_growth_increment(mission_success: bool, help_successes: &[f32], helper_benevolence_mean: f32, failure_burden: f32) -> f32` 純粋関数
  - 式 F-14: `ΔG_c = μ_1 · MissionSuccess_c + μ_2 · Σ_h HelpSuccess(h→c) + μ_3 · B_helpers_avg(c) - μ_4 · FailureBurden_c`
  - `compute_maturation_probability(experience_norm: f32, trust: f32, reputation: f32, helper_benevolence_mean: f32, policy: &ReciprocityLifecyclePolicy) -> f64` 純粋関数
  - 式 F-15: `P_mature(c) = σ( ν_0 + ν_1 · E_c^norm + ν_2 · T_c + ν_3 · Rep_c + ν_4 · B_helpers_avg(c) )`
  - 既存 maturity 判定器との結合（`classify_maturity` 内で参照される成長量として統合）
* **テストコードによる検証:**
  1. `MissionSuccess = false, help_successes = [], helper_benevolence_mean = 0, failure_burden = 0` のとき `ΔG_c = 0` となること
  2. `mission_success = true` で正の成長増分が得られること
  3. `helper_benevolence_mean` 増加に伴い `ΔG_c` が単調増加すること
  4. `failure_burden` 増加に伴い `ΔG_c` が単調減少すること
  5. `P_mature` が `[0, 1]` に bounded されること
  6. `helper_benevolence_mean` 増加に伴い `P_mature` が単調増加すること
  7. `ν_4 = 0` のとき既存の maturation 判定と一致すること（下位互換性）
* **計装方法・観測対象:** `(μ_1, μ_2, μ_3, μ_4)` のパラメータ空間上で成長増分 $ΔG_c$ の感度分析を行う。`helper_benevolence_mean` を `[0, 1]` で sweep し、`P_mature` のシグモイド応答曲線を観測する。`ν_4` を変化させたときの成熟確率上昇度合いを定量化し、benevolence が maturation rate に与える影響の効果量（Cohen's d）を `n = 10^4` のシミュレーションで推定する。`μ_1 : μ_2 : μ_3 : μ_4` の比率 sweep により、mission success と helper benevolence の成長寄与度を比較する。

#### ✅ チケット M1.76-11: ReciprocityEvent インジェスション + reputation/hazard recompute パイプライン

* **対象不変条件 / 規範:** RFC §15.10.6、§15.10.7、§41C.3 M1.x、v2.3-g §12E EventProjection。ReciprocityEvent は EventBus 上の `DarviumEventKind::Reciprocity` イベントから materialize される `ReciprocityEventProjection` として扱う。policy version を固定した上で ReputationProfile と GC hazard を再計算し、その結果をスナップショット比較可能でなければならない (MUST)。本チケットは RFC §41C.3 の **M1.x（replayable reputation/hazard recompute）** に対応する。
* **実装スコープ:**
  - `ReciprocityEventProjection`: `EventProjection` トレイトを実装し、`DarviumEventKind::Reciprocity` イベントから ReciprocityEvent 系列を materialize
  - `ReciprocityEventStore`（ProjectionCatalog 経由でアクセス可能）: メモリ内 event registry（`HashMap<String, Vec<ReciprocityEvent>>` by `source_graph_id`）
  - `ingest_reciprocity_event(store: &mut ReciprocityEventStore, event: DarviumEvent) -> Result<(), DarviumError>`（EventBus から受け取ったイベントを投影）
  - `recompute_all_profiles(store: &ReciprocityEventStore, metrics: &HashMap<WorkflowGraphId, GraphMetrics>, policy: &ReciprocityLifecyclePolicy) -> HashMap<WorkflowGraphId, ReputationProfile>`: 全 graph の ReputationProfile 一括再計算
  - `recompute_all_gc_hazards(profiles: &HashMap<WorkflowGraphId, ReputationProfile>, lifecycle_scores: &HashMap<WorkflowGraphId, f32>, policy: &ReciprocityLifecyclePolicy) -> HashMap<WorkflowGraphId, f32>`
  - `ReciprocityReplaySnapshot`: 再計算結果のスナップショット（profile/Hazard の組を policy_version 付きで保持）
  - `compute_replay_comparison(before: &ReciprocityReplaySnapshot, after: &ReciprocityReplaySnapshot) -> ReciprocityDiffReport`
* **テストコードによる検証:**
  1. 空 store に対して `recompute_all_profiles` が空の `HashMap` を返すこと
  2. 同一 event stream を 2 回 ingestion しても同一の ReputationProfile が再現されること（deterministic replay）
  3. 異なる policy version で recompute した結果が異なる場合、`ReciprocityDiffReport` に差分が正確に記録されること
  4. 1 件の event 追加後の recompute 結果が、追加前と異なること（event が計算に反映されること）
  5. `policy_version` が snapshot に正確に記録されること
* **計装方法・観測対象:** 固定シードで生成した event stream を `n = 100` 件 pipeline に通し、各イベント追加後の ReputationProfile と GC hazard の逐次更新軌跡を時系列として記録する。同一 stream・同一 policy での replay 結果が完全一致すること（全フィールドのビットレベル一致）を `n = 1000` 回の独立 replay で確認する。policy version 変更前後の diff report の項目数・内容を観測し、どのパラメータ変更がどのスコアに影響するかのトレーサビリティを検証する。

#### ✅ チケット M1.76-12: 単調性テストスイート（MUST monotonicity tests）

* **対象不変条件 / 規範:** RFC §41B.20.8 Testing discipline「Monotonicity tests (MUST)」。他条件一定で `direct_score` 増加 → `survival_probability` 非減少、`indirect_score` 増加 → GC hazard 非増加、同能力 helper 間で benevolence 高い方が proposal ranking で不利にならない。本チケットは RFC §41C.3 の **M2.x（perturbation suite + ranking stability gate）** の一部として位置づける。
* **実装スコープ:**
  - `MonotonicityTestSuite` 構造体（全単調性条件の定義と自動検証器）
  - `MonotonicityCondition` 列挙型（`DirectScoreIncrease`, `IndirectScoreIncrease`, `ReputationIncrease`, `BenevolenceHelperRanking`）
  - `check_monotonicity(suite: &MonotonicityTestSuite) -> MonotonicityReport`
  - `MonotonicityReport { conditions_passed: Vec<(MonotonicityCondition, bool)>, failure_details: Vec<String> }`
  - 以下の MUST 条件を個別テスト関数として実装:
    1. `test_direct_score_survival_monotonicity`: 他条件一定で direct_score 増加 → survival_probability 非減少
    2. `test_indirect_score_gc_hazard_monotonicity`: 他条件一定で indirect_score 増加 → GC hazard 非増加
    3. `test_reputation_gc_hazard_monotonicity`: 他条件一定で Reputation 増加 → GC hazard 非増加
    4. `test_benevolence_helper_ranking_monotonicity`: 同能力 helper 間で benevolence 高い方が ranking で不利にならない
* **テストコードによる検証:**
  1. 条件 1: `direct_score = 0.0, 0.25, 0.5, 0.75, 1.0` の各点で `compute_survival_probability(compute_gc_hazard(...))` の出力が非減少であること
  2. 条件 2: `indirect_score = 0.0, 0.25, 0.5, 0.75, 1.0` の各点で `compute_gc_hazard()` の出力が非増加であること
  3. 条件 3: `Reputation.final_score = 0.0, 0.25, 0.5, 0.75, 1.0` の各点で `compute_gc_hazard()` の出力が非増加であること
  4. 条件 4: S, T, Rep, N, d を固定した 2 候補（B=0.3 vs B=0.9）で `softmax_helper_selection()` の ranked probability が高 benevolence 候補で高くなること
  5. 各条件をランダムパラメータ設定 $n = 1000$ 回の sweep でも維持されること
* **計装方法・観測対象:** 全単調性条件のパス/フェイルをブール値として記録する。5 点 sweep に加え、ランダムパラメータ sweep $n = 1000$ での単調性違反発生率を観測する（期待値: 0）。違反が検出された場合、単調性破綻を引き起こすパラメータ領域を特定し、`MonotonicityReport.failure_details` に記録する。条件 4 の helper ranking 単調性については、benevolence 差 `ΔB` を `[0.001, 0.5]` で sweep し、ranking reversal が発生する臨界 `ΔB` 閾値を検出する。

#### ✅ チケット M1.76-13: 決定論的リプレイテスト（MUST replay test）

* **対象不変条件 / 規範:** RFC §41B.20.8 Testing discipline「Replay test (MUST)」、v2.3-g §12C DarviumEventBus replay。同一 event stream（EventBus 経由の DarviumEvent 列）、同一 policy version、同一 EventBus clock なら ReputationProfile と GC hazard の再計算結果は一致すること (MUST)。本チケットは RFC §41C.3 の **M1.x** に対応する。
* **実装スコープ:**
  - `ReciprocityReplayScenario { event_stream: Vec<DarviumEvent>, policy: ReciprocityLifecyclePolicy, clock_schedule: Vec<u64>, initial_profiles: HashMap<WorkflowGraphId, ReputationProfile> }`（`DarviumEventKind::Reciprocity` を含むイベント列を使用）
  - `run_reciprocity_replay(scenario: &ReciprocityReplayScenario) -> ReciprocityReplayTrace`
  - `ReciprocityReplayTrace { profiles: HashMap<WorkflowGraphId, ReputationProfile>, hazards: HashMap<WorkflowGraphId, f32>, snapshots: Vec<ReciprocityReplaySnapshot> }`
  - `ReplayTraceComparator::assert_bitwise_eq(a: &ReciprocityReplayTrace, b: &ReciprocityReplayTrace)`
  - golden trace 保存および回帰比較機構
* **テストコードによる検証:**
  1. 全く同一の scenario を 2 回実行し、trace の全フィールドがビットレベルで一致することを確認（$n = 10$ 回の独立実行）
  2. policy version のみ変更した scenario で、差分が期待されたフィールドにのみ現れること
  3. VirtualClock 進行スケジュールのみ変更した場合、時刻依存項にのみ差分が限定されること
  4. event stream の順序を維持したまま再実行した場合に完全一致すること
* **計装方法・観測対象:** replay trace 中の各スナップショット間の差分ノルム $||trace_A(t) - trace_B(t)||$ を時間発展として記録する。$n = 100$ 回の独立 replay における最大差分量が 0 であることの検定により、決定論的再現性を保証する。golden trace からの乖離を将来の regression 検出に利用するため、`trace_hash: String` を trace に付与し、回帰テスト種として保存する。

#### ✅ チケット M1.76-14: 摂動テストスイート（SHOULD perturbation）

* **対象不変条件 / 規範:** RFC §41B.20.8 Testing discipline「Perturbation test (SHOULD)」。1 件の help success 追加で village 全体が崩壊的に並び替わらないこと。1 helper の微小な trust change で helper set が全入れ替えしないこと。本チケットは RFC §41C.3 の **M2.x** に対応する。
* **実装スコープ:**
  - `ReciprocityPerturbationGenerator` トレイト（`apply(snapshot) -> PerturbedSnapshot`）
  - 摂動種: `HelpSuccessAddition(1件)`, `TrustDelta(0.01増減)`, `LocalityDistanceDelta(微小変更)`, `AcceptedOfferToOneRejected(置換)`, `SingleHelperReputationDelta(微調整)`
  - `ReciprocityPerturbationSuite`: 全摂動種を baseline と perturbed のペアで実行
  - `StabilityRegressionSummary { flip_rate: f64, churn_delta: f64, hazard_drift: f64, survival_drift: f64, oscillation_detected: bool, village_churn_delta: f64 }`
  - `OscillationDetector`: 摂動前後の ranking 順位変動を追跡し、無限ループ的振動を検出
* **テストコードによる検証:**
  1. help success 1 件追加で helper ranking flip rate が上限閾値（例: 0.20）を超えないこと
  2. trust を 0.01 微増減したときの village churn delta が許容範囲内であること
  3. accepted offer 1 件を rejected に置換したときの survival probability drift が許容範囲内であること
  4. 1 helper の reputation 微調整で helper set の全入れ替えが発生しないこと
  5. 各摂動種で oscillation が検出されないこと（`oscillation_detected == false`）
* **計装方法・観測対象:** baseline と perturbed 間の ranking flip rate、village churn delta、hazard drift、survival drift を各摂動種について $n = 100$ 回の独立実行で観測する。摂動強度 $\sigma$ を sweep し、`flip_rate(σ)` の応答曲線をプロットする。摂動強度の臨界値 $\sigma_c$ を同定し、較正パラメータ設定の推奨範囲を推定する。`benevolent_survival_advantage` の摂動前後変化、`gc_hazard_drift_under_small_patch`、`ranking_flip_rate_under_small_patch` の補助メトリクスも同時記録する。

#### ✅ チケット M1.76-15: プロパティベース不変条件ファジング（SHOULD property-based test）

* **対象不変条件 / 規範:** RFC §41B.20.8 Property-based test。生成対象: workflow population size、child/adult ratio、distance matrix、help event stream、harm/reject noise、policy coefficients。検証性質: benevolence monotonicity、hazard non-negativity、probability boundedness、no negative reputation、no silent overflow/NaN、child in grace period は一時的低 reputation でも GC されない。本チケットは RFC §41C.3 の **M2.x** の一部に対応する。
* **実装スコープ:**
  - `proptest` 戦略群: `workflow_population_strategy()`, `child_adult_ratio_strategy()`, `distance_matrix_strategy()`, `help_event_stream_strategy()`, `harm_reject_noise_strategy()`, `policy_coefficient_strategy()`
  - `ReciprocityInvariantSuite`: 全不変条件の定義と自動検証器
  - 検証 invariant 群:
    1. `benevolence_monotonicity`: dir/ind 増加で benevolence 非減少
    2. `hazard_non_negativity`: GC hazard が常に非負（softplus 保証）
    3. `probability_boundedness`: 全確率出力が `[0, 1]` に bounded
    4. `no_negative_reputation`: ReputationScore 全成分が非負
    5. `no_silent_overflow_nan`: NaN/Inf が一切発生しない
    6. `grace_period_child_protection`: Grace Period 中の child は一時的低 reputation でも GC されない
  - failing seed export → replay fixture 昇格機構
* **テストコードによる検証:**
  1. ランダム population 全域で invariant 1-6 が破れないこと（$n \ge 10^4$ ケース）
  2. 極端なパラメータ設定（全係数 0、全係数最大、population サイズ極値）で invariant violation がゼロであること
  3. Grace Period child に対して、`reputation = 0.0, benevolence = 0.0` でも GC hazard が既存 Grace Period の保護効力により有限に留まること
  4. 検出された violation が failing seed として export され、fixture に昇格可能であること
* **計装方法・観測対象:** fuzz ケース全体に対する invariant violation 率（期待値: 0）を記録する。パラメータ空間における violation clustering を検出し、脆弱なパラメータ領域の有無を観測する。`grace_period_child_protection` invariant について、Grace Period 中の child に対する GC hazard の分布と Grace Period 超過後の分布を比較し、保護効果の統計的有意差を Welch の t 検定（$p < 0.05$）で検証する。failing seed は replay fixture に昇格した数をカウントし、発見されたエッジケースの蓄積を監視する。

#### ✅ チケット M1.76-16: 多目的較正目的関数 F-16 + 較正ハーネス

* **対象不変条件 / 規範:** RFC §15.10.8 式 F-16。`J(θ) = λ_1 · AUC_benevolent>nonbenevolent + λ_2 · HelpSuccessRate - λ_3 · VillageChurnP95 - λ_4 · FalseNewRate - λ_5 · ReviewLoad - λ_6 · InstabilityPenalty`。「善良な workflow が非善良 workflow より survival ranking 上位に来る確率」を ranking 指標として含む multi-objective 較正。本チケットは RFC §41C.3 の **M4.x（human-reviewed calibration）** の較正目的部分に対応する。
* **実装スコープ:**
  - `ReciprocityCalibrationObjective` 構造体: 6 成分の重み `λ_1..λ_6` と各成分の計算器
  - `compute_auc_benevolent_survival(profiles: &[SurvivalPair]) -> f64`: 善良群と非善良群の survival ranking AUC
  - `compute_calibration_objective(metrics: &ReciprocityOperationalMetrics, weights: &[f64; 6]) -> f64`: 式 F-16 の合成値
  - `ReciprocityCalibrationHarness`: パラメータ θ（`ReciprocityLifecyclePolicy` の全 calibration candidate）を受け取り、replay/perturbation/simulation を実行し `J(θ)` を評価
  - `CalibrationReport`: θ 設定値・`J(θ)` 値・各成分値・実験ID を保持
  - 実験系列管理: 各実行に `exp-{yyyymmdd}-{seq}` + 親実験ID
* **テストコードによる検証:**
  1. 同一 θ で複数回 `compute_calibration_objective` を呼び出し、決定論的に同一 `J(θ)` が返ること
  2. `λ_1 = 0, λ_2 = 0, λ_3 = 0, λ_4 = 0, λ_5 = 0, λ_6 = 0` のとき `J(θ) = 0` となること
  3. `AUC_benevolent>nonbenevolent` の計算が、ランダム ranking（AUC ≈ 0.5）と完全分離 ranking（AUC ≈ 1.0）を正しく区別すること
  4. 極端なパラメータ θ（全係数 0、全係数最大）で `J(θ)` が NaN/Inf を返さないこと
* **計装方法・観測対象:** $\lambda$ 重みベクトルを sweep し、目的関数 $J(θ)$ の超平面応答を観測する。AUC 成分の計算について、benevolence 上位 20% 群と下位 20% 群の survival ranking 分布を ROC 曲線として可視化する。パラメータ θ の 1-at-a-time 感度分析により、∂J/∂θ_i を推定し、どの calibration candidate が目的関数を支配しているかを同定する。

#### ✅ チケット M1.76-17: 合成村シミュレーター（Phase 3: Synthetic ecosystem simulation）

* **対象不変条件 / 規範:** RFC §15.10.9 Phase 3、§41C.3 M3.x。Training Plane の safe sandbox scope で synthetic population を走らせ、優しい世界が emergent に成立するかを検証する。simulator は production path を汚染せず、Training Plane または fake execution path に限定する (MUST)。本チケットは RFC §41C.3 の **M3.x** に対応する。
* **実装スコープ:**
  - `ReciprocitySimulatorConfig { population_size, child_ratio, mission_rate, max_ticks, policy, seed }`
  - `SyntheticPopulationGenerator`: child/adult population を固定シードで生成
  - `MissionStreamGenerator`: 一定 rate で mission を生成
  - `HelpInteractionSimulator`: HELP プロトコル（offer → accept/reject → execute → succeed/fail）を benevolence-biased でシミュレート
  - `TrustReputationRecomputeLoop`: tick ごとに trust/reputation を再計算（既存 + M1.76-5 の拡張）
  - `LifecycleGcLoop`: tick ごとに GC hazard を再計算（既存 + M1.76-6 の拡張）
  - `TickObserver`: 各 tick の状態（profiles, hazards, villages, helper assignments）を記録
  - `ReciprocitySimulationResult { metric_series, final_state, experiment_id }`
* **テストコードによる検証:**
  1. 同一 seed で 2 回 `run_simulation()` を実行し、全 tick の状態がビットレベルで一致すること（deterministic replay）
  2. `child_ratio = 0` のとき child-support 関連指標が全て 0 になること
  3. `max_ticks = 0` のとき空の metric series が返ること
  4. 善良（benevolence 高）な workflow 群と非善良群の生存率差が正であること（優しい世界の創発）
  5. `policy.lambda_gc_base = 0` で GC が一切発生しないこと
* **計装方法・観測対象:** 時系列メトリクス群（`benevolence_score_p50/p95`, `direct_reciprocity_p50/p95`, `indirect_reciprocity_p50/p95`, `reputation_final_p50/p95`, `benevolent_survival_advantage`, `harmful_gc_rate`）を tick ごとに収録する。善良群と非善良群の survival ratio 差を経時的に観測し、「優しい世界」が emergent に成立するまでの収束時間と定常状態を特徴づける。`gamma_benevolence` を sweep し、benevolence の survival 優位が出現する臨界強度を同定する。village churn、false-new rate、review-load への副作用も同時観測し、既存 metrics の悪化がないことを確認する。

#### ✅ チケット M1.76-18: 運用メトリクス観測パイプライン（Additional operational metrics）

* **対象不変条件 / 規範:** RFC §41B.20.7 Additional operational metrics。v2.3-e §41B.15 の metrics に加え、`benevolence_score_p50/p95`, `direct_reciprocity_p50/p95`, `indirect_reciprocity_p50/p95`, `reputation_final_p50/p95`, `benevolent_survival_advantage`, `harmful_gc_rate`, `helper_accept_rate`, `help_abandon_rate`, `child_survival_rate`, `ranking_flip_rate_under_small_patch`, `gc_hazard_drift_under_small_patch` の 11 指標を監視する。本チケットは RFC §41C.3 の全フェーズにまたがる横断的観測基盤である。
* **実装スコープ:**
  - `ReciprocityOperationalMetrics` 構造体: 11 指標 + 時系列データ（`Vec<f64>` per metric）
  - `compute_benevolent_survival_advantage(profiles: &[ReputationProfile], hazards: &[f32]) -> f64`: benevolence 上位 20% と下位 20% の survival ratio 差
  - `compute_harmful_gc_rate(events: &[ReciprocityEvent], gc_decisions: &[GCDecision]) -> f64`: harmful score 上位群の GC rate
  - `compute_helper_accept_rate(help_sessions: &[HelpSession]) -> f64`
  - `compute_help_abandon_rate(help_sessions: &[HelpSession]) -> f64`
  - `compute_child_survival_rate(profiles: &[ReputationProfile], maturity_states: &[WorkflowMaturity]) -> f64`
  - `ReciprocityMetricsObserver`: M1.76-17 の `TickObserver` に統合可能な observer hook
  - 全 metrics の時系列出力器（JSON Lines または CSV）
* **テストコードによる検証:**
  1. `benevolent_survival_advantage` が全 workflow 同一 benevolence のとき 0 になること
  2. `harmful_gc_rate` が harmful event 0 件のとき 0 になること
  3. `helper_accept_rate` が全 accept のとき 1.0、全 reject のとき 0.0 となること
  4. `child_survival_rate` が 0 child のとき 0 または適切な fallback 値になること
  5. 空データに対して各指標がパニックせず `0.0` または `f64::NAN` を明確に返すこと
* **計装方法・観測対象:** M1.76-17 の合成村シミュレーター上で全 11 指標を tick ごとに収録し、各指標の時系列プロット（p50/p95 帯域付き）を生成する。`benevolent_survival_advantage` の収束曲線、`harmful_gc_rate` の経時変化、`ranking_flip_rate_under_small_patch` と `gc_hazard_drift_under_small_patch` の摂動強度依存性を観測する。これらの指標を M1.75-7 の village metrics と結合し、既存 operational metrics（false-new rate / review-load / ranking stability）への副作用を監視するダッシュボード的観測基盤とする。

#### ✅ チケット M1.76-19: 較正フェーズ (Phase 0-4) 実装＋human-reviewed calibration rollout

* **対象不変条件 / 規範:** RFC §15.10.9 Calibration phases (Phase 0-4)、§41C.3 M4.x + M5.x（Kind World 拡張）、§15.9.1 MagnificentSevenParams、v2.3-g §12C Event Architecture calibration candidates。最終的な係数更新は human-reviewed でなければならない (MUST NOT auto-update to production)。rollout は canary environment policy から始める。Event Architecture の較正候補（`EVENTBUS_DEFAULT_TIMEOUT_MS` 等）は本較正ループの対象に含まれる。本チケットは RFC §41C.3 の **M4.x（human-reviewed calibration rollout）** の中核 + 全 Phase 統合に対応する。M5.x（Kind World 較正）は M1.76-KW1〜KW4 で個別実装され、本チケットの Phase 3 を目的関数 J_kw で拡張する。本チケットの Phase 3 は F-16（compute_calibration_objective）を一次目的関数として実装し、J_kw 対応は拡張ポイントとして設計する。
* **実装スコープ:**
  - `CalibrationPhase` 列挙型: `Phase0(PureFunctionValidation)`, `Phase1(DeterministicReplay)`, `Phase2(SmallPerturbation)`, `Phase3(SyntheticEcosystem)`, `Phase4(HumanReviewed)`
  - `PhaseGate` 構造体: 各 Phase の PASS/FAIL 状態を保持し、Phase 4 実行前に全先行 Phase の PASS をアサート
  - `Phase0Runner`: M1.76-3〜M1.76-10 の純粋関数（F-1〜F-15）に合成入力を直接与えて出力値域・単調性・非負性を検証（ユニットテスト再実行ではなく関数直接呼び出し）
  - `Phase1Runner`: M1.76-13 の replay 機構を使用し、5 シード（12345, 67890, 11111, 22222, 99999）でのビットレベル再現性を検証
  - `Phase2Runner`: M1.76-14 の perturbation suite を直接呼び出し、embedding noise・trust delta・usage increment の摂動に対する churn P95 / JSD bounds を検証
  - `Phase3Runner`: M1.76-17 の合成村シミュレーターを実行し、`simulation_result_to_operational_metrics()` 変換経由で `ReciprocityCalibrationHarness::evaluate()` に自動投入。sweep パラメータは MagnificentSevenParams（gamma_benevolence, lambda_gc_base, direct_reciprocity_weight, indirect_reciprocity_weight, softmax_temperature, gc_interval, child_ratio）を優先対象とする
  - `Phase4Runner`: 候補係数セット生成 → replay/simulation 評価 → `CalibrationRolloutReport` 生成 → human review queue 配送
  - `CalibrationRolloutReport { candidate_coefficients: Vec<HashMap<String,f64>>, evaluation_results: Vec<ReciprocityCalibrationResult>, diff_from_production: HashMap<String,(f64,f64)>, human_review_ticket: Option<String>, policy_version_update: Option<String> }`
  - `simulation_result_to_operational_metrics()`: `ReciprocitySimulationResult` → `ReciprocityOperationalMetrics`（6 成分）の変換関数
  - Human review queue 連携（M1-1 の `HumanReviewQueue` または `FakeHumanChannel` を使用）
  - canary environment policy 分離（段階的 rollout のための environment tag + `HashMap<String, String>` 環境別 policy version 管理）
  - Event Architecture 較正候補の sweep はスタブ対応（Phase 3 の ParameterRange リストに含めるのみ、評価はダミー値）
* **テストコードによる検証:**
  1. `Phase0Runner` の全検証がパスすること（全純粋関数の出力値域・単調性・非負性）
  2. `Phase0Runner` の境界値テスト（f64::MAX, 0.0, f64::MIN_POSITIVE 入力）が panic しないこと
  3. `Phase1Runner` の replay 検証が 5 シード全てでビットレベル一致すること
  4. `Phase1Runner` が policy version 変更時の不一致を検出できること
  5. `Phase2Runner` の perturbation 検証が全摂動軸で bounds 内であること
  6. `Phase3Runner` が seed 固定で再現可能な simulation sweep を出力すること
  7. `Phase3Runner` の OFAT sweep で MagnificentSevenParams 各パラメータの J(θ) 感度を観測できること
  8. `Phase3Runner` が `simulation_result_to_operational_metrics()` 経由で sweep 結果を自動評価できること
  9. `PhaseGate` が Phase 0-3 のいずれか FAIL 時に Phase 4 をブロックすること
  10. `Phase4Runner` の生成する差分レポートが human review queue へ配送可能であること
  11. auto-update が production へ即時反映されないこと（`MUST NOT` の実装確認）
  12. `CalibrationRolloutReport` に canary/production の環境別 policy version が含まれること
  13. 全 Phase 直列実行（Phase 0→1→2→3→4）が 1 サイクル完走すること
  14. 既存 1063 テストが全 PASS すること
* **計装方法・観測対象:** 各 Phase の実行時間、通過/不通過ステータス、検出された異常件数を記録する。Phase 0-3 の全検証通過が Phase 4 の候補係数生成の前提条件であることを `PhaseGate` でアサートする。Phase 3 の sweep 結果は CSV 形式で標準出力に書き出す（OFAT sweep の各パラメータ × J(θ) 応答曲面）。Phase 4 の human review ticket 生成から承認までのレイテンシを観測対象とし、policy version 更新履歴を系列管理する。canary → production の 2 段階 rollout の進行状態を環境別 policy version で監視する。

#### ✅ チケット M1.76-20: 実験レポート生成と系列管理の統合

* **対象不変条件 / 規範:** 既存の observational-testing / experiment-reporting discipline。全チケットは「コードが動くこと」ではなく、「観測可能な振る舞いが特徴づけられ、実験系列として記録されること」を完了条件とする。各実験に実験ID・親実験IDを付与し、全トレーサビリティを担保する。本チケットは RFC §41C.3 の全フェーズにまたがる横断的報告基盤である。
* **実装スコープ:**
  - `ReciprocityExperimentReport` 構造体: 全 20 チケットの実験結果を統合
  - `SimulationRunner` / `CalibrationHarness` の出力を統合する Markdown / JSON report writer
  - レポート必須セクション: replay trace 完全性、metrics summary、failing seeds、best-known parameter bundle、open anomalies、Phase 0-4 通過状況
  - 実験系列管理: 全実験実行に `exp-{yyyymmdd}-{seq}` ID を自動付与、親実験IDを記録
  - `rules/darvium/experiment-reporting.md` に準拠した report skeleton 適用
* **テストコードによる検証:**
  1. M1.76-3〜M1.76-18 の各実験結果が単一レポートへ欠落なく統合されること
  2. empty metrics や failure-only ケースでも壊れたレポートを出さず、必須フィールドを維持すること
  3. failing seed と golden trace 参照がレポート中で相互整合していること
  4. 実験ID の重複が発生しないこと（同一セッション内でユニーク保証）
* **計装方法・観測対象:** レポート生成自体の完全性を監視対象とし、各実験系列に対する missing field 率、未解決 anomaly の件数、best-known parameter bundle の更新履歴長を追跡する。実験系列の蓄積に伴う再現性、説明可能性、回帰検出感度の改善をメタ指標として観測し、reciprocity-awareness 実装が「導入された」だけでなく「観測と較正の対象として運用可能になった」ことを完了条件とする。

#### ✅ チケット M1.76-21: 外部イベント購読基盤 — `EventSubscriber` + `WebSocketEventChannel` モック実装

* **対象不変条件 / 規範:** RFC §12D External Event Subscription。EventChannel を介して外部システムからの DarviumEvent 購読・受信を可能にする。WebSocket 経由の購読は v2.3-g で新たに定義された機能であり、メモリ内モックでプロトコル検証を行う。購読したイベントは EventBus の `subscribe()` 経由で内部分配される (MUST)。
* **実装スコープ:**
  - `EventSubscriber` 構造体: `subscription_id`, `filter: EventFilter`, `channel: Box<dyn EventChannel>`, `status: SubscriberStatus`, `event_count: u64`
  - `SubscriberManager`: 購読の登録・解除・一覧を管理。`register(filter, channel) -> SubscriptionId`, `unregister(id)`, `list() -> Vec<EventSubscriber>`, `distribute(event: &DarviumEvent) -> Result<()>`
  - `FakeWebSocketEventChannel`: メモリ内バッファで WebSocket に相当する双方向通信を模倣。`EventChannel` トレイト実装。
  - `ExternalEventClient` トレイト: 外部システムからのイベント購読・受信を抽象化（`fn connect(&self, url) -> Result<Box<dyn EventChannel>>`, `fn disconnect(&self, id)`）
  - `FakeExternalEventClient`: 固定シードで購読イベント系列を生成するメモリ内モック実装
* **テストコードによる検証:**
  1. `SubscriberManager` への購読登録 → 該当フィルタ条件のイベント配送 → 購読解除の一連操作が一貫していること
  2. 複数の購読者が異なる `EventFilter` で特定の event_kind のみを受信すること
  3. `FakeWebSocketEventChannel` の `send` → `receive` ラウンドトリップ
  4. 購読解除後のイベント配送が行われないこと
  5. `ExternalEventClient` から受信したイベントが EventBus の `publish()` を経由して全購読者へ分配されること
* **計装方法・観測対象:** 購読者数 $n_{sub} \in [1, 100]$、イベント発行数 $n_{event} = 1000$ の条件で、各購読者の受信完全性（フィルタ条件に合致する全イベントの 100% 受信）を検証する。購読フィルタの精度（偽陽性率 0%、偽陰性率 0%）を計測する。

#### ✅ チケット M1.76-22: Event Architecture 運用メトリクス観測パイプライン統合

* **対象不変条件 / 規範:** M1.76-18 の運用メトリクス観測に加え、v2.3-g §12C Event Architecture の運用メトリクス（EventBus スループット、イベント消失率、クロック単調増加性、TwoWay 解決率、quarantine 率）を統合する。これらのメトリクスは既存の観測パイプラインと一貫した形式で収集されなければならない (MUST)。
* **実装スコープ:**
  - Event Architecture メトリクス構造体: `EventBusMetrics { total_published, total_clock_advances, two_way_opened, two_way_resolved, two_way_aborted, two_way_timeout, quarantine_count, replay_count, subscribe_count }`
  - `FakeEventBus` へのメトリクス収集 hook 追加（各メソッド呼び出し時にカウンタ更新）
  - `EventBusMetricsObserver`: 既存の `ReciprocityMetricsObserver` と統合可能な observer hook
  - Event Architecture メトリクスの時系列出力器
  - メトリクス補助監視指標: `event_throughput_per_clock_tick`, `two_way_resolution_rate`, `quarantine_ratio`
* **テストコードによる検証:**
  1. EventBus 操作（publish/open/resolve/abort）後に該当メトリクスカウンタが正確に増加すること
  2. `total_published` が実際の publish 呼び出し回数と一致すること
  3. `two_way_resolution_rate = resolved / opened` が全インタラクション完了後に 1.0 になること
  4. `quarantine_ratio` が quarantine 操作後に正しく計算されること
  5. メトリクス観測が EventBus の論理動作に影響を与えないこと（透過性）
* **計装方法・観測対象:** $n = 1000$ のランダム EventBus 操作系列における各メトリクスの経時変化を記録する。EventBus 操作とメトリクスカウンタの完全一致（1操作 = 1カウント）を検証する。メトリクス観測有無による EventBus のスループット差が統計的に有意でないことを確認する（t 検定、$p > 0.05$）。

#### ✅ チケット M1.76-23: 全ドメイン横断 Event Architecture 一貫性検証

* **対象不変条件 / 規範:** RFC §12C 全13種類の DarviumEventKind が、既存の全ドメイン（Search・Training・Conversational・Reciprocity・HELP・Lifecycle・GC・Repair・Fusion・HITL）において一貫した canonical envelope で publish されること。全ドメインイベントが EventBus 経由で統一された replay・subscribe・projection の対象となること。
* **実装スコープ:**
  - 全ドメイン種別ごとの `DarviumEvent` 生成ヘルパー関数 (`make_search_event`, `make_training_event` 等)
  - 全ドメイン EventKind の統合 replay テストシナリオ
  - ドメイン横断の EventBus 一貫性検証スイート:
    1. 全13種の event_kind が publish → replay で完全取得可能
    2. 全13種の event_kind が subscribe フィルタで正しく分別可能
    3. 全13種の event_kind が ProjectionCatalog 経由で正しく配送される
  - ドメイン間の event 相互汚染検出器（Search イベントが Training 領域に漏れていないことの検証）
* **テストコードによる検証:**
  1. 全13種の event_kind で publish → replay → kind 一致の確認
  2. 全13種の event_kind を混在 publish し、subscribe フィルタで各 kind のみを正しく受信できること
  3. 各 domain projection が自身の event_kind 以外のイベントを受け取らないこと（kind filter の完全性）
  4. 全13種のイベントが同一の `DarviumEventBus` を通じて一貫したクロック進行を示すこと
  5. 全13種のイベントの JSON シリアライズ/デシリアライズラウンドトリップが完全であること
* **計装方法・観測対象:** 13種の event_kind を各 $n = 100$ 件、計 1300 イベントをランダム順に publish し、replay 時の完全取得率、kind フィルタ精度、クロック単調増加性、projection 配送完全性を総合計測する。ドメイン横断の一貫性スコア（全指標の加重平均）を算出し、1.0 を完了条件とする。

#### ✅ チケット M1.76-KW1: Kind World 成立条件定数 + J_kw 目的関数実装

* **対象不変条件 / 規範:** RFC §15.9 SocialAcceleration（能力拡大速度・コスト減少の非線形性）、§41B.20.7 ExtendedOperationalMetrics、§41C.3 目的関数設計。Kind World の成立は「ワークフロー人口の継続的増加」「実務遂行能力カバー率の拡大」「再利用効率の向上」「単位コストの単調減少」「村の健全な形成と知識交換」「慈悲的集団の非慈悲的集団に対する優位」の 6 条件をすべて同時に満たすことで定義される。これらは F-16 の機構健全性とは独立した、エコシステム繁栄指標として設計しなければならない (MUST)。
* **実装スコープ:**
  - `constants.rs` に Kind World 条件ターゲット閾値を Safety Invariant として追加:
    - `KW_MIN_POPULATION_GROWTH_RATE: f64 = 0.01` — 最低人口成長率（1 tick あたり 1%）
    - `KW_MIN_CAPABILITY_COVERAGE_SHANNON: f64 = 0.5` — 最小 Shannon 多様性指数
    - `KW_MIN_REUSE_RATIO: f64 = 0.3` — 最低再利用比率
    - `KW_MAX_COST_EFFICIENCY_DECAY: f64 = 0.95` — コスト効率改善比の上限（1.0 未満で単調減少）
    - `KW_MIN_VILLAGE_FORMATION_SCORE: f64 = 0.3` — 最低村形成スコア
    - `KW_VILLAGE_CHURN_LOWER: f64 = 0.05` — 適切な村流動性下限
    - `KW_VILLAGE_CHURN_UPPER: f64 = 0.30` — 適切な村流動性上限
    - `KW_CROSS_VILLAGE_INTERACTION_MIN: f64 = 0.1` — 最小村間相互作用率
    - `VILLAGE_DISTANCE_THRESHOLD: f64 = 0.2` — 村所属判定の距離閾値（Calibration Candidate、感度分析推奨範囲 [0.1, 0.5]）
    - `VILLAGE_MIN_SIZE: usize = 3` — 最小村サイズ（Safety Invariant、3 未満の村はクラスタとみなさない）
  - `MagnificentSevenParams` 構造体 — 較正ループで sweep する 7 つの主要パラメータ:
    - `gamma_benevolence: f64` — 慈悲スコア重み。デフォルト 0.15、sweep 範囲 [0.0, 0.5]
    - `lambda_gc_base: f64` — GC ベースハザード。デフォルト 1.0、sweep 範囲 [0.1, 2.0]
    - `direct_reciprocity_weight: f64` — 直接互恵性重み。デフォルト 0.4、sweep 範囲 [0.1, 0.8]
    - `indirect_reciprocity_weight: f64` — 間接互恵性重み。デフォルト 0.3、sweep 範囲 [0.1, 0.8]
    - `softmax_temperature: f64` — ヘルパ選択のランダム性。デフォルト 0.5、sweep 範囲 [0.1, 1.0]
    - `gc_interval: u64` — GC 実行間隔（tick）。デフォルト 3、sweep 範囲 [1, 10]
    - `child_ratio: f64` — 子ワークフロー比率。デフォルト 0.3、sweep 範囲 [0.1, 0.5]
  - `KindWorldAssessment` 構造体: `is_kind_world: bool`, `flags: [bool; 8]`, `j_kw: f64`（8 測定閾値に対応する条件フラグと総合評価）。
    6 概念条件（人口増加・能力カバー率・再利用効率・コスト減少・村健全性・慈悲的優位）は 8 測定閾値（定数 8 個）に分解される。
  - `compute_kind_world_objective()` 純粋関数: $J_{kw}(\theta) = \alpha_1 J_{pop} + \alpha_2 J_{cov} + \alpha_3 J_{reuse} + \alpha_4 J_{cost} + \alpha_5 J_{village} + \alpha_6 J_{penalty}$
    - $J_{pop}$ = min(population_growth_rate / KW_MIN_POPULATION_GROWTH_RATE, 1.0)
    - $J_{cov}$ = min(capability_coverage / KW_MIN_CAPABILITY_COVERAGE_SHANNON, 1.0)
    - $J_{reuse}$ = min(reuse_ratio / KW_MIN_REUSE_RATIO, 1.0)
    - $J_{cost}$ = 1.0 - min(cost_efficiency / (1.0 - KW_MAX_COST_EFFICIENCY_DECAY), 1.0)
    - $J_{village}$ = compute_village_health_score(KW3 の 4 村指標) / 1.0
    - $J_{penalty}$ = max(0, -\Delta_{cov})，ただし $\Delta_{cov}$ = compute_benevolent_vs_non_benevolent_coverage_ratio() - 1.0。
      慈悲的集団の能力カバー率が非慈悲的集団を下回る（$\Delta_{cov} < 0$）場合のみ正値、上回る場合は 0。
  - 重み係数 $\alpha_i$ を constants.rs に Calibration Candidate として定義: `KW_ALPHA_POP: f64 = 0.25`, `KW_ALPHA_COV: f64 = 0.20`, `KW_ALPHA_REUSE: f64 = 0.15`, `KW_ALPHA_COST: f64 = 0.20`, `KW_ALPHA_VILLAGE: f64 = 0.10`, `KW_ALPHA_PENALTY: f64 = 0.10`
* **テストコードによる検証:**
  1. 全 8 条件フラグが成立（閾値超過）時に `is_kind_world == true` となること
  2. 全 8 条件フラグが不成立（閾値未満）時に `is_kind_world == false` となること
  3. $J_{kw}$ が $[0, 1]$ 範囲に収まること（NaN/Inf が一切出現しないこと）
  4. $J_{pop}$ の単調性: population_growth_rate 増加に伴い $J_{pop}$ が非減少であること
  5. 全重みの合計が 1.0 であること（$\sum \alpha_i = 1.0$ の静的アサート）
  6. 空入力（全 metrics が 0）に対して panic せず $J_{kw} = 0$ を返すこと
  7. 慈悲的集団の能力拡大速度が非慈悲的集団を下回る場合に $J_{penalty} > 0$ となること
  8. 慈悲的集団と非慈悲的集団の能力拡大速度が等しい場合に $J_{penalty} = 0$ となること
  9. 閾値境界値テスト：各指標が閾値の ±0.001 で成立/不成立が切り替わること
  10. `KindWorldAssessment` の JSON シリアライズ/デシリアライズラウンドトリップ
* **計装方法・観測対象:** 本チケットは純粋関数の実装のみを対象とし、シミュレーターとの統合は行わない。全関数の出力が $[0, 1]$ 範囲かつ NaN/Inf フリーであることを $n = 10000$ のランダム入力で検証する。目的関数の勾配方向が直感的期待と一致することを確認する（単調性検定）。

#### ✅ チケット M1.76-KW2: エコシステム成長メトリクス計装

* **対象不変条件 / 規範:** RFC §15.9 SocialAcceleration（トップレベル KPI）、§41B.20.7 ExtendedOperationalMetrics。エコシステムの成長は「人口増加」「能力カバー率拡大」「再利用促進」「コスト低減」の4次元で計測されなければならない (MUST)。これらのメトリクスは慈悲的集団と非慈悲的集団で層別集計され、比較可能でなければならない (MUST)。
* **実装スコープ:**
  - `EcosystemGrowthMetrics` 構造体: `tick: u64`, `population_growth_rate: f64`, `capability_coverage_shannon: f64`, `reuse_ratio: f64`, `cost_efficiency: f64`, `benevolent_vs_non_benevolent_coverage_ratio: f64`
  - `compute_population_growth_rate(population: &[SimWorkflowState], previous_count: usize) -> f64`: 人口成長率 = (current_count - previous_count) / max(previous_count, 1)。減少時は負値、増加時は正値。
  - `compute_capability_coverage_shannon(population: &[SimWorkflowState]) -> f64`: ワークフローの能力（position/experience の 2 次元空間）を 10×10 グリッドに量子化し、Shannon 多様性指数 $H = -\sum p_i \log p_i$ を計算。最大エントロピー $H_{\max} = \log(100)$ で除算して $[0, 1]$ に正規化。
  - `compute_reuse_ratio(events: &[ReciprocityEvent], sessions: &[SimHelpSession]) -> f64`: 同一 workflow が複数回ヘルプ提供または依頼を受けている割合。再利用回数 / 全インタラクション数。
  - `compute_cost_efficiency(sessions: &[SimHelpSession]) -> f64`: コスト効率改善度 = (失敗セッション数 + 放棄セッション数) / 全セッション数 を反転した値。1.0 に近いほど効率的。
  - `compute_benevolent_vs_non_benevolent_coverage_ratio(population: &[SimWorkflowState]) -> f64`: 慈悲的集団（上位 20%）の能力カバー率 / 非慈悲的集団（下位 20%）の能力カバー率。> 1.0 で慈悲的優位を示す。
  - `EcosystemGrowthObserver`: ReciprocityMetricsObserver と統合可能な observer。各 tick の SimulationTickSnapshot + population + sessions + events を入力として 4 指標を計算し、`ExtendedOperationalMetrics` に `ecosystem_growth: EcosystemGrowthMetrics` フィールドとして追加。
* **テストコードによる検証:**
  1. `compute_population_growth_rate`: 増加時正値 / 減少時負値 / 0 変動時 0.0 / 空人口時 0.0
  2. `compute_capability_coverage_shannon`: 全ワークフロー同一 position で 0.0 / 全ワークフロー均一分散で 1.0 に近い値 / 空 population で 0.0
  3. `compute_reuse_ratio`: 全セッションが異なる workflow 間で行われた場合 0.0 / 全セッションが同一 workflow の再利用で行われた場合 1.0
  4. `compute_cost_efficiency`: 全セッション成功で 1.0 / 全セッション失敗で 0.0 / 空セッションで 1.0（無入力の完全効率）
  5. `compute_benevolent_vs_non_benevolent_coverage_ratio`: 慈悲的・非慈悲的で能力分布が同一の場合 1.0 / 慈悲的の能力分布が広い場合 > 1.0
  6. 全 5 関数の空入力が panic せず 0.0 または 1.0（cost_efficiency のみ）を返すこと
  7. 全 5 関数の出力が $[0, 1]$ 範囲（population_growth_rate のみ範囲制約なし）かつ NaN/Inf フリーであること
  8. `EcosystemGrowthObserver` が `ExtendedOperationalMetrics` に正しい形式でデータを追加できること
  9. 慈悲的集団の能力カバー率が非慈悲的集団に対して統計的に有意に大きい場合に `benevolent_vs_non_benevolent_coverage_ratio > 1.0` となること
* **計装方法・観測対象:** 4 成長指標を各 tick で計算し、時系列変化を CSV 出力する。特に `benevolent_vs_non_benevolent_coverage_ratio` は Kind World の core signal として監視する。本チケットでは純粋関数の検証に留め、シミュレーター統合は M1.76-KW4 で実施する。出力範囲外の値や NaN/Inf が発生した場合は即座にテスト失敗とする。

#### ✅ チケット M1.76-KW3: 村間相互作用・知識拡散トラッキング

* **対象不変条件 / 規範:** RFC §15.9.4 村間相互作用指標、§41B.3 Child/adult/local village（村は「静的なクラスではなく、アダルトの導出近傍」）、§15.10.9 Phase 3 合成生態系、M1.75-7 村の安定性・動態指標。村は「空間的近接性に基づく自律的クラスタ」として形成され、村間の適切な相互作用と知識拡散がエコシステム全体の健全性の指標となる。村の形成強度が強すぎる（churn < 0.05、凝集過多）も弱すぎる（churn > 0.30、流動過多）も不健全とみなす。
* **設計方針:** RFC §41B.3 により、村は静的なクラスではなく位置から導出される近傍である。村IDは `SimWorkflowState` の永続フィールドではなく、`assign_village_ids` が tick ごとに返す一時的な割り当てラベルとして扱う。これにより村の動的性質を正確に反映し、位置変化に伴う村再編成を自然に表現する。
* **実装スコープ:**
  - `assign_village_ids(population: &[SimWorkflowState]) -> Vec<Option<usize>>`: ワークフローの position（2次元座標）に基づいて DBSCAN 類似の空間クラスタリングを実行し、各ワークフローに対応する一時的な村 ID 割り当てベクタを返す（`SimWorkflowState` には永続フィールドを追加しない）。最小村サイズは `constants.rs` の `VILLAGE_MIN_SIZE`、距離閾値は `VILLAGE_DISTANCE_THRESHOLD` で制御。この関数は純粋関数であり、population を変更しない。
  - `VillageInteractionMetrics` 構造体: `tick: u64`, `village_count: usize`, `cross_village_interaction_rate: f64`, `village_formation_strength: f64`, `knowledge_diffusion_rate: f64`, `village_flow_balance: f64`, `mean_village_size: f64`, `village_size_variance: f64`
  - `compute_cross_village_interaction_rate(sessions: &[SimHelpSession], village_assignments: &[Option<usize>]) -> f64`: 異なる村 ID 間で発生したヘルプセッションの割合 = 村間セッション数 / 全セッション数。各セッションの helper/requester に対応する `village_assignments` の村ラベルが異なる場合を「村間」としてカウントする。セッション数が 0 の場合は 0.0。
  - `compute_village_formation_strength(village_assignments: &[Option<usize>], population: &[SimWorkflowState]) -> f64`: silhouette 類似スコア。各ワークフローの position と所属村の重心との距離の逆数平均。$[0, 1]$ に正規化。村数 0（全員 None）の場合は 0.0。
  - `compute_knowledge_diffusion_rate(population: &[SimWorkflowState], current_assignments: &[Option<usize>], previous_assignments: &[Option<usize>]) -> f64`: 村間の知識（experience）分散の時間変化率。各村の平均 experience の標準偏差が時間とともに減少する速度 = 知識拡散速度。
  - `compute_village_flow_balance(current_assignments: &[Option<usize>], previous_assignments: &[Option<usize>]) -> f64`: 村の churn 率。村間を移動したワークフロー数 / 両 tick で生存かつ村所属のワークフロー数。$[0.05, 0.30]$ を適正範囲とし、範囲外はペナルティ対象。空割り当ての場合は 0.0。
  - `compute_village_health_score(formation_strength: f64, flow_balance: f64, cross_rate: f64, diffusion_rate: f64) -> f64`: 4 つの村指標を合成して $[0, 1]$ の総合健全性スコアを計算 = (formation_strength + flow_balance_health + cross_rate + diffusion_rate) / 4。flow_balance_health は churn が適正範囲 [KW_VILLAGE_CHURN_LOWER, KW_VILLAGE_CHURN_UPPER] 内なら 1.0、範囲外なら 0.0。この関数の出力は M1.76-KW1 の $J_{village}$ 成分として使用される。
  - `VillageInteractionObserver`: `EcosystemGrowthObserver` と統合可能。各 tick で `assign_village_ids` → 各 compute 関数 → `compute_village_health_score` の順で実行し、`VillageInteractionMetrics` を生成する。村割り当ての履歴（前 tick の assignments）を内部状態として保持する。
* **テストコードによる検証:**
  1. `assign_village_ids`: 空間的に密集したワークフロー群が同一村 ID を割り当てられること
  2. `assign_village_ids`: 孤立したワークフローが村未所属（`None`）になること
  3. `assign_village_ids`: 全ワークフローが同一位置の場合、単一村に全員所属すること
  4. `assign_village_ids`: 空 population で空ベクタを返すこと
  5. `compute_cross_village_interaction_rate`: 全セッションが同一村内の場合 0.0 / 全セッションが村間の場合 1.0 / 空セッションで 0.0 / village_assignments が空で 0.0
  6. `compute_village_formation_strength`: 全ワークフローが各村の重心に密接している場合 1.0 に近い値 / 各村内の分散が大きい場合 0.0 に近い値 / 全員 None で 0.0
  7. `compute_knowledge_diffusion_rate`: 各村の平均 experience が等しい場合 0.0（拡散完了）/ 乖離が大きい場合正値
  8. `compute_village_flow_balance`: churn 0 で 0.0 / 全員移動で 1.0 / 空 assignments で 0.0
  9. 空 population / 空 assignments の全関数が panic せず 0.0 を返すこと
  10. 村数 0（全員 None）の場合の graceful ハンドリング（`cross_village_interaction_rate = 0.0`, `village_formation_strength = 0.0`, `compute_knowledge_diffusion_rate = 0.0`）
  11. 既存の `SimWorkflowState` 生成テストが変更不要で通過すること（フィールド追加なし）= 後方互換性
  12. 村形成が強すぎる（churn < 0.05）場合のペナルティ計算が正しいこと
  13. 村形成が弱すぎる（churn > 0.30）場合のペナルティ計算が正しいこと
  14. 適正範囲（churn ∈ 0.05-0.30）でペナルティが 0 であること
* **計装方法・観測対象:** 各村のサイズ分布（平均・分散）、村間相互作用率、知識拡散速度の時系列を CSV 出力する。村形成強度が高すぎる（凝集・排他的）または低すぎる（流動的で共同体形成なし）場合を検出し、compute_village_health_score 経由で J_kw の J_village 成分に反映する。既存の M1.75-7 村指標（village_churn, helper_jsd）は変更せず、新規指標として追加する。全指標が $[0, 1]$ 範囲かつ NaN/Inf フリーであることを検証する。

#### ✅ チケット M1.76-KW-REAL-P1: SimulationContext 基盤

* **対象不変条件 / 規範:** RFC §4A.1 シミュレーション個人（5機構: WorkflowNode::AgentStep, SubWorkflow, WorkflowGraph, EdgeMeta, MemoizedGraph）、RFC §4A.2 位置・村（5機構中 SpacePositionEmbedding, 位置分解 41B-2）。個人は WorkflowGraph として表現されることを確認する。1 個の WorkflowGraph = 1 人の「人」であり、以後この解釈を全チケットで不変とする (MUST NOT reinterpret)。

* **背景:** 本チケットは M1.76-KW-REAL シリーズ 6 チケットの第 1 弾であり、後続 5 チケットすべての基盤となる。既存の `simulation.rs` は flat struct `SimWorkflowState` を使用しており、実際の Darvium の個人表現（MemoizedGraph）と無関係な「おもちゃのモデル」であった。本チケットはこの構造を実際の Darvium 部品で置き換える。

  **「シミュレーションはツールであって目的ではない」** — 本シミュレーション基盤は社会加速度理論の数学的検証のための実験装置であり、それ自体が目的ではない。既存の 57 機構の監査結果（🟢REAL 37 / 🟡PARTIAL 5 / 🔴MISSING 13、2026-05-26 時点）に基づき、以下の 3 原則を厳守する：
  1. **存在するものは本物の部品をそのまま使う**: 既存実装を直接呼び出す。コピーもラップも独自再実装も禁止。
  2. **存在しないものは abstract 実装とし、将来の置換を保証する**: trait で抽象化し、後日本物の実装ができ次第差し替え可能にする。
  3. **理論検証に必要な範囲に限定する**: J_kw の 5 因子乗算結合モデル（RFC §15.9.2: S_viab × S_capa × S_coop × S_effi × S_fair）の 14 下位成分の算出に直接関係するものだけを実装範囲とする。6 成分加重和から 5 因子乗算結合への移行に伴い、新たに必要な 8 指標（mean_lifecycle_score, child_survival_rate, mean_freshness, mean_benevolence_aggregate, mean_reciprocity_score, help_success_rate, trust_inheritance_fidelity, execution_success_rate）の収集インターフェースを P6 で定義する。

* **実装スコープ:**
  - `SimWorkflowState` を `SimulationContext` で置き換え: 現行の `simulation.rs` にある flat struct（69-95行）を、実際の `MemoizedGraph`（trust.rs:23-36）をラップする `SimulationContext` に置き換える。
    ```rust
    pub struct SimulationContext<'a> {
        pub memoized_graph: &'a mut MemoizedGraph,
        pub trust_profiles: HashMap<NodeId, TrustProfile>,
        pub village_assignments: HashMap<NodeId, VillageAssignment>,
        pub positions: HashMap<NodeId, SpacePositionEmbedding>,
        pub tick: u64,
        pub rng: StdRng,
    }
    ```
  - `MemoizedGraph` の全ノード = person エンティティ。ノード数 = 人口。
  - 新規ノード追加（出生）は `MemoizedGraph` に `WorkflowNode::SubWorkflow` として追加。ノード削除（死亡）は GC lifecycle を経由するが、本チケットでは削除インターフェースのみ定義し、実際の GC 制御は P5 で実装する。
  - 位置分解（RFC §41B-2）: `spaceposition.rs` の `decompose_position` を完成させ、各次元成分への分解を実装する。既存の `update_space_position`（spaceposition.rs:108）および `l2_distance`（spaceposition.rs:144）はそのまま流用。
  - ノード ID 生成: 出生時に一意の `NodeId` を生成する関数。`NodeId` は既存型をそのまま使用。
  - `SimulationContext` に `help_sessions: Vec<HelpSession>` フィールドを追加（P4 で使用するが、P1 では構造体定義のみ）。

* **依存関係:** なし。本チケットが KW-REAL シリーズの最初の実装単位である。既存の全テストが本変更後も PASS することを確認する。

* **テストコードによる検証:**
  1. `SimulationContext` が正しく生成され、初期ノード数が指定通りであること
  2. ノード追加（出生）が `MemoizedGraph` に新しい `WorkflowNode::SubWorkflow` を追加すること
  3. ノード削除が `MemoizedGraph` から指定ノードを削除すること
  4. 位置分解が正しく各次元に分解されること（既存位置更新テストを流用・拡張）
  5. `NodeId` 生成が毎回一意の ID を返すこと
  6. 後方互換性: 既存の KindWorldMetricsInput / EcosystemGrowthObserver / VillageInteractionObserver のテストが本変更後も全 PASS すること

* **計装方法・観測対象:** `SimulationContext` 生成時の初期ノード数・初期位置分布を CSV 出力。ノード追加・削除の操作をログ出力（操作種別, NodeId, tick）。位置分解の各次元値を JSON 出力。既存の `SimWorkflowState` 使用箇所をすべて洗い出し、置き換え漏れ確認のためのカバレッジ計装を含める。

#### ✅ チケット M1.76-KW-REAL-P5: ライフサイクル・成熟機構

* **対象不変条件 / 規範:** RFC §4A.7 ライフサイクル・成熟（8機構: LifecycleScore, 5状態GC機械, GC Interval, Child Protection F-10, Minimum Survival Experience, experience_count, Child Growth F-14, Maturation Probability F-15）、RFC §4A.8 信頼・継承（2機構: Trust Inheritance, Reputation Inheritance）、RFC §4A.9 時間・鮮度（2機構: 二軸時間, BlendedFreshness F_time）。P4（6フェーズループ）の GC 処理の前提となる。本チケット完了前に P4 を実装してはならない (MUST)。

* **背景:** 本チケットは M1.76-KW-REAL シリーズ 6 チケットの第 2 弾であり、P1（SimulationContext）完了後に実装する。P4（6 フェーズループ）の GC 処理（フェーズ 4）で使用される全機構を提供する。監査の結果、LifecycleScore は未実装（simulation.rs に簡略 inline 実装のみ）、GC 状態機械は 3/5 状態のみ実装（Protected と Active が欠落）、信頼継承・評判継承・BlendedFreshness は未実装である。**「シミュレーションはツールであって目的ではない」** — 不足機構は trait で抽象化し、将来の本実装に置き換え可能にする。理論検証（J_kw への影響確認）に必要な最小限の実装に留める。

* **実装スコープ:**
  - **子供・成人定義（RFC §41B-3, 41B-4）**: `experience_count` に基づく子供/成人の判定。`fn classify_maturity(experience_count: u64) -> Maturity`。`experience_count < CHILD_MATURITY_THRESHOLD` で `is_child = true`。`enum Maturity { Child, Adult }`。
  - **`MIN_SURVIVAL_EXPERIENCE` 定数**: constants.rs に定義。F-10（Child Protection）が参照する閾値。`experience_count < MIN_SURVIVAL_EXPERIENCE` の個人は GC 削除から完全保護。default: `3`。
  - `LifecycleScore` 構造体（RFC §41C）: `kind_world.rs` に正式定義。freshness, success, trust, usage, reputation の幾何平均として計算。
  - GC 5状態機械の完全実装: 現行の `event.rs` の `GcEvent`（3 variant: SoftDeleted, HardDeleteCandidate, Tombstoned）に Protected, Active を追加。状態遷移関数 `fn transition_gc_state(current: GcEvent, hazard: f64) -> GcEvent`。遷移: Protected→Active（経験値達成）、Active→SoftDeleted（hazard 超過）、SoftDeleted→HardDeleteCandidate（猶予経過）、HardDeleteCandidate→Tombstoned（完全削除）。
  - 信頼継承: `trust.rs` に `fn inherit_trust(parent: &TrustProfile, child: &mut TrustProfile, decay: f64)`。
  - 評判継承: `trust.rs` に `fn inherit_reputation(parent: &ReputationProfile, child: &mut ReputationProfile, decay: f64)`。
  - `ExperienceNormalization`（F-5）: `reciprocity.rs` に `compute_experience_normalization`。非線形正規化（初期の急成長、成熟後の飽和）。
  - **二軸時間管理（RFC §4A.9）**: 既存の `clock/mod.rs`（ManualClock / SystemClock / FrozenClock）を SimulationContext 内で保持。シミュレーション tick を Virtual Time、UTC を Human Time として二軸管理。
  - `BlendedFreshness`（F_time, RFC §8.2）: `clock/mod.rs` に `fn compute(&self, last_access: Instant, virtual_ticks: u64) -> f64`。Human Time と Virtual Time の混合重みで Freshness を計算。

* **依存関係:** P1（SimulationContext）完了後に実装する。本チケットは P4 の前提条件であり、P4 を開始する前に完了しなければならない。

* **テストコードによる検証:**
  1. `classify_maturity` が経験値 0 で `Child`、閾値以上で `Adult` を返すこと
  2. GC 5状態遷移が Protected→Active→SoftDeleted→HardDeleteCandidate→Tombstoned の順序と各遷移条件を満たすこと
  3. Protected 状態の個人は hazard が高くても Tombstoned に遷移しないこと（安全機構）
  4. `inherit_trust` / `inherit_reputation` が減衰係数 0.0 で親と同じ値を、1.0 で 0 を返すこと
  5. `compute_experience_normalization` が経験値 0 で 0.0、大規模値で 1.0 に漸近すること
  6. `BlendedFreshness` が経過時間 0 で 1.0、経過時間大で 0.0 に漸近すること
  7. 既存の compute_gc_hazard / compute_child_protection / compute_survival_probability のテストが全 PASS すること

* **計装方法・観測対象:** 各個人の LifecycleScore 成分を CSV 出力（tick, node_id, survival_probability, gc_hazard, maturation_probability, is_protected, maturity）。GC 状態遷移イベントをログ出力（遷移元→遷移先, node_id, tick）。経験値分布のヒストグラムを 10 tick ごとに JSON 出力。

#### ✅ チケット M1.76-KW-REAL-P4: 6 フェーズシミュレーションループ

* **対象不変条件 / 規範:** RFC §4A.5 HELP 相互支援（8機構: Proposal→Offer→Decision→Execution→Success + F-11/F-12/F-13）、RFC §4A.6 互恵性・生存（9機構: F-1〜F-4, F-7〜F-9 + ReciprocityScore 構造）。P1 の SimulationContext、P5 の GC 5状態機械を駆動するメインループ。

* **背景:** 本チケットは M1.76-KW-REAL シリーズ 6 チケットの第 3 弾であり、P1 + P5 完了後に実装する。KW-REAL シリーズの中核であり、実際の Darvium 部品を駆動する 6 フェーズ tick ループを実装する。**「シミュレーションはツールであって目的ではない」** — 本ループは Kind World 成立条件の探索のための実験装置であり、それ自体が製品ではない。実装は実際の Darvium 部品（help.rs, reciprocity.rs）を「本物のまま」呼び出すことに集中する。P2（GMR抽象化）と P3（実行抽象化）は未完成でも構わない。該当フェーズではスタブを呼び出し、tick ループ全体の動作検証を先行させる段階的アプローチを許可する。

* **実装スコープ:**
  - 既存の `simulation.rs` の `run_simulation` を完全書き換え、以下の 6 フェーズ tick ループ：
    1. **人口成長**: P2 の SubWorkflow/NEW/COMPOSE/Differential Inference による新ノード生成。`child_ratio` に従い既存ノードから子ノードを生成。**P2 未完成時**: `child_ratio` 確率で既存ノードを WorkflowNode 単位で複製するスタブで代用。
    2. **位置更新 + 村クラスタリング**: `update_space_position`（既存 real）→ `build_local_village_radius` / `build_local_village_topk`（既存 real）。P5 の `classify_maturity` で成人のみを村アンカー候補とする。
    3. **HELP プロトコル**: 実際の `help.rs` の `should_offer_help`（445行） / `decide_help_offer`（493行） / `HelpSession::new`（247行）を直接呼び出し。HELP セッションは複数 tick にまたがる（Proposal→Offer→Decision→Execution→Success の 5 段階状態遷移）。各 tick ではアクティブな全 HelpSession を `advance_help_sessions`（既存）で進め、新規 Proposal は `offer_help_probability` に従い生成。`help.rs` の既存実装を一切変更せず外部から呼び出す。
    4. **互恵性計算 → GC hazard → 生存**: 実際の `reciprocity.rs`（F-1〜F-4, F-7〜F-10, F-14, F-15）を直接呼び出し。`compute_gc_hazard`（286行）→ `compute_survival_probability`（329行）→ P5 の GC lifecycle 遷移。**GC フェーズは `gc_interval` の周期でのみ実行**: `tick % gc_interval == 0` のときのみ GC 一連処理を実行、それ以外ではスキップ。
    5. **能力拡散**: P2 の `DifferentialInference` で Workflow パターンを拡散。HELP 成功時に helper の知識を helpee に伝播。**P2 未完成時**: HELP 成功時に helper の AgentStep を 1 つ helpee にコピーする単純スタブで代用。
    6. **J_kw 測定**: P6 の `collect_final_metrics` → `compute_kind_world_objective`。最終 tick でのみ実行。
  - 各 tick は上記 6 フェーズを逐次実行（フェーズ間に暗黙依存関係のため並列不可）。
  - 固定シード `StdRng::seed_from_u64(12345)` を全実行で使用。

* **依存関係:**
  - **必須**: P1（SimulationContext）が完了していること
  - **必須**: P5 のうち GC 5状態機械 + 子供/成人定義 + MIN_SURVIVAL_EXPERIENCE が完了していること
  - **スタブ可**: P2（GMR）と P3（実行抽象化）は未完成でも tick ループの動作検証を開始可能
  - **非依存**: P6（計装更新）は未完了でも println! で代用可能

* **テストコードによる検証:**
  1. 6 フェーズ tick ループが 1 tick 以上を完走すること（複数 tick の進行確認）
  2. HELP プロトコルが `help.rs` の `should_offer_help` を実際に呼び出していること（モックでなく本物の呼出し確認）
  3. GC が `gc_interval` の周期でのみ実行されること（`tick % gc_interval != 0` では GC 関連関数が呼ばれない）
  4. 村クラスタリングが既存の `build_local_village_radius` を呼び出していること
  5. 全 6 フェーズを 100 tick 実行しても panic せず完了すること（耐久テスト）
  6. 固定シード実行で結果が完全再現すること（同一 seed で 2 回実行し同一 J_kw）
  7. 異なる `child_ratio` で最終人口が変化すること（パラメータ感受性の確認）
  8. スタブモード（P2/P3 未完成）と本実装モードの両方で動作すること

* **計装方法・観測対象:** 各 tick の各フェーズ実行回数を CSV 出力（tick, phase1_births, phase2_villages, phase3_proposals, phase3_successes, phase4_gc_events, phase5_diffusions）。HELP 発動回数・成功率・平均セッション長を時系列観測。村の形成・解散イベントを記録。100 tick ごとに SimulationContext スナップショットを JSON 出力。

#### ✅ チケット M1.76-KW-REAL-P2: GMR 抽象化層

* **対象不変条件 / 規範:** RFC §4A.3 GMR・能力拡張（8機構: ハードゲート AG-01〜AG-07, DeterminismScore, ApplicabilityScore, Stage5分岐, COMPOSE, NEW, Differential Inference, GraphPatch）。AG-06（Semantic Channel）と AG-07（Structural Proxy Channel）は既存実装（search/applicability.rs）をそのまま流用。AG-01〜AG-05、DeterminismScore、Stage5 分岐、COMPOSE、NEW、Differential Inference は abstract 実装。

* **背景:** 本チケットは M1.76-KW-REAL シリーズ 6 チケットの第 4 弾であり、P4 の人口成長フェーズ（フェーズ 1）と能力拡散フェーズ（フェーズ 5）で使用される GMR 機構を実装する。監査の結果、AG-06/AG-07 は 🟢 REAL、AG-01〜AG-05 / DeterminismScore / Stage5 分岐 / COMPOSE / NEW / Differential Inference は 🔴 MISSING。**「シミュレーションはツールであって目的ではない」** — 不足機構は trait で抽象化し、将来の本実装（ANN 検索パイプライン等）に置き換え可能にする。シミュレーション用の簡略化された代用実装で理論検証を可能にする。

* **実装スコープ:**
  - `DeterminismScore` 構造体（RFC §24）: `fn compute(&self, workflow: &WorkflowGraph) -> f64`。各 AgentStep の determinism 値の SoftMin 合成。シミュレーション内では determinism フィールドの平均値で代用。
  - `ApplicabilityScore` 構造体: AG-01〜AG-05 を abstract 実装（AG-06/AG-07 は既存流用）:
    - AG-01 RewardSignalChannel: 履歴成功率で代用
    - AG-02 UtilityChannel: 期待効用で代用
    - AG-03 NoveltyChannel: Embedding 間コサイン距離で代用
    - AG-04 UrgencyChannel: デッドライン残り tick 数で代用
    - AG-05 SafetyChannel: リスクスコアで代用
  - `Stage5Decision` 構造体（RFC §24）: `fn decide(candidate: &ApplicabilityOutcome) -> Stage5Branch`。5 方向分岐（REUSE / PATCH / COMPOSE / NEW / ABORT）をスコアベースの確率的選択で決定。
    ```rust
    pub enum Stage5Branch { Reuse, Patch, Compose, New, Abort }
    ```
  - `compose_workflows` 関数（composition.rs）: `fn compose(a: &WorkflowGraph, b: &WorkflowGraph) -> WorkflowGraph`。2 つの WorkflowGraph のノードを統合し、共通部分を結合。
  - NEW 機構: `fn new_workflow_from(seed: &WorkflowGraph, rng: &mut StdRng) -> WorkflowGraph`。既存 WorkflowGraph に微小変異を加えて新規生成。
  - `DifferentialInference` 構造体: `fn infer(&self, source: &WorkflowGraph, target: &mut WorkflowGraph, rng: &mut StdRng) -> Vec<GraphPatch>`。不足 AgentStep を特定し `GraphPatch`（patch.rs:102, real）として差分生成。`apply_patch_atomic`（patch.rs:273, real）で適用。
  - 全 abstract 実装に trait 定義:
    ```rust
    pub trait ApplicabilityChannel { fn score(&self, candidate: &ApplicabilityCandidate) -> f64; }
    pub trait CapabilityGenerator { fn generate(&self, seed: &WorkflowGraph, rng: &mut StdRng) -> WorkflowGraph; }
    ```

* **依存関係:** P1 の型定義を使用するが、P1 完了を待たず独立開発可能。P4 はスタブモードで動作可能なため実装順序の制約なし。

* **テストコードによる検証:**
  1. `DeterminismScore::compute` が全 determinism = 1.0 で 1.0、全 0.0 で 0.0 を返すこと
  2. AG-01〜AG-05 の各チャネルが $[0, 1]$ 範囲のスコアを返すこと
  3. `Stage5Decision::decide` が高スコア候補に REUSE/COMPOSE を、低スコアに ABORT を割り当てること
  4. `compose_workflows` が 2 つの WorkflowGraph を正しく統合すること
  5. NEW 機構で生成された WorkflowGraph が seed と同一構造ではないこと
  6. `DifferentialInference::infer` が生成する GraphPatch が `apply_patch_atomic` で適用可能であること
  7. 既存の `search/applicability.rs` のテストが全 PASS すること

* **計装方法・観測対象:** 各 AG チャネルのスコア分布を JSON 出力。Stage5 分岐の選択確率を集計（REUSE/PATCH/COMPOSE/NEW/ABORT の割合）。GraphPatch のサイズ分布を観測。

#### ✅ チケット M1.76-KW-REAL-P3: ワークフロー実行抽象化

* **対象不変条件 / 規範:** RFC §4A.4 ワークフロー実行（3機構: compile_to_steps, SideEffectSet, ErrorMode）。WorkflowGraph（DAG）を実行可能な step list に変換し、各 step の実行結果を管理する。SideEffectSet は既存実装（types.rs:4658-4689, 🟢 REAL）をそのまま流用。

* **背景:** 本チケットは M1.76-KW-REAL シリーズ 6 チケットの第 5 弾であり、シミュレーション内で個人（WorkflowGraph）を「実行可能にする」変換機構を提供する。監査の結果、SideEffectSet は 🟢 REAL、compile_to_steps と ErrorMode は 🔴 MISSING。**「シミュレーションはツールであって目的ではない」** — compile_to_steps は単なるトポロジカルソートであり、将来の本物のコンパイラ（IL 生成等）で置き換える trait として定義する。SideEffectSet は既存の本物の型をそのまま流用し、新規型を作らない。

* **実装スコープ:**
  - `compile_to_steps` 関数: `fn compile_to_steps(graph: &WorkflowGraph) -> Result<Vec<NodeId>, CycleDetectedError>`。petgraph の `toposort` を使用。循環依存を検出したらエラーを返す。
  - `SideEffectSet`（RFC §12）: `SimulationContext` 内で保持。既存実装（types.rs:4658-4689）をそのまま使用。外部 API 呼出し等の副作用は模擬（宣言のみ記録、実際の呼出しは行わない）。
  - `ErrorMode` 列挙型（RFC §7A/§8.3）:
    ```rust
    pub enum ErrorMode { FailOnAny, SkipOnError, Degrade, RetryOnError(u32) }
    ```
  - `StepExecutionResult` 構造体:
    ```rust
    pub struct StepExecutionResult {
        pub node_id: NodeId,
        pub status: StepStatus,
        pub output: Option<String>,
        pub error: Option<String>,
        pub duration_ticks: u64,
    }
    pub enum StepStatus { Success, Failure, PartialSuccess, Skipped }
    ```

* **依存関係:** P1 の型定義（NodeId, WorkflowGraph）を必要とするが、P1 完了を待たず独立開発可能。P4 はスタブモードで動作可能なため実装順序の制約なし。

* **テストコードによる検証:**
  1. `compile_to_steps` が線形 DAG を正しい順序の step list に変換すること
  2. 分岐 DAG（FanOut + Collect）も正しくトポロジカルソートすること
  3. 循環依存 DAG に対して `CycleDetectedError` を返すこと
  4. `ErrorMode::FailOnAny` で step 失敗時に即座にエラーを返すこと
  5. `ErrorMode::SkipOnError` で失敗 step をスキップして続行すること
  6. `ErrorMode::RetryOnError(3)` で最大 3 回リトライすること
  7. 既存の GraphPatch / apply_patch_atomic / validate_patch_result のテストが全 PASS すること

* **計装方法・観測対象:** compile_to_steps の変換結果を出力（ノード数, エッジ数, step list 長, 循環依存の有無）。StepExecutionResult の status 分布（Success/Failure/PartialSuccess/Skipped の割合）を集計。

#### チケット M1.76-KW-REAL-P6: 計装インターフェース更新

* **対象不変条件 / 規範:** RFC §4A.10 J_kw 社会加速度測定（7機構: J_kw目的関数, 5因子最小値ゲート, S_viability, S_capability, S_cooperation, S_efficiency, S_fairness）、RFC §15.9.2（5因子乗算結合モデル）、RFC §15.9.3（14指標のエコシステム成長指標）。RFC の 5 因子モデルへの改訂に伴い、KindWorldMetricsInput に 8 フィールドを追加し、compute_kind_world_objective を旧 6 成分加重和から新 5 因子乗算結合に書き換える。新旧両方の方式で J_kw を計算し比較する互換性診断を実装する。

* **背景:** 本チケットは M1.76-KW-REAL シリーズ 6 チケットの最終（第 6 弾）であり、P4（6 フェーズループ）完了後に実装する。P1 で導入した `SimulationContext` を既存の計装関数が受け取れるようインターフェースを更新する。**「シミュレーションはツールであって目的ではない」** — 5 因子乗算結合モデルへの改訂は J_kw の数学的完全性のための変更であり、シミュレーション基盤そのものの目的化を防ぐための措置である。新旧 J_kw の比較診断出力を実装し、5 因子モデルの挙動が既存知見と矛盾しないことを検証する。

* **実装スコープ:**
  - `KindWorldMetricsInput` に以下 8 フィールドを追加:
    - `mean_lifecycle_score: f64` [0,1]
    - `child_survival_rate: f64` [0,1]
    - `mean_freshness: f64` [0,1]
    - `mean_benevolence_aggregate: f64` [0,1]
    - `mean_reciprocity_score: f64` [0,1]
    - `help_success_rate: f64` [0,1]
    - `trust_inheritance_fidelity: f64` [0,1]
    - `execution_success_rate: f64` [0,1]
  - `compute_kind_world_objective` を旧 6 成分加重和から新 5 因子乗算結合（RFC §15.9.2）に書き換え。出力は `KindWorldAssessment` に 5 因子値（S_viab, S_capa, S_coop, S_effi, S_fair）および 14 下位成分値を追加。
  - `collect_final_metrics`: 引数型を `ReciprocitySimulationResult` → `SimulationContext` に変更。内部で以下を抽出:
    - 人口 → memoized_graph.graph.node_count()
    - 各村サイズ → village_assignments から集計
    - 能力カバレッジ → positions の分散から計算
    - HELP 統計 → help_sessions 履歴から集計
    - GC 統計 → GC 状態分布から集計
    - 新 8 指標 → SimulationContext の各フィールドから収集
  - `EcosystemGrowthObserver::observe`: SimulationContext を受け取れるよう新規メソッド追加。
  - `VillageInteractionObserver::observe`: SimulationContext を受け取れるよう新規メソッド追加。
  - `KindWorldAssessment.flags`: 旧 8 二値フラグに代わり 5 因子最小値ゲートの成立状態を出力。旧 8 フラグは diagnostics として別途出力（後方互換性のため構造体からは削除せず optional 化）。
  - 既存 observer の既存フィールド・既存メソッドシグネチャは削除しない。新規メソッドを追加する形で対応。
  - **互換性診断関数** `compare_j_kw_models(old_metrics: &OldMetrics, new_metrics: &KindWorldMetricsInput) -> JkwModelComparison` を実装。新旧両方の J_kw を計算し、差分・順位相関・各成分寄与率を比較する。`#[cfg(test)]` で隔離し、通常ビルドには含めない。

* **依存関係:** P4（6 フェーズループ）完了後に実装する。P4 で SimulationContext に追加される全フィールドを metrics に含める必要があるため、P4 より先に実装してはならない (MUST NOT)。

* **テストコードによる検証:**
  1. `collect_final_metrics` が SimulationContext から正しく metrics を抽出すること
  2. `KindWorldMetricsInput` の 16 フィールド（旧 8 + 新 8）が全て正しく設定されること
  3. `compute_kind_world_objective` が 5 因子乗算結合を正しく計算すること（5 因子各値が [0,1] 範囲）
  4. 5 因子乗算結合で、1 因子を 0.0 に設定した場合に J_kw = 0 となること（マスキング防止の確認）
  5. 新旧 observer の出力が一致すること（後方互換性）
  6. 互換性診断 `compare_j_kw_models` が新旧 J_kw の差分を正しく報告すること
  7. 全取得 metrics が $[0, 1]$ 範囲かつ NaN/Inf フリーであること
  8. 既存の compute_kind_world_objective（旧 6 成分）テストが廃止後も互換性診断として参照可能であること

* **計装方法・観測対象:** collect_final_metrics の出力結果を JSON 出力（全 14 下位成分 + 5 因子値 + J_kw）。新旧モデル比較診断の結果を CSV 出力（旧 6 成分 J_kw, 新 5 因子 J_kw, 各成分の差分率）。旧 8 二値フラグと新 5 因子最小値ゲートの成立状況を比較出力。各 metrics 成分の値を tick 別に時系列出力。


#### チケット M1.76-KW4: Kind World 較正ループ実行

* **対象不変条件 / 規範:** RFC §15.9.2（5 因子乗算結合モデル）、§15.10.9 Calibration phases (Phase 3-4)、§41C.3 M4.x。本チケットは M1.76-KW-REAL（P1〜P6）で構築した「本物の Darvium 部品で駆動するシミュレーション」上で、Nelder-Mead 最適化による自動較正（内側ループ）と、AI による結果解釈・定数調整（外側ループ）の二重ループを実装する。目的関数は 5 因子乗算結合 $J_{kw}(\theta) = S_{viab} \times S_{capa} \times S_{coop} \times S_{effi} \times S_{fair}$ (RFC §15.9.2)。Kind World 達成条件は $J_{kw} > 0.8 \land \min(S_i) > 0.6$（旧 8 二値フラグに代わる 5 因子最小値ゲート）。最終的な係数更新は human-reviewed でなければならない (MUST NOT auto-update to production)。

* **背景:** 本チケットは KW-REAL シリーズ（P1〜P6）の完了後に実装する。KW-REAL は 57 機構を実際の Darvium 部品で駆動するシミュレーション基盤を提供し、本チケットはその上で較正ループを実行する。**「シミュレーションはツールであって目的ではない」** — 較正ループは J_kw 最大化のための実験装置であり、較正そのものが目的化してはならない。以下の点に留意する：(1) 得られた最適パラメータは simulation.rs 上の値であり、本番 Darvium 定数に直接反映してはならない（human review 必須）、(2) 内側ループの Nelder-Mead は「探索の道具」であって「解を保証するもの」ではない — 収束しなかった場合は探索範囲の設計が誤っている可能性を示唆する、(3) 外側ループは 24 サイクルで必ず打ち切り、未収束のままでも中間結果を Human review queue に配送する。

* **実装スコープ:**

  **内側ループ（自動最適化 — Nelder-Mead 直接探索）:**
  - **依存関係**: M1.76-KW-REAL-P1〜P6 全チケット完了後に実装する。特に KW-REAL-P4（6 フェーズループ）の SimulationContext と KW-REAL-P6（計装更新）の `collect_final_metrics` を評価関数の入力として使用する。
  - `NelderMeadOptimizer` 構造体: 7 次元（MagnificentSevenParams）の Nelder-Mead 最適化器。既存実装（kind_world.rs:1307以降）のアルゴリズム核（反射・拡大・収縮・縮小の各操作）は流用するが、`evaluate` 関数のインターフェースは KW-REAL の `SimulationContext` に対応するよう書き換える。
    - `fn new(params: &MagnificentSevenParams, ranges: &[(f64, f64); 7]) -> Self`
    - `fn run(&mut self, max_iterations: usize) -> OptimizationReport`
    - 内部で `evaluate(params) -> f64` を呼び出し。`evaluate` は KW-REAL の 6 フェーズシミュレーションを 1 回実行し $J_{kw}$ を返す。
  - `OptimizationReport` 構造体: `best_params`, `best_j_kw`, `assessment`, `iterations`, `history`, `converged`, `experiment_id`
  - $J_{kw}$ 評価フロー（KW-REAL 上）:
    1. `SimulationContext::new(memoized_graph, config, rng)` で初期化
    2. 6 フェーズ tick ループを `KW4_SIMULATION_TICKS` 回実行
    3. `collect_final_metrics(&context)` で metrics 収集
    4. `compute_kind_world_objective(&metrics)` で $J_{kw}$ 計算
  - 探索範囲定数（constants.rs）: 7 パラメータ各々に (min, max) を定義
  - 収束条件: シンプレックス頂点間の $J_{kw}$ 分散 < $1 \times 10^{-6}$ または最大 200 iteration

  **外側ループ（実験者主導 — AI 解釈サイクル）:**
  - 内側ループの結果（OptimizationReport）を解釈、探索範囲や定数を調整
  - 1 サイクル = 「定数調整 → `cargo test`（内側ループ実行）→ 結果記録」
  - 8 サイクルごとに中間報告（平易な日本語、5因子分析 + 14下位成分評価、探索範囲評価）
  - 最大 24 サイクルで打ち切り。$J_{kw} > 0.8$ かつ $\min(S_{viab}, S_{capa}, S_{coop}, S_{effi}, S_{fair}) > 0.6$ → Kind World 達成

  **`ExperimentRecord` 構造体:**
  - `experiment_id: String`, `experiment_cycle: u32`, `report: OptimizationReport`, `timestamp: String`
  - 系列管理: 各サイクルに `kw4-{timestamp}-{seq}` 形式の experiment_id を割り当て

  **`kw4_optimize` テスト関数:**
  - `#[test]` 属性、kind_world.rs の `mod tests` に実装
  - Nelder-Mead 各 iteration の $J_{kw}$ とパラメータを CSV 形式で逐次出力
  - 最終結果（OptimizationReport）を JSON 形式で出力
  - 収束判定 + Kind World 成立判定を出力
  - KW-REAL の 6 フェーズシミュレーションを評価関数として使用

* **テストコードによる検証:**
  1. Nelder-Mead が 1 次元凸関数（y = (x-3)²）で理論解 x=3 に収束すること
  2. `evaluate` 関数が同一パラメータで同一 $J_{kw}$ を返すこと（決定論的）
  3. 内側ループが 1 回の `cargo test` 内で約 100〜160 回のシミュレーションを実行し収束すること
  4. 各 iteration の履歴 CSV + 最終 JSON レポートが標準出力に書き出されること
  5. 各 cargo test の結果が `experiments.md` に記録されること
  6. 8 サイクルごとに平易な日本語で中間報告が生成されること
  7. $J_{kw} > 0.8$ かつ $\min(S_{viab}, S_{capa}, S_{coop}, S_{effi}, S_{fair}) > 0.6$ で Kind World 達成と判定すること
  8. 1 因子を故意に低く設定した場合、J_kw が乗算結合により強く減衰すること（マスキング防止の確認）
  9. 最大 24 サイクルで外側ループを終了すること
  10. 最終結果が Human review queue に配送されること
  11. 既存テスト（KW1/KW2/KW3/KW-REAL）が本チケット追加後も全 PASS すること

* **計装方法・観測対象:** 内側ループの全 iteration 履歴（CSV: iter, J_kw, 5因子値, 14下位成分, 7 params）と最終 OptimizationReport（JSON）を出力。外側ループの各サイクル結果を experiments.md に Markdown 形式で記録。$J_{kw}$ 内訳（5因子 $S_{viab}, S_{capa}, S_{coop}, S_{effi}, S_{fair}$ と全 14 下位成分値）と 5 因子最小値ゲート成立状況を各 experiment で観測。旧 6 成分 J_kw との比較診断も同時に出力し、モデル移行の追跡可能性を担保する。KW-REAL で計装された全 component-level metrics（HELP 発動回数、GC hazard 分布、村形成率等）をサブ計測として記録する。

---

### 3B. マイルストーン M-0.65：Preset Registry 基盤（v2.3-i 追加）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。
>
> **⚠️ このマイルストーンの位置づけ:** 本節は v2.3-i で RFC に追加された二重 Preset Registry アーキテクチャ（BakedPresetRegistry / MutablePresetRegistry / ResolvedWorkflowRegistry）を実装するための追加的チケット群である。既存の M-1（型定義基盤・FakeImpl）の上に構築され、M-0.5（HumanChannel）とは独立である。全ての PresetWorkflow の load / validate / resolve 操作はメモリ内データ構造でエミュレーションされ、SQLite / LadybugDB は使用しない。v2.3-i の StructMem / Corpus2Skill 実装化・Preset Registry 層・起動時検証・root preset 保護の各 RFC 改訂に対応する。

#### チケット M-0.65-a: Preset Registry データ型定義（ArtifactOriginKind / RegistrySource / CapabilityFamily / PresetRootPolicy / PresetMetadata / PresetValidationReason / PresetValidationFailure）

* **対象不変条件 / 規範:** RFC §8 MemoizedGraph metadata（artifact_origin_kind / preset_source_info / root_policy / capability_family / registry_source の5新規フィールド）、RFC §8.5〜§8.9 Preset Registry データ型、§23 推奨データ型。全列挙型の variant 数・構造体フィールド名・意味論は RFC 定義と完全一致しなければならない (MUST)。
* **実装の背景と目的:** v2.3-i で追加された Preset Registry アーキテクチャを支える基盤データ型を RFC §8 の Rust 疑似コードおよび §23 の推奨型定義に従って実装する。これらの型は以降の M-0.65-b〜i 全チケットで参照される。
* **実装スコープ:**
  - `ArtifactOriginKind` 列挙型: `PresetSystem`, `PresetUser`, `SearchGenerated`, `TrainingDerived`, `FusionDerived`, `Conversational`, `Manual`（7 variant）
  - `RegistrySource` 列挙型: `BakedPlatform`, `MutableUser`, `MutableWorkspace`（3 variant、§23 準拠）
  - `CapabilityFamily` 列挙型: `StructMem`, `Corpus2Skill`, `Search`, `Training`, `General`（5 variant、§23 準拠）
  - `PresetRootPolicy` 構造体: `immutable_root: bool`, `root_pinned: bool`, `boot_critical: bool`, `capability_family: CapabilityFamily`（§23 準拠）
  - `PresetMetadata` 構造体: `workflow_id: String`, `kind: PresetKind`, `preset_source: RegistrySource`, `preset_scope: String`, `preset_trust_class: TrustClass`, `boot_critical: bool`, `immutable_root: bool`, `root_pinned: bool`, `depends_on: Vec<String>`, `knowledge_capability: Option<CapabilityFamily>`, `version: String`
  - `PresetKind` 列挙型: `PresetWorkflow`
  - `TrustClass` 列挙型: `Trusted`, `Untrusted`
  - `PresetValidationReason` 列挙型: 12 variant（`InvalidPresetSchema`, `DuplicateWorkflowId`, `ReservedNamespaceViolation`, `WorkflowNotFound`, `CrossRegistryDependencyViolation`, `CircularReference`, `InvalidInputMapping`, `OutputBindingMismatch`, `BootCriticalPresetMissing`, `BootCriticalPresetInvalid`, `MutableOverrideForbidden`, `PresetPolicyViolation`）
  - `PresetValidationFailure` 構造体: `workflowid: Option<String>`, `source: RegistrySource`, `source_path: Option<String>`, `reasons: Vec<PresetValidationReason>`, `detected_at: SystemTime`
  - 全型に `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` を付与
* **テストコードによる検証:**
  1. 全 enum variant の網羅的インスタンス生成テスト（列挙型 variant ごとに最低1インスタンス生成可能であること）
  2. JSON シリアライズ/デシリアライズのラウンドトリップ（全フィールド一致率 100%）
  3. `PresetRootPolicy` の全フィールドにアクセス可能であることのフィールド単位確認
  4. `PresetValidationFailure` の `reasons: Vec<PresetValidationReason>` が複数理由を同時保持可能であること
  5. `ArtifactOriginKind`, `RegistrySource`, `CapabilityFamily` の各 variant がパターンマッチ網羅的であること
* **計装方法・観測対象:** RFC §8 の型定義と実装型のフィールド一対一対応を人手照合し、過不足率 0% を確認する。JSON シリアライズ/デシリアライズのラウンドトリップ成功率を $n = 1000$ で計測する。

#### チケット M-0.65-b: MemoizedGraph 5 新規フィールド追加 + GcState::Protected

* **対象不変条件 / 規範:** RFC §8 MemoizedGraph metadata（artifact_origin_kind / preset_source_info / root_policy / capability_family / registry_source）、§15 GcState::Protected。既存 MemoizedGraph の全フィールド・全メソッドのシグネチャは変更されてはならない (MUST NOT)、追加のみ許容する。
* **実装の背景と目的:** v2.3-i で追加された Preset Registry と MemoizedGraph の接続を実現する。MemoizedGraph に出自種別・preset 情報・root 保護ポリシー・capability 分類・registry source の 5 フィールドを追加し、GcState に `Protected` 状態を追加して root preset の GC 完全除外を可能にする。
* **実装スコープ:**
  - `MemoizedGraph` に5フィールド追加:
    - `artifact_origin_kind: ArtifactOriginKind`（dafault: ArtifactOriginKind::Manual）
    - `preset_source_info: Option<PresetSourceInfo>`（None で非 preset を示す）
    - `root_policy: PresetRootPolicy`
    - `capability_family: CapabilityFamily`（default: CapabilityFamily::General）
    - `registry_source: Option<RegistrySource>`（None で非 registry を示す）
  - `PresetSourceInfo` 構造体: `registry_source: RegistrySource`, `preset_metadata: PresetMetadata`, `loaded_at: SystemTime`, `validated_at: SystemTime`（RFC §8 準拠）
  - `GcState` に `Protected { reason: String }` variant 追加（Active と同様の通常操作可能だが、GC 遷移対象外）
  - `cold_start_new()` および `inherit_from_parent()` に新規フィールドの初期化ロジック追加
  - M-0.65-a で定義した全型の use import / pub use 再公開
* **テストコードによる検証:**
  1. `cold_start_new()` で生成した MemoizedGraph の新規5フィールドが期待通りのデフォルト値を持つこと
  2. `inherit_from_parent()` で新規フィールドが適切に継承されること
  3. `GcState::Protected { reason: "root_preset".into() }` のインスタンス生成とパターンマッチ
  4. `GcState::Protected` から `SoftDeleted` / `HardDeleteCandidate` / `Tombstoned` への遷移が不可能であることの状態遷移検証
  5. 既存 MemoizedGraph テストコードが新規フィールド追加後も一切の変更なくコンパイル・通過すること（後方互換性）
* **計装方法・観測対象:** 既存テストスイートの全テストが新規フィールド追加後に同一結果を返すことを確認する（退行検出率 100%）。`GcState` の全状態遷移可能性行列 $T \in \{0,1\}^{5\times5}$（5状態 = Protected/Active/SoftDeleted/HardDeleteCandidate/Tombstoned）を列挙し、Protected から他の 4 状態への遷移が全て禁止されていることを検証する。

#### チケット M-0.65-c: BakedPresetRegistry + MutablePresetRegistry データ構造と基本操作（load / validate / get）

* **対象不変条件 / 規範:** RFC §8.5 BakedPresetRegistry（immutable, platform-critical, boot-fatal）、§8.6 MutablePresetRegistry（user-extensible, graceful degradation, quarantine）。BakedPresetRegistry は起動時の展開・検証失敗が boot-fatal であること、MutablePresetRegistry は不合格エントリを quarantine し registry 全体の起動は阻止しないことを必須とする。
* **実装の背景と目的:** 二重 registry 構造のうち、baked（バイナリ埋め込み immutable）と mutable（ファイルシステム由来ユーザー拡張可能）の 2 つの registry データ構造と、その基本操作（load / validate / get / quarantine）を実装する。M-0.65-e の ResolvedWorkflowRegistry が両者を統合する。
* **実装スコープ:**
  - `PresetWorkflow` 構造体: `workflow_id: String`, `metadata: PresetMetadata`, `graph: PresetWorkflowGraph`, `source: RegistrySource`
  - `PresetWorkflowGraph` 構造体（stub: 実際の WorkflowGraph は M-2 以降で定義。本マイルストーンでは ID とフィールドプレースホルダのみ保持）
  - `BakedPresetRegistry` 構造体: `presets: Vec<PresetWorkflow>`, `load_epoch: u64`
    - `fn new(presets: Vec<PresetWorkflow>) -> Self`: 構築時に全 preset を内部保持
    - `fn get(&self, workflow_id: &str) -> Option<&PresetWorkflow>`: ID 検索
    - `fn all(&self) -> &[PresetWorkflow]`: 全件取得
    - `fn expand_and_validate(&mut self) -> Result<(), PresetValidationFailure>`: 展開＋検証（エラー時 boot-fatal を示す Result）
  - `MutablePresetRegistry` 構造体: `presets: Vec<PresetWorkflow>`, `quarantined: Vec<PresetValidationFailure>`, `source_dir: String`
    - `fn new(source_dir: String) -> Self`: 空の状態で構築
    - `fn load_from_json(&mut self, json: &str) -> Result<(), PresetValidationFailure>`: JSON 文字列をパース・検証・登録
    - `fn get(&self, workflow_id: &str) -> Option<&PresetWorkflow>`: ID 検索
    - `fn presets(&self) -> &[PresetWorkflow]`: 全合格 preset 取得
    - `fn quarantined_failures(&self) -> &[PresetValidationFailure]`: 隔離済み不合格エントリ取得
    - `fn scan_directory(&mut self) -> Vec<PresetValidationFailure>`: ディレクトリ走査＋全ファイル検証（§8.7 手順 4-10 に相当）
* **テストコードによる検証:**
  1. BakedPresetRegistry 正常系: 有効な PresetWorkflow を登録 → `get()` で同一内容が取得可能
  2. BakedPresetRegistry 異常系: 不正な PresetWorkflow の登録 → `expand_and_validate()` が `Err` を返すこと（boot-fatal 相当）
  3. MutablePresetRegistry 正常系: 有効な JSON を `load_from_json()` で登録 → `presets()` に含まれること
  4. MutablePresetRegistry 異常系: 不正な JSON を `load_from_json()` → `quarantined_failures()` に記録され `presets()` には含まれないこと
  5. MutablePresetRegistry 混合系: 正常 5 + 異常 3 の JSON を逐次登録 → `presets()` が 5 件、`quarantined_failures()` が 3 件であること（graceful degradation 確認）
  6. MutablePresetRegistry 空ディレクトリ走査: `scan_directory()` が空の合格リストと空の隔離リストを返すこと
* **計装方法・観測対象:** BakedPresetRegistry の boot-fatal 条件が期待通りに動作することを確認し、fatal エラー時のプロセス停止シグナル（panic または abort）が必ず発生することを検証する（ただし単体テストでは catch_unwind で捕捉）。MutablePresetRegistry の graceful degradation 能力を、正常:異常の比率を変えた 10 通りの混合ケースで測定し、異常エントリが quarantine され正常エントリのみが registry に昇格することを確認する。

#### チケット M-0.65-d: 12 段階起動時検証手順の実装と逐次実行

* **対象不変条件 / 規範:** RFC §8.7 12段階起動時検証手順。12段階の逐次実行が完全に保証され、段階の省略・順序変更・並列化は禁止 (MUST NOT)。各段階の失敗条件が RFC 定義と一致すること。
* **実装の背景と目的:** RFC §8.7 で numbered procedure として規定された 12 段階の起動時検証手順を実装する。この手順は baked preset の展開・検証（boot-fatal）、mutable preset の走査・検証（graceful degradation）、統合・診断ログ出力の全行程をカバーする。各段階の失敗が正しく fatal / quarantine に振り分けられることが生命線である。
* **実装スコープ:**
  - `StartupValidationProcedure` 構造体: 12段階の逐次実行をカプセル化
  - `fn execute(&mut self, baked: &mut BakedPresetRegistry, mutable: &mut MutablePresetRegistry) -> Result<ResolvedWorkflowRegistry, StartupError>`
    - 内部で 12 段階を直列実行（各段階の成功/失敗を診断ログに記録）
    - Step 1-3: BakedPresetRegistry の展開・検証（展開→parse/validate→critical 確認）
    - Step 4-10: MutablePresetRegistry の走査・検証（scan→parse→schema validate→graph validate→cross-reference validate→policy validate→accept/reject）
    - Step 11: 統合（ResolvedWorkflowRegistry 生成）
    - Step 12: 診断ログ出力（DarviumEvent 発行）
  - `StartupError` 列挙型: `BakedFatal { failures: Vec<PresetValidationFailure> }`, `MutableDegraded { failures: Vec<PresetValidationFailure> }`, `MutableResolvedWithQuarantine { accepted: usize, quarantined: Vec<PresetValidationFailure> }`
  - `DiagnosticLog` 構造体: `step: u32`, `status: StepStatus`, `failures: Vec<PresetValidationFailure>`, `timestamp: SystemTime`
  - 段階別検証関数（各段階に対応する小関数）:
    - `step1_expand_baked(baked) -> Result<()>`
    - `step2_parse_validate_baked(baked) -> Result<()>`
    - `step3_check_boot_critical(baked) -> Result<()>`
    - `step4_scan_mutable_dir(mutable) -> Result<()>`
    - `step5_parse_json_candidates(mutable) -> Vec<Result<PresetWorkflow, PresetValidationFailure>>`
    - など
* **テストコードによる検証:**
  1. 全12段階正常系: 有効な baked + 有効な mutable → ResolvedWorkflowRegistry が正常構築されること
  2. Baked fatal 系（3種）: baked preset が空 → Step 3 で fatal。baked preset が不正 → Step 2 で fatal。boot-critical な baked preset が欠落 → Step 3 で fatal
  3. Mutable quarantine 系（6種）: 不正スキーマ / 重複 ID / 予約名違反 / 未解決依存 / 循環参照 / ポリシー違反の各ケースで該当エントリが quarantine されること
  4. 混合系: fatal に至らない baked 正常 + mutable に不正混入 → ResolvedWorkflowRegistry が正常構築され mutable の隔離リストが非空であること
  5. 各段階の省略不可能性: 途中段階をスキップした場合のテスト（コンパイル時またはテスト時に検出）
* **計装方法・観測対象:** 12段階の逐次実行における段階別成功/失敗分布を $n = 100$ のランダム入力系列で観測する。baked fatal 時は常にプロセス停止（panic）、mutable 不合格時は quarantine に留まり全体の起動継続を確認する。各段階の診断ログが正しく記録されることを検証する。

#### チケット M-0.65-e: ResolvedWorkflowRegistry + 依存方向制約 + 名前空間予約

* **対象不変条件 / 規範:** RFC §8.8 依存方向制約（baked→baked MUST, mutable→baked MAY, mutable→mutable MAY, baked→mutable MUST NOT）、§8.8 名前空間予約（`platform.*` / `builtin.*` / `system.*`）、§8.9 ResolvedWorkflowRegistry（二重 registry 統合・collision policy）。名前空間予約ルールおよび依存方向制約はいかなる状況下でもバイパスしてはならない (MUST NOT)。
* **実装の背景と目的:** BakedPresetRegistry と MutablePresetRegistry を統合した runtime の単一 lookup 面（ResolvedWorkflowRegistry）を実装する。名前空間衝突解決（baked 優先）、依存方向検証、source provenance 追跡の全機能を提供する。
* **実装スコープ:**
  - `ResolvedWorkflowRegistry` 構造体: `baked: BakedPresetRegistry`, `mutable: MutablePresetRegistry`
    - `fn resolve(&self, workflow_id: &str) -> Option<&PresetWorkflow>`: baked 優先の ID 解決
    - `fn all_resolved(&self) -> Vec<&PresetWorkflow>`: 全解決済み workflow 一覧
    - `fn source_of(&self, workflow_id: &str) -> Option<RegistrySource>`: source provenance 追跡
    - `fn check_dependency_constraints(&self) -> Vec<PresetValidationFailure>`: 依存方向制約の全件検証
    - `fn check_namespace_reservation(&self) -> Vec<PresetValidationFailure>`: 名前空間予約違反の全件検証
    - `fn resolve_collisions(&mut self) -> Vec<PresetValidationFailure>`: 全 collision 解決
  - `NamespacePolicy` 構造体: `reserved_prefixes: Vec<String>`（`["platform", "builtin", "system"]`）
    - `fn is_reserved(workflow_id: &str) -> bool`
    - `fn validate_mutable(workflow_id: &str) -> Result<(), PresetValidationReason>`
  - 依存方向検証関数:
    - `validate_dependency(source: RegistrySource, target_id: &str, target_registry: RegistrySource) -> Result<(), PresetValidationReason>`
  - Collision 解決ロジック:
    - baked-baked 衝突: build defect → fatal（PresetValidationReason::DuplicateWorkflowId）
    - mutable-mutable 衝突: startup validation failure
    - mutable が baked ID と衝突: reject（PresetValidationReason::MutableOverrideForbidden）
* **テストコードによる検証:**
  1. 正常解決: baked 1件 + mutable 2件（異なる ID）→ `resolve()` で全件解決可能
  2. Baked 優先解決: baked と mutable に同一 ID → `resolve()` が baked 側を返すこと
  3. 依存方向4種網羅（各 $n = 10$ ランダムケース）:
     - baked→baked OK
     - mutable→baked OK
     - mutable→mutable OK
     - baked→mutable MUST NOT（エラー検出）
  4. 名前空間予約違反検出: mutable が `platform.*` の workflow_id を使用 → `ReservedNamespaceViolation`
  5. Collision ポリシー全3種:
     - baked-baked 重複: fatal
     - mutable-mutable 重複: validation failure
     - mutable が baked ID と衝突: reject（silent override 禁止）
  6. `source_of()` が正しい RegistrySource を返すこと
* **計装方法・観測対象:** 依存方向制約の全4種 + 例外ケースについて $n = 100$ のランダム依存グラフを生成し、禁止方向の依存が 100% 検出されることを確認する。Collision ポリシーが常に baked 優先・mutable 拒否の順序で解決されることを検証する。

#### チケット M-0.65-f: DarviumEventKind::PresetRegistry + 5 種 PresetRegistryEvent

* **対象不変条件 / 規範:** RFC §12C DarviumEventKind に `PresetRegistry(PresetRegistryEvent)` variant 追加、§12C 5種 sub-event（StartupValidationStarted / StartupValidationCompleted / PresetAccepted / PresetQuarantined / CollisionResolved）。既存の DarviumEventKind の variant は一切変更してはならない (MUST NOT)。
* **実装の背景と目的:** Preset Registry の起動時検証・登録・衝突解決の各イベントを DarviumEvent 体系に統合する。これにより preset validation の全行程が Event Bus 上で監査可能になる。
* **実装スコープ:**
  - `DarviumEventKind` に `PresetRegistry(PresetRegistryEvent)` variant 追加
  - `PresetRegistryEvent` 列挙型（RFC §12C 準拠）:
    - `StartupValidationStarted { source: RegistrySource, timestamp: SystemTime }`
    - `StartupValidationCompleted { accepted_count: usize, quarantined_count: usize, timestamp: SystemTime }`
    - `PresetAccepted { workflow_id: String, source: RegistrySource }`
    - `PresetQuarantined { failure: PresetValidationFailure }`
    - `CollisionResolved { workflow_id: String, resolution: String }`
  - 既存の `DarviumEventKind` の全 variant（13種）は維持（MUST NOT）
  - `FakeEventBus` 上で PresetRegistryEvent の publish / subscribe テスト
* **テストコードによる検証:**
  1. `DarviumEventKind::PresetRegistry(PresetRegistryEvent::StartupValidationStarted { .. })` のインスタンス生成
  2. 全5種の PresetRegistryEvent が `Debug + Clone + PartialEq + Serialize + Deserialize` を実装していること
  3. 既存の DarviumEventKind variant（System / Search / WorkflowExecution / Training 等）が本追加後も変更なくコンパイル可能であること
  4. PresetRegistryEvent を DarviumEventBus に publish → replay で同一内容が取得可能であること
  5. `EventFilter` の `kind_filter` で PresetRegistryEvent のみをフィルタリング可能であること
* **計装方法・観測対象:** 5種の PresetRegistryEvent 全 variant の publish → replay 完全性（消失率 0%）を $n = 1000$ で検証する。既存 variant の非影響性を確認（全既存 variant の publish + replay が変更前と同一結果であること）。

#### チケット M-0.65-g: Startup repair scan への preset validation phase 前置統合

* **対象不変条件 / 規範:** RFC §18 Startup repair scan（preset validation phase 前置）、§8.7 12段階手順と repair scan の逐次実行順序。preset validation phase は startup repair scan の前に実行されなければならない (MUST)。両者の実行順序の逆転は禁止 (MUST NOT)。
* **実装の背景と目的:** v2.3-i で追加された preset validation phase を既存の startup repair scan（M1.5-3）の前に前置する。起動時の処理順序を「preset validation → repair scan → normal operation」に確定し、PresetValidationFailure の診断ログ出力を EventBus 経由で統合する。
* **実装スコープ:**
  - `StartupOrchestrator` 構造体（または既存の起動シーケンス関数の拡張）:
    - `fn execute_startup(baked: &mut BakedPresetRegistry, mutable: &mut MutablePresetRegistry, repair_worker: &mut dyn RepairWorker, event_bus: &mut dyn DarviumEventBus) -> Result<SystemStartupState, StartupError>`
    - 内部実行順序:
      1. Preset validation phase: StartupValidationProcedure::execute()
      2. Preset validation の診断ログ出力（PresetRegistryEvent 発行）
      3. Startup repair scan（既存 M1.5-3 の Repair Worker 呼び出し）
      4. Normal operation への移行
  - PresetValidationFailure の configuration-plane diagnostic としての区別（ConsistencyState の状態遷移は発生させず、診断情報のみ記録）
  - startup repair scan 完了後に ResolvedWorkflowRegistry が利用可能であることの表明
* **テストコードによる検証:**
  1. 正常系: preset validation success + repair scan success → 正常起動
  2. Preset fatal: baked preset boot-fatal → repair scan 実行前に起動中断
  3. Preset quarantine + repair success: mutable に不正エントリ混入（quarantine） + repair scan は正常 → 起動継続し quarantine リストが非空
  4. Preset success + repair failure: preset 正常 + repair scan で未完了トランザクション検出 → repair 後正常起動
  5. 実行順序の逆転防止: preset validation 完了前に repair scan を呼び出そうとした場合のコンパイルエラーまたはパニック確認
* **計装方法・観測対象:** preset validation phase と repair scan の逐次実行が $n = 100$ のランダム failure 注入下で正しい順序を維持することを確認する。PresetValidationFailure を ConsistencyState とは独立した configuration-plane diagnostic として記録し、修復サイクルの対象外であることを検証する。

#### チケット M-0.65-h: Preset Registry 関連定数 5 種の constants 定義

* **対象不変条件 / 規範:** RFC §22 PRESET_NAMESPACE_RESERVED、PRESET_BAKED_VALIDATION_TIMEOUT_MS を含む v2.3-i 新規定数。Safety Invariant（変更禁止）と Calibration Candidate（実験的調整可）の分類を遵守する。
* **実装の背景と目的:** RFC §22 で指定された preset registry 関連定数を `src/constants.rs` に追加する。これらの定数は M-0.65-c/d/e/g で参照されるため、本マイルストーン内で事前定義する。
* **実装スコープ:**
  - `PRESET_NAMESPACE_RESERVED: &[&str] = &["platform", "builtin", "system"]`（**Safety Invariant** — 変更禁止。Mutable からの予約名使用を禁止する root policy）
  - `PRESET_BAKED_VALIDATION_TIMEOUT_MS: u64 = 5000`（**Calibration Candidate** — 上げると大規模 preset の検証余裕増、下げると startup 高速化。推奨感度分析範囲: 1000〜30000）
  - `PRESET_MUTABLE_MAX_COUNT: usize = 1000`（**Safety Invariant** — mutable preset の最大登録数超過を防止。変更禁止）
  - `PRESET_MUTABLE_MAX_DEPTH: usize = 10`（**Calibration Candidate** — preset dependency graph の最大深さ制限。推奨範囲: 3〜20）
  - `PRESET_BAKED_MIN_COUNT: usize = 2`（**Safety Invariant** — 最低限必要な baked preset 数（StructMem / Corpus2Skill の root preset 2 件を想定）。変更禁止）
  - 各定数に分類コメント（Safety Invariant / Calibration Candidate）と日本語意図説明を付与
* **テストコードによる検証:**
  1. 定数値がコンパイル時に確定していることのアサート（`const` 評価）
  2. `PRESET_NAMESPACE_RESERVED` の各要素が空文字列でないこと
  3. `PRESET_BAKED_VALIDATION_TIMEOUT_MS > 0` のコンパイル時確認
  4. `PRESET_MUTABLE_MAX_COUNT >= PRESET_BAKED_MIN_COUNT` の妥当性確認
  5. `PRESET_BAKED_MIN_COUNT >= 2` の根拠アサート（StructMem + Corpus2Skill の 2 root preset 最低保証）
* **計装方法・観測対象:** 全 5 定数が RFC §22 の定数表と一対一対応することの静的検証。Safety Invariant 3 件の変更禁止がコードレビューにより確認されること。Calibration Candidate 2 件のデフォルト値で invariant が成立することを確認する。

#### チケット M-0.65-i: StructMem / Corpus2Skill root preset の BakedPresetRegistry 登録（stub）

* **対象不変条件 / 規範:** RFC §8.5 BakedPresetRegistry（StructMem / Corpus2Skill root preset 包含）、§10.1 root preset の性格（baked registry 所属 / immutable / root-pinned / GC 対象外）、§10.3 workflow root と knowledge root の両立可能性、§15 GcState::Protected。root preset は GcState::Protected により GC から完全除外され、RegistrySource::BakedPlatform として紐付けられなければならない (MUST)。
* **実装の背景と目的:** StructMem / Corpus2Skill の root preset を BakedPresetRegistry に登録する stub 実装である。各 root preset は起動時に展開・検証され、GcState::Protected により GC から永久保護される。実際のワークフローグラフ本体（WorkflowGraph の具体的なノード・エッジ構成）は本マイルストーンのスコープ外であり、単一ノードのプレースホルダグラフで代用する。ワークフローの具体的な実装は M1 以降の StructMem / Corpus2Skill 実装フェーズで行う。
* **実装スコープ:**
  - `fn create_structmem_root_preset() -> PresetWorkflow`:
    - `workflow_id: "root.structmem.core.v1"`
    - `metadata.boot_critical: true`
    - `metadata.immutable_root: true`
    - `metadata.root_pinned: true`
    - `capability_family: CapabilityFamily::StructMem`
    - プレースホルダグラフ（最小単一ノード）
  - `fn create_corpus2skill_root_preset() -> PresetWorkflow`:
    - `workflow_id: "root.corpus2skill.core.v1"`
    - 同様の属性、`capability_family: CapabilityFamily::Corpus2Skill`
  - `fn bootstrap_root_presets() -> Vec<PresetWorkflow>`:
    - StructMem + Corpus2Skill の 2 root preset を生成
    - 各 preset に GcState::Protected の設定指示（Metadata で表現）
  - BakedPresetRegistry の構築時に `bootstrap_root_presets()` の結果を投入
  - RegistrySource::BakedPlatform の自動紐付け
* **テストコードによる検証:**
  1. 2件の root preset が正しく生成され、`workflow_id` / `capability_family` / `boot_critical` / `immutable_root` / `root_pinned` が期待値と一致すること
  2. `bootstrap_root_presets()` の返却件数が 2 であること
  3. BakedPresetRegistry に登録後、`get("root.structmem.core.v1")` および `get("root.corpus2skill.core.v1")` が有効な PresetWorkflow を返すこと
  4. 登録された root preset を GcState::Protected 相当としてマーク可能であること
  5. M-0.65-c の BakedPresetRegistry テスト（boot-fatal 条件）が root preset 存在下でも正常動作すること
* **計装方法・観測対象:** 2件の root preset が起動時に必ず存在することを BakedPresetRegistry の `all()` で確認する。root preset が GcState::Protected によって通常の GC ライフサイクル（Active→SoftDeleted→...）から完全に除外されることを状態遷移検証で確認する。

---

### ── 第4段階：本物LLMの局所的・段階的な投入（M2 〜 M3） ──

> **DB**: LLM は本物になるが、ストレージは依然メモリ内完結。SQLite / LadybugDB 不要。

システムの論理的な土台が完成したため、本物のLLMへの接続を、限定された領域から安全網を張った状態で段階的に解禁するフェーズです。

### 9. マイルストーン M2：Limited real LLM（API接続・予算管理）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### チケット M2-1: `BuildQueryStep` 専用のプロンプトペイロード生成器のスキーマ整合性検証

* **対象不変条件 / 規範:** §9.4 Query側生成手順、§9.2 Canonical schema
* **実装スコープ:** 受信したミッションテキストから、規定された JSON 風の `QueryDesignText` フォーマットの文字列を組み立てるシリアライザ。
* **テストコードによる検証:** 生成されたプロンプト用のテキストが、カンマ欠落などの文法エラーを起こしておらず、規定されたキー（`"workflow_purpose"`, `"node_list"` 等）を過不足なく含んでいるかをアサート。
* **計装方法・観測対象:** 自然言語のミッション多様体テキスト（シャノン文字エントロピー $H_{raw}$ ）を入力し、シリアライザを通じて JSON スキーマ風 `QueryDesignText` の文字列を出力。 出力テキストの文字出現頻度確率空間における圧縮シャノンエントロピー低減率 $\Delta H = H_{raw} - H_{json}$ の実測。 構造化によるキーワード（`"node_list"`等）の固定周期出現に起因する、文字列全体のトポロジー的規則性の増加（情報冗長度 $R = 1 - H/H_{max}$ の急激な上昇プロファイル）、および出力されたスキーマが代数的全射（未定義のキー配列への迷走が完全にゼロ）を充足していることの構文検証。

#### チケット M2-2: 実LLM応答遅延をシミュレートした `SearchBudget` の減算・タイムアウト遮断テスト

* **対象不変条件 / 規範:** §13.6 ガード条件「SearchBudget の上限超過時は SearchBudgetExceeded を返すこと」
* **実装スコープ:** 本物のネットワーク遅延を模したスリープ（例: `tokio::time::sleep`）をはさみながら、`wall_clock_ms_used` をカウントアップする統合テスト。
* **テストコードによる検証:** `max_wall_clock_ms` を意図的に非常に短い値（例: 10ミリ秒）に設定したテスト環境を構築。 モックの応答時間がそれを超えた瞬間に、ループが第2イテレーションへ進入することを遮断し、安全に `Abort` 状態へ遷移することを確認。
* **計装方法・観測対象:** 実時間軸に対して平均 $\mu = 1500\text{ms}$ 、分散 $\sigma^2 = 500\text{ms}$ の対数正規分布または指数分布に従う擬似遅延パルスを注入し、`max_wall_clock_ms = 1000\text{ms}` に設定。 超許容時間境界を跨いでから、状態機械が `Abort` へ遷移を完了するまでの時間軸上のオーバーシュート量 $\Delta \tau = \tau_{abort} - \tau_{limit}$ の極値分布。 $\Delta \tau$ が、非同期実行ランタイムのコンテキストスイッチの量子時間幅の内部に完全拘束されていること、およびオーバーシュート量の統計分布が一般化極値分布（GEV）の形状パラメータ $\xi \le 0$ （有界 Weibull 領域）に適合していることの実証。

#### チケット M2-3: LLM自己評価スコア $c_s$ に対する過信頼補正係数（DISCOUNT）適用の数理テスト

* **対象不変条件 / 規範:** §14.2 & §14.3 PatchConfidence 計算規則（非対称重み切り替え）
* **実装スコープ:** LLMから返ってきた生の自己評価スコアに `SELF_CONF_DISCOUNT (0.85)` を乗算し、さらにその値が `0.50` 未満の場合にバリデータ側の重み $w_v$ を `0.40` から `0.50` へ引き上げる動的スイッチロジック。
* **テストコードによる検証:**

1. 生スコア `0.90` が入力された場合：$0.90 \times 0.85 = 0.765$ として通常重みが適用されることを確認。
2. 生スコア `0.45` が入力された場合：`PATCH_SELF_CONF_SWITCH_THRESHOLD (0.50)` を下回るため、動的重み切り替え規則が発動し、重みが自動で $w_s=0.20, w_v=0.50$ へスイッチして幾何平均が計算されることを、数理演算の出力値レベルでアサート。

* **計装方法・観測対象:** LLMの生スコア $c_s$ を、切り替え境界の周辺 $[0.499, 0.501]$ の領域で、IEEE 754 浮動小数点数の最小ビット分解能 $\Delta c_s = 10^{-7}$ 刻みで高密度走査注入。 パッチ信頼度関数の $c_s$ に対する数値微分係数 $D_{\text{num}} = \frac{\Delta PatchConfidence}{\Delta c_s}$ の不連続な階段関数적ジャンププロファイル。 境界点 $c_s = 0.50$ を跨いだ瞬間に, 微分係数が有限の跳躍（$\delta$関数的フラックスの累積）をなす幾何学的多様体の曲率曲線の実測、および浮動小数点演算の丸め誤差に起因する境界判定のブレ幅（ジッター幅 $\le 10^{-7}$ ）の完全固定検証。

---

### 10. マイルストーン M2.5：Real query-policy evaluation

> **DB**: メモリ内完結。SQLite / LadybugDB 不要（「テスト用SQLiteテーブル」は括弧書きの代替手段であり、主実装はメモリ内レジストリ）。

#### チケット M2.5-1: 探索イテレーションごとの証拠性監査ログ（`SearchTrace`）永続化ロジックの検証

* **対象不変条件 / 規範:** §13.3 SearchTrace データモデル、§12A.5 SearchTrace 拡張、v2.3-g §12E `SearchTraceProjection`（`DarviumEventKind::Search` の EventProjection）。
* **実装スコープ:** v2.3-g では SearchTrace は EventBus 上の `SearchTraceProjection` として動作する。探索ループが回るたびに、その時点の `SearchBudgetSnapshot`、採用した `SearchOutcome`、および判断の正当化根拠（`justification`）を `DarviumEventKind::Search` イベントとして EventBus へ publish し、`SearchTraceProjection` がこれを materialize して SearchTrace を再構成する。互換性のため従来のメモリ内レジストリへの直接追記も併存させる。加えて、SearchTrace を単なる forensics 用ログではなく、reuse rate / false-new rate / fallback frequency / oscillation count / review queue ingress reason / ranking stability 観測の導出基盤として利用できるよう、必要な outcome metadata と理由コードを保持する。
* **テストコードによる検証:** 3回往復した探索ループの終了後、レジストリからトレース配列を取り出し、要素数が正確に `3` であること、および各要素内の `iteration` カウンタが `0, 1, 2` と単調増加で記録されていることを確認する。加えて、複数の outcome パターン（reuse / patch / refine / compose / new / human review）を流し込んだとき、trace から fallback 系遷移と review ingress reason を集計可能であることを確認する。
* **計装方法・観測対象:** 探索ループを 1 から $MAX\_ITERATIONS$ まで自励駆動させ、各ステップで生成される `SearchTrace` をメモリチェーンへアペンド。トレースログチェーンが構築する有向木の「情報量トポロジー的記述長さ $L_{trace}(t)$」の時間発展。イテレーション進行に対する累積エントロピーの単調非減少（時間反転をかけた際の過去ログのユニーク復元性）、および各要素の固有ハッシュ値（`query_design_text_hash`, `justification_hash`）の一時系列上の自由エネルギー変化を追跡し、情報の散逸率が完全に $\Delta I / \Delta t = 0$ （lossless 保存則）を充足していることの代数的一貫性を実証する。さらに SearchTrace から reuse rate、false-new rate、fallback frequency、oscillation frequency、review ingress reason、ranking stability 推定量を導出し、v2.3 の補助性能メトリクス観測基盤として扱う。

#### チケット M2.5-2: 同一シード下における `SearchTrace` の決定論的再現性（Deterministic Replay）テスト

* **対象不変条件 / 規範:** §13.5 優先要件「同一 Fake 入力に対して deterministic replay 可能であること」
* **実装スコープ:** 過去に生成された `SearchTrace` のインプット、および固定シードのPRNGを完全に同じ条件でサーチエンジンに再投入するリプレイ機能。
* **テストコードによる検証:** まったく同じミッションと同一シードを渡し、2回独立してサーチエンジンを実行。 出力された1回目の `SearchTrace` の全ハッシュ値（`query_design_text_hash` 等）と、2回目の実行で得られたトレースのハッシュ値がビットレベルで完全に一致（再現）することを確認。
* **計装方法・観測対象:** 同一の初期ミッション多様体入力、および完全に同一のPRNG固定シード（シード値12345）を与えた独立なサーチランを $10^3$ 回パラレルに完全並行実行。生成された全 `SearchTrace` 配列の、ビットレベルでの全ハッシュ距離（ハミング距離 $D_H$ ）。浮動小数点数の演算順序の揺らぎや、Tokio非同期ランタイムのスレッドタスク割り当てのランダム性が混入する状況下においても、出力されるトレース配列全体のハミング距離が全サンプル間で厳密に $D_H \equiv 0$ （リアプノフ指数 $\lambda = -\infty$：完全吸収定常状態）を維持していることのビットレベル決定論的再現性を実証する。さらに、この replay 性を false-new rate、ranking stability、fallback frequency、repair convergence 比較のための較正基盤として利用できるよう、trace 再構成後の指標抽出一貫性も観測対象に含める。

---

### 11. マイルストーン M3：Real proposal generation（パッチ生成と昇格）

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。

#### チケット M3-1: 構造化 JSON パッチ操作（`PatchOperation`）パース及び具象オブジェクト生成ロジックの検証

* **対象不変条件 / 規範:** §14.1 & §14.2 構造化出力要求契約
* **実装スコープ:** LLMが返したと想定される `{"patch_ops": [...], "self_confidence": ...}` 形式の生テキストをパースし、Rustの強力な型である `Vec<PatchOperation>` の配列へと変換するデシリアライザ。
* **テストコードによる検証:** 正当なパッチJSONを流し込み、`PatchOperation::AddNode` や `PatchOperation::ReplaceNode` の内部フィールドが、欠落なく正確なメモリ内構造体オブジェクトにマッピングされることを確認。
* **計装方法・観測対象:** LLMの生成したJSON文字列の任意の文字位置に対し、確率 $p_m$ でランダムな文字列・文法破壊（カッコの不整合、エスケープ文字の不正挿入など）を混入させるプロパティベースの変異アンサンブルを生成。 構文破壊ノイズ強度 $p_m$ に対する、パース成功率の臨界相転移曲線（パーコレーション閾値）。 ある破壊密度 $p_c$ を超えた瞬間に、パース成功率が 1.0 から 0.0 へと急峻に降下（1次相転移）するプロファイルを実測。 パース失敗時における例外トラップ効率が厳密に 100% であり、Rustの型安全領域（`LlmError::MalformedJson` への完全全射射影）に安全に封じ込められていることの境界値検証。

#### チケット M3-2: グラフパッチ適用の完全アトミック性（`apply_patch_atomic`）不変条件テスト

* **対象不変条件 / 規範:** 健全性命題3、§14.4 「apply_patch_atomic は atomic に実行すること。途中失敗時はグラフを元の状態に戻さなければならない (MUST)」。加えて v2.3 では、validate フェーズは cycle の不在だけでなく、compile / execute 前 validation と整合する構造健全性、ならびに parallel admissibility を壊す不整合も拒否対象である。
* **実装スコープ:** §14.4 に規定された「1. clone -> 2. apply all -> 3. validate -> 4. swap」の4フェーズコミット構造を持つパッチ適用エンジン本体の実装。validate フェーズでは DAG 健全性、変数スコープ健全性、ならびに frontier-based scheduling に矛盾する構造不整合を検出する。パッチ適用の各段階（clone・apply・validate・swap）を `DarviumEventKind::WorkflowExecution` イベントとして EventBus へ publish する（`emit_patch_event(phase, patch)`）。
* **テストコードによる検証:** 5番目の操作にわざと存在しないノードIDへのエッジ追加（`NodeNotFound` エラーを誘発するバグ）を仕込んだ、全10件の操作からなる `GraphPatch` を作成。これを適用した際、関数は当然 `Err(PatchError::NodeNotFound)` を返すが、**呼び出し元のオリジナル（Goldグラフ）のノード数・エッジ数がパッチ適用前の状態と1ビットも変わらずクリーンに維持されていること**を厳格にアサートする。加えて、cycle は生成しないが ready frontier の独立性や変数スコープ健全性を壊すような不正パッチを投入し、それらも validate failure として atomic に拒否されることを確認する。
* **計装方法・観測対象:** 障害パルス注入（サンプル数 $N \ge 10^3$ ）時における、ロールバックエンジンの処理軌道。計装プローブにより、フェーズ1で複製された `g_candidate` がクリーンに drop され、オリジナル（Goldグラフ）のディープコピー前後におけるオブジェクト等価性（`Eq`）の全ビットバリデーション、およびポインタ番地の完全な書き換えセパレーション（全ビットハミング距離 $D_H \equiv 0$ の不変性）を保っていることのアトミック性（不変データ構造の独立性）の実証。

#### チケット M3-3: 複数スレッドパッチ競合時の楽観的並行性制御（GraphVersion CAS）の排除テスト

* **対象不変条件 / 規範:** §8.4 & 設計前提 P-09「更新消失を防ぐために楽観的並行性制御を使用すること。不一致の場合は `CacheError::CasConflict` を返すこと」
* **実装スコープ:** `update_graph_cas` メソッドの実装。引数で渡された `expected_version` が、現在 WorkflowCache に保存されている `version` カウンタと一致している場合のみ書き換えを許可する比較交換（CAS）ロジック。
* **テストコードによる検証:** バージョン `0` のグラフを2つの非同期タスクが同時に読み込んだ状況を擬似的に再現。

1. タスクAがバージョン `0` でCAS更新を要求 $\rightarrow$ 成功し、WorkflowCache のバージョンが `1` にインクリメントされる。
2. タスクBが遅れて古い期待値であるバージョン `0` のままCAS更新を要求 $\rightarrow$ WorkflowCache 側が拒否し、確実に `Err(CacheError::CasConflict)` が返却されることをアサート。

* **計装方法・観測対象:** 同一の `graph_id` と古い `GraphVersion = 0` を共有した $10^3$ 個の並行スレッド（書き込み要求アンサンブル）を生成し、メモリ内の更新関数 `update_graph_cas` に向けて一斉に条件付き書き込みをシグナル注入。多重スレッド衝突負荷密度に対する更新消失（データ上書きバグ）の発生確率 $P_{lost}$。CAS判定超曲面プローブの計測に加え、`CacheError::CasConflict` 検知後に最新バージョンでの再試行ループ（SHOULD）を自励駆動させた際のライブロック（Livelock）相および飢餓（Starvation）リスクの統計的識別。各スレッドが再試行を繰り返して最終的な成功（吸収状態）に達するまでのリトライ回数の確率密度分布 $P(n_{retry})$ を算出し、その分布が有限の指数尾（エクスポネンシャル・テール）に拘束され、無限にリトライが続く重い尾（パワーロー・テール）を形成していないかの検証。衝突確率がどれほど極大化した状態であっても、データ上書きバグが厳密に $P_{lost} \equiv 0.00000$ を維持し続けることの統計的検証。

#### チケット M3-4: 訓練アセットのプロダクション昇格（Promotion Gates）防衛線検証テスト

* **対象不変条件 / 規範:** §17 健全性命題「Training Isolation / Promotion Discipline 不変条件」
* **実装スコープ:** 訓練成果（`PromotionCandidate`）をプロダクション環境の RepositoryPair へ移行させる昇格判定ゲート（`promote_candidate_to_production`）の実装。
* **テストコードによる検証:** 意図的に「成功率が閾値（0.80）を下回る」または「人間からの feedback rating が Bad である」不適格な `PromotionCandidate` をメモリ内で生成し、昇格関数に投入。 ゲートロジックがそれを確実にブロックし、プロダクション側の `RepositoryPair` の正本（Source of truth）が1ミリも書き換えられない（汚染されない）ことを検証。
* **計装方法・観測対象:** 成功率 $s \in [0.0, 0.79]$ 、人間フィードバック $rating = Bad$ 、または一貫性状態 $consistency\_state = NeedsRepair$ を持つ不適格な訓練アセット（汚染アンサンブル）を $10^5$ 件生成し、昇格関数に連続投入。 昇格判定ゲートにおける不適格候補の通過フラックス（偽陽性率： $FPR$ ）。 帰無仮説 $H_0: \text{不適格候補はプロダクション環境へ昇格する}$ と定義した際、計装ゲートのハード遮断条件により、すべての不適格アセットがエラーとして $10^5$ 件全弾トラップされ、プロダクション側 RepositoryPair のハッシュ整合性が 1 ビットも変更されないこと（ $FPR = 0$ ）、検定統計量の棄却域が有意水準 $\alpha = 0.00$ 限界（完全排除）を指すことの複雑系シミュレーション実証。

---

### ── 第5段階：エンドツーエンド実環境結合（M4） ──

> **DB**: メモリ内完結。SQLite / LadybugDB 不要。
> **⚠️ この第5段階（チケット M4-1 〜 M4-4）をもって全13フェーズ完了。実データベースの導入・結合はここから先の別フェーズとして計画すること。**

すべての安全網、アトミックコミット、バリデータ、および確率的テストを通過したロジック層の上で、最終的な実行エンジン（OpenFang）とリポジトリ融合（v2.0）の実I/Oを安全に駆動させる最終フェーズです。

### 12. マイルストーン M4：Real executor end-to-end

> **DB**: メモリ内完結。SQLite / LadybugDB 不要（融合誕生テストもエミュレーション）。
> **⚠️ このマイルストーンをもって全13フェーズ完了。実データベース導入はここから先の別フェーズ。**

#### チケット M4-1: WorkflowGraph から Layer 1 実行命令へのコンパイラ（`compile_to_steps`）健全性テスト

* **対象不変条件 / 規範:** 健全性命題2、§7.1 コンパイル規則
* **実装スコープ:** トポロジカルソート（`petgraph::algo::toposort`）を実行し、依存関係順に並んだ `Vec<OpenFangStep>` の平坦な配列を出力するコンパイル関数本体。
* **テストコードによる検証:** 複雑な分岐と集約を含むDAGグラフをインプットし、出力されたステップ配列の並び順が、グラフの依存順序（DependsOn）を絶対に破っていないことを全ステップにわたってアサート。
* **計装方法・観測対象:** ノード数最大512の、ランダムに接続された複雑な DAG グラフ（部分順序集合：Poset）を $10^3$ 個自動生成し、コンパイラ関数 `compile_to_steps` を実行。 元グラフのハッセ図（Hasse Diagram）が規定する部分順序関係の、出力 `Vec<OpenFangStep>` 配列上のインデックス順序への保存特性。 出力配列の全要素のインデックスペア $(i, j)$ において、元グラフでエッジ $i \to j$ が存在する場合、配列内のインデックス位置が常に $\text{idx}(i) < \text{idx}(j)$ を満たしていることの全射順序同型インバリアント、および方向反転率（エッジ順序の逆転）が確率空間上で厳密に 0.00 であることの、代数的トポロジー検証。

#### チケット M4-2: SubWorkflow 展開時における変数名前空間（Namespace Stack）隔離不変条件の検証

* **対象不変条件 / 規範:** §7.1 変数名前空間規則「SubWorkflow 内の変数は {workflow_uuid}/{original_var_name} 形式で名前空間化すること」
* **実装スコープ:** コンパイルコンテキスト（`CompilerContext`）内の名前空間スタックのプッシュ/ポップ、および変数名の自動プレフィックス付与ロジック。
* **テストコードによる検証:** 親グラフと子サブワークフローの双方で意図的に全く同じ変数名（例: `"result"`）を使用。 コンパイルを実行した結果、出力されたステップ内の変数名が自動的に `{sub_uuid}/result` へと安全に隔離・リネームされ、親側の変数と衝突（破壊的オーバーライト）を起こさないことをアサート。
* **計装方法・観測対象:** ネスト深さ $d = 10$ に達する多重レイヤの SubWorkflow を構築し、全レイヤのノードで意図的に衝突する同一変数名 `"result"` を宣言してコンパイルを駆動。 各コンパイルコンテキストの `namespace_stack` が生成する変数集合の、ハッシュ空間上での直和（Disjoint Union）特性。 各階層 $i$ でリネーム出力された変数名集合 $V_i$ と、別の階層 $j$ の変数名集合 $V_j$ の交わりを計算し、全ペアにおいて $\chi(V_i \cap V_j) = 0, \forall i \neq j$ （ただし $\chi$ は集合の要素数カウンタ関数）の完全なる空集合性をアサート。 名前空間の衝突による破壊的オーバーライトの確率が $10^5$ 回のランダムネスト生成において厳密に 0 であることの実証。

#### チケット M4-2.5: ExternalApiClient 抽象トレイトの定義

* **対象不変条件 / 規範:** §11.1 AG-03 ハードゲート、§13.6 ガード条件、§14.4 副作用プロファイル
* **実装の背景と目的:** M0-2 では `writes_external_api` フラグで副作用を検出するが、実際の外部 API 呼び出しを抽象化するトレイトが存在しない。本チケットでは外部 API 呼び出しを抽象化する `ExternalApiClient` トレイトと、何も実行しない `FakeExternalApiClient` を定義する。これにより M4-3 の実 OpenFang API 結合時に、トレイトの別実装を追加するだけで置き換えが完了する。
* **実装スコープ:**
  - `ExternalApiClient` トレイト: `fn execute_step(&self, step: &OpenFangStep) -> Result<StepResult, DarviumError>` 及び副作用プロファイルのメタデータ問い合わせ
  - `FakeExternalApiClient`: 呼び出しを受け付け、常に成功を模した結果を返す（副作用は記録のみで実際の I/O は行わない）
  - エラー型: `DarviumError::ExternalApi(String)` バリアント追加
* **テストコードによる検証:**
  1. `FakeExternalApiClient` がトレイト境界を充足することのコンパイル時検証
  2. 任意の `OpenFangStep` を渡しても Fake がパニックせず `Ok` を返すこと
  3. 呼び出し記録（何回・どの step が呼ばれたか）が正確に追跡可能であること
  4. トレイトのオブジェクト安全性確認（`Box<dyn ExternalApiClient>`）
* **計装方法・観測対象:** トレイト境界を通過する命令呼び出しの全二重記録による完全監査可能性。`FakeExternalApiClient` の呼び出しカウンタと仮想命令ステップ数の完全一致 ($\sigma^2 = 0$) の検証。

#### チケット M4-3: 不可逆副作用（AG-03）ハードゲートによる実プロバイダ呼び出しの絶対抑止テスト

* **対象不変条件 / 規範:** §11.1 AG-03 ハードゲート、§13.6 ガード条件（UnsafeSearchTransition の排除）
* **実装スコープ:** 検索候補、あるいは生成されたワークフローのノードの中に `side_effects.irreversible == true` のフラグを持つステップが存在する場合、実行フェーズへの進入を強制拒否するセキュリティゲート。
* **テストコードによる検証:** このフラグを `true` に設定したノードを含むグラフを意図的に作成し、実行エンジン（`WorkflowExecutor`）の入り口へ投入。 実 OpenFang API へのHTTPリクエストが発生する手前のローカルなバリデーションの段階で、ゲートが100%確実に処理を遮断し、エラーを返すことを検証。
* **計装方法・観測対象:** `side_effects.irreversible == true` フラグを強制設定したノードを持つグラフを作成し、LLMプロンプトインジェクションやポインタカオスを模した $10^6$ 回の並行実行パルス（トリガーシグナル）を最上層から印加。 ローローカルの `WorkflowExecutor` ゲートをすり抜けて、Layer 1 クライアントの HTTP 送信ソケットバッファ（実命令発行レイヤ）へ到達したパルス漏洩計数値 $C_{leak}$ の計測。 $10^6$ 回の超高密度パルス駆動ストレステストの終了後において、カウンターが厳密に $C_{leak} \equiv 0$ を維持していること、障壁の透過確率（トンネルフラックス）が極値統計限界の下で $P_{tunnel} < 10^{-15}$ 以下に漸近していることの、実回路レベル防衛線ベリフィケーション。

#### チケット M4-4: エキスパート融合誕生（`BirthCommit`）時における両ストアアトミック永続化不変条件の最終検証

* **対象不変条件 / 規範:** §37 & §37.1 Birth Commit Discipline、§41.1 融合不変条件（Source-of-truth preservation）。加えて v2.3 では、`BirthNeedsRepair` / `BirthQuarantined` の状態にある fusion result は retrieval / selection path に露出してはならず、startup repair scan により安全状態へ収束させなければならない。v2.3-g (§12C) DarviumEventBus 上に `DarviumEventKind::Fusion` イベント（出生成功／失敗／検疫）を発行し、EventProjection である FusionTrace として追跡可能でなければならない。
* **実装スコープ:** 複数のリポジトリペアから特定のExpert Namespaceを抽出し、新しいリポジトリ（`FusionResultPair`）を安全に誕生させる `finalize_birth` オーケストレーターの最深部。加えて、birth failure 後の状態が startup repair worker に捕捉されることを前提に、repair / quarantine discipline と接続する。すべての birth commit 試行（成功・失敗・検疫）は `DarviumEventKind::Fusion` として DarviumEventBus に発行し、FusionTrace EventProjection 経由で参照可能にする。
* **テストコードによる検証:**

1. コミット処理を実行し、SQLite側（メタデータ）とLadybugDB側（グラフ構造・エビデンス）の双方が成功した場合にのみ、新しいリポジトリの誕生状態が `BirthCommitted` に遷移することを確認。加えて `BirthCommitted` への遷移と同時に `DarviumEventKind::Fusion` イベント（種別 `FusionBirthSucceeded`）が EventBus に発行されることをアサート。
2. 片側の永続化処理（例: LadybugDBのノードインサート）で意図的にエラーを発生させた場合、コミットが即座にアボートされ、誕生状態が `BirthNeedsRepair` または `BirthQuarantined` に隔離され、中途半端に壊れたリポジトリがプロダクションの retrieval selection path に絶対に露出しないことを厳格にアサートする。加えて、アボート時に `DarviumEventKind::Fusion` イベント（種別 `FusionBirthFailed` または `FusionBirthQuarantined`）が EventBus に発行されることを確認。さらに、シミュレートされた再起動後に startup repair scan がこれらの状態を捕捉し、安全状態へ収束させることも確認して、全フェーズのテストファースト実装を完了する。

* **計装方法・観測対象:** エキスパート融合オーケストレーターが `finalize_birth` を実行する直前の materialization フェーズにおいて、1ミリ秒から1ナノ秒単位の間隔で疑似的なハードウェアクラッシュパルス（プロセス強制終了シグナル）を割り込ませる、全 5,000 パターンのタイムスライス破壊アンサンブルを走行。システム復旧後における、SQLite側（メタデータ）と LadybugDB側（知識オブジェクト）の状態の空間配置（相分離プロファイル）。不整合状態を検知した復旧システムが Startup Repair Scan を通じて安全な定常状態（Tombstone / Quarantined）へ収束するまでの回復ダイナミクスを追跡。不整合ポテンシャルが完全にゼロにクエンチされるまでの修復減衰定数 $\Gamma$ に対し、クラッシュのタイムスライス位置（ナノ秒軸上の割り込みタイミング）による $\Gamma$ の不連続な変動包絡線を同定。双方のストアの状態ベクトルをプロットした際、システム状態が `BirthCommitted` または `BirthNeedsRepair / BirthQuarantined` のいずれかの極小値エネルギー状態（アトラクタ）の一方に 100% 完全相分離され、片側だけが成功した不整合状態の残存フラックス（発生確率）が全アンサンブルをとおして厳密に $P_{defect} \equiv 0.00000$ であることの、多重定常動態の最終ベリフィケーション。

---

### 12. マイルストーン M2.75：v2.3 較正・並列実行・運用境界の補強

> **DB**: 依然としてメモリ内完結。SQLite / LadybugDB 不要。v2.3 で新たに強調された ranking stability / training plane / frontier concurrency の観測と規範検証を行う。

#### チケット M2.75-1: GED境界近傍における ranking stability / false-new rate の較正テスト

* **対象不変条件 / 規範:** v2.3 で強化された補助性能指標（ranking stability, false-new rate, small patch drift 観測）、および `GED_GRAPH_SIZE_LIMIT` 近傍での ranking drift / oscillation 観測要求。
* **実装スコープ:** 小さなグラフ編集距離（GED）を持つ候補群を生成し、reuse / patch / new のランキングと選択結果が小変動に対してどの程度安定であるかを計測する property-based calibration harness を追加する。`SearchTrace` と candidate ranking 出力から、false-new rate、top-k 順位変動、decision oscillation を導出する。
* **テストコードによる検証:** ほぼ同一な候補グラフに対して微小な GED 摂動のみを加えたアンサンブルを多数生成し、選択結果が過敏に `New` へ飛びやすくなっていないこと、ならびに top-ranked candidate が微小変動で頻繁に反転しないことを確認する。
* **計装方法・観測対象:** `GED_GRAPH_SIZE_LIMIT` 近傍のサイズ帯で、グラフ編集距離 $d_{GED} \in [0, d_{max}]$ を連続的に変化させた候補群を生成する。選択結果系列から false-new rate、top-1 survival rate、順位相関係数、decision oscillation frequency を抽出し、$d_{GED}$ に対する感度曲線をプロットする。小摂動領域で ranking drift が非連続に跳ね上がらず、較正対象として扱える滑らかな応答曲線を持つことを確認する。

#### チケット M2.75-2: Training Plane safe-scope auto-approval と production plane review 強制の分離検証

* **対象不変条件 / 規範:** v2.3 の Training Plane / safe sandbox 例外規範。production plane では副作用を持つ `GenerateNew` は human review を要し、training plane の safe sandbox に限って明示的に安全な scope のみ auto-approval を認めうる。
* **実装スコープ:** proposal に対して plane 属性（production / training-sandbox）と side-effect profile を付与し、review 強制・auto-approval 例外・拒否のいずれになるかを決定する policy classifier を実装する。`HumanReviewQueue` と連携し、review queue depth の観測基盤も整備する。
* **テストコードによる検証:** 同一 proposal であっても、production plane では review queue へ送られ、training plane の safe sandbox では auto-approval 条件を満たすときのみ実行継続が許されることを確認する。safe scope 外の training proposal は review または拒否へ落ちることも対照的に確認する。
* **計装方法・観測対象:** plane 属性と副作用ベクトルの全組合せに対して policy classifier を走らせ、review queue ingress rate、safe-scope auto-approval fraction、誤 auto-approval 率、queue depth 変動を観測する。training sandbox 例外が review backlog を減らしつつ、unsafe proposal を production path へ漏らさないことを統計的に検証する。

#### チケット M2.75-3: frontier-based parallel execution の ready set partition 検証

* **対象不変条件 / 規範:** v2.3 で規範化された frontier-based parallel execution。ready frontier 上のノード集合は、構造上独立であり、かつ `SideEffectSet` / `ErrorMode` / `CollectStrategy` 上安全なものに限って concurrency-admissible batch として並列実行されるべきである。
* **実装スコープ:** `WorkflowGraph` / `SearchWorkflowGraph` から current frontier を抽出し、各ノードの incoming 依存解消状態、side effect profile、error mode、collect/fan-out 境界を参照して parallel batch へ分割する scheduler 補助関数を実装する。`petgraph` は DAG 性検証および frontier 形成の基礎に用い、最終的な batch 判定は RFC 規範に従う。
* **テストコードによる検証:** 互いに依存を持たない read-only ノード群が同一 batch にまとめられること、`irreversible` / `writes_external_api` / persistent mutation を含むノードが別 batch へ分離されること、`Conditional` 未解決や `Collect(WaitAll)` 未充足のノードが ready 扱いされないことを確認する。さらに、単なる `toposort` 線形化では concurrency-admissible set を十分に表現できず、frontier-based scheduling が必要であることを例示ケースで検証する。
* **計装方法・観測対象:** ランダム DAG アンサンブル上で ready frontier サイズ、実際に形成された並列 batch 数、serial fallback 数、side-effect 競合による分離率を記録する。frontier のサイズ分布と batch partition の関係、ならびに serial-only 実行と比較した並列度の差分を観測し、v2.3 の実行意味論における「並列可能なものは安全条件下で並列化されるべき」という規範の検証基盤とする。

#### チケット M2.75-c-1: ConversationsPort トレイト定義 & 会話型データ構造体

* **対象不変条件 / 規範:** v2.3-c §16B.1–§16B.7 で規範化された全会話型（ConversationalEvent, ConversationalIngestionPolicy, ConversationCategoryRule, ConversationalKnowledgeCategory 他9の enum/struct）、§16B.1 の「LLM は trigger phrase detector ではなく policy-conditioned classifier として動作する」原則、v2.3-g §12C `DarviumEventKind::Conversational` との整合、および Table Spec §5 の型定義。全型が Rust の型システムで表現可能であり、Fake-First 原則に従いポートトレイトを分離すること。
* **実装スコープ:**
  - `ConversationsPort` トレイト: `fn ingest_event(&self, event: ConversationalEvent) -> Result<String, DarviumError>`、`fn get_proposal(&self, event_id: &str) -> Option<ConversationalClassificationProposal>`、`fn record_gate_decision(&self, decision: &ConversationalGateDecision) -> Result<(), DarviumError>`、`fn query_fragments(&self, namespace: &str, category: Option<ConversationalKnowledgeCategory>) -> Vec<ConversationalFragmentMeta>`、`fn query_policy(&self, policy_id: &str) -> Option<ConversationalIngestionPolicy>`
  - `ConversationalEvent` ↔ `DarviumEvent` 変換: `ConversationalEvent` の ingest 時に `DarviumEventKind::Conversational` の DarviumEvent を EventBus へ publish
  - `FakeConversationsPort`: 上記全メソッドを `HashMap<String, ...>` で実装したメモリ内実装。EventBus publish の Fake 実装を含む
  - 全会話型の Rust struct / enum 定義（RFC §16B.1–§16B.7 に従い、DarviumError の `ConversationalIngestionError` バリアントも追加）
  - `DarviumError` に `ConversationalIngestionError(String)` バリアント追加
* **テストコードによる検証:**
  1. 全 11 struct + 9 enum が `#[derive(Debug, Clone, PartialEq)]` を実装可能であることのコンパイル時確認
  2. `FakeConversationsPort` が `ConversationsPort` トレイト境界を充足することのコンパイル時検証
  3. `trait` が `dyn ConversationsPort` としてオブジェクト安全であることの確認（`Box<dyn ConversationsPort>`）
  4. `ConversationalEvent` → `ingest_event()` → `get_proposal()` の一連の操作がメモリ内で一貫していること（read-after-write consistency）
  5. `policy_score = 0.85` の提案に対し `record_gate_decision()` が正しく `Drop` / `CreateTrainingMissionAndFragment` を記録できること
  6. カテゴリ `Noise` の提案に対するゲート判断が `Drop` として記録されること
* **計装方法・観測対象:** 全トレイトメソッドの呼び出しを `HashMap` 操作のカウンタで計測する。型定義の完全性は、Table Spec §5 の全フィールドに対して Rust 構造体に同名・同型のフィールドが存在することを人手照合可能な一覧として記録する。`FakeConversationsPort` の呼び出し記録により、トレイト経由の全操作が `FakeConversationsPort` の内部状態に対して完全に再現可能であること（非決定論的外部依存ゼロ）を確認する。

#### チケット M2.75-c-2: LlmProposalPort トレイト定義 & FakeLlmProposer

* **対象不変条件 / 規範:** v2.3-c §16B.1「LLM による policy-conditioned classification proposal」の分離原則。LLM 側は非決定論的であってよいが、ポート境界で提案を構造化し、FakeLlmProposer は決定論的に振る舞わなければならない。§16B.2 Editorial requirement「classification proposal MAY be nondeterministic, but persistence, state transition, namespace assignment, promotion eligibility, and canonical exposure SHALL be governed by deterministic gates」の実現基盤。
* **実装スコープ:**
  - `LlmProposalPort` トレイト: `fn classify_conversational_event(&self, event: &ConversationalEvent, categories: &[ConversationalKnowledgeCategory]) -> ConversationalClassificationProposal`
  - `FakeLlmProposer`: 発話内容のハッシュ値（`SipHash`）をシードに `StdRng` で決定論的カテゴリを割り当てる実装。各カテゴリの出現確率は設定可能な重みベクトル $W = (w_{UserProfile}, w_{UserPreference}, ..., w_{Unknown})$ で制御し、$\sum w = 1.0$ となるよう正規化する。`policy_score` はカテゴリ別の平均 $\mu_c$ と分散 $\sigma_c^2$ からサンプリングする。
  - `LlmProposalConfig` 構造体: `category_weights: HashMap<ConversationalKnowledgeCategory, f32>`, `category_score_params: HashMap<ConversationalKnowledgeCategory, (f32, f32)>`, `pii_probability: f32`, `temporality_weights: HashMap<InferredTemporality, f32>`, `scope_weights: HashMap<InferredScope, f32>`, `promotion_hint_weights: HashMap<PromotionEligibilityHint, f32>`, `seed: u64`
* **テストコードによる検証:**
  1. `FakeLlmProposer` が同じ入力（`(utterance, seed)` の組）に対して常に同一の `ConversationalClassificationProposal` を返す決定論的再現性の確認（$n = 1000$ 回の反復で `assert_eq!`）
  2. 異なる発話内容に対して、カテゴリ分布が設定した重み $W$ に従うことの統計的検定（カイ二乗適合度検定、有意水準 $\alpha = 0.01$）
  3. `pii_probability = 0.0` の設定で `contains_pii` が常に `false` になること
  4. `pii_probability = 1.0` の設定で `contains_pii` が常に `true` になること
  5. `seed` 変更により異なる系列が生成されること（系列間の順位相関係数 $\tau < 0.3$ で確認）
* **計装方法・観測対象:** `FakeLlmProposer` の分類結果系列 $\{c_1, c_2, ..., c_n\}$ ($n \ge 10000$) を固定シード `StdRng::seed_from_u64(config.seed)` で生成し、各カテゴリの出現比率 $\hat{p}_k = count(c_i = k) / n$ と設定重み $w_k$ の間の KL ダイバージェンス $D_{KL}(W || \hat{P}) = \sum_k w_k \log(w_k / \hat{p}_k)$ を計測する。$D_{KL} < 0.01$ で重み設定が正確に反映されていることを検証する。また、`policy_score` のカテゴリ別標本平均 $\bar{x}_c$ が設定平均 $\mu_c$ に対して $|\bar{x}_c - \mu_c| < 0.05$ であることを $n \ge 1000$ で確認する。

#### チケット M2.75-c-3: ConversationalGate 決定論的判定エンジン

* **対象不変条件 / 規範:** §16B.2 の `decide_conversational_ingest()` 擬似コードの完全かつ正確な実装。決定論的ゲートは以下の不変条件を強制する: (a) `Noise` / `Unsafe` カテゴリは必ず `Drop` される (MUST)、(b) PII ポリシー `Reject` 時は必ず `Drop` される (MUST)、(c) `policy_score < min_policy_score` では必ず `Drop` される (MUST)、(d) `llm_confidence < rule.minimum_llm_confidence` では必ず `CreateTrainingMission` かつ `requires_human_review = true` となる (MUST)、(e) 同一提案に対する判定結果は常に一意かつ再現可能である (MUST)。§16.2 の境界図「LLM (may be nondeterministic) → Deterministic Gate (code path)」の分離原則。
* **実装スコープ:**
  - `decide_conversational_ingest(event, proposal, policy) -> ConversationalGateDecision` 純粋関数（外部状態・IO 依存ゼロ）
  - `lookup_category_rule(policy, category) -> ConversationCategoryRule` 補助関数
  - `new_training_mission_id() -> String` 補助関数（ULID 生成、現段階では `FakeClock` の時刻から生成）
  - `drop_decision(event, reason_code) -> ConversationalGateDecision` 補助関数
  - ゲート判定結果を `DarviumEventKind::Conversational` イベントとして EventBus へ publish する emit 機能（`emit_gate_decision(event, decision)`）
  - `ConversationalGateAction` 6 値の網羅的遷移カバレッジを保証するテスト母体
  - `ConversationalGateReasonCode` enum（`CATEGORY_REJECTED`, `PII_REJECTED`, `POLICY_SCORE_TOO_LOW`, `LOW_CONFIDENCE_REVIEW_REQUIRED`, `SANDBOX_AUTO_INGEST`, `REVIEW_GATED_INGEST`）
* **テストコードによる検証:**
  1. 全 11 カテゴリ $\times$ `policy_score $\in$ {0.0, 0.5, 1.0}` $\times$ `llm_confidence $\in$ {0.0, 0.5, 1.0}` $\times$ `contains_pii $\in$ {true, false}` $\times$ `pii_handling $\in$ {Reject, RedactBeforePersist, AllowSandboxOnly}` $\times$ `auto_ingest_to_sandbox $\in$ {true, false}` $\times$ `allow_auto_sandbox_ingest $\in$ {true, false}` の網羅的組合せ（11 $\times$ 3 $\times$ 3 $\times$ 2 $\times$ 3 $\times$ 2 $\times$ 2 = 2376 ケース）を自動生成し、期待される `ConversationalGateAction` が出力されることを確認
  2. カテゴリ `Noise` / `Unsafe` の全ケースで `action == Drop` かつ `reason_code == "CATEGORY_REJECTED"` であること
  3. `contains_pii == true && pii_handling == Reject` の全ケースで `action == Drop` かつ `reason_code == "PII_REJECTED"` であること
  4. `policy_score < min_policy_score` の全ケースで `action == Drop` かつ `reason_code == "POLICY_SCORE_TOO_LOW"` であること
  5. `llm_confidence < rule.minimum_llm_confidence` の全ケースで `action == CreateTrainingMission` かつ `requires_human_review == true` であること
  6. `auto_ingest_to_sandbox == true && allow_auto_sandbox_ingest == true` で他の条件がすべて合格している場合、`action == CreateTrainingMissionAndFragment` であること
  7. すべての `reason_code` 値（6種）が網羅されていること
  8. 同一入力に対する再現性: 各ケースを $n = 10$ 回繰り返し、全ての出力フィールドが完全一致すること
* **計装方法・観測対象:** ゲート判定の入力条件空間 $X$ を 7 次元超直方体 $X = C_{11} \times S_{[0,1]} \times C_{[0,1]} \times B_{pii} \times P_{pii} \times B_{auto} \times B_{allow}$ と定義し、各軸の離散点で全網羅 $|X_{grid}| = 2376$ ケースを実行する。出力 action の経験分布 $\hat{P}(action | X_{grid})$ を観測し、以下のサブグループ比率が仕様と合致することを確認:
  - `Drop` 比率 = $|{x: reason_code \in \{``CATEGORY_REJECTED", ``PII_REJECTED", ``POLICY_SCORE_TOO_LOW"\}}| / |X_{grid}|$
  - `CreateTrainingMission`（low confidence）比率 = $|{x: reason_code = ``LOW_CONFIDENCE_REVIEW_REQUIRED"\}| / |X_{grid}|$
  - `CreateTrainingMissionAndFragment`（sandbox auto-ingest）比率 = $|{x: reason_code = ``SANDBOX_AUTO_INGEST"\}| / |X_{grid}|$
  - `CreateTrainingMission`（review-gated）比率 = $|{x: reason_code = ``REVIEW_GATED_INGEST"\}| / |X_{grid}|$
  さらに、決定論的ゲート関数 $f: X \rightarrow ConversationalGateDecision$ の出力を 3 回の独立実行で比較し、全結果が厳密に等しいこと ($f_1(x) = f_2(x) = f_3(x), \forall x \in X_{grid}$) を確認することで、非決定論要因の完全排除を検証する。

#### チケット M2.75-c-4: フラグメント管理と Consolidation エンジン

* **対象不変条件 / 規範:** §16B.5 の統合条件（multi-turn/multi-day consolidation policy）。フラグメントは ConsolidationPolicy で宣言された閾値をすべて満たすまで CandidateKnowledgeDocument に束ねてはならない (MUST NOT)。§17 第6 Invariant「全4段階を経なければ production canonical knowledge に到達してはならない」のうち第3段階（Fragment→CandidateKnowledgeDocument）を実装する。矛盾スコアが `max_contradiction_score` を超える場合のデフォルト安全動作は coexistence + lineage relation であり、destructive merge は禁止 (MUST NOT)。
* **実装スコープ:**
  - `ConversationalFragmentRegistry`: フラグメントの作成、更新、有効期限切れ、カテゴリ別・名前空間別の問合せを扱うメモリ内レジストリ
  - `ConsolidationCandidateAssembler`: 同一名前空間・同一カテゴリのフラグメント群から ConsolidationCandidateSet を生成する関数。各メトリクス（`distinct_event_count`, `distinct_day_count`, `semantic_coherence`, `trace_completeness`, `temporal_stability`, `contradiction_score`）の計算ロジックを含む。現フェーズでは `semantic_coherence` / `trace_completeness` / `temporal_stability` / `contradiction_score` は `FakeLlmProposer` の出力分布からの派生値として決定論的に計算する。
  - `consolidation_eligible(candidate: &ConsolidationCandidateSet, policy: &ConsolidationPolicy) -> (bool, Vec<String>)`: 全閾値判定関数。不合格の場合は理由コード一覧を返す。
  - `ConsolidationAction` enum: `EligibleForCandidate`, `InsufficientEvents`, `InsufficientDays`, `InsufficientCoherence`, `InsufficientTrace`, `InsufficientStability`, `ExcessiveContradiction`, `Coexistence`, `HumanReviewRequired`
* **テストコードによる検証:**
  1. 全閾値を満たす `ConsolidationCandidateSet`（例: `distinct_events=5, distinct_days=3, coherence=0.85, trace=0.90, stability=0.80, contradiction=0.10`）が `consolidation_eligible() == true` となること
  2. `distinct_event_count < min_distinct_events` のとき不合格となること（他の条件は全て満たす状態で）
  3. `distinct_day_count < min_distinct_days` のとき不合格となること
  4. `semantic_coherence < min_semantic_coherence` のとき不合格となること
  5. `trace_completeness < min_trace_completeness` のとき不合格となること
  6. `temporal_stability < min_temporal_stability` のとき不合格となること
  7. `contradiction_score > max_contradiction_score` のとき `auto_canonicalization` が禁止され、代わりに `Coexistence` または `HumanReviewRequired` が選択されること
  8. `require_origin_trace == true` で `trace_completeness < 1.0` のとき不合格となること
  9. フラグメントが `Tombstoned` または TTL 超過時、レジストリの問合せ結果から除外されること（GC 動作）
* **計装方法・観測対象:** 評価関数 $g: S \times P \rightarrow \{合格, 不合格\}$ の出力を $S$（候補セット空間）と $P$（ポリシー空間）の直積上で系統的にサンプリングする。各次元を独立に変化させた 1-at-a-time 感度分析により、threshold boundary 近傍 ($\theta_i \pm \delta, \delta = 0.01$) での判定の不連続点を検出する。統合比率 $R_{consolidate} = |\{s: g(s, p) = 合格\}| / |\{s\}|$ を $n \ge 10000$ のランダム候補セットアンサンブル上で観測し、`ConsolidationPolicy` のデフォルト値における期待統合率を求める。矛盾スコアが `max_contradiction_score` を超える $n \ge 1000$ ケースで、destructive merge（観測値として同一知識オブジェクトへの強制統合）の発生率が厳密に 0 であることを確認する。

#### チケット M2.75-c-5: ConversationalPromotionGate 昇格判定

* **対象不変条件 / 規範:** §16B.7 の昇格条件。conversational-origin CandidateKnowledgeDocument が CanonicalDocument へ昇格するためには 9 条件すべての連言 (conjunction) を満たさなければならない (MUST)。具体的には:
  1. `promotion_status == Approved`
  2. `completeness_score >= 0.80`
  3. `trace_completeness >= 0.80`
  4. `contradiction_score <= 0.20`
  5. `distinct_day_count >= 2`
  6. `training_good_ratio >= TRAINING_PROMOTION_MIN_GOOD_RATIO (= 0.70)`
  7. `sandbox_success_rate >= TRAINING_PROMOTION_MIN_SUCCESS_RATE (= 0.80)`
  8. `requires_human_review == false` または human approval 記録済み
  9. dual-store commit intent（単一 `op_id`）が生成済み
  一条件でも不足する場合は昇格してはならない (MUST NOT)。また §16B.7「conversational-origin knowledge MUST NOT become a CanonicalDocument without first passing through a CandidateKnowledgeDocument stage」の強制。
* **実装スコープ:**
  - `promotion_eligible(gate: &ConversationalPromotionGate, policy: &ConversationalIngestionPolicy, human_approved: bool, has_commit_intent: bool) -> (bool, Vec<String>)`: 9 条件の連言判定関数。不合格理由の一覧を返す。
  - `PromotionGateScore` 構造体: 各条件の充足状況と全体スコアを保持
  - `evaluate_candidate(candidate: &CandidateKnowledgeDocument, training_feedback: &[TrainingFeedback], sandbox_runs: &[TrainingRunLog]) -> ConversationalPromotionGate`: CandidateKnowledgeDocument と Training Plane の実績から ConversationalPromotionGate を生成する評価関数
* **テストコードによる検証:**
  1. 全 9 条件を満たす入力に対して `promotion_eligible() == true` となること
  2. 各条件を 1 つだけ欠いた 9 通りの入力を生成し、それぞれ `promotion_eligible() == false` かつ理由コードに該当条件の識別子が含まれること
  3. `promotion_status != Approved` のとき、他の全条件を満たしても不合格となること
  4. `requires_human_review == true && human_approved == false` のとき不合格となること
  5. `has_commit_intent == false` のとき不合格となること
  6. `training_good_ratio` を 0.0 から 1.0 まで 0.05 刻みで変化させ、`TRAINING_PROMOTION_MIN_GOOD_RATIO = 0.70` を境に判定が切り替わることの確認
  7. `sandbox_success_rate` を同様に 0.0 から 1.0 まで変化させ、`TRAINING_PROMOTION_MIN_SUCCESS_RATE = 0.80` を境に判定が切り替わることの確認
* **計装方法・観測対象:** 昇格判定関数 $h: G \rightarrow \{true, false\}$ を $G = S_{completeness} \times S_{trace} \times S_{contradiction} \times N_{days} \times S_{good} \times S_{success} \times B_{review} \times B_{approval} \times B_{commit}$ の9次元空間上で評価する。各次元の閾値境界近傍 ($\theta_i \pm \varepsilon, \varepsilon = 0.01$) における判定の一致率（閾値未満で false、閾値以上で true）を $n = 1000$ サンプルで検証する。昇格率 $R_{promote} = |\{g: h(g) = true\}| / |\{g\}|$ を閾値デフォルト値設定下のランダム候補アンサンブル $n = 10000$ で観測し、期待昇格率を特徴づける。`training_good_ratio` に対する昇格率の感度関数 $S(\theta_{good}) = dR_{promote} / d\theta_{good}$ を数値微分により推定し、閾値近傍での判定の不連続ジャンプが急峻であること ($|S(\theta_{good})| > 10.0$ at $\theta_{good} = 0.70$) を確認する。

#### チケット M2.75-c-6: 会話インジェスション End-to-End フロー結合実験

* **対象不変条件 / 規範:** §17 第6 Invariant「Conversational Ingestion Invariant — conversational origin knowledge は ConversationalEvent → Fragment/SandboxMemoryEvent → CandidateKnowledgeDocument → CanonicalDocument の全4段階を経なければ production canonical knowledge に到達してはならない (MUST NOT)。いずれかの段階をスキップして直接 production canonical knowledge を生成する経路は、gate の存在如何にかかわらず禁止する。」v2.3-g では ConversationalEvent は `DarviumEventKind::Conversational` として EventBus へ publish され、各段階のゲート判定結果も DarviumEvent として記録される。§16B.5 の図書館化段階規約（4段階パイプライン + 段階間 lineage）。全段のゲートが正しく接続されていることの統合検証。
* **実装スコープ:**
  - v2.3-g EventBus 統合: ConversationalEvent の ingest 時に `DarviumEventKind::Conversational` イベントを EventBus へ publish。各ゲート通過・遮断・エラーを対応する DarviumEvent として記録。
  - `ConversationalIngestionPipeline`: `FakeConversationsPort`, `FakeLlmProposer`, `decide_conversational_ingest()`, `ConversationalFragmentRegistry`, `ConsolidationCandidateAssembler`, `promotion_eligible()` を直列接続するパイプラインオーケストレーター
  - `PipelineConfig`: 全段のポリシー設定、LlmProposalConfig、ConsolidationPolicy を保有
  - `pipeline_step(event) -> StepResult`: 1イベントをパイプラインに通し、各段の出力を EventBus へ publish する関数
  - `PipelineObserver`: 各段の通過・遮断・エラーを EventBus 経由のイベント系列として記録する観測器
  - `SyntheticConversationGenerator`: 固定シード `StdRng` で発話系列（ユーザー発話、カテゴリラベル付き）を生成する。カテゴリ分布、1日あたりイベント数、日数跨ぎパターンを制御可能。
* **テストコードによる検証:**
  1. `SyntheticConversationGenerator` が同一シードから同一系列を生成する決定論的再現性の確認
  2. 全イベントが `Noise` / `Unsafe` カテゴリに分類される合成会話系列（$n = 100$）を投入し、全イベントが第1段階（Gate）で `Drop` され、以降の段階に一切到達しないことの確認（$C_{leak} = 0$）
  3. 高価値カテゴリ（`UserProfile`, `FactualClaim`）のみで構成される合成会話系列（$n = 100$, 最低3日跨ぎ）を投入し、一定数のイベントが Consolidation を経て CandidateKnowledgeDocument まで到達することを確認
  4. `allow_auto_sandbox_ingest = false` の設定で、`CreateTrainingMission` のみ発行され `CreateTrainingMissionAndFragment` が発行されないことの確認
  5. `min_policy_score = 1.0`（全拒否設定）で全イベントが第1段階で `Drop` されることの確認
  6. 全段パイプライン通過後も、production namespace への直接書き込みが一度も発生していないことの確認（`ConversationsPort` の記録から検証）
* **計装方法・観測対象:** 合成会話系列 $\{e_1, e_2, ..., e_n\}$ を固定シードで $n = 1000$ 生成し、各イベントのパイプライン通過経路を段階別状態ベクトル $v_i = (a_{gate}, a_{fragment}, a_{consolidation}, a_{promotion})$ で記録する。各 $a_{stage}$ は当該段階を通過したか (1) ・遮断されたか (0) を示すバイナリ値である。全イベントの段階別通過率 $\hat{p}_{stage} = \sum_i a_{i,stage} / n$ を観測し、以下の制約が成立することを確認:
  - $\hat{p}_{gate} = 1.0$（全イベントが少なくとも Gate を通過する）
  - $\hat{p}_{drop} + \hat{p}_{gate-pass} = 1.0$（Gate 通過か Drop かは排反かつ完全）
  - $\hat{p}_{canonical} \le \hat{p}_{candidate} \le \hat{p}_{fragment} \le \hat{p}_{gate-pass}$（monotonic stage-pass constraint）
  さらにパイプライン全体のスループット $T_{pipe} = n / t_{total}$（/μs）を計測し、全段結合時のオーバーヘッドが線形 $O(n)$ であること、および段間の中間状態数がイベント数に対して劣線形 $O(\log n)$ であることを観測する。段階をスキップする不正経路（`CanonicalDocument` を Gate 通過のみで生成する等）の試行を $n = 100$ 回注入し、すべてがコンパイル時または実行時に拒否されることを $P_{bypass} = 0$ として検証する。

#### チケット M2.75-c-7: 会話閾値パラメータの感度分析・較正実験

* **対象不変条件 / 規範:** v2.3-c で追加された7定数（§22 A.x）の較正可能性。これらの定数は Calibration Candidates に分類され、実験的チューニング対象である。ただし Safety Invariants に分類されるべき性質（矛盾時 coexistence、4段階スキップ禁止）の変更は許可されない。較正ループは `[仮説] \rightarrow [定数変更] \rightarrow [cargo test] \rightarrow [観測] \rightarrow [解釈] \rightarrow [記録] \rightarrow [反復]` の形式に従う。
* **実装スコープ:**
  - `ConversationalCalibrationHarness`: M2.75-c-6 のパイプラインをパラメータ化し、7定数の任意の組合せで実行可能な実験ハーネス
  - 目的関数 $J_{conv}(\theta) = \alpha_1 \cdot R_{consolidate}(\theta) + \alpha_2 \cdot R_{promote}(\theta) - \alpha_3 \cdot T_{latency}(\theta) - \alpha_4 \cdot P_{bypass}(\theta)$ の定義（$\theta$ は7次元パラメータベクトル）
  - デフォルト重み: $\alpha_1 = 0.3, \alpha_2 = 0.4, \alpha_3 = 0.2, \alpha_4 = 0.1$（較正候補）
  - 1-at-a-time 感度分析: 各パラメータ $\theta_i$ をデフォルト値 $\theta_i^{(0)}$ の $\pm 50\%$ 範囲で変化させ、他の6パラメータを固定した際の $J_{conv}$ の変動を記録
  - 実験系列管理: 各実行に実験ID `exp-{yyyymmdd}-{seq}` と親実験IDを付与
* **テストコードによる検証:**
  1. デフォルト定数設定下で $n = 5000$ イベントのパイプライン実験を3回繰り返し、$J_{conv}$ の実験間変動係数 $CV = \sigma / \mu < 0.05$ であることの確認（結果の再現性）
  2. `CONVERSATIONAL_CONSOLIDATION_MIN_EVENTS` を 1 から 10 まで変化させたとき、$R_{consolidate}$ が単調減少することの確認（$R_{consolidate}(k) > R_{consolidate}(k+1)$ for all $k$）
  3. `CONVERSATIONAL_CONSOLIDATION_MIN_COHERENCE` を 0.0 から 1.0 まで 0.1 刻みで変化させたとき、$R_{consolidate}$ が単調減少することの確認
  4. `CONVERSATIONAL_CONTRADICTION_COEXISTENCE_DEFAULT = true` 設定下で、矛盾スコア超過時に destructive merge が発生しないことの確認（$n = 1000$）
  5. `CONVERSATIONAL_CONTRADICTION_COEXISTENCE_DEFAULT` を `false` に変更不可能であること（Safety Invariant であり、コンパイル時または不変条件テストで拒否されること）の確認
* **計装方法・観測対象:** パラメータ空間 $\Theta \subset \mathbb{R}^7$ 上で以下の観測を行う:
  1. **1-at-a-time 感度曲線**: 各 $\theta_i$ を $[\theta_i^{(0)} \times 0.5, \theta_i^{(0)} \times 1.5]$ の範囲で20等分した点で $J_{conv}$ を評価し、感度 $S_i = \partial J_{conv} / \partial \theta_i$ を中心差分 $S_i(\theta_i) \approx (J_{conv}(\theta_i + h) - J_{conv}(\theta_i - h)) / (2h)$ で推定する。$|S_i| > 0.5$ のパラメータを高感度パラメータとして同定する。
  2. **目的関数地形**: デフォルト値近傍 $\theta_i \in [\theta_i^{(0)} \times 0.8, \theta_i^{(0)} \times 1.2]$ の超直方体領域でラテン方格サンプリング $n = 200$ 点を実行し、$J_{conv}(\theta)$ の経験的分布（平均・標準偏差・分位数）および大域的最大値 $\theta^* = argmax_\theta J_{conv}(\theta)$ を推定する。
  3. **較正推奨値**: 感度分析と目的関数地形から、$J_{conv}$ を最大化するパラメータ設定値とその信頼区間を報告する。デフォルト値からの乖離が $\theta^*$ において統計的に有意であること（$p < 0.05$、Welch の t 検定）を付記する。
  各実験の結果は実験系列として記録され、実験ID・親実験ID・パラメータ設定・$J_{conv}$ 値・感度ベクトル $S$ の完全なトレーサビリティを維持する。


---

## 💡 開発チームへの実装展開ガイド

このチケット分解により、開発チームは以下のステップで機械的に開発を進めることができます。

1. **チケットの順番通りに Rust の `tests/` ディレクトリに空のテスト関数（`#[test]`）を作成する。**
2. テストをパスさせるために必要な**最小限のデータ構造と純粋関数**を `src/` 側に記述する。
3. M-0.5 に達した段階で、`rand::rngs::StdRng` を用いたシード固定の確率的テストを導入し、ノイズに対するシステムの耐久性を高める。
4. M2 に到達するまでは、PCのネットワークを切断した状態（完全ローカル環境）であっても `cargo test` が100%グリーンかつミリ秒単位で高速作動する状態を維持する。
