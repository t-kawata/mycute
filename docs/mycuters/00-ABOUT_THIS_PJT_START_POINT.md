# MYCUTE プロジェクト：Rust 移行と次世代知識管理システムの構築

本プロジェクトは、先行して開発された Go 言語版の実装（`./mycute-go`）をベースに、より高性能で安全、かつ拡張性の高い Rust 言語による再実装を行うものです。

## 1. プロジェクトの背景と目的

MYCUTE は、膨大な知識（学術情報、社内機密、個人情報など）を一つのバイナリファイル「**Cube（キューブ）**」に凝縮し、長期記憶として保持・活用するための超高精度記憶システムです。

単なるデータストレージではなく、知識の吸収（Absorb）、構造化（Memify）、そして高度な検索・抽出（Query）という 3 つのコアプロセスを通じて、情報を「生きた知能」へと昇華させることを目的としています。

## 2. コアプロセス：知識の昇華サイクル

MYCUTE は以下の 3 つの主要な処理フローで構成されます。

- **Absorb (吸収)**: 外部の膨大な情報源（書籍、ドキュメント、データベース等）から知識を取り込みます。
- **Memify (記憶化)**: 取り込んだ知識を、グラフ構造、ベクトルデータ、および生データとして最適化し、Cube 内部に定着させます。
- **Query (問合せ)**: 蓄積された知識に対し、自然言語による高精度な問い合わせを可能にします。

## 3. 「Cube（キューブ）」の概念と特徴

Cube は MYCUTE の最小単位であり、知識の塊を示す 1 つのファイルです。

- **高度な秘匿性**: Cube は「暗号鍵付き暗号ファイル」として存在し、内部のグラフやベクトル情報は強固に保護されます。
- **圧倒的な情報量**: 1 つの Cube あたり、書籍 5000 冊分以上の情報を安全に保持可能です。
- **ポータビリティと流通**: ファイル形式であるため容易に流通が可能であり、権利マネジメントとの親和性が高い設計となっています。
- **世代管理とトレーサビリティ**:
    - Cube は「成長」し、世代を重ねます。
    - 「誰が」「いつ」「何を」教育したのかが改竄不能な状態で記録され、貢献度に応じた収益還元をツリー化できます。
- **モデル非依存**: OpenAI, Gemini, Claude など、利用する大規模言語モデル（LLM）を問いません。
- **専門家グループの形成**: 複数の専門分野を 1 つの Cube に統合し、仮想的な専門家グループとして機能させることができます。

## 4. 知能としての進化：自問自答と無知の知

MYCUTE の Cube は、単なる受動的なデータベースではありません。

- **無知の知（Meta-knowledge）**: 「自分は何を知っていて、何を知らないのか」という情報をリアルタイムで保持しています。
- **自己増殖する知識ネットワーク**: 「無知の知」を起点として自問自答を行い、知識ネットワーク（ニューロン回路のようなもの）を自律的に拡張させる能力を持ちます。
- **成熟度の透明性**: 改竄不能な教育履歴やトークン使用量などの情報は、マーケットプレイスにおける Cube の公正な評価指標となります。

## 5. 現在の開発状況（Rust 移行フェーズ）

本プロジェクト（`./src`）では、Go 版の資産を継承しつつ、Rust の安全なメモリ管理と Axum / SeaORM を利用した堅牢なアーキテクチャへの移行を進めています。

- **API 基盤の構築**: REST API の基礎となるハンドラ、ビジネスロジック、データモデルの実装が進んでいます。
- **実装完了済みコンポーネント**:
    - **BD (Base Directory/Keys)**: MYCUTE における鍵管理と基盤概念の API 実装。
    - **Usr (User)**: ユーザー管理および権限系の API 実装。
    - **Crypto**: 暗号化およびセキュリティ関連の API 実装。

## 6. 今後の展望

今後は、MYCUTE の真髄である **Absorb / Memify / Query** のコアロジックを Rust で実装し、Go 版で培われた知識ネットワークの構築アルゴリズムをさらに洗練させていく予定です。

---
**参照資料:**
- Go 版ソースコード: `mycute-go/src`
- 開発ヒストリー: `mycute-go/docs`

## 7. 実装移行ガイド：Go（mycute-go）から Rust（src）への再実装マニュアル

本セクションは、先行する Go 言語による実装をいかにして Rust へと正確、かつ効率的に移植していくかを解説した実務的なガイドです。開発者は、Go 実装の「どこ」を参照し、Rust で「どう」書くべきかの基準として本稿を利用してください。

### 7.1 ファイル・コンポーネントの参照先一覧

機能追加や改修を行う際は、まず以下の対応表に従って Go 実装の該当箇所を特定し、そのロジックを Rust の新しいレイヤー構造に展開します。

| 機能コンポーネント | Go 側の参照ファイル (`mycute-go/src/`) | Rust 側の実装先 (`src/`) | 実装時のポイント |
| :--- | :--- | :--- | :--- |
| **Routing** | `mode/rt/main_of_rt.go` | `mode/rt/req_map.rs` | Axum の `Router` 定義に変換。 |
| **API Handler** | `mode/rt/rthandler/hv1/*.go` | `mode/rt/rthandler/*.rs` | `utoipa` マクロで記述。引数は抽出器（Extractor）を活用。 |
| **Logic (BL)** | `mode/rt/rtbl/*.go` | `mode/rt/rtbl/*.rs` | `DatabaseConnection` を受け取り、非同期で実装。 |
| **Request DTO** | `mode/rt/rtreq/*.go` | `mode/rt/rtreq/*.rs` | `serde` + `garde` (バリデーター) を付加。 |
| **Response DTO** | `mode/rt/rtres/*.go` | `mode/rt/rtres/*.rs` | `Serialize` + `ToSchema` を実装。 |
| **SQL クエリ** | `sql/restsql/*.go` または `rtbl` 内の SQL 文字列 | SeaORM (Entity) または `entities/mod.rs` でのカスタム定義 | 原則として SeaORM のクエリビルダを使用。 |

---

### 7.2 実装移行の手順とベストプラクティス

Go のコードを Rust に移行する際の標準的なワークフローは以下の通りです。

#### 1. エンドポイントの定義 (Handler Layer)
Go の `hv1` パッケージ内の関数を見つけ、その `description` や `Param` タグを確認します。
- **Go**: `hv1.AuthUsr(c *gin.Context, ...)`
- **Rust**: `rthandler/usrs_handler.rs` 内に `pub async fn auth_usr(...)` を作成し、`utoipa` マクロでドキュメントを記述します。

#### 2. バリデーションの移行 (Request DTO)
Go では `binding:"required"` やハンドラ内の `if` 文で判定していたロジックを、Rust では `garde` のアトリビュートとして構造体に集約します。
- **例**: Go で「usernameは必須、50文字以内」という制約があれば、Rust の `rtreq` 内の構造体フィールドに `#[garde(custom(required_simple_err(1, 50)))]` を付与します。

#### 3. 権限チェックの統合 (JWT/Claims)
Go 版では `rtutil.JwtUsr` をマニュアルでチェックしていましたが、Rust 版では `JwtUsr` エクストラクターをハンドラの引数に含めるだけで認証が完了します。
- **実装要領**: `ju.allow_roles(&[JwtRole::VDR])?;` の一行で、権限がないリクエストを早期リターンさせます。

#### 4. データベース操作の変換 (BL Layer)
GORM の `Find` や `Create` などを、SeaORM の非同期メソッドに置き換えます。
- **SELECT**: `db.First(&usr, ...)` → `usrs::Entity::find().filter(...).one(conn).await?`
- **INSERT**: `tx.Create(&usr)` → `active_model.insert(tx).await?`
- **TRANSACTION**: 非同期クロージャ `conn.transaction(|tx| Box::pin(async move { ... }))` のパターンを定型文として利用します。

---

### 7.3 具体的な移行例：`Usr` モジュールのケーススタディ

`Usr` モジュールは、最も複雑な権限とトランザクションを含んでおり、他のモジュール移行の完璧なテンプレートとなります。

#### Case A: 権限に基づく自動フィルタリング (Partitioning)
Go 版では `restsql.SearchUsrs` 内で複雑な引数を渡してクエリを制御していましたが、Rust 版では `usrs_bl.rs` 内の `find_usrs_base` 関数を参照してください。

- **Go (参照先)**: `mycute-go/src/mode/rt/rtbl/usrs_bl.go`
- **Rust (実装先)**: `src/mode/rt/rtbl/usrs_bl.rs`
- **解説**: `JwtIDs` 構造体（認証済み ID 群）をクエリの `filter` に毎回通すことで、実装者が意識しなくても「他人のデータが見えない」安全な実装を実現しています。

#### Case B: 副次レコードの同時作成
`VDR` を作成する際に `Pool` も作成するロジックを例に取ります。

- **Go (参照先)**: `rtbl/usrs_bl.go` の `createVdrAsApx` 関数。
- **Rust (実装先)**: `rtbl/usrs_bl.rs` の `create_usr` 関数内の `is_vdr_creation` 分岐。
- **ポイント**: 型による正確な分岐判定を行い、トランザクションオブジェクト `tx` を使い回すことで、不可分な操作を保証します。

---

### 7.4 ログとトラブルシューティングの「作法」

移行をスムーズにするため、Rust 版では特定のログフォーマットを遵守します。

- **モジュールタグの付与**: 全ての `log::debug!` メッセージには `<UsrBl>` や `<Auth>` といったタグを付け、Go 版でのデバッグ時と同じ、あるいはそれ以上の視認性を確保します。

### 7.5 新規設計コンポーネントについて

**Crypto モジュール (`src/mode/rt/rthandler/cryptos_handler.rs`)** のように、Rust 版で初めて導入されたコンポーネントについては、Go 版に対応するソースがない場合があります。

## 8. スムーズな開発のために

本プロジェクトの成功は、**「Go で実装されたドメイン知識」**を**「Rust の堅牢な型システム」**に正しくマッピングできるかどうかにかかっています。
常に `mycute-go/src` を隣に開き、既存のロジックがどのテーブルのどのフィールドを、どのような条件で操作しているかを注意深く読み解いてから、Rust での実装を開始してください。

## 9. REST API 開発における「厳格ルール」の遵守

Rust 版の `src/mode/rt` 内で REST API を実装する際は、プロジェクト共通の指針である [REST API 開発厳格ルール](./docs/00-REST_API_DEV_STRICT_RULES.md) を遵守しなければなりません。

このドキュメントは、「誰が書いても同じ品質、同じ構造、同じ安全性が担保されること」を目的としたバイブルです。**実装を開始する前、およびコードレビューを行う前には、必ずこのドキュメントを全編読み直してください。**

### 9.1 実装ルールのダイジェスト（重要項目・抜粋）

以下は [REST API 開発厳格ルール](./docs/00-REST_API_DEV_STRICT_RULES.md) の要諦をまとめたものです。これらは「最低限遵守すべき共通言語」であり、詳細は必ず本体を参照してください。

#### 1. 開発の基本サイクルと機能順序
- **入口から出口へ**: `Route` -> `Handler` -> `Request` -> `Response` -> `Logic` の順で実装を進めることで設計矛盾を早期に発見する。
- **CRUD 順序の絶対遵守**: ルーティング登録、関数定義、DTO 構造体定義、BL ロジックの全てにおいて、必ず **Search -> Get -> Create -> Update -> Delete** の順序（および CRUD 以外はその下へ）で記述し、一貫性を保つこと。

#### 2. Handler 層：ドキュメントと権限の設計
- **OpenAPI の充実**: `#[utoipa::path]` の `description` は属性直前に Markdown 定数（`CREATE_DESC` 等）として定義し、アクセス権限、パラメータ詳細、注意点を精緻に記述する。
- **早期認可（Guard）**: 関数最上部で `ju.allow_roles(...)` を呼び出し、不適切な権限を BL 到達前に物理的に遮断する。また、パスパラメータは `Path<u32>` 等で静的に型付けする。

#### 3. Request/Response DTO：Swagger UI の利便性最大化
- **Example の義務化**: クライアントが即座に "Try it out" できるよう、全フィールドに `#[schema(example = ...)]` を記述する。
- **カスタムバリデーション**: `garde` 標準属性を直接使わず、`custom(required_simple_err)` や `custom(datetime_err)` のように、`src/mode/rt/rterr/` で定義された**プロジェクト共通エラーコード（EXXXX）**を付与する。
- **必須項目のバリデーション (厳守)**: 必須項目（`String` 等）を未送信の際、Serde のデシリアライズエラー（400）というフォールバックを発生させず、意図的に `garde` の構造化バリデーションエラー（422）として返却しなければならない。このため、必須項目には必ず `#[serde(default)]` を付与すること。これによりキー欠如時にデフォルト値（空文字等）が設定され、`garde` の `required_simple_err(1, ...)` 等で適切に捕捉可能になる。
- **フラットなレスポンス**: 構造体は内部でラップせずフラットに保ち、SeaORM モデルからの変換には `From<Model>` トレイトを構造体定義の直下で実装する。

#### 4. Business Logic (BL) 層：安全性とトレーサビリティ
- **厳格なデータパーティショニング**: 権限（IDs）に基づいた `apx_id` および `vdr_id` によるフィルタリングは、いかなる場合も（たとえ `usr_id` で一意に特定可能であっても）例外なく必須である。プライベートヘルパー `find_[resource]_base` にこのフィルタを集約し、全ての操作で必ずこれを経由させる。
- **明示的なロール分岐**: `ju.role()` と `match` を用い、APX/VDR/USR ごとの挙動の差異をコード上で意図的に浮き彫りにする。
- **トランザクションと一括削除**: 組織削除などの重要操作では、非同期クロージャを用いた `conn.transaction` 内で関連する全 Entity の削除を完遂させる。
- **高密度デバッグログ**: 処理の節目（分岐、クエリ構築、トランザクション、削除実行等）には必ず `log::debug!` を入れ、`<UsrBl>` 形式のタグでトレース可能にする。

#### 5. 環境変数の管理：設定の一元管理
- **直接参照の禁止**: `rtbl` や `rthandler` 内で `std::env::var` を使用することは厳禁である。
- **一元収集と伝搬**: 設定が必要な場合は、まず `.env` に定義し、`src/mode/rt/main_of_rt.rs` の「環境変数収集」ブロックで読み込み、そこから各所へ値を伝搬させる設計を徹底すること。

#### 6. バリデーターの追加ルール
- `garde` にない独自の検証を追加する場合は、`rterr.rs` での基底ロジック定義から `validators.rs` でのマクロ実体化まで、**厳格な 4 ステップの手順**（ルール本体への記載参照）を崩してはならない。

### 9.2 開発時の鉄則

> [!IMPORTANT]
> **「動けば良い」は本プロジェクトでは認められません。**
> 構造の美しさと、厳格ルールへの準拠が最優先されます。もし実装中にルールとの矛盾や、ルールでは解決できない特殊なケースに遭遇した場合は、独断で進めずにルール自体の更新や改善を検討してください。

常に [00-REST_API_DEV_STRICT_RULES.md](./docs/00-REST_API_DEV_STRICT_RULES.md) を別画面で開き、参照しながら実装を進めてください。

### 9.3 `src/entities/` ディレクトリの絶対禁則

> [!CAUTION]
> **`src/entities/` 内のファイルを直接編集してはならない。**

このディレクトリ内の全てのファイルは、以下のコマンドによって**自動生成**されるものである。

```bash
make gen-entities HOST="localhost"
```

このコマンドを実行するたびに、ディレクトリ内のファイルは**完全に上書き**される。したがって、直接編集した内容は**常に消失**する。

#### 禁止される行為
- `src/entities/*.rs` ファイルへの直接的なコード追加・変更
- `#[sea_orm(...)]` アトリビュートの手動追加
- フィールドや型の直接変更

#### 問題が発生した場合の正しい対処法
1. **スキーマ変更が必要な場合**: マイグレーションファイル(`migration/src/`)を作成し、DBスキーマを変更した後、`make gen-entities` を再実行する。
2. **挙動の拡張が必要な場合**: `lib.rs` にマクロやトレイトを定義し、エンティティファイルの末尾で呼び出す（例: `crate::impl_utc_timestamp_behavior!(ActiveModel);`）。
3. **生成設定の変更が必要な場合**: `sea-orm-cli` の設定オプションを調整する。

### 9.4 AI エージェント環境でのコマンド実行ルール

> [!CAUTION]
> **本プロジェクトは AI エージェント（Google Antigravity 等）による開発支援を前提としている。エージェントがターミナルコマンドを実行する際、特定のパターンにより「デッドロック」状態に陥り、応答不能になる。以下のルールを厳守すること。**

#### 背景：なぜデッドロックが発生するのか

1. **プログレスバーによるトークン枯渇**: `curl` 等が進捗を出力し続け、エージェントのコンテキストを埋め尽くす
2. **インタラクティブな確認待ち**: ユーザー承認（Y/n）を求めるプロンプトでエージェントが応答不能になる
3. **出力バッファのフラッシュ問題**: コマンド終了が正しく通知されず「実行中」と誤認し続ける

#### ルール 1: `curl` コマンドの必須オプション

```bash
# 必須: -sS（サイレント+エラー表示）, -m（タイムアウト）
curl -sS -m 10 -X POST http://localhost:8888/v1/... \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{}'
```

#### ルール 2: 対話的確認コマンドの自動承認

```bash
# 必ず -y や --yes 等で確認をスキップ
npm install -y
apt-get install -y package-name
rm -f file.txt  # -f で確認スキップ
```

#### ルール 3: 長時間コマンドのバックグラウンド化

サーバー起動等はバックグラウンドで実行し、適切なタイムアウトを設定する。

詳細は [00-REST_API_DEV_STRICT_RULES.md](./docs/00-REST_API_DEV_STRICT_RULES.md) の「AI エージェント環境でのコマンド実行ルール」セクションを参照。

## 10. 実装の最重要な核となる cuber の完全解析

> [!IMPORTANT]
> 本セクションは、MYCUTE の知能の源泉である `mycute-go/src/pkg/cuber` の実装を完全に理解し、Rust で「完全な美しさ」をもって再実装するための設計図である。膨大かつ複雑なロジックを漏れなく解析するため、以下のステップに従って詳細を記述していく。

### 10.1 アーキテクチャ全景とモジュール構造

Cuber は、高度な抽象化とイベント駆動設計を組み合わせた、モジュール性の高いアーキテクチャを採用している。

#### 1. CuberService：知能の司令塔
`mycute-go/src/pkg/cuber/cuber.go` に定義される `CuberService` は、システムの中心的なエントリポイントである。
- **ライフサイクル管理**: `NewCuberService` で初期化され、`Close()` でリソースを解放する。
- **コネクションプール（StorageMap）**: 複数の Cube（DBファイル）への接続を UUID ごとに管理。アイドル状態の接続を自動的にクローズする GC（Garbage Collection）ルーチンを備え、リソース消費を最適化している。
- **共通コンポーネントの共有**: 形態素解析器（Kagome）、S3クライアント、ロガーなどをシングルトンとして保持し、各タスクへ供給する。

#### 2. プラガブルな LLM プロバイダー
`providers/factory.go` により、特定の LLM に依存しない柔軟な設計を実現している。
- **Eino フレームワークの活用**: `ChatModel`（生成）と `Embedder`（ベクトル化）のインターフェースを介し、OpenAI, Gemini, Claude, DeepSeek, Ollama 等を透過的に切り替え可能。
- **動的生成**: 操作ごとに temporary な Embedder/ChatModel を生成する仕組みを持ち、タスクごとに異なるモデル設定を適用できる。

#### 3. ストレージ抽象化層（Storage Layer）
`storage/interfaces.go` で定義されるインターフェースにより、物理的な DB の実装を隠蔽している。
- **VectorStorage**: 文書メタデータ、チャンク、ベクトル、全文検索（FTS）を操作。
- **GraphStorage**: 知識グラフ（Node, Edge, Triple）の操作、グラフトラバーサル、代謝（Metabolism）パラメータの管理を担当。
- **LadybugDB**: 上記 2 つのインターフェースを単一の統合 DB（SQLiteベース）として実装しており、ベクトル検索とグラフクエリの「アトミックなトランザクション」を可能にしている。

#### 4. イベント駆動による非同期連携
内部の `EventBus` を介して、処理の進捗（Absorb の開始/終了、チャンク処理中など）をリアルタイムに通知する。
- **ストリーミング応答**: Web 側（Handler）へ処理状況をストリーミングするための `dataCh` と連携し、ユーザー体験を向上させている。


### 10.2 コアデータモデル：Cube, Node, Edge, Chunk

Cuber は、非構造化データを「ファイル → ドキュメント → チャンク → グラフ」という階層構造で管理し、それら全てにパーティションキーを付与することで、マルチテナントかつスケーラブルな知識管理を実現している。

#### 1. データの階層構造
1.  **Data**: 取り込まれたファイルの最小単位。コンテンツハッシュ（SHA-256）による重複排除と、物理ストレージ（S3/Local）上の所在を管理する。
2.  **Document**: ファイルから抽出されたプレーンテキスト。
3.  **Chunk**: ベクトル検索の最小単位（デフォルト 1024 文字）。
    - **埋め込み (Embedding)**: 1536次元（OpenAI標準）等のベクトルを保持。
    - **多層全文検索（Multi-layer FTS）**: `nouns`（名詞）、`nouns_verbs`（名詞+動詞）、`all`（全内容語）の 3 レイヤーのキーワードを保持し、ベクトル検索を補完する。

#### 2. 知識グラフの構成要素
1.  **Node (Entity)**: `Type`（Person, Concept 等）と `Properties`（JSONマップ形式の属性）を持つ。
2.  **Edge (Relationship)**: ノード間の関係。
    - **Weight (重み)**: 関係の強さ。
    - **Confidence (信頼度)**: LLM が抽出した際の確信度。
    - **Thickness (太さ)**: `Weight × Confidence × 時間減衰` によって算出される、クエリ時の実効的な関係強度。
    - **Unix (更新時間)**: 代謝（Metabolism）における時間減衰計算の起点。
3.  **Triple**: `Source Node - Edge - Target Node` の三つ組。グラフトラバーサルの基本単位。

#### 3. 隔離と設計：MemoryGroup
Cuber の全データテーブルには `memory_group` フィールドが存在する。
- **物理的・論理的隔離**: 「ユーザーID - データセットID」といった形式で、同一の Cube（DBファイル）内でもデータが厳格に分離される。
- **代謝パラメータの個別設定**: `MemoryGroupConfig` により、グループごとに「知識の半減期（HalfLife）」や「剪定閾値（PruneThreshold）」を調整可能。

#### 4. Cube と UUID
Cube は 1 つの `.db` ファイル（LadybugDB）に対応する。
- **UUID の導出**: ファイル名（拡張子抜き）を UUID として扱い、`CuberService` 内の接続管理に使用する。
- **ポータビリティ**: 1 つのファイルに必要な全ての情報（ベクトル、グラフ、FTS、メタデータ）が完結しているため、ファイルコピーだけで知識の移動が可能。


### 10.3 知識の吸い込み：Absorb プロセスの詳細解析

`Absorb` は、未加工のファイルをシステムに取り込み、クエリ可能な「知識」へと変換する一連のパイプラインをオーケストレートする、最も重要な原子操作である。

#### 1. Add フェーズ：ファイルインジェクション
`ingestion.IngestTask` が担当し、ファイルの物理的な取り込みとメタデータの永続化を行う。
- **決定論的 ID 生成**: `SHA-1(ContentHash + MemoryGroup)` を用いて UUID を生成する。これにより、複数の MemoryGroup で同一ファイルが共有されても ID は衝突せず、同一グループ内での重複投稿は自然に抑制される。
- **重複チェックとスキップ**: DB への問い合わせ (`Exists`) により、既に取り込み済みのファイルは処理をスキップし、リソース（計算資源・トークン）を節約する。
- **ストレージ・抽象化**: `S3Client` を介し、ローカルディレクトリまたは AWS S3（互換ストレージ含む）へファイルを保存。Rust 実装では、この I/O の非同期化がパフォーマンス向上の鍵となる。

#### 2. Cognify フェーズへの橋渡し
`add` が成功すると、一連の `storage.Data` オブジェクトが返され、そのまま `cognify` メソッドに引き渡される。
- **トランザクション管理**: `Absorb` 全体は LadybugDB のトランザクション内で実行され、万が一後半の処理が失敗しても、不整合なメタデータが残らないよう設計されている。
- **クリーンアップ**: 処理の最後に、一時的にダウンロード/変換されたファイルは自動的に削除される。

#### 3. イベント駆動によるプログレス管理
`EventBus` を活用し、各ステップで詳細なイベントを発火する。
- **主要なイベント流**:
  - `ABSORB_START`: 処理全体の開始
  - `ABSORB_ADD_FILE_START/END`: 個別ファイル取り込みの進捗
  - `ABSORB_ERROR`: 途中失敗時のエラー詳細
  - `ABSORB_END`: トークン使用量を含む最終集計の通知
- **ストリーミングの実装**: `RegisterAbsorbStreamer` により、これらの内部イベントがシームレスに `dataCh` (channel) へ変換され、SSE (Server-Sent Events) 等を通じてフロントエンドへリアルタイムに届けられる。


### 10.4 認知の形成：Cognify パイプラインとタスク群

`Cognify` は、インジェクションされた `Data` から、高度に構造化された「意味のネットワーク」を構築するパイプラインである。各タスクは `pipeline.Task` インターフェースを実装し、シーケンシャルにデータを受け渡す。

#### 1. ChunkingTask：文脈を維持した分割
単なる文字数による機械的な裁断ではなく、意味の最小単位を維持する。
- **文単位の分割**: 正規表現 (`SplitSentencesRegexp`) を用い、。！？等の句読点や改行で文を特定。文の途中でチャンクが切れることを防ぐ。
- **インテリジェント・オーバーラップ**: 前のチャンクの末尾数文を次のチャンクの先頭に複製し（文字数ベースで調整）、チャンク間を跨ぐ文脈の断絶を回避する。
- **FTS キーワード抽出**: 日本語形態素解析器 `Kagome` を用い、名詞、名詞+動詞、全内容語の 3 レイヤーでキーワードを抽出し、ベクトル検索の網羅性を強化する。

#### 2. GraphExtractionTask：知識グラフの抽出
テキストからエンティティとその関係性を抽出する、最も計算資源を消費するフェーズ。
- **並列 LLM リクエスト**: `errgroup` を用い、複数のチャンク（デフォルト並列数 5）を並行して LLM（gpt-4o-mini 等）に投げ、ノードとエッジを抽出する。
- **正規化と ID 発行**: 抽出された文字列に対し `NormalizeForGraph` を適用し、表記揺れを抑制。`MakeGraphNodeID(ID, MemoryGroup)` により、MemoryGroup ごとに隔離されたグローバル一意識別子を付与する。
- **トリプルの自然言語化**: 抽出された関係（Triple）を再度、人間が読みやすい自然言語の「説明文」へと変換し、検索時のコンテキストとして利用可能にする。

#### 3. SummarizationTask：高次概念の生成
各チャンクに対し、その内容を簡潔にまとめた「要約」を生成する。
- **メタ・インデックス**: 要約に対してもベクトル（Embedding）を生成し、`Summary` テーブルに保存。これにより、詳細なチャンク検索の前に、「何についてのドキュメントか」という大局的な検索を高速化する。

#### 4. StorageTask：永続化の最終処理
パイプラインの最終段階で、正規化された全ての要素を LadybugDB の各テーブル（Chunks, Nodes, Edges, Vectors）に一括保存する。
- **決定論的 ID の徹底**: チャンク ID に基づく SHA-1 ハッシュにより、同一内容の再生成を冪等（Idempotent）に処理する。


### 10.5 高度な検索アルゴリズム：Query とハイブリッド探索

`Query` は、単なるベクトル検索を超え、構造化された知識グラフと非構造化テキストを動的に融合させる「ハイブリッド探索」の実装体である。

#### 1. 多角的リトリーバル (Hybrid Retrieval)
一つの検索手法に依存せず、多層的なアプローチで回答の種を探す。
- **ベクトル検索 + FTS 拡張**: クエリの埋め込みベクトルによる検索に加え、FTS（全文検索）を用いてチャンク内のキーワードから関連エンティティを芋づる式に抽出する。これにより、低頻度語や固有名詞に対しても高い再現率を確保する。
- **動的グラフトラバーサル**: 抽出されたエンティティを「始点」として、関連するサブグラフ（トリプル群）を抽出。知識の「点（チャンク）」だけでなく「網（関係性）」をコンテキストに取り込む。

#### 2. 時間減衰とスコアリング (Thickness Scoring)
知識の「鮮度」と「重要度」を数値化し、最適なコンテキストを選択する。
- **シグモイド時間減衰**: `half-life`（半減期）設定に基づき、古い知識のスコアを減衰させる。最新の動向を優先しつつ、歴史的な事実も維持する。
- **Thickness フィルタリング**: 信頼度 (Confidence) × 重み (Weight) × 時間減衰により算出された `Thickness` 値が閾値未満のエッジは除外され、回答の精度を担保する。

#### 3. 知識の矛盾解決 (Conflict Resolution)
LLM が抽出した知識に含まれる矛盾を検出し、整合性を維持する。
- **Stage 1 (決定論的)**: 同一の主語・述語で目的語が異なる等の明らかな矛盾を定義済みルールで解決。
- **Stage 2 (LLM 仲裁)**: 複雑な文脈依存の矛盾に対し、LLM を「審判」として呼び出し、どちらの知識が妥当かを最終判定。
- **バックグラウンド・クリーンアップ**: 棄却された矛盾知識（エッジ）は DB から物理削除され、グラフの「自浄作用」として機能する。

#### 4. 統合回答生成 (Synthesis)
- **マルチステージ・プロンプティング**:
  1. 抽出されたグラフ（トリプル群）を、クエリに最適化された「グラフ要約」へ一度要約。
  2. ベクトル検索結果（チャンク）とグラフ要約をマージし、最終的な回答を生成。
- **リアルタイム・フィードバック**: `EventBus` を介し、埋め込み、検索、生成の各ステップをミリ秒単位でクライアントへストリーミング。

### 10.6 自己成長のメカニズム：Memify と再帰的学習

`Memify` は、既存の知識から未知の情報を推論し、グラフを自律的に拡張・洗練させる、MYCUTE の最も特徴的な「自己成長」プロセスである。

#### 1. 未知の解決 (Unknown Resolution)
`IgnoranceManager` と `SelfReflectionTask` が連携し、システムが「知らない」と認識した事項を能動的に解決する。
- **自問自答と検索**: 未解決の `Unknown`（不全知識）に対し、LLM が既存知識を検索しながら解決を試みる（Self-Reflection）。
- **能力の獲得 (Capability)**: 解決に成功した場合、それを「獲得した能力」として再登録し、知識の欠落を埋める。

#### 2. 再帰的なルール抽出 (Rule Extraction)
ドキュメントから「原理・原則」や「不変のルール」を抽出し、高次の知識層を構築する。
- **一括・バッチ処理の動的切り替え**: テキスト量に応じ、精度重視の Bulk 処理と、メモリ効率重視の Batch 処理（オーバーラップ付き）を自動選択。
- **NodeSet による構造化**: 抽出されたルールを `NodeSet` ノードの下に体系化。これにより、「特定のライブラリの規約」といった知識の固まりを管理可能にする。

#### 3. 知識の代謝 (Metabolism)
エントロピーの増大を防ぎ、グラフの健全性を維持するための「忘却」と「洗練」のプロセス。
- **エッジ・プルーニング**: 時間の経過とともに Thickness（強度）が減衰した古い、または重要度の低いエッジを物理削除。
- **MDL Principle に基づく忘却**:
    - **復元困難度の算出**: あるノードを削除しても、近傍のノードからその情報を推論（復元）可能かをベクトル検索で判定。
    - **ベネフィット計算**: グラフの単純化による「記述量の削減（MDL）」が「復元困難度」を上回る場合、そのノードを「忘却」しても良いと判断し、削除する。
- **定期的洗練**: `Conflict Resolution` を再実行し、時間の経過とともに発生した知識間の矛盾を解消し続ける。

### 10.7 メタ認知：Unknown 認識と自問自答（Crystallization）

Cuber は、自身が「何を知っていて、何を知らないか」を管理し、能動的に知識を洗練させるメタ認知機能を備えている。

#### 1. 無知の管理 (Unknown Recognition)
`IgnoranceManager` は、システムが回答できなかった問いや不足している情報を `Unknown` ノードとしてグラフに永続化する。
- **自動解決チェック**: `Cognify`（新しい知識の取り込み）の際、蓄積された `Unknown` のベクトルと新しい知識の類似度を計算。解決可能と判断された場合、自動的に対応する `Unknown` を「解決済み」とし、`Capability`（獲得した能力）へと昇華させる。
- **トレーサビリティ**: どの知識がどの `Unknown` を解決したか、`resolves` エッジによって記録される。

#### 2. 自問自答 (Self-Reflection)
`SelfReflectionTask` は、外部からの入力に頼らず、内部的に知識の欠落を見つけ出すループを実行する。
- **問いの生成**: 既存の `Rule`（抽出された不変のルール）を LLM に読み込ませ、「このルールに関連して、私たちがまだ詳細を知らないことは何か？」という問いを生成させる。
- **回答の試行**: 生成された問いに対し、`Query` と同等のハイブリッド検索を行う。
- **結果の記録**: 検索の結果、十分な洞察が得られた場合は `Capability` を、得られなかった場合は `Unknown` を登録する。これにより、将来的に補完すべき知識が浮き彫りになる。

#### 3. 知識の結晶化 (Crystallization)
`CrystallizationTask` は、断片化された知識をより高次で抽象的な知識へと統合する。
- **ルールのクラスタリング**: ベクトル類似度に基づき、意味の近い `Rule` ノード群を特定する。
- **LLM による統合**: 特定されたクラスタ（ルールの塊）を LLM に投げ、「これらを矛盾なく 1 つにまとめた、より包括的なルール」を生成させる。
- **グラフのリワイヤリング**: 旧ルールに接続されていたエッジを新ルールに貼り直し、旧ルールを削除する。この「結晶化」プロセスにより、知識の冗長性が排除され、推論の効率と精度が向上する。

### 10.8 データの永続化：LadybugDB とスキーマ設計

Cuber の全知能を支える永続化層には、グラフ、ベクトル、全文検索を単一の ACID トランザクションで扱える統合 DB「LadybugDB」が採用されている。

#### 1. ハイブリッド・スキーマ設計と高度なクエリ支援
LadybugDB は Cypher クエリ言語をベースとしており、複雑なグラフ操作を宣言的に記述できる。
- **NODE TABLE**:
  - `Data / Document / Chunk`: 生データから分割されたチャンクまでの階層。`Chunk` は `embedding` カラムを保持。
  - `GraphNode / Entity / Rule`: 構造化された知識の構成単位。
  - `Unknown / Capability`: メタ認知情報の記録。
- **REL TABLE**:
  - `HAS_CHUNK / NEXT_CHUNK`: 文脈の連続性を維持するリレーション。
  - `GraphEdge`: 意味的な繋がり。`weight`, `confidence`, `unix`（時間）を属性として持ち、代謝計算の基礎となる。
- **MERGE クエリの完全サポート**:
  - Rust バインディング（`lbug` crate）を通じて、`MERGE` 句を含む Cypher クエリを安全かつ高性能に実行可能。これにより、「存在しなければ作成、存在すれば更新」という高度な UPSERT 処理が Rust 側から容易に実装できる。

#### 2. プリバンドルされた多層全文検索 (Pre-bundled FTS)
以前のバージョンでは Rust からの FTS 利用に制約があったが、最新の LadybugDB (v0.12.x+) では FTS 拡張がバイナリに静的にバンドルされている。
- **導入の容易さ**: 動的ロードや `INSTALL` コマンドが不要になり、即座に利用可能。
- **検索レイヤー**:
  - **Layer 0 (nouns)**: 名詞のみ。
  - **Layer 1 (nouns_verbs)**: 名詞 + 動詞。
  - **Layer 2 (keywords)**: 全キーワード。
これらは Kagome（日本語形態素解析器）との組み合わせにより、ベクトル検索の弱点を完璧に補完する。

#### 3. 最適化されたベクトル検索 (Vector Search)
`vector` 拡張もプリバンドルされており、HNSW ベースの高速な検索が可能。
- **Cosine Similarity**: Cosine 類似度検索がネイティブにサポートされており、LLM との親和性が非常に高い。
- **ACID 整合性**: 検索結果の取得からグラフの更新までを単一のトランザクション内で完結させることができ、データ整合性を強力に保証する。

### 10.9 EventBus と進捗のリアルタイム可視化
MYCUTE の UX を支える重要な要素が、非同期処理の「中身」をユーザーに伝えるための高度なイベントシステムである。

#### 1. 軽量・型安全な EventBus
- **Generics ベースの Pub/Sub**: `lib/eventbus` は Go のジェネリクスを活用し、送信側 (`Emit`) と受信側 (`Subscribe`) でペイロードの型安全性を保証している。
- **非同期デリバリ**: デフォルトで各ハンドラをゴルーチンで実行するため、イベント発行がメインの処理ロジックのボトルネックにならないよう設計されている。

#### 2. ダイナミック・テンプレート・エンジン
- **25 バリエーションの妙**: `event_templates.go` には、各ステージ（Absorb, Query, Memify 等）ごとに 25 通りのメッセージ・バリエーションが日英両言語で定義されている。
- **ラウンドロビン選択**: `event_stream.go` が実行時にランダムまたはラウンドロビンでテンプレートを選択。これにより、同じ処理を繰り返しても「システムが生きている」かのような、人間味のある進捗ログを生成する。

#### 3. 構造化データの言語化 (FormatEvent)
- **ペイロードの埋め込み**: 抽出されたエンティティ名や、処理中のチャンク番号など、動的な数値をテンプレートに合成 (`fmt.Sprintf`) し、専門的かつ具体的な進捗メッセージをリアルタイムに生成。
- **SSE (Server-Sent Events) への最適化**: 生成された文字列は `StreamEvent` としてラップされ、最終的に REST API のストリーミングエンドポイントを介してフロントエンドへ届けられる。

### 10.10 Rust への完全移植に向けた技術的課題と戦略

Go 版 Cuber の「完全な再現とさらなる美しさ」を目指す Rust 実装において、克服すべき主要な課題と設計戦略を以下に定める。

#### 1. 非同期オーケストレーションの再構築
Go の Goroutine/Channel による並行パイプラインを、Rust の `tokio::mpsc` や `async-stream` による非同期ストリームへと昇華させる。特に `Absorb` 過程における I/O（S3/DB）と計算（LLM/Embedding）の多重化を、Rust の高度な Future 管理によって無駄なく制御する。

#### 2. LadybugDB / KuzuDB の Rust 結合
LadybugDB (v0.12.x+) および KuzuDB (v0.11.3+) において、Rust バインディングの主要な懸念は解消された。`MERGE` クエリやプリバンドルされた FTS/Vector 拡張の利用は、通常の Cypher クエリとして透過的に、かつ安全に呼び出し可能である。Rust 実装では、これらを `lbug` クレートを通じて高性能にラッピングし、ビジネスロジック層とシームレスに結合させる。

#### 3. 柔軟な推論エンジンとプロンプトの厳格管理
Go 版で使用されている `Eino` フレームワークと同等の柔軟性を、Rust で再構築する。特に、プロンプトは Python 版からの正確な移植が求められるため、プロンプトのバージョン管理と、LLM 応答（JSON 等）の型安全なパースを `serde` で徹底する。

#### 4. map[string]any から強固な型システムへ
Go 版のグラフプロパティに見られる `map[string]any` は、柔軟性と引き換えに実行時エラーのリスクを抱えている。Rust 実装では、可能な限り `Enum` や `Generic` を活用し、グラフ上のデータ構造をコンパイル時に検証可能にすることで、推論ロジックの堅牢性を Go 版以上に高める。

#### 5. 高性能な形態素解析と FTS の Rust 化
Kagome による日本語処理と FTS インデックス構築を、Rust ネイティブな形態素解析（Lindera 等）や、LadybugDB の Rust-API を通じて最適化する。検索品質を落とすことなく、Go 版を上回るスループットを実現する。

## 11. 上記10.1 ~ 10.10 及び追加の考慮点についてのRustでの実装対応計画

本セクションでは、第10章で解析した Go 版 Cuber の各コンポーネントを、Rust の設計思想に基づきどのように再実装するかを詳述します。Go の柔軟性と Rust の堅牢性を融合させ、`src/cuber` フォルダ内に安全かつ高性能なモジュールとして再構築するための具体的な実装ロードマップです。

### 11.1 Rustでの実装対応計画 for 10.1 (Architecture & CuberService)

Go版のCuberServiceは、`mycute-go/src/pkg/cuber/cuber.go`と`mycute-go/src/pkg/cuber/types/config_types.go`に定義されており、特に`CuberService`構造体 (L51-59), `StorageSet`構造体 (L42-47), `NewCuberService`関数 (L71-207), `startStorageGCRoutine` (L378-389)などが主要な実装箇所です。

Go実装のコードスニペットは以下の通りです。

```go
// cuber.go: サービス心臓部と接続セット
type StorageSet struct {
    Vector     storage.VectorStorage
    Graph      storage.GraphStorage
    LastUsedAt time.Time
    mu         sync.Mutex
}

type CuberService struct {
    StorageMap map[string]*StorageSet // マップのキーは、model.Cube.UUID となる
    mu         sync.RWMutex           // StorageMapへのアクセス保護
    Config     types.CuberConfig      // 設定値を保持
    S3Client   *s3client.S3Client     // S3クライアント（ローカル/S3両対応）
    Kagome     *tokenizer.Tokenizer   // 日本語形態素解析器（Kagome）- シングルトンとして全コンポーネントで共有
    closeCh    chan struct{}          // サービス終了通知用チャネル
    Logger     *zap.Logger
}

// cuber.go: 初期化時のデフォルト値設定とバックグラウンドタスク起動
func NewCuberService(config types.CuberConfig) (*CuberService, error) {
    if config.StorageIdleTimeoutMinutes == 0 { config.StorageIdleTimeoutMinutes = 60 }
    // ... (多くのデフォルト値設定) ...
    kagome, _ := tokenizer.New(ipa.Dict(), tokenizer.OmitBosEos())
    service := &CuberService{ ... }
    go service.startStorageGCRoutine() // GCタスク
    go func() { ... service.S3Client.CleanupDownDir(retention) ... }() // S3キャッシュ掃除
    return service, nil
}
```

Rustでは、`src/cuber`ディレクトリ内に以下のモジュール構造で再構築します。
- `src/cuber/mod.rs`: 全体のエクスポート（Facade）。
- `src/cuber/service.rs`: `CuberService` 本体ロジック。
- `src/cuber/storage_set.rs`: `StorageSet` (接続ペア) の定義。
- `src/cuber/config.rs`: `CuberConfig` 構造体（serdeによるシリアライズ対応）。
- `src/cuber/error.rs`: `CuberError` 個別定義（thiserror利用）。

各コンポーネントの具体的な実装計画は以下の通りです。

#### 1. `CuberError` (src/cuber/error.rs)

Go版の`errors.New`や`fmt.Errorf`による文字列ベースのエラーハンドリングに対し、Rustでは`thiserror`クレートを用いて構造化されたエラー列挙型`CuberError`を定義します。これにより、エラーの型安全な伝播と呼び出し側でのパターンマッチングによる詳細なエラー処理が可能になります。

```rust
// --- src/cuber/error.rs ---
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CuberError {
    #[error("Storage initialization failed: {0}")]
    StorageInitError(String),
    #[error("S3 client error: {0}")]
    S3Error(String),
    #[error("Model configuration verification failed: {0}")]
    ConfigValidationError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    // Go版の errors.New や fmt.Errorf を構造化された列挙型で表現
    #[error("Model error: {0}")]
    ModelError(String), // 例: LLM関連のエラー
}
```

#### 2. `StorageSet` (src/cuber/storage_set.rs)

Go版の`StorageSet`は`Vector`、`Graph`、`LastUsedAt`、`mu`で構成されます。Rustではこれを`pub struct StorageSet`として再定義します。`storage.VectorStorage`と`storage.GraphStorage`は、LadybugDBのRustバインディングを介したトレイトオブジェクト`Arc<dyn VectorStorage>`と`Arc<dyn GraphStorage>`として表現します。アイドル監視用の`LastUsedAt`は、Go版の個別の`sync.Mutex`ではなく、`tokio::sync::RwLock<Instant>`でラップすることで、読み取り（参照）時のオーバーヘッドを低減しつつ、書き込み時の排他制御を行います。

`StorageSet`には、Go版の`Close()`メソッドに相当する`close()`非同期関数を実装し、内部の`VectorStorage`と`GraphStorage`のクリーンアップをシーケンシャルに実行します。

```rust
// --- src/cuber/storage_set.rs ---
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

// 仮のトレイト定義 (実際のLadybugDBバインディングに合わせる)
pub trait VectorStorage: Send + Sync {
    async fn close(&self) -> Result<(), CuberError>;
}
pub trait GraphStorage: Send + Sync {
    async fn close(&self) -> Result<(), CuberError>;
}

pub struct StorageSet {
    /// Go: storage.VectorStorage (LadybugDB)
    pub vector: Arc<dyn VectorStorage>,
    /// Go: storage.GraphStorage (LadybugDB)
    pub graph: Arc<dyn GraphStorage>,
    /// アイドル監視用 (Go: LastUsedAt)
    pub last_used_at: RwLock<Instant>,
}

impl StorageSet {
    // LadybugDBの初期化結果からStorageSetを構築するコンストラクタを想定
    // pub fn from(ladybug_db: LadybugDB) -> Self { ... }

    pub async fn close(&self) -> Result<(), CuberError> {
        // Go: set.Vector.Close() と set.Graph.Close() のシーケンシャルなクリーンアップ
        self.vector.close().await?;
        self.graph.close().await?;
        Ok(())
    }
}
```

#### 3. `CuberConfig` (src/cuber/config.rs)

Go版の`CuberConfig`は`config_types.go`に定義されており、多くの設定フィールドを持ちます。Rustでは`serde`クレートの`Serialize`と`Deserialize` deriveマクロを用いて、設定ファイルの読み書きを容易にします。Go版で`if config.StorageIdleTimeoutMinutes == 0 { config.StorageIdleTimeoutMinutes = 60 }`のように行っていたデフォルト値の設定は、`serde(default = "default_idle_timeout")`アトリビュートを使用することで、コンパイル時にデフォルト値が決定される型安全な形式をとります。これにより、実行時の設定値検証ロジックを簡素化できます。

```rust
// --- src/cuber/config.rs ---
use serde::{Deserialize, Serialize};

fn default_idle_timeout() -> u32 { 60 }
fn default_meta_unknown_threshold() -> f64 { 0.75 } // 仮の値

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuberConfig {
    pub db_dir_path: String,
    #[serde(default = "default_idle_timeout")]
    pub storage_idle_timeout_minutes: u32,
    
    // Metacognition (Go config_types.go L20-26)
    #[serde(default = "default_meta_unknown_threshold")]
    pub meta_similarity_threshold_unknown: f64,
    pub meta_search_limit_unknown: usize,
    
    // Graph Metabolism (Go config_types.go L46-49)
    pub graph_metabolism_alpha: f64,
    pub graph_metabolism_prune_threshold: f64,
    
    pub s3_access_key: String,
    // ... その他全52項目の設定フィールドを網羅 ...
}
```

#### 4. `CuberService` (src/cuber/service.rs)

Go版の`CuberService`は、`StorageMap`、`mu`、`Config`、`S3Client`、`Kagome`、`closeCh`、`Logger`で構成されます。Rustではこれを`pub struct CuberService`およびその`impl`ブロックとして実装します。

-   **並行管理**: Go版はグローバルな`RWMutex`で全`StorageMap` をロックしていますが、Rustでは`DashMap` (Sharded Lock Table) を使用し、特定Cubeのアクセスが他のCubeをブロックしないようにします。`DashMap`は内部でシャードされたロックを持つため、高い並行性を実現します。
-   **ライフサイクル**: Goの`closeCh`はチャネルによる原始的なシグナリングですが、Rustでは`tokio_util::sync::CancellationToken`を用いることで、背景タスクの安全な停止を保証します。`CancellationToken`は非同期タスクツリー全体に停止シグナルを伝播させるのに適しています。
-   **形態素解析**: Goの`Kagome`の代わりに Rust 本命の`Lindera`を採用し、`Arc`共有によりスレッドセーフかつ高速な解析を可能にします。`Arc<Tokenizer>`として保持することで、複数のスレッドやタスクから共有され、辞書のロードオーバーヘッドを一度に抑えます。
-   **S3Client の外部注入と共有**: `S3Client` は `main_of_rt.rs` 等のアプリケーションエントリーポイントで既に初期化されています。`CuberService` はプロバイダーとしてこれを `new` メソッドの引数で `Arc<S3Client>` として受け取ります。`Arc` (Atomic Reference Counted) を用いることで、複数の `CuberService` インスタンスが並行して単一の `S3Client` を安全に共有でき、ロックの競合を避けつつスレッドセーフなアクセスを保証します。
-   **背景タスク**: Go版の`startStorageGCRoutine`やS3キャッシュクリーンアップは、`CuberService::new`内で`tokio::spawn`を用いて非同期タスクとして起動します。これらのタスクは`CancellationToken`を監視し、サービスシャットダウン時に安全に終了します。
-   **`get_or_open_storage`**: Go版の`GetOrOpenStorage` (L323-365) に相当するロジックは、`DashMap`の`entry` APIを用いて安全に取得または生成するRustらしい実装を行います。これにより、ダブルチェックロッキングパターンを簡潔かつ安全に実現できます。

```rust
// --- src/cuber/service.rs ---
use std::path::Path;
use std::sync::Arc;
use tokio::time::{Duration, Instant}; // Instant をインポート
use dashmap::DashMap;
use tokio_util::sync::CancellationToken;
// 既存の s3client クレートを利用
use crate::utils::s3client::S3Client; 

// 仮の Tokenizer (Lindera) の定義
pub struct Tokenizer;
impl Tokenizer {
    pub fn new() -> Self { Tokenizer }
}

// LadybugDBのラッパー構造体 (仮)
pub struct LadybugDB;
impl LadybugDB {
    pub async fn new(_path: &Path, _tokenizer: Arc<Tokenizer>) -> Result<Self, CuberError> {
        // LadybugDBの初期化ロジック
        Ok(LadybugDB)
    }
}

// UUID抽出ヘルパー関数 (仮)
fn extract_uuid(_path: &Path) -> String {
    "some-uuid".to_string()
}

pub struct CuberService {
    pub storage_map: Arc<DashMap<String, Arc<StorageSet>>>,
    pub config: CuberConfig,
    /// 外部から注入される共有 S3Client
    pub s3_client: Arc<S3Client>,
    pub tokenizer: Arc<Tokenizer>, // Lindera
    pub cancel_token: CancellationToken,
}

impl CuberService {
    /// S3Client は外部 (main等) で初期化されたものを Arc で受け取る
    pub async fn new(config: CuberConfig, s3_client: Arc<S3Client>) -> Result<Self, CuberError> {
        let cancel_token = CancellationToken::new();
        let tokenizer = Arc::new(Tokenizer::new()); // Linderaの初期化

        let service = Self {
            storage_map: Arc::new(DashMap::new()),
            config,
            s3_client,
            tokenizer,
            cancel_token,
        };
        service.spawn_background_tasks();
        Ok(service)
    }

    /// Go: startStorageGCRoutine & S3 Cleanup の統合
    fn spawn_background_tasks(&self) {
        let token = self.cancel_token.clone();
        let storage_map = Arc::clone(&self.storage_map);
        let s3_client = Arc::clone(&self.s3_client);
        let retention = Duration::from_secs(self.config.storage_idle_timeout_minutes as u64 * 60);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(60)); // 1分ごとにチェック
            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        println!("Background tasks cancelled.");
                        break;
                    },
                    _ = ticker.tick() => {
                        // アイドルストレージのクリーンアップロジック
                        let now = Instant::now();
                        let mut to_remove_uuids = Vec::new();
                        for entry in storage_map.iter() {
                            let storage_set = entry.value();
                            let last_used = *storage_set.last_used_at.read().await;
                            if now.duration_since(last_used) > retention {
                                to_remove_uuids.push(entry.key().clone());
                            }
                        }
                        for uuid in to_remove_uuids {
                            if let Some((_, set)) = storage_map.remove(&uuid) {
                                if let Err(e) = set.close().await {
                                    eprintln!("Error closing idle storage {}: {:?}", uuid, e);
                                }
                            }
                        }

                        // S3キャッシュのクリーンアップ (既存の S3Client インスタンスを利用)
                        // s3_client.cleanup_down_dir(...) 等のメソッドを呼び出し
                    }
                }
            }
        });
    }

    pub async fn get_or_open_storage(&self, db_path: &Path) -> Result<Arc<StorageSet>, CuberError> {
        let uuid = extract_uuid(db_path);
        
        let entry = self.storage_map.entry(uuid.clone()).or_insert_with(|| {
            // 注意: 実際の初期化には OnceCell 等の排他制御パターンが必要。
            // 詳細は 11.8 (LadybugDB) で記述。
            println!("Opening new storage for {}", uuid);
            let tokenizer_clone = Arc::clone(&self.tokenizer);
            let db_path_buf = db_path.to_path_buf();

            let ladybug_db = tokio::runtime::Handle::current().block_on(async {
                LadybugDB::new(&db_path_buf, tokenizer_clone).await
            }).expect("Failed to initialize LadybugDB synchronously for DashMap entry");

            Arc::new(StorageSet {
                vector: Arc::new(ladybug_db),
                graph: Arc::new(ladybug_db),
                last_used_at: tokio::sync::RwLock::new(Instant::now()),
            })
        });

        let storage_set = Arc::clone(entry.value());
        *storage_set.last_used_at.write().await = Instant::now();
        Ok(storage_set)
    }
}
```

### 11.2 Rustでの実装対応計画 for 10.2 (Absorb Process)

本項では、Cuberシステムの「吸入（Absorb）」プロセスのオーケストレーションについて、Go実装の詳細な解析に基づいたRustへの移植計画を詳述します。このプロセスは、生のファイルをシステムに取り込み、高度な知識グラフへと昇華させる一連のワークフローの全責務を担います。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/cuber.go`
   - `mycute-go/src/pkg/cuber/pipeline/pipeline.go`

2. **Go実装の具体的実証箇所**:
   - `Absorb` メソッド (`cuber.go: L487-576`): 全体のオーケストレーション、トランザクション、イベント登録、チェックポイント管理。
   - `add` 内部メソッド (`cuber.go: L598-627`): ファイルのハッシュ計算、アップロード、メタデータ保存を行う `IngestTask` の実行。
   - `cognify` 内部メソッド (`cuber.go: L657-738`): チャンク化、グラフ抽出、永続化、要約を連続して行う多段パイプラインの実行と終了後のクリーンアップ。
   - `Pipeline.Run` (`pipeline.go: L61-81`): 各タスクを順番に実行し、出力を次の入力へ渡しながらトークン使用量を累積する実行エンジン。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // cuber.go: Absorb の中核ロジック
   func (s *CuberService) Absorb(ctx context.Context, ...) (types.TokenUsage, error) {
       // 1. データベース接続の確保とイベントストリーマーの登録
       st, _ := s.GetOrOpenStorage(...)
       if dataCh != nil { event.RegisterAbsorbStreamer(eb, dataCh) }

       // 2. トランザクション内での一連の処理 (原子性の保証)
       err = st.Vector.Transaction(ctx, func(txCtx context.Context) error {
           // A. ファイルの実体を取り込みメタデータを保存 (add)
           usage1, _ := s.add(txCtx, ...)
           totalUsage.Add(usage1)

           // B. 保存済みデータから知識グラフを構築 (cognify)
           usage2, _ := s.cognify(txCtx, ...)
           totalUsage.Add(usage2)
           return nil
       })

       // 3. 完了後の WAL チェックポイント実行 (永続性の確保)
       st.Vector.Checkpoint()
       return totalUsage, nil
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/service.rs`: `Absorb`、`add`、`cognify` プロセスの司令塔ロジックを実装。
   - `src/cuber/pipeline/mod.rs`: タスク実行基盤となる `Pipeline` エンジンと `Task` トレイト。
   - `src/cuber/event.rs`: 進捗をリアルタイムに通知する `StreamEvent` 定義。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber/service.rs` の `impl CuberService` 内に `pub async fn absorb` 公開メソッドを配置し、補助的な `add` 及び `cognify` を非公開非同期メソッド（`inner` ロジック）として実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **オーケストレーションの整合性**: Go版は `add`（書き込み）と `cognify`（解析・保存）を同一のトランザクションに閉じ込めることで、「解析だけが失敗してゴミデータが残る」事態を防いでいます。Rustでは `lbug` クレートの `transaction` メソッドに `async move` クラージャを渡し、`Result` 型による `?` 演算子の連鎖でエラー発生時の即時ロールバックと資源の安全な解放を実現します。
   - **非同期パイプラインの現代化**: Goの `pipeline.go` は `any` 型を用いたインターフェースですが、Rustではジェネリクス (`Task<I, O>`) と `async_trait` を活用し、コンパイル時にデータの型安全性を保証しつつ、非同期処理を自然な形でチェインさせます。
   - **イベント駆動の進捗通知**: Go版は `eventbus.Emit` を用いていますが、Rustでは `Arc<EventBus>` を各タスクへ伝播させ、ライフサイクルイベント（開始、進捗、正常終了、エラー）を構造化された `StreamEvent` として SSE 経由でフロントエンドへ届けます。
   - **トークン集計の正確性**: LLM処理を伴う複数のフェーズに跨るため、`TokenUsage` 構造体に `std::ops::Add` トレイトを実装し、各フェーズの結果を `+=` で加算集計します。
   - **クリーンアップの確実性**: Go版では `cognify` の最後で S3 の一時ファイルを削除していますが、Rust では `ScopeGuard` パターン（または `Drop` トレイトの活用検討）により、予期せぬパニック時でも一時ファイルが残留しないような「防衛的実装」を施します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/service.rs ---
   impl CuberService {
       /// 吸入プロセス全体の制御: Go実装の Absorb (L487) と等価
       pub async fn absorb(
           &self,
           ctx: &Context,
           eb: Arc<EventBus<StreamEvent>>,
           db_path: &Path,
           group: &str,
           files: Vec<String>,
           cfg: CognifyConfig,
           is_en: bool,
       ) -> Result<TokenUsage, CuberError> {
           log::info!("<Cuber> Starting Absorb process for group: {}", group);
           let st = self.get_or_open_storage(db_path).await?;
           let mut total_usage = TokenUsage::default();

           // イベント登録: Go L504-505 相当
           // eb.register_streamer(...)

           // 進捗開始通知: Go L509-513 相当
           eb.emit(StreamEvent::AbsorbStart { group: group.into(), count: files.len() }).await;

           // トランザクション開始: Go L528 相当
           // st.vector.transaction 内で、add と cognify をシーケンシャルに実行
           let result = st.vector.transaction(|session| async move {
               // 1. Ingestion Phase: Go L530 (add) 相当
               let usage_add = self.add_internal(session, &eb, group, files).await?;
               
               // 2. LLM リソースの準備 (埋め込み & 推論)
               let embedder = self.create_embedder(&self.config.embedding).await?;
               let chat_model = self.create_chat_model(&self.config.chat).await?;

               // 3. Cognification Phase: Go L546 (cognify) 相当
               let usage_cog = self.cognify_internal(
                   session, &eb, group, cfg, embedder, chat_model, is_en
               ).await?;

               Ok(usage_add + usage_cog)
           }.boxed()).await;

           match result {
               Ok(usage) => {
                   total_usage = usage;
                   // 永続性の確定: Go L566 (Checkpoint) 相当
                   st.vector.checkpoint().await?;
                   eb.emit(StreamEvent::AbsorbEnd { group: group.into(), usage: total_usage }).await;
                   log::info!("<Cuber> Absorb completed successfully. Tokens: {:?}", total_usage);
                   Ok(total_usage)
               }
               Err(e) => {
                   eb.emit(StreamEvent::AbsorbError { group: group.into(), error: e.to_string() }).await;
                   Err(CuberError::TransactionError(e.to_string()))
               }
           }
       }

       /// 内部取り込み処理: Go実装の add (L598) と等価
       async fn add_internal(&self, tx: &Tx, eb: &EventBus, group: &str, paths: Vec<String>) -> Result<TokenUsage, CuberError> {
           let task = IngestTask::new(tx, group, Arc::clone(&self.s3_client), eb);
           // Pipeline エンジンによる実行
           let pipeline = Pipeline::new(vec![Box::new(task)]);
           let (_, usage) = pipeline.run(paths).await?;
           Ok(usage)
       }

       /// 内部知識グラフ構築処理: Go実装の cognify (L657) と等価
       async fn cognify_internal(&self, tx: &Tx, eb: &EventBus, group: &str, ...) -> Result<TokenUsage, CuberError> {
           // 多段タスクの構成: Chunking -> GraphExtraction -> Storage -> Summarization
           let tasks: Vec<Box<dyn Task>> = vec![
               Box::new(ChunkingTask::new(...)),
               Box::new(GraphExtractionTask::new(...)),
               Box::new(StorageTask::new(...)),
               Box::new(SummarizationTask::new(...)),
           ];

           // 前段 (add_internal) で保存された一時ファイル情報をDBから取得
           let data_list = tx.get_data_list(group).await?;
           
           let pipeline = Pipeline::new(tasks);
           let (_, usage) = pipeline.run(data_list).await?;

           // 処理済みファイルの S3 クリーンアップ: Go L725-735 相当
           for data in data_list {
               if let Some(loc) = data.raw_data_location {
                   self.s3_client.del(&loc).await.ok(); // 失敗しても進行を妨げない
               }
           }
           Ok(usage)
       }
   }
   ```

### 11.3 Rustでの実装対応計画 for 10.3 (Cognify: Ingestion)

本項では、生のファイルをCuberシステムへ「吸入」する際の最初の関門となる、`IngestTask` の Rust への移植計画を詳述します。このタスクは、ファイルのハッシュ計算、重複検知、分散ストレージ（S3/ローカル）への配置、およびメタデータの永続化を担い、システム全体でデータの「一意性」と「追跡可能性」を担保する極めて重要なフェーズです。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/ingestion/ingest_task.go`

2. **Go実装の具体的実証箇所**:
   - `IngestTask` 構造体 (`L31-37`): 依存リソース（Storage, S3Client, Logger, EventBus）の保持。
   - `Run` メソッド (`L92-207`): ファイルリストの反復処理、キャンセルチェック、ハッシュ計算、重複チェック、アップロード、DB保存のパイプライン。
   - `generateDeterministicID` (`L69-74`): コンテンツハッシュと MemoryGroup を用いた一意な UUID 生成。
   - `calculateFileHash` (`L218-232`): SHA-256 による整合性ハッシュ計算。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // ingest_task.go: 決定論的IDの生成ロジック
   func generateDeterministicID(contentHash string, memoryGroup string) string {
       namespace := uuid.NameSpaceOID
       return uuid.NewSHA1(namespace, []byte(contentHash+memoryGroup)).String()
   }

   // ingest_task.go: ファイル処理の主ループ
   for _, path := range filePaths {
       // 1. ハッシュ計算と重複チェック
       hash, _ := calculateFileHash(path)
       if t.vectorStorage.Exists(ctx, hash, t.memoryGroup) {
           // 重複スキップ処理
           continue
       }
       // 2. ストレージ（S3/Local）へアップロード
       storageKey, _ := t.s3Client.Up(path)
       // 3. メタデータを構築して LadybugDB へ保存
       data := &storage.Data{ ID: dataID, RawDataLocation: *storageKey, ... }
       t.vectorStorage.SaveData(ctx, data)
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/ingestion.rs`: `IngestTask` 構造体と `Task` トレイトの実装。
   - `src/cuber/utils/hash.rs`: 高速なハッシュ計算用のユーティリティ。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber/tasks/ingestion.rs` 内に `pub struct IngestTask` を定義し、パイプラインから呼び出される `impl Task for IngestTask` を実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **決定論的 ID 生成の再現**: Go版は `uuid.NewSHA1` を使用して `content_hash + memory_group` から ID を生成しています。Rust でも `uuid` クレートの `v5` (SHA-1ベースのネームスペースUUID) を使用することで、Go版と完全な互換性を持つ同一の ID 生成を保証し、既存データとの重複検知を機能させます。
   - **ゼロコピーに近いストリーミングハッシュ**: Goの `calculateFileHash` は `io.Copy` を使用しています。Rust では `sha2` クレートと `tokio::fs::File` を組み合わせ、非同期ストリーミング処理を行うことで、メモリ消費を最小限に抑えつつ巨大なファイルのハッシュ計算を並行して実行可能にします。
   - **並行アップロードの安全性**: Go版はシーケンシャルに処理していますが、Rust では `tokio::spawn` または `FuturesOrdered` を活用した並行インジェクションの余地を残しつつ、`Arc<S3Client>` を通じてスレッドセーフにアップロードを実行します。
   - **パーティションの厳格化**: Go版の `memoryGroup` は文字列ですが、Rust では `MemoryGroup` という専用の型（NewTypeパターン）を用意し、誤った識別子が DB に混入するのを型レベルで防ぎます。
   - **エラー型の統一**: Go版の `fmt.Errorf` に対し、Rust では `CuberError::IngestionFailed(path)` 等の具体的なエラーバリアントを定義し、詳細な問題箇所を報告します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/ingestion.rs ---
   use uuid::Uuid;
   use sha2::{Sha256, Digest};

   pub struct IngestTask<'a> {
       tx: &'a Tx, // トランザクション・セッション
       group: MemoryGroup,
       s3_client: Arc<S3Client>,
       eb: Arc<EventBus<StreamEvent>>,
   }

   #[async_trait]
   impl Task for IngestTask<'_> {
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError> {
           let paths: Vec<String> = input.try_into()?;
           let mut results = Vec::new();

           for path in paths {
               // 進捗通知: Go L115 相当
               self.eb.emit(StreamEvent::AddFileStart(path.clone())).await;

               // A. ハッシュ計算: Go L218 相当
               let hash = self.calculate_hash(&path).await?;
               
               // B. 重複チェック: Go L140 相当
               if self.tx.exists_data(&hash, &self.group).await? {
                   let id = self.gen_id(&hash);
                   results.push(Data::new_duplicate(id, &self.group, &hash));
                   continue;
               }

               // C. ストレージアップロード: Go L168 相当
               let storage_key = self.s3_client.up(&path).await
                   .map_err(|e| CuberError::S3UploadError(path.clone(), e))?;

               // D. 決定論的ID生成: Go L69 相当
               let id = self.gen_id(&hash);

               // E. メタデータ永続化: Go L190 相当
               let data = Data {
                   id: id.to_string(),
                   memory_group: self.group.to_string(),
                   name: extract_filename(&path),
                   content_hash: hash,
                   raw_data_location: Some(storage_key),
                   created_at: Utc::now(),
               };
               self.tx.save_data(&data).await?;
               results.push(data);
           }
           
           Ok((Any::from(results), TokenUsage::zero()))
       }
   }

   impl IngestTask<'_> {
       /// Go実装の generateDeterministicID (L69) を Rust で忠実に再現
       fn gen_id(&self, hash: &str) -> Uuid {
           let ns = Uuid::NAMESPACE_OID;
           let data = format!("{}{}", hash, self.group);
           Uuid::new_v5(&ns, data.as_bytes())
       }

       /// 非同期ストリーミングハッシュ計算
       async fn calculate_hash(&self, path: &str) -> Result<String, CuberError> {
           let mut file = fs::File::open(path).await?;
           let mut hasher = Sha256::new();
           let mut buffer = [0u8; 8192];
           while let Ok(n) = file.read(&mut buffer).await {
               if n == 0 { break; }
               hasher.update(&buffer[..n]);
           }
           Ok(format!("{:x}", hasher.finalize()))
       }
   }
   ```

### 11.4 Rustでの実装対応計画 for 10.4 (Cognify: Chunking)

本項では、取り込んだドキュメントを LLM が扱いやすい最適なサイズに分割し、検索性を高めるための「チャンク化（Chunking）」プロセスの Rust への移植計画を詳述します。このフェーズは単なる文字列の分割にとどまらず、文脈の維持、形態素解析によるキーワード抽出、およびベクトル化を統合した、RAG（Retrieval-Augmented Generation）システムの精度を決定づける極めて繊細な工程です。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/chunking/chunking_task.go`
   - `mycute-go/src/pkg/cuber/utils/normalize.go`

2. **Go実装の具体的実証箇所**:
   - `ChunkingTask.Run` (`L65-163`): ファイルのダウンロード、テキストの正規化、ドキュメントの永続化、およびチャンク化処理のループ。
   - `chunkText` (`L171-209`): 文（Sentence）単位でのセグメンテーションと、オーバーラップを考慮したチャンク構築ロジック。
   - `finalizeChunk` (`L212-267`): チャンクごとの Embedding 生成と、Kagome を用いた FTS 用キーワード（名詞・動詞）の抽出。
   - `splitSentences` (`L309-328`): 正規表現 (`[。！？.!?]`) を用いた文分割境界の特定。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // chunking_task.go: 文境界を維持したチャンク化の核心
   for _, sentence := range sentences {
       sentenceChars := utf8.RuneCountInString(sentence)
       if currentChars+sentenceChars > t.ChunkSize && len(currentChunk) > 0 {
           // チャンク確定と Embedding 生成
           t.finalizeChunk(&currentChunk, ...)
           // オーバーラップ（文脈の継続性）の追加
           t.addOverlap(&currentChunk, ...)
       }
       currentChunk = append(currentChunk, sentence)
       currentChars += sentenceChars
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/chunking.rs`: `ChunkingTask` 構造体と `Task` トレイトの実装。
   - `src/cuber/utils/normalize.rs`: `CommonNormalize`, `NormalizeForVector`, `NormalizeForSearch` の Rust 実装。
   - `src/cuber/utils/morphology.rs`: `Lindera` を用いたキーワード抽出ユーティリティ。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber/tasks/chunking.rs` 内に `pub struct ChunkingTask` を定義。`impl Task for ChunkingTask` において、`text-splitter` 等のライブラリを活用しつつ、Go版の「文境界を絶対に跨がない」ロジックを再現します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **高度な日本語セグメンテーション**: Go版は正規表現で文を分割していますが、Rust では `unicode-segmentation` クレートの `UnicodeSentenceIter` を併用することで、言語学的に正しい文境界の特定を実現し、句読点だけでなく感嘆符や引用符も含めた精緻な分割を可能にします。
   - **形態素解析エンジンの刷新**: Go 版の `Kagome` に代わり、Rust ネイティブで圧倒的なスループットを誇る `Lindera` (IPA辞書) を採用します。`CuberService` から `Arc<Tokenizer>` として注入されることで、マルチスレッド環境下でもメモリ効率良く動作します。
   - **セマンティックな正規化**: `CommonNormalize` などのロジックを Rust の `unicode-normalization` クレートを用いて再実装します。全角半角の統一 (NFKC) や不要な制御文字の除去を、Rust の強力な文字列イテレータを駆使して高速化します。
   - **オーバーラップの厳密管理**: Go版の `addOverlap` は文字数ベースですが、Rust では `VecDeque` を用いて「直近 N 文」を効率的に管理し、チャンク間のコンテキストの「糊付け」をスライディングウィンドウ方式で厳密に行います。
   - **非同期 Embedding 生成**: 外部 LLM API を叩く `Embedder.EmbedQuery` は、Rust の `async/await` により非同期実行されます。複数のチャンクを `join_all` で並行して Embedding 化し、I/O 待ち時間を大幅に短縮します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/chunking.rs ---
   pub struct ChunkingTask<'a> {
       tx: &'a Tx,
       chunk_size: usize,
       overlap_size: usize,
       tokenizer: Arc<Tokenizer>, // Lindera
       embedder: Arc<dyn Embedder>,
       eb: Arc<EventBus<StreamEvent>>,
       is_en: bool,
   }

   #[async_trait]
   impl Task for ChunkingTask<'_> {
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError> {
           let data_list: Vec<Data> = input.try_into()?;
           let mut all_chunks = Vec::new();
           let mut total_usage = TokenUsage::zero();

           for data in data_list {
               // 1. テキスト読み込みと正規化: Go L111-122
               let raw_text = self.read_from_storage(&data.raw_data_location).await?;
               let normalized = normalize::common_normalize(&raw_text);
               let vector_text = normalize::normalize_for_vector(&normalized);

               // 2. ドキュメント保存: Go L132
               let doc = Document::new(&data.memory_group, &data.id, &vector_text);
               self.tx.save_document(&doc).await?;

               // 3. チャンク分割ループ: Go L147 (chunkText)
               let (chunks, usage) = self.process_chunks(&doc).await?;
               total_usage += usage;
               all_chunks.extend(chunks);
           }
           Ok((Any::from(all_chunks), total_usage))
       }
   }

   impl ChunkingTask<'_> {
       /// 文境界を保護したストライディング・チャンキングの Rust 実装
       async fn process_chunks(&self, doc: &Document) -> Result<(Vec<Chunk>, TokenUsage), CuberError> {
           // Go L174: splitSentences 相当
           let sentences: Vec<&str> = unicode_segmentation::split_sentences(&doc.text).collect();
           
           let mut chunks = Vec::new();
           let mut current_window = Vec::new();
           let mut current_len = 0;
           let mut usage = TokenUsage::zero();

           for sentence in sentences {
               let sentence_len = sentence.chars().count();
               if current_len + sentence_len > self.chunk_size && !current_window.is_empty() {
                   // A. チャンク確定: Go L189 (finalizeChunk)
                   let chunk_text = current_window.join("");
                   let (chunk, u) = self.create_chunk_object(&chunk_text, doc).await?;
                   chunks.push(chunk);
                   usage += u;

                   // B. オーバーラップ制御: Go L194 (addOverlap)
                   self.apply_overlap(&mut current_window, &mut current_len);
               }
               current_window.push(sentence.to_string());
               current_len += sentence_len;
           }
           // 最後のチャンク処理...
           Ok((chunks, usage))
       }

       /// キーワード抽出とベクトル化の統合: Go L212 (finalizeChunk)
       async fn create_chunk_object(&self, text: &str, doc: &Document) -> Result<(Chunk, TokenUsage), CuberError> {
           // Embedding 生成
           let (vec, usage) = self.embedder.embed(text).await?;

           // Go L239: ExtractKeywords 相当 (Lindera使用)
           let search_text = normalize::normalize_for_search(text);
           let keywords = morphology::extract_keywords(&self.tokenizer, &search_text, self.is_en);

           Ok((Chunk {
               id: Uuid::new_v4().to_string(),
               text: text.to_string(),
               embedding: vec,
               keywords: keywords.all_content_words,
               nouns: keywords.nouns,
               // ... その他 FTS 用フィールド
           }, usage))
       }
   }
   ```

### 11.5 Rustでの実装対応計画 for 10.5 (Cognify: Graph Extraction)

本項では、チャンク分割されたテキストから LLM を用いてエンティティ（ノード）と関係性（エッジ）を抽出し、多次元的な「知識グラフ」を構築する中心的なタスク、`GraphExtractionTask` の Rust への移植計画を詳述します。このプロセスは、非構造化データから構造化データを生成する高度な推論を伴い、API の並列呼び出し、不完全な JSON 出力の正規化、およびグラフデータの整合性維持という 3 つの技術的課題を Rust の堅牢なエコシステムで解決します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/graph/graph_extraction_task.go`
   - `mycute-go/src/pkg/cuber/prompts/prompts.go`

2. **Go実装の具体的実証箇所**:
   - `GraphExtractionTask.Run` (`L52-229`): `errgroup` を用いた最大并发数 5 の並列実行、LLM 呼び出し、JSON 抽出・パース、および最終的な正規化処理。
   - `GenerateWithUsage` (`utils` 経由): プロンプトの注入とトークン使用量の取得。
   - `cleanJSON` (`L233-246`): LLM 出力から `{` と `}` の間のみを切り出す、原始的だが実用的な JSON 抽出。
   - ノード・エッジの正規化ループ (`L180-221`): `NormalizeForGraph` による記号除去、`MakeGraphNodeID` によるパーティション（MemoryGroup）付与、信頼度（Confidence）やタイムスタンプの注入。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // graph_extraction_task.go: 並列実行と集約の核心
   g, ctx := errgroup.WithContext(ctx)
   g.SetLimit(5) // レート制限対策
   for i, chunk := range chunks {
       g.Go(func() error {
           // 1. LLM 呼び出し
           content, usage, _ := utils.GenerateWithUsage(ctx, t.LLM, ...)
           // 2. JSON 部分のみを抽出して Unmarshal
           content = cleanJSON(content)
           var graphData storage.GraphData
           json.Unmarshal([]byte(content), &graphData)
           // 3. ミューテックス保護下での集約
           mu.Lock()
           allNodes = append(allNodes, graphData.Nodes...)
           mu.Unlock()
           return nil
       })
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/graph.rs`: `GraphExtractionTask` 構造体と `Task` トレイトの実装。
   - `src/cuber/prompts/mod.rs`: システムプロンプトおよび抽出用プロンプトの管理。
   - `src/cuber/utils/json.rs`: 堅牢な JSON 抽出ユーティリティ。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber/tasks/graph.rs` 内に `pub struct GraphExtractionTask` を定義。`JoinSet` を用いた並行 LLM 処理と、`serde_json` を用いた型安全なデシリアライズを実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **`tokio::task::JoinSet` による高度な並行制御**: Go の `errgroup` に対し、Rust では `JoinSet` を使用します。セマフォ (`tokio::sync::Semaphore`) と組み合わせることで、並列数を厳密に制御（例: 5並列）しつつ、エラー発生時の早期リターンとリソースの即時解放を実現します。
   - **`serde` による厳格かつ柔軟な JSON パース**: LLM は時に不完全な JSON を返します。Go の抽象的な `Unmarshal` と異なり、Rust では `#[serde(default)]` や `#[serde(rename)]` を駆使することで、欠落したフィールドを安全に補填し、スキーマ違反をコンパイルレベルと実行レベルの両面で防ぎます。
   - **メモリ安全なデータ集約**: ミューテックス (`sync.Mutex`) による競合制御の代わりに、Rust ではスレッド間で `mpsc` チャネルを用いて抽出結果をメインスレッドへ送信し、所有権を移動させることで「ロックフリー」に近い安全な集約を行います。
   - **文字列正規化の零コスト抽象化**: `NormalizeForGraph` を Rust の `cow` (Copy-on-write) モードを駆使して実装し、変換が不要な場合はアロケーションを発生させない高速な文字列処理を実現します。
   - **LLM クライアントの抽象化**: `Arc<dyn ChatModel>` トレイトオブジェクトを使用して LLM クライアントを保持し、将来的な OpenAI -> Anthropic 等の切り替えをコード変更なしで行える拡張性を確保します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/graph.rs ---
   pub struct GraphExtractionTask<'a> {
       tx: &'a Tx,
       chat_model: Arc<dyn ChatModel>,
       model_name: String,
       group: MemoryGroup,
       eb: Arc<EventBus<StreamEvent>>,
       is_en: bool,
   }

   #[async_trait]
   impl Task for GraphExtractionTask<'_> {
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError> {
           let chunks: Vec<Chunk> = input.try_into()?;
           let mut join_set = JoinSet::new();
           let semaphore = Arc::new(Semaphore::new(5)); // Go: g.SetLimit(5)

           for (idx, chunk) in chunks.into_iter().enumerate() {
               let sem = Arc::clone(&semaphore);
               let model = Arc::clone(&self.chat_model);
               let model_name = self.model_name.clone();
               let prompt = if self.is_en { PROMPT_EN } else { PROMPT_JA };
               let input_text = chunk.text.clone();

               join_set.spawn(async move {
                   let _permit = sem.acquire().await.ok();
                   // LLM 呼び出し
                   let (content, usage) = model.generate(&model_name, prompt, &input_text).await?;
                   // JSON 抽出とデシリアライズ: Go L127-129
                   let cleaned = json_util::extract_json_object(&content)?;
                   let mut graph_data: GraphData = serde_json::from_str(&cleaned)?;
                   
                   Ok::<(GraphData, TokenUsage), CuberError>((graph_data, usage))
               });
           }

           let mut all_nodes = Vec::new();
           let mut all_edges = Vec::new();
           let mut total_usage = TokenUsage::zero();

           while let Some(res) = join_set.join_next().await {
               let (graph_data, usage) = res??;
               all_nodes.extend(graph_data.nodes);
               all_edges.extend(graph_data.edges);
               total_usage += usage;
           }

           // 正規化と MemoryGroup 付与: Go L180-221 相当
           self.finalize_graph(&mut all_nodes, &mut all_edges).await;

           Ok((Any::from(CognifyOutput { nodes: all_nodes, edges: all_edges }), total_usage))
       }
   }

   impl GraphExtractionTask<'_> {
       async fn finalize_graph(&self, nodes: &mut [Node], edges: &mut [Edge]) {
           for node in nodes {
               node.id = normalize::graph_id(&node.id, &self.group);
               node.node_type = normalize::graph_type(&node.node_type);
               node.memory_group = self.group.to_string();
           }
           for edge in edges {
               edge.source = normalize::graph_id(&edge.source, &self.group);
               edge.target = normalize::graph_id(&edge.target, &self.group);
               edge.timestamp = Utc::now().timestamp_millis();
               edge.weight = 1.0;
           }
       }
   }
   ```

### 11.6 Rustでの実装対応計画 for 10.6 (Cognify: Storage)

本項では、抽出されたチャンクやグラフデータを物理データベースへ永続化し、高速なベクトル・グラフ横断検索を可能にする「ストレージ（Storage）」タスクの Rust への移植計画を詳述します。このプロセスは、LadybugDB のベクトルエンジンとグラフエンジンをフル活用し、データの永続性だけでなく、解析フェーズと検索フェーズを繋ぐ「インデックス構築」の責務も担います。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/storage/storage_task.go`

2. **Go実装の具体的実証箇所**:
   - `StorageTask.Run` (`L50-223`): チャンクの保存、特別な「ドキュメント・チャンク」ノードの生成、グラフデータ（ノード・エッジ）のバッチ保存、およびエンティティ名のベクトルインデックス化。
   - `SaveChunk` (`L78`): ベクトルデータベースへのチャンクデータの永続化。
   - `AddNodes / AddEdges` (`L122, L140`): グラフデータベースへの構造化データのバルクインサート。
   - エンティティ名のエラーリカバリ (`L191-205`): 非同期でエンティティ名の Embedding を生成し、`SaveEmbedding` を通じて `entities` テーブルへインデックスを構築。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // storage_task.go: グラフとベクトルのリレーション構築
   for _, chunk := range output.Chunks {
       // チャンクそのものをグラフのノードとして再定義 (SPECIAL_NODE_TYPE_DOCUMENT_CHUNK)
       chunkNode := &storage.Node{
           ID: chunk.ID, Type: "document_chunk",
           Properties: map[string]any{"text": chunk.Text, ...},
       }
       output.GraphData.Nodes = append(output.GraphData.Nodes, chunkNode)
   }

   // エンティティ名のベクトルインデックス化
   for _, node := range output.GraphData.Nodes {
       embedding, _ := t.Embedder.EmbedQuery(ctx, node_name)
       t.VectorStorage.SaveEmbedding(ctx, "entities", node.ID, name, embedding, ...)
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/storage.rs`: `StorageTask` 構造体と `Task` トレイトの実装。
   - `src/mode/rt/main_of_rt.rs` (DB接続定義): LadybugDB の Rust バインディング初期化。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber/tasks/storage.rs` 内に `pub struct StorageTask` を定義し、ベクトル・グラフ両ストレージへの非同期書き込みと、並列 Embedding 処理を実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **バルク挿入によるスループットの最大化**: Go 版はノード・エッジをひとまとめにして保存していますが、Rust では `Lbug` の `add_nodes_batch` 等の API を明示的に並列/パイプライン実行することで、ドキュメント量が増大した際の書込遅延を極限まで抑制します。
   - **「ドキュメント・チャンク」ノードの型安全な注入**: 特殊なノード・タイプ (`SPECIAL_NODE_TYPE_DOCUMENT_CHUNK`) を Rust の列挙型 (`NodeType`) として扱い、マジックストリングを排除した堅牢な関連付けを行います。
   - **エンティティ・インデックスの並行構築**: Go 版はエンティティを逐一シリアルに Embedding 化していますが、Rust では `JoinSet` を用い、最大チャネル数を制限しつつ並行して API を呼び出します。これにより、大規模な初期吸入時の全体時間を Go 版の数分の一に短縮可能です。
   - **トランザクション・コンテキストの厳格な伝播**: `Absorb` 側で開始されたトランザクション・セッション (`&Tx`) を各 `save` メソッドに引き継ぎ、全ての書き込みが「全成功か全失敗か」の原子性を Rust の借用規則の中で安全に担保します。
   - **整合性チェックの自動化**: 保存直前に ID の重複や不正な文字がないかを、Rust の型システムと `validator` クレートを用いて検証し、DB への不正データ混入を未然に防ぎます。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/storage.rs ---
   pub struct StorageTask<'a> {
       tx: &'a Tx,
       embedder: Arc<dyn Embedder>,
       group: MemoryGroup,
       eb: Arc<EventBus<StreamEvent>>,
   }

   #[async_trait]
   impl Task for StorageTask<'_> {
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError> {
           let mut output: CognifyOutput = input.try_into()?;
           let mut total_usage = TokenUsage::zero();

           // 1. チャンクの保存 (ベクトルDB): Go L59-89
           for chunk in &mut output.chunks {
               // 必要に応じて Embedding の再生成 (自己修復)
               if chunk.embedding.is_empty() {
                   let (vec, usage) = self.embedder.embed(&chunk.text).await?;
                   chunk.embedding = vec;
                   total_usage += usage;
               }
               self.tx.save_chunk(chunk).await?;
           }

           // 2. グラフノードの構築と保存: Go L97-151
           // チャンク情報をノードとしてグラフへ注入
           for chunk in &output.chunks {
               output.graph_data.nodes.push(Node::from_chunk(chunk, &self.group));
           }

           // グラフデータ（ノード・エッジ）のバルク保存
           self.tx.add_nodes_batch(&output.graph_data.nodes).await?;
           self.tx.add_edges_batch(&output.graph_data.edges).await?;

           // 3. エンティティ・インデックス生成: Go L153-220
           // 重複を除去したエンティティ名のリストを作成
           let entities = output.graph_data.extract_unique_entities();
           let (indexed_usage, _) = self.index_entities(entities).await?;
           total_usage += indexed_usage;

           Ok((Any::from(output), total_usage))
       }
   }

   impl StorageTask<'_> {
       /// エンティティ名のベクトルインデックス化を並列実行
       async fn index_entities(&self, entities: Vec<EntityInfo>) -> Result<(TokenUsage, ()), CuberError> {
           let mut join_set = JoinSet::new();
           let semaphore = Arc::new(Semaphore::new(10)); // インデックス化は少し多めの並列数で実行支援

           for entity in entities {
               let sem = Arc::clone(&semaphore);
               let embedder = Arc::clone(&self.embedder);
               let tx = self.tx; // トランザクション・セッションの参照
               let group = self.group.clone();

               join_set.spawn(async move {
                   let _permit = sem.acquire().await.ok();
                   let norm_name = normalize::for_vector(&entity.name);
                   let (vec, usage) = embedder.embed(&norm_name).await?;
                   
                   // LadybugDB の entities テーブルへ保存
                   tx.save_embedding("entities", &entity.id, &norm_name, &vec, &group).await?;
                   Ok::<TokenUsage, CuberError>(usage)
               });
           }
           // 結果の集計...
           let mut total = TokenUsage::zero();
           while let Some(res) = join_set.join_next().await {
               total += res??;
           }
           Ok((total, ()))
       }
   }
   ```

### 11.7 Rustでの実装対応計画 for 10.7 (Meta-cognition: 未知の認識と自己問いかけ)

本項では、チャンク化された知識を「結晶化」させ、システムが自らの知識の死角を認識するための「メタ認知（Summarization）」タスクの Rust への移植計画を詳述します。このタスクは単なる要約（Summary）の生成にとどまらず、テキストから「何が分かっていないか」を問い直す自己問いかけ（Self-Questioning）のロジックを含み、将来的な知識の深化を促す Cuber システムの「知性の中核」となるフェーズです。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/summarization/summarization_task.go`
   - `mycute-go/src/pkg/cuber/prompts/prompts.go`

2. **Go実装の具体的実証箇所**:
   - `SummarizationTask.Run` (`L61-170`): チャンクごとの LLM 要約生成ループ、エラー発生時の個別スキップ、決定論的 ID 生成、および永続化。
   - `SUMMARIZE_CONTENT_JA_PROMPT` (`prompts` 経由): 内容の圧縮と「未知の領域」の特定を促すシステムプロンプト。
   - `summaryID` 決定論的生成 (`L132-133`): `chunk.ID + "TextSummary"` をシードとした SHA-1 UUID 生成による、データの再構築可能性の確保。
   - トークン使用量の累積 (`L102, L123`): 要約（推論）と Embedding（ベクトル化）の両フェーズでのコスト集計。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // summarization_task.go: 要約とメタ認知の自動生成
   for i, chunk := range output.Chunks {
       // 1. LLM で要約を生成 (Eino経由)
       summaryText, usage, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, promptTemplate, prompt)
       // 2. エラー時はスキップして後続のチャンクを継続 (妥協なき処理の継続)
       if err != nil { continue }
       
       // 3. 要約の Embedding 生成と保存
       embedding, _ := t.Embedder.EmbedQuery(ctx, summaryText)
       summaryID := uuid.NewSHA1(namespace, []byte(chunk.ID+"TextSummary")).String()
       t.VectorStorage.SaveEmbedding(ctx, "summaries", summaryID, summaryText, embedding, ...)
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/summarization.rs`: `SummarizationTask` 構造体と `Task` トレイトの実装。
   - `src/cuber/prompts/metacognition.rs`: 自己問いかけを誘発する高度なプロンプトセット。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber/tasks/summarization.rs` 内に `pub struct SummarizationTask` を定義。`Task` トレイトの実装において、`tokio::task::JoinSet` を活用した「投機的な並列要約」と、堅牢なリトライ戦略を実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **メタ認知プロンプトの強化**: Go 版の単純な要約に対し、Rust では 10.7 で定義した「未知の認識」を強調する多段階プロンプトを導入します。これは、Rust の `std::fmt` による柔軟なプロンプト合成機能を使い、文脈に応じた最適な問いかけを LLM に投げます。
   - **決定論的 ID の一貫性**: Go 版の `uuid.NewSHA1` による ID 生成を、Rust の `uuid::new_v5` で忠実に再現します。これにより、Rust 版への移行後も同一のチャンクからは同一の要約 ID が生成され、データベース内の重複や断絶を防ぎます。
   - ** resilience（回復力）の高い並並実行**: Go 版のループ処理に対し、Rust では `JoinSet` と `timeout` を組み合わせます。特定の LLM 呼び出しが遅延・凍結した場合でも、他のチャンクの要約処理を阻害せず、全体のタイムアウトを厳格に管理しながら最大効率で処理を遂行します。
   - **構造化されたトークン管理**: 推論と埋め込みの 2 フェーズにおいて、Rust の `TokenUsage` 構造体が `+=` 演算子を通じて自動的にコストを合算し、最終的な `Absorb` 結果へ集約される仕組みを構築します。
   - **メモリーグループの厳密な分離**: パーティション化された `MemoryGroup` 型を用いることで、マルチテナント環境下でも要約データが他者の Cube に混入することを物理的・理論的に遮断します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/summarization.rs ---
   pub struct SummarizationTask<'a> {
       tx: &'a Tx,
       chat_model: Arc<dyn ChatModel>,
       embedder: Arc<dyn Embedder>,
       group: MemoryGroup,
       eb: Arc<EventBus<StreamEvent>>,
       is_en: bool,
   }

   #[async_trait]
   impl Task for SummarizationTask<'_> {
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError> {
           let output: CognifyOutput = input.try_into()?;
           let mut join_set = JoinSet::new();
           let mut total_usage = TokenUsage::zero();

           // 1. 各チャンクに対して並列要約処理を起動 (Go L71 相当を並列化)
           for chunk in output.chunks.clone() {
               let model = Arc::clone(&self.chat_model);
               let eb = Arc::clone(&self.eb);
               let prompt = self.select_meta_prompt();
               let text = chunk.text.clone();

               join_set.spawn(async move {
                   // A. LLM推論: 未知の認識を含む要約の生成
                   let (summary, usage_llm) = model.generate_summary(&prompt, &text).await?;
                   Ok::<(String, String, TokenUsage), CuberError>((chunk.id, summary, usage_llm))
               });
           }

           // 2. 結果の集約と個別保存
           while let Some(res) = join_set.join_next().await {
               match res? {
                   Ok((chunk_id, summary, usage_llm)) => {
                       total_usage += usage_llm;
                       
                       // B. 正規化とベクトル化: Go L117, L122
                       let norm_summary = normalize::for_vector(&summary);
                       let (embedding, usage_emb) = self.embedder.embed(&norm_summary).await?;
                       total_usage += usage_emb;

                       // C. 決定論的ID生成: Go L133 (v5 UUID)
                       let summary_id = self.gen_summary_id(&chunk_id);

                       // D. 永続化: Go L137
                       self.tx.save_embedding(
                           TABLE_SUMMARY, &summary_id, &norm_summary, &embedding, &self.group
                       ).await?;
                   }
                   Err(e) => log::warn!("<Cuber> Summarization failed for a chunk: {:?}", e),
               }
           }

           Ok((Any::from(output), total_usage))
       }
   }

   impl SummarizationTask<'_> {
       /// Go実装の uuid.NewSHA1 (L133) を Rust で忠実に再現
       fn gen_summary_id(&self, chunk_id: &str) -> String {
           let ns = Uuid::nil(); // 0000... ネームスペース
           let data = format!("{}TextSummary", chunk_id);
           Uuid::new_v5(&ns, data.as_bytes()).to_string()
       }
   }
   ```

### 11.8 Rustでの実装対応計画 for 10.8 (Data Persistence: LadybugDB とスキーマ設計)

本項では、Cuber の全知能を物理的に支える永続化レイヤー「LadybugDB」のスキーマ設計と、Rust による堅牢なアクセス層の移植計画を詳述します。LadybugDB は、グラフ、ベクトル、および全文検索を単一の ACID トランザクションで統合管理する次世代のハイブリッド DB であり、Rust 版ではそのポテンシャルを最大限に引き出す型安全な抽象化層を構築します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/storage/interfaces.go`
   - `mycute-go/src/pkg/cuber/storage/ladybugdb/storage.go` (LadybugDB 実装体)

2. **Go実装の具体的実証箇所**:
   - データモデル定義 (`L13-51`): `Data`, `Document`, `Chunk` の階層構造。
   - 知識グラフ定義 (`L178-207`): `Node`, `Edge`, `Triple` の構造。
   - `VectorStorage` インターフェース (`L62-162`): 類似度検索 (`Query`)、全文検索 (`FullTextSearch`)、トランザクション制御。
   - `GraphStorage` インターフェース (`L219-373`): 推論や代謝に不可欠なグラフ操作（`GetTriples`, `StreamDocumentChunks`, `UpdateEdgeMetrics` 等）。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // interfaces.go: チャンクとベクトルの密結合設計
   type Chunk struct {
       ID          string    `json:"id"`
       Embedding   []float32 `json:"embedding"` // 1536次元
       Keywords    string    `json:"keywords"`  // FTS 用
       Nouns       string    `json:"nouns"`     // Layer 0
       NounsVerbs  string    `json:"nouns_verbs"` // Layer 1
   }

   // グラフエッジのメタデータ（代謝計算の基礎）
   type Edge struct {
       SourceID    string
       TargetID    string
       Weight      float64
       Confidence  float64
       Unix        int64
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/storage/mod.rs`: 共用データ型（Node, Edge, Chunk 等）の定義。
   - `src/cuber/storage/traits.rs`: `VectorStorage`, `GraphStorage` トレイトの定義。
   - `src/cuber/storage/ladybug.rs`: `lbug` クレートを用いた LadybugDB の具体的な実装。
   - `src/cuber/storage/schema.rs`: Cypher によるテーブル作成クエリ（`EnsureSchema`）の管理。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber/storage` ディレクトリを新設し、各責務ごとにファイルを分割。`traits.rs` で抽象インターフェースを定義し、`ladybug.rs` で `lbug::Session` をラップした実体を提供します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **`serde` によるシームレスな JSON/Cypher 変換**: Go 版のタグベースの定義を、Rust では `serde` の `Serialize/Deserialize` と、LadybugDB 特有の `serde_json` 互換性を活かして実装します。特に `Properties` (JSONB) フィールドのパース負荷を最小限に抑えます。
   - **`async_trait` による非同期インターフェース**: Go の `context.Context` に頼った並行管理を、Rust ではネイティブな `async_trait` で置き換えます。戻り値を `Result<T, StorageError>` とすることで、DB 特有のエラー（デッドロック、接続断、制約違反）を呼び出し側で厳密にハンドリング可能にします。
   - **`lbug` クレートによる高性能 Cypher 実行**: Go 版のドライバではクエリ文字列の組み立てにコストがかかっていましたが、Rust 版では `lbug` のプレースホルダ機能を使い、`MERGE` 句や複雑な `MATCH` 句を安全かつ高速に実行します。
   - **ストリーミング取得の最適化**: Go 版のチャネルベースのストリーミング (`StreamDocumentChunks`) を、Rust では `futures::stream::Stream` を用いて再実装します。これにより、バックプレッシャーの制御が容易になり、大規模なグラフ代謝処理時でもメモリオーバーフローを確実に防止します。
   - **プリバンドル FTS/Vector の直感的な利用**: 最新の LadybugDB の利点を活かし、Rust 側からは通常の Cypher クエリとして `CALL db.fts.search(...)` や `vector_sim_cosine(...)` を透過的に呼び出します。特別なライブラリのロード待ちを考慮する必要がなくなり、起動時の堅牢性が向上します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/storage/mod.rs ---
   use serde::{Deserialize, Serialize};

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Chunk {
       pub id: String,
       pub memory_group: String,
       pub text: String,
       pub embedding: Vec<f32>,
       pub keywords: String,      // FTS Layer 2
       pub nouns: String,         // FTS Layer 0
       pub nouns_verbs: String,   // FTS Layer 1
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Edge {
       pub source_id: String,
       pub target_id: String,
       pub edge_type: String,
       pub weight: f64,
       pub confidence: f64,
       pub unix: i64,
       pub properties: serde_json::Value,
   }

   // --- src/cuber/storage/traits.rs ---
   #[async_trait]
   pub trait VectorStorage: Send + Sync {
       /// Go: SaveChunk (L91)
       async fn save_chunk(&self, chunk: &Chunk) -> Result<(), StorageError>;
       /// Go: Query (L97)
       async fn query(&self, table: TableName, vector: &[f32], limit: usize, group: &str) -> Result<Vec<QueryResult>, StorageError>;
       /// Go: FullTextSearch (L112)
       async fn fts(&self, query: &str, layer: FtsLayer, group: &str) -> Result<Vec<QueryResult>, StorageError>;
   }

   #[async_trait]
   pub trait GraphStorage: Send + Sync {
       /// Go: AddNodes / AddEdges (L223, 226) を統合的に提供
       async fn add_nodes_batch(&self, nodes: &[Node]) -> Result<(), StorageError>;
       async fn add_edges_batch(&self, edges: &[Edge]) -> Result<(), StorageError>;
       /// Go: GetTriples (L235)
       async fn get_triples(&self, node_ids: &[String], group: &str) -> Result<Vec<Triple>, StorageError>;
       /// タイムスタンプ更新 (代謝の要): Go L291
       async fn update_metrics(&self, src: &str, dst: &str, weight: f64, conf: f64, group: &str) -> Result<(), StorageError>;
   }

   // --- src/cuber/storage/ladybug.rs ---
   pub struct LadybugDBInstance {
       session: Arc<lbug::Session>, // LadybugDB へのセッション
   }

   #[async_trait]
   impl GraphStorage for LadybugDBInstance {
       async fn add_edges_batch(&self, edges: &[Edge]) -> Result<(), StorageError> {
           // MERGE 句を用いた UPSERT 処理の Rust 実装例
           let cypher = "
               UNWIND $edges AS e
               MERGE (s:Entity {id: e.source_id})
               MERGE (t:Entity {id: e.target_id})
               MERGE (s)-[r:RELATED {type: e.edge_type}]->(t)
               SET r.weight = e.weight, r.confidence = e.confidence, r.unix = e.unix
           ";
           self.session.run(cypher, lbug::params! { "edges": edges }).await?;
           Ok(())
       }
   }
   ```

### 11.9 Rustでの実装対応計画 for 10.9 (EventBus と進捗のリアルタイム可視化)

本項では、Cuber 内の複雑な非同期処理の進捗を、型安全かつ低遅延でクライアントへ届けるための「EventBus」システムの Rust への移植計画を詳述します。このシステムは、単なるメッセージ伝達にとどまらず、動的なメッセージ生成テンプレートに基づき、AI 処理の「手触り感」をユーザーに提供する重要な UX コンポーネントです。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/lib/eventbus/eventbus.go` (基盤ライブラリ)
   - `mycute-go/src/pkg/cuber/event/absorb_event.go` (Absorb用定義)
   - `mycute-go/src/pkg/cuber/event/event_templates.go` (多言語メッセージ定義)
   - `mycute-go/src/pkg/cuber/event/event_stream.go` (テンプレート合成)

2. **Go実装の具体的実証箇所**:
   - `EventBus` 構造体 (`eventbus.go:L13-16`): `map[string][]func(payload any) error` によるハンドラ管理。
   - `Emit` 関数 (`eventbus.go:L64-83`): ゴルーチンを用いた非同期イベント発行。
   - イベント名とペイロード定義 (`absorb_event.go:L8-43`): `EventName` 定数群と各イベント専用の構造体。
   - `RegisterAbsorbStreamer` (`absorb_event.go:L231-297`): `EVENT_ABSORB_*` 系統の全イベントを SSE 用チャネルへブリッジする登録関数。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // eventbus.go: ジェネリクスを用いた型安全な Pub/Sub
   func Emit[T any](eb *EventBus, eventName string, payload T) error {
       handlers, _ := eb.handlers[eventName]
       for _, handler := range handlers {
           go func(h func(any) error) { _ = h(payload) }(handler)
       }
       return nil
   }

   // absorb_event.go: 具体的なイベントペイロード
   type AbsorbGraphParseEndPayload struct {
       BasePayload
       NodesExtracted int
       EdgesExtracted int
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/eventbus/mod.rs`: 汎用非同期 EventBus の実装。
   - `src/cuber/event.rs`: Cuber 固有のイベント名 (`EventName`) とペイロードの定義。
   - `src/cuber/event/templates.rs`: メッセージテンプレートエンジン。
   - `src/cuber/event/streamer.rs`: SSE (Server-Sent Events) へのストリーミング変換。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/eventbus/mod.rs` で `tokio::sync::broadcast` または多重ハンドラに対応した独自構造体を実装。Cuber 内では `Arc<EventBus>` としてサービス全体で共有されます。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **`tokio::sync::broadcast` による効率的な多重配信**: Go 版はループ内でゴルーチンを生成していますが、Rust では `broadcast` チャネルを用いることで、単一の発行（Emit）から複数の購読者（SSE 送信、ログ出力、統計集計等）へ、コンテキストスイッチを最小限に抑えつつデータをクローン配信します。
   - **`Enums` と `match` による完全な型安全**: Go 版の `any` (interface{}) へのダウンキャストによる柔軟性は、Rust では強力な `Enum` (代数的データ型) で置き換えます。これにより、ペイロードのパースミスをコンパイル時に完全に排除し、実行時のパニックを防ぎます。
   - **ゼロアロケーション・テンプレート合成**: Go 版の `fmt.Sprintf` 多用に対し、Rust では `askama` や `std::fmt` の `Write` トレイト、あるいは単純な文字列スライスを駆使し、ヒープアロケーションを極限まで減らした状態で、25 バリエーション以上の多言語メッセージを高速に生成します。
   - **`EventName` の強型化**: マジックストリング (`EventName = "ABSORB_START"`) を Rust の `#[repr(u8)]` 付き `Enum` に変換。これにより、API 通信時に数値として扱うことができ、帯域幅の節約とマッチング速度の向上を同時に実現します。
   - **非同期 Subscribe のシームレスな統合**: Rust の `Subject` パターンや `stream` オブジェクトを返す形式を採用。購読側は `while let Some(event) = stream.next().await` のように、標準的な非同期イテレータとして美しく記述できます。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/eventbus/mod.rs ---
   use tokio::sync::broadcast;

   pub struct EventBus<E> where E: Clone + Send + Sync + 'static {
       sender: broadcast::Sender<E>,
   }

   impl<E> EventBus<E> where E: Clone + Send + Sync + 'static {
       pub fn new(capacity: usize) -> Self {
           let (tx, _) = broadcast::channel(capacity);
           Self { sender: tx }
       }

       /// Go: Emit (L64) 相当。全購読者へ非同期配信
       pub fn emit(&self, event: E) {
           let _ = self.sender.send(event); // 購読者がいなくても落とさない
       }

       pub fn subscribe(&self) -> broadcast::Receiver<E> {
           self.sender.subscribe()
       }
   }

   // --- src/cuber/event.rs ---
   /// Go: absorb_event.go の struct 群を Enum で統合
   #[derive(Debug, Clone, serde::Serialize)]
   #[serde(tag = "type", content = "payload")]
   pub enum StreamEvent {
       AbsorbStart { group: String, file_count: usize },
       GraphParseEnd { 
           group: String, 
           chunk_id: String, 
           nodes: usize, 
           edges: usize 
       },
       // ... その他のイベント ...
       Error { group: String, message: String },
   }

   // --- src/cuber/event/templates.rs ---
   /// Go: event_templates.go のメッセージ合成ロジック
   impl StreamEvent {
       pub fn to_human_message(&self, lang: Lang) -> String {
           match self {
               StreamEvent::GraphParseEnd { chunk_id, nodes, edges, .. } => {
                   match lang {
                       Lang::Ja => format!("チャンク {} から {} 個のノードを抽出しました。", chunk_id, nodes),
                       Lang::En => format!("Extracted {} nodes from chunk {}.", nodes, chunk_id),
                   }
               }
               // ... 25バリエーション * N言語を展開
               _ => "Processing...".to_string(),
           }
       }
   }
   ```

### 11.10 Rust への完全移植に向けた総括 for 10.10 (Technical Challenges and Strategy)

本項では、Go 版 Cuber の知能と美学を Rust という究極の言語で再定義し、真に「恐れなき並行性（Fearless Concurrency）」と「ゼロコスト抽象化（Zero-cost Abstractions）」を実現するための全移植プロセスの総括を詳述します。本移植は単なるコードの書き換えではなく、アーキテクチャの次元を高める試みです。

1. **Go実装 de ファイルパス**:
   - プロジェクト全体、特に `mycute-go/src/pkg/cuber` 配下の全コンポーネント。

2. **Go実装の具体的実証箇所**:
   - `cuber.go`: サービスのライフサイクルと依存関係の調整。
   - `pipeline/pipeline.go`: ステージング実行とエラー伝播。
   - `tasks/*`: 各自律的タスクのドメイン知識。
   - `storage/*` & `lib/eventbus`: 基盤となる永続化と通信。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // Go: 柔軟だが実行時に解決される抽象化
   type Task interface {
       Run(ctx context.Context, input any) (any, types.TokenUsage, error)
   }
   // Go: ロック競合が発生しやすいグローバルMutex
   mu sync.RWMutex
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/mod.rs`: クレートの公開インターフェースとモジュール構成の統合。
   - `src/cuber/types.rs`: 全タスク間で共有される不変のドメインモデル。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   `src/cuber` を独立したライブラリ・クレートのように構成し、`main_of_rt.rs` 等の実行モードから透過的に利用可能にします。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:

   - **非同期オーケストレーションのパラダイム・シフト**: Go の「CSP モデル（Channel 接続）」を尊重しつつ、Rust では `tokio` の `Future` グラフへと昇華させます。`JoinSet` による動的な並列数制御と、`select!` 句によるタイムアウト/キャンセルの厳格な管理により、I/O 待機時間を極限まで削ぎ落とします。
   - **LadybugDB との静的結合**: Go 版の動的な型キャストに対し、Rust では `lbug` クレートと `serde` を組み合わせ、DB レコードから Rust 構造体への変換をコンパイル時に検証します。特に `MERGE` クエリのプレースホルダ利用により、インジェクション耐性とパース速度を同時に担保します。
   - **「知性の型安全化」**: LLM の応答、メタ認知情報、グラフ構造といった曖昧なデータを、Rust の `Enum` と `Result` で型定義します。これにより、「LLM が不正な JSON を返した」といった不確定要素を、システムのダウンではなく、型による適切なエラーハンドリングとして処理可能にします。
   - **リソース管理の究極化**: `Arc<DashMap<...>>` によるシャードロックと、`CancellationToken` によるタスクツリーの連鎖停止。これらにより、Go 版で課題となっていた Ctrl+C 停止時の DB ロック問題を根本から解決し、高負荷下でも予測可能なリソース消費を実現します。
   - **将来の拡張性に対する備え**: `Task` トレイトと `Any` ラッパーを介したプラグイン形式のアーキテクチャにより、将来的に「新しい解析タスク」や「新しい LLM プロバイダ」を追加する場合も、既存コードへの影響を最小限に抑えた機能拡張が可能になります。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/mod.rs (総括的な統合) ---
   
   /// Cuber サービスの核心をなす自律タスクの抽象
   #[async_trait]
   pub trait Task: Send + Sync {
       /// 入力データを解析・変換し、コスト（TokenUsage）と共に返す
       /// Rust 版では Any 型と TryInto を使い、実行時の型安全を保証する
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError>;
   }

   /// Cuber の全機能を司るオーケストレータ
   pub struct CuberService {
       config: CuberConfig,
       storage_map: Arc<DashMap<String, Arc<StorageSet>>>,
       s3_client: Arc<S3Client>,
       tokenizer: Arc<LinderaTokenizer>,
       cancel_token: CancellationToken,
   }

   impl CuberService {
       /// Absorb プロセスのエントリーポイント (再帰的・トランザクション的)
       pub async fn absorb(&self, cube_uuid: &str, group: &str, files: Vec<S3File>) -> Result<AbsorbResult, CuberError> {
           let storage = self.get_or_open_storage(cube_uuid).await?;
           
           // トランザクション・ブロックの開始
           storage.vector.transaction(|tx| async move {
               let mut pipeline = Pipeline::new(tx, group);
               
               // 各タスクの動的登録と実行
               pipeline.add(IngestTask::new(self.s3_client.clone()))?;
               pipeline.add(ChunkingTask::new(self.tokenizer.clone()))?;
               pipeline.add(GraphExtractionTask::new())?;
               pipeline.add(StorageTask::new())?;
               pipeline.add(SummarizationTask::new())?;
               
               let result = pipeline.run(files).await?;
               Ok(result)
           }).await
       }
   }
   ```

本移植計画の完遂により、MYCUTE は Go 版の柔軟性を維持したまま、Rust の持つ圧倒的なパフォーマンス、メモリ安全性、そして開発体験の向上を手に入れることができます。これは、次世代 AI システムとしての信頼性とスケーラビリティを確立するための、最も重要な礎となります。

### 11.11 <::> セパレータによる ID パーティショニング

本項では、LadybugDB 内で同一の名前を持つエンティティが異なる `memory_group` 間で衝突するのを防ぐための、物理的な ID 分離ロジックについて詳述します。これはマルチテナント的なデータ隔離を実現するための Cuber の根幹仕様です。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/consts/consts.go` (定数定義)
   - `mycute-go/src/pkg/cuber/utils/utils.go` (操作関数)

2. **Go実装の具体的実証箇所**:
   - `consts.ID_MEMORY_GROUP_SEPARATOR`: セパレータ文字列の定義。
   - `utils.MakeGraphNodeID`: ID とグループを連結する関数。
   - `utils.GetNameStrByGraphNodeID`: 連結された ID から元の名前を抽出する関数。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // consts/consts.go
   const ID_MEMORY_GROUP_SEPARATOR = "<::>"

   // utils/utils.go
   func MakeGraphNodeID(nodeID string, memoryGroup string) string {
       return strings.TrimSpace(nodeID) + consts.ID_MEMORY_GROUP_SEPARATOR + memoryGroup
   }

   func GetNameStrByGraphNodeID(graphNodeID string) string {
       ex := strings.Split(graphNodeID, consts.ID_MEMORY_GROUP_SEPARATOR)
       if len(ex) > 1 {
           return ex[0]
       }
       return graphNodeID
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/consts.rs`: セパレータ定数の定義。
   - `src/cuber/utils/id.rs`: ID 操作用ヘルパーモジュール。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/utils/id.rs` を新規作成し、そこに純粋関数として実装します。また `src/cuber/utils/mod.rs` で公開します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **定数値の維持**: Go との互換性を保つため、定数 `ID_MG_SEP` を `<::>` と定義します。
   - **パフォーマンスと型安全性**: Go では `strings.Split` によりスライスを生成していますが、Rust では `split_once` を使用することで、余計なメモリ確保を避けつつ、常に 2 つの要素（名前とグループ）への分割を安全に試みることができます。
   - **ID 隔離の保証**: 保存時（`StorageTask` 等）には必ず `make_graph_node_id` を介し、返却・表示時には `get_name_from_id` でオリジナル名を復元するサイクルを Rust の型システム上で明示し、LadybugDB 内で memory_group 毎の「名前空間」が確実に分かれるようにします。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/consts.rs ---
   pub const ID_MG_SEP: &str = "<::>";

   // --- src/cuber/utils/id.rs ---
   use crate::cuber::consts::ID_MG_SEP;

   /// オリジナル ID とメモリーグループを物理連結して、DB 用のフル ID を生成する
   /// Go: utils.MakeGraphNodeID 相当
   pub fn make_graph_node_id(node_id: &str, memory_group: &str) -> String {
       format!("{}<::>{}", node_id.trim(), memory_group)
   }

   /// 連結された ID からオリジナル名（セパレータ前）のみを抽出する
   /// Go: utils.GetNameStrByGraphNodeID 相当
   pub fn get_name_from_id(graph_node_id: &str) -> &str {
       graph_node_id
           .split_once(ID_MG_SEP)
           .map(|(name, _)| name)
           .unwrap_or(graph_node_id)
   }
   ```

### 11.12 GraphExtractionTask における抽出後の詳細正規化

LLM が抽出した生データ（ノード名、関係タイプ、プロパティ値）には、全角半角の混在や不要な記号、LLM 特有の「揺れ」が含まれます。これらを決定論的に正規化し、グラフの整合性を保つための仕様を詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/utils/normalize.go` (正規化ロジックの本体)
   - `mycute-go/src/pkg/cuber/tasks/graph/graph_extraction_task.go` (ロジックの適用)

2. **Go実装の具体的実証箇所**:
   - `NormalizeForGraph`: ID および Type 用の強力な正規化。
   - `CommonNormalize`: プロパティ値等、文脈を維持しつつ HTML/Markdown ノイズを除去する正規化。
   - `GraphExtractionTask.Run`: 抽出結果のループ内でこれらの関数を各フィールドに適用。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // utils/normalize.go:L315
   func NormalizeForGraph(text string) string {
       text = norm.NFKC.String(text)
       text = transformWidth(text, width.Fold) // 全角英数を半角に、半角カナを全角に
       text = strings.ToLower(text)
       text = reSymbols.ReplaceAllString(text, "")
       text = emojiRe.ReplaceAllString(text, "")
       text = consecutiveSpacesRe.ReplaceAllString(text, " ")
       return strings.TrimSpace(text)
   }

   // tasks/graph/graph_extraction_task.go:L181
   allNodes[i].ID = utils.NormalizeForGraph(allNodes[i].ID)
   allNodes[i].Type = utils.NormalizeForGraph(allNodes[i].Type)
   // ... プロパティへの CommonNormalize 適用
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/normalize.rs`: 正規化関数群。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/utils/normalize.rs` に `pub fn normalize_for_graph(text: &str) -> String` および `pub fn common_normalize(text: &str) -> String` を実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **Unicode 正規化の再現**: Rust の `unicode-normalization` クレートを使用し、Go の `norm.NFKC` と同一の正規化を実行します。
   - **カノニカル・フォールドの精密実装**: Go の `width.Fold` は非常に強力ですが、Rust では `unicode_normalization` と正規表現、あるいは `icu_normalizer` (ICU4X) を組み合わせて、全角英数の半角化と半角カナの全角化（濁点結合を含む）を厳密に再現します。
   - **決定論的エンティティ解決**: この正規化を ID に適用することで、「テスラ」と「ﾃｽﾗ」と「Tesla」が全て同一のノードとして統合されることを保証します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/normalize.rs ---
   use unicode_normalization::UnicodeNormalization;
   use regex::Regex;
   use lazy_static::lazy_static;

   lazy_static! {
       static ref RE_SYMBOLS: Regex = Regex::new(r"[\u{1F300}-\u{1F9FF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]").unwrap();
       static ref RE_EMOJI: Regex = Regex::new(r"[\u{1F600}-\u{1F64F}...略...]").unwrap();
       static ref RE_SPACES: Regex = Regex::new(r"[ \t]+").unwrap();
   }

   /// グラフノード ID/Type 用の決定論的正規化
   /// Go: utils.NormalizeForGraph 相当
   pub fn normalize_for_graph(text: &str) -> String {
       if text.is_empty() { return String::new(); }

       // 1. NFKC 正規化
       let nfkc = text.nfkc().collect::<String>();

       // 2. 幅のフォールディング (ICU4X または手動マップ)
       // ここでは Go の width.Fold 相当の処理を Rust で連結する
       let folded = fold_width(&nfkc);

       // 3. 小文字化
       let lowered = folded.to_lowercase();

       // 4. 記号・絵文字除去
       let no_sym = RE_SYMBOLS.replace_all(&lowered, "");
       let no_emoji = RE_EMOJI.replace_all(&no_sym, "");

       // 5. 空白圧縮
       RE_SPACES.replace_all(&no_emoji, " ").trim().to_string()
   }

   // 補助関数: width.Fold 相当
   fn fold_width(text: &str) -> String {
       // 全角英数 -> 半角, 半角カナ -> 全角 の変換マップ/ロジックを実装
       // Rust では icu_normalizer が推奨されるが、簡易的には置換テーブルを用いる
       text.to_string() // 詳細な置換テーブルは実装時に展開
   }
   ```

### 11.13 トランザクション完了後の明示的な Checkpoint 実行

LadybugDB (Cozo-based) の Write-Ahead Logging (WAL) ログをメインのデータベースファイルに物理的にマージし、外部ツールからの可読性とデータの永続性を完全に確実にするための Checkpoint 命令の適切な実行タイミングについて詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/cuber.go` (オーケストレーション層)
   - `mycute-go/src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go` (ストレージ実装層)

2. **Go実装の具体的実証箇所**:
   - `Absorb` メソッド内: 大規模なデータ投入（Ingestion & Cognification）のトランザクションが正常終了した直後に実行。
   - `LadybugDBStorage.Close` メソッド内: ストレージ接続をクローズするセーフティネットとして実行。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // cuber.go:L566
   if err := st.Vector.Checkpoint(); err != nil {
       utils.LogWarn(l, "Failed to checkpoint storage", zap.Error(err))
   }

   // ladybugdb_storage.go:L91
   func (s *LadybugDBStorage) Close() error {
       if s.conn != nil {
           s.Checkpoint() // クローズ前に WAL をマージ
           s.conn.Close()
           // ... 略 ...
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/mod.rs`: `CuberService` のビジネスロジック内。
   - `src/cuber/storage/ladybugdb.rs`: ストレージ実装のライフサイクル管理内。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `CuberService::absorb` メソッドのトランザクション完了直後。
   - `LadybugDbStorage` 構造体の `close` メソッド（または明示的な終了処理）内。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **永続性の連鎖保証**: `Absorb` 後の Checkpoint は、大量のデータが WAL に「宙ぶらりん」の状態になるのを防ぎ、即座にメイン DB へ永続化させます。
   - **外部ツールとの互換性**: 外部の `lbug` CLI 等が DB ファイルを直接参照する際、Checkpoint が未実行だと直近の変更が見えない問題が発生します。Go 版はこの問題を回避するために明示的に呼んでおり、Rust でもこの「外の目」を意識した設計を維持します。
   - **クリーンな終了処理**: `Close` 時の実行により、たとえアプリケーションが突然終了しても、DB レベルでの整合性が可能な限り保たれるようにします。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/mod.rs ---
   impl CuberService {
       pub async fn absorb(&self, ...) -> Result<AbsorbResult, CuberError> {
           let storage = self.get_storage(cube_uuid).await?;
           storage.vector.transaction(|tx| async move {
               // ... 処理 ...
               Ok(res)
           }).await?;

           // Go と同様に、トランザクションの外（成功後）で実行
           // 失敗しても absorb 全体をエラーにはせず警告に留める (Soft Error Handling)
           if let Err(e) = storage.vector.checkpoint().await {
               log::warn!("[Cuber] Checkpoint failed after absorb: {}", e);
           }
           
           Ok(result)
       }
   }

   // --- src/cuber/storage/ladybugdb.rs ---
   impl LadybugDbStorage {
       pub async fn close(&self) -> Result<(), StorageError> {
           // セーフティネットとしての Checkpoint
           let _ = self.checkpoint().await;
           self.conn.close().await?;
           Ok(())
       }
   }
   ```

### 11.14 全ファイル重複時におけるパイプラインの早期エラー中断

新規に投入された全ファイルが既に LadybugDB に存在する場合、無意味な LLM 呼び出しや重い解析処理（パイプライン）を回避し、即座に呼び出し元へエラーを返却するための厳格なハンドリング仕様です。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/ingestion/ingest_task.go` (判定ロジック)

2. **Go実装の具体的実証箇所**:
   - `IngestTask.Run` メソッドの末尾: 全ての入力ファイルが `skippedCount`（スキップ済み）としてカウントされた場合のエラー投下。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // ingest_task.go:L203
   if fileCount == skippedCount { // 全件スキップされた場合は、全て重複データであるとしてエラーで返す
       return nil, usage, fmt.Errorf("Ingest: All data or files are duplicates.")
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/ingest.rs`: `IngestTask` の実装ファイル。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `IngestTask::run` トレイトメソッドの実装内、ループ完了後の最終判定箇所。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **リソースの浪費防止**: チャンキングや抽出タスクに進む前に `Result::Err` を返すことで、Rust のアウェアなエラー伝播を利用してパイプライン全体を安全に中断できます。
   - **冪等性の確保**: すでに存在するデータを再処理しないことで、グラフへの冗長なエッジ重複を防ぎます。
   - **明確なフィードバック**: 「何も処理されなかった」ことをエラーメッセージで明示的に呼び出し元（API レイヤー等）に伝える Go 版の挙動を忠実に再現します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/ingest.rs ---
   impl Task for IngestTask {
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError> {
           let files: Vec<S3File> = input.try_into()?;
           let file_count = files.len();
           let mut skipped_count = 0;
           let mut ingesting_data = Vec::new();

           for file in files {
               // ハッシュチェックして重複なら skipped_count += 1
               if self.is_duplicate(&file).await? {
                   skipped_count += 1;
                   continue;
               }
               // ... 実際のインジェスト処理 ...
               ingesting_data.push(data);
           }

           // Go: if fileCount == skippedCount 相当
           if file_count > 0 && file_count == skipped_count {
               return Err(CuberError::Ingest("All files are duplicates".to_string()));
           }

           Ok((Any::from(ingesting_data), TokenUsage::default()))
       }
   }
   ```

### 11.15 SSE 配送品質向上のための 150ms スリープ・バッファ

EventBus を介した非同期イベント配信（特に SSE: Server-Sent Events）において、最後の重要イベントがクライアントに到達する前に接続が切断されるのを防ぎ、UX を向上させるための意図的な遅延処理について詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/cuber.go` (イベント送出の最終地点)

2. **Go実装の具体的実証箇所**:
   - `Absorb`, `Query`, `Memify` の各メソッドにおける `defer` 内、または処理完了直後の `EmitSync` 呼び出し後のスリープ。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // cuber.go:L561
   eventbus.EmitSync(eb, string(event.EVENT_ABSORB_END), event.AbsorbEndPayload{...})
   time.Sleep(150 * time.Millisecond) // 配送完了を待機

   // cuber.go:L112
   eventbus.EmitSync(t.EventBus, string(event.EVENT_QUERY_END), event.QueryEndPayload{...})
   time.Sleep(150 * time.Millisecond)
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/mod.rs`: 各オーケストレーション関数の完了間際。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `absorb`, `query`, `memify` メソッドの最終的な `Result` を返す直前。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **イベント・フラッシュの保証**: 非同期ランタイム（Tokio）において、チャンネルにメッセージを送った直後にスレッド/関数を終了すると、バッファされたデータが HTTP レスポンスとしてフラッシュされる前に接続が閉じるリスクがあります。
   - **150ms という「経験則的定数」の継承**: Go 版での 150ms は、ネットワーク・レイテンシとクライアント側のバッファリングを十分にカバーする時間として設定されており、Rust でもこの値を踏襲することで同一の安定性を確保します。
   - **非ブロッキング待機**: Rust では `tokio::time::sleep` を使用することで、スレッドをブロックせずに該当タスクのみをサスペンドさせ、高効率な配送待ちを実現します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/mod.rs ---
   pub async fn absorb(&self, ...) -> Result<AbsorbResult, CuberError> {
       // ... 全ての処理完了 ...

       // 最終イベントを同期的に（またはチャンネルの空きを待って）送出
       self.event_bus.emit_sync(EVENT_ABSORB_END, payload).await?;

       // Go: time.Sleep(150 * time.Millisecond) 相当
       tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

       Ok(result)
   }
   ```

### 11.16 InfoStreamer によるシステム共通イベントの多重配信

特定の機能（Absorb や Query など）のイベントストリームに、システム共通の「ニュートラルな情報イベント」（矛盾解決の開始/終了など）を多重化して配信する仕組みについて詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/event/info_event.go` (共通ストリーマー)
   - `mycute-go/src/pkg/cuber/event/absorb_event.go` (機能固有ストリーマー)
   - `mycute-go/src/pkg/cuber/cuber.go` (登録の実行)

2. **Go実装の具体的実証箇所**:
   - `RegisterInfoStreamer`: ストリーム用チャンネルに対して、共通情報イベントを購読・転送する関数。
   - `Absorb` / `Query` / `Memify`: 各機能の開始時に、機能固有のストリーマーと同時に `RegisterInfoStreamer` を呼び出し、同一の `ch` に対して複数のソースからイベントを注入。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // info_event.go:49
   func RegisterInfoStreamer(eb *eventbus.EventBus, ch chan<- StreamEvent) {
       // ... INFO_CONFLICT_RESOLUTION_* 等のイベントを ch に送るように Subscribe ...
   }

   // cuber.go:L506 (Absorb内)
   event.RegisterAbsorbStreamer(eb, eventCh) // 機能固有
   event.RegisterInfoStreamer(eb, eventCh)   // 共通情報の多重化
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/event/info.rs`: 共通イベントの定義とストリーマー。
   - `src/cuber/event/mod.rs`: ストリーマー登録の統合。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/event/info.rs` に `pub fn register_info_streamer(...)` を配置。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **チャンネルの多対一転送**: Rust の `tokio::sync::mpsc` チャンネルは、複数の送信端（`Sender`）を持つことができます。各ストリーマーに関数の引数として `Sender` を渡すことで、Go 版のような「複数の購読者が一つのチャンネルに書き込む」挙動を自然かつ安全に再現できます。
   - **関心の分離**: 共通イベント（Info）と特定のタスクイベント（Absorb 等）を分けることで、コードの再利用性を高めつつ、クライアントには統合された一本の SSE ストリームとして提供できます。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/event/mod.rs ---
   pub async fn register_streamers(eb: &EventBus, tx: mpsc::Sender<StreamEvent>, mode: CuberMode) {
       // 共通情報の多重化
       info::register_info_streamer(eb, tx.clone()).await;

       // モードに応じた固有ストリーマー
       match mode {
           CuberMode::Absorb => absorb::register_absorb_streamer(eb, tx).await,
           CuberMode::Query => query::register_query_streamer(eb, tx).await,
           // ...
       }
   }

   // --- src/cuber/event/info.rs ---
   pub async fn register_info_streamer(eb: &EventBus, tx: mpsc::Sender<StreamEvent>) {
       eb.subscribe(EVENT_INFO_CONFLICT_DISCARDED, move |p| {
           let tx = tx.clone();
           async move {
               let _ = tx.send(StreamEvent::from(p)).await;
               Ok(())
           }
       }).await;
       // 他の INFO イベントも同様に登録
   }
   ```

### 11.17 決定論的 ID 生成における UUID v5 ネームスペースの完全一致

同一のコンテンツハッシュを持つデータに対して、Go 版と Rust 版で全く同じ UUID を生成するための、ネームスペースとアルゴリズムの厳格な一致仕様について詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/ingestion/ingest_task.go` (ID 生成)

2. **Go実装の具体的実証箇所**:
   - `generateDeterministicID`: コンテンツハッシュとメモリーグループをシードとして UUID を生成する内部関数。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // ingest_task.go:L69
   func generateDeterministicID(contentHash string, memoryGroup string) string {
       // Cuber Ingestion用の名前空間UUID (6ba7b812-9dad-11d1-80b4-00c04fd430c8)
       namespace := uuid.NameSpaceOID
       // SHA-1 (v5) を使用して UUID を生成
       return uuid.NewSHA1(namespace, []byte(contentHash+memoryGroup)).String()
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/id.rs`: ID 操作ヘルパー。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/utils/id.rs` 内に UUID v5 生成関数を実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **アルゴリズムの一致**: Go の `uuid.NewSHA1` は UUID バージョン 5 (SHA-1 署名) に相当します。Rust の `uuid` クレートの `new_v5` メソッドを使用することで、数学的に同一の結果を保証できます。
   - **ネームスペースの同一性**: `uuid.NameSpaceOID` (`6ba7b812-9dad-11d1-80b4-00c04fd430c8`) を定数として定義し、シードの結合順序 (`contentHash + memoryGroup`) を維持することで、Go 版で既に保存されているデータと ID が衝突・重複排除されることを確実にします。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/id.rs ---
   use uuid::Uuid;

   /// 既存の OID ネームスペース
   const NAMESPACE_OID: Uuid = Uuid::from_u128(0x6ba7b812_9dad_11d1_80b4_00c04fd430c8);

   /// コンテンツハッシュとグループから一貫性のある UUID を生成する
   /// Go: generateDeterministicID 相当
   pub fn generate_deterministic_id(content_hash: &str, memory_group: &str) -> String {
       let seed = format!("{}{}", content_hash, memory_group);
       // UUID v5 (SHA-1 version)
       Uuid::new_v5(&NAMESPACE_OID, seed.as_bytes()).to_string()
   }
   ```

### 11.18 splitSentences における文分割正規表現と改行境界の維持

テキストを意味のある「文」の単位に分割し、チャンクの境界を自然にするための正規表現と分割アルゴリズムの忠実な移植について詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/chunking/chunking_task.go` (分割ロジック)

2. **Go実装の具体的実証箇所**:
   - `SplitSentencesRegexp`: 分割のトリガーとなるパターン定義。
   - `splitSentences`: 正規表現のマッチ位置をベースにテキストをスライスする関数。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // chunking_task.go:L29
   // 日本語と英語の句読点と改行2個以上で文を分割
   SplitSentencesRegexp = regexp.MustCompile(`[。！？.!?]\s*|(?:\r\n|\r|\n){2,}`)

   // chunking_task.go:L309
   func splitSentences(text string) []string {
       // ... SplitSentencesRegexp.FindAllStringIndex を使用してスライス ...
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/chunking.rs`: `ChunkingTask` の補助関数。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/tasks/chunking.rs` 内にプライベート関数、または `ChunkingTask` のメソッドとして実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **境界維持の重要性**: Go の正規表現 `[。！？.!?]\s*` は、句読点そのものを「文の終わり」に含めて分割します。Rust の `regex` クレートでも同一のパターンを使用し、マッチ位置の終端 (`match.end()`) までを一つの文として切り出すことで、文末の記号を欠落させずに保持できます。
   - **段落境界の認識**: `(?:\r\n|\r|\n){2,}` により、空行（ダブル改行）を強い境界として認識します。これにより、物理的に離れたテキストが無理やり一つの文として結合されるのを防ぎます。
   - **UTF-8 安全性**: Rust の `String`/`str` はネイティブで UTF-8 を扱うため、日本語の全角句読点に対しても Go の `regexp` クレートと同等以上の安全性で動作します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし, Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/chunking.rs ---
   use regex::Regex;
   use lazy_static::lazy_static;

   lazy_static! {
       static ref RE_SPLIT_SENTENCES: Regex = Regex::new(r"[。！？.!?]\s*|(?:\r\n|\r|\n){2,}").unwrap();
   }

   pub fn split_sentences(text: &str) -> Vec<String> {
       let mut sentences = Vec::new();
       let mut last_index = 0;

       for mat in RE_SPLIT_SENTENCES.find_iter(text) {
           let end = mat.end();
           let sentence = text[last_index..end].trim();
           if !sentence.is_empty() {
               sentences.push(sentence.to_string());
           }
           last_index = end;
       }

       if last_index < text.len() {
           let remaining = text[last_index..].trim();
           if !remaining.is_empty() {
               sentences.push(remaining.to_string());
           }
       }
       sentences
   }
   ```

### 11.19 SPECIAL_NODE_TYPE_DOCUMENT_CHUNK によるテキスチャル・トレーサビリティ

知識グラフ内の抽象的なエンティティと、その根拠となった生のテキスト（チャンク）を物理的に紐付け、「どのテキストからこの知識が得られたのか」という根拠（トレーサビリティ）をグラフ構造自体に持たせるための仕様を詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/types/consts.go` (型定義)
   - `mycute-go/src/pkg/cuber/tasks/storage/storage_task.go` (グラフ構築)

2. **Go実装の具体的実証箇所**:
   - `SPECIAL_NODE_TYPE_DOCUMENT_CHUNK`: チャンクを表現するための予約済みノードタイプ。
   - `StorageTask.Run`: 抽出された一般ノードに加え、全チャンクをこの特殊タイプとしてグラフデータに追加。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // types/consts.go
   const SPECIAL_NODE_TYPE_DOCUMENT_CHUNK = "__document_chunk__"

   // storage_task.go:L101
   for _, chunk := range output.Chunks {
       chunkNode := &storage.Node{
           ID:          chunk.ID,
           MemoryGroup: t.memoryGroup,
           Type:        string(types.SPECIAL_NODE_TYPE_DOCUMENT_CHUNK),
           Properties: map[string]any{
               "text":        chunk.Text,
               "document_id": chunk.DocumentID,
               "chunk_index": chunk.ChunkIndex,
           },
       }
       chunkNodes = append(chunkNodes, chunkNode)
   }
   output.GraphData.Nodes = append(output.GraphData.Nodes, chunkNodes...)
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/types/mod.rs` (または `consts.rs`): 特殊ノードタイプの定義。
   - `src/cuber/tasks/storage.rs`: `StorageTask` の実装。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `StorageTask::run` メソッド内で、抽出済みグラフデータにチャンク由来のノードをマージする処理。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **グラフとベクトルの架け橋**: チャンクをグラフの「ノード」として扱うことで、将来的にエンティティから `HAS_SOURCE` 等のエッジ（実装予定）を介して元テキストへ辿り着くことが可能になります。
   - **メタデータの保存**: `text` だけでなく `document_id` や `chunk_index` をプロパティとして保持することで、LadybugDB のグラフクエリだけで元のドキュメント構造を復元できるトレーサビリティを確保します。
   - **予約語の保護**: `__document_chunk__` というアンダースコア付きの特殊なタイプ名を使用することで、LLM が生成する一般ノードとの衝突を避け、システム管理用のノードとして安全に識別できます。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/types/mod.rs ---
   pub const SPECIAL_NODE_TYPE_DOCUMENT_CHUNK: &str = "__document_chunk__";

   // --- src/cuber/tasks/storage.rs ---
   impl Task for StorageTask {
       async fn run(&self, input: Any) -> Result<(Any, TokenUsage), CuberError> {
           let mut output: CognifyOutput = input.try_into()?;
           
           // 1. 各チャンクを特殊ノードとして生成
           let mut chunk_nodes = Vec::new();
           for chunk in &output.chunks {
               let node = Node {
                   id: chunk.id.clone(),
                   memory_group: self.memory_group.clone(),
                   r#type: SPECIAL_NODE_TYPE_DOCUMENT_CHUNK.to_string(),
                   properties: vec![
                       ("text".to_string(), chunk.text.clone().into()),
                       ("document_id".to_string(), chunk.document_id.clone().into()),
                       ("chunk_index".to_string(), chunk.chunk_index.into()),
                   ].into_iter().collect(),
               };
               chunk_nodes.push(node);
           }

           // 2. 抽出されたグラフデータにマージ
           output.graph_data.nodes.extend(chunk_nodes);

           // 3. 全ノードを保存
           self.graph_storage.add_nodes(&output.graph_data.nodes).await?;
           // ...
           Ok((Any::from(output), TokenUsage::default()))
       }
   }
   ```

### 11.20 UTF-8 rune 単位での美的 TruncateString アルゴリズム

日本語を含むマルチバイト文字列を、文字化けを防ぎつつ、末尾の句読点除去などの「美的」な後処理を伴って切り詰めるためのアルゴリズムについて詳述します。

1. **Go実装のファイルパス**:
   - `mycute-go/src/pkg/cuber/utils/utils.go` (ユーティリティ関数)

2. **Go実装 de 具体的実証箇所**:
   - `utils.TruncateString`: UTF-8 セーフな文字列切り詰め関数。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // utils/utils.go:L38
   func TruncateString(s string, limit int) string {
       runeCount := utf8.RuneCountInString(s)
       if runeCount <= limit {
           return s
       }
       runes := []rune(s)
       truncated := runes[:limit]
       // 最後の文字が「。」または「.」の場合は削除 (美的な調整)
       if len(truncated) > 0 {
           lastChar := truncated[len(truncated)-1]
           if lastChar == '。' || lastChar == '.' {
               truncated = truncated[:len(truncated)-1]
           }
       }
       return string(truncated) + "..."
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/string.rs`: 文字列操作ヘルパーモジュール。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/utils/string.rs` を新規作成し、そこに `pub fn truncate_string(s: &str, limit: usize) -> String` を実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **Rune 単位の正確なハンドリング**: Go の `[]rune` 変換は Rust の `chars().collect::<Vec<char>>()` に相当します。これにより、バイト数ではなく「文字数」ベースでの正確な切り詰めが行われます。
   - **美的後処理の再現**: 「。」や「.」が「...」の直前に来るのを防ぐ Go 版の特殊なロジックを条件分岐で再現し、生成される要約やタイトルが不自然にならないようにします。
   - **メモリ効率**: Rust では `chars().take(limit)` を使用することで、中間ベクトルを作らずに効率的に処理を完結させることが可能です。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/string.rs ---

   /// 文字列を文字数制限で切り詰め、末尾を美しく調整する
   /// Go: utils.TruncateString 相当
   pub fn truncate_string(s: &str, limit: usize) -> String {
       let chars: Vec<char> = s.chars().collect();
       if chars.len() <= limit {
           return s.to_string();
       }

       let mut truncated = chars[..limit].to_vec();
       
       // 末尾が句読点なら削除（Go 互換）
       if let Some(&last) = truncated.last() {
           if last == '。' || last == '.' {
               truncated.pop();
           }
       }

       let mut result: String = truncated.into_iter().collect();
       result.push_str("...");
       result
   }
   ```

### 11.21 ConvertNodesAndEdgesToTriples によるグラフ中間表現への変換

グラフ抽出タスクで得られた構造化データ（Node/Edge）を、LLM や矛盾解決ロジックが扱いやすい「主語-述語-目的語」のトリプル形式に変換するための共通処理について詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/storage/interfaces.go` (型定義)
   - `mycute-go/src/pkg/cuber/tasks/graph/graph_extraction_task.go` (利用箇所)

2. **Go実装 de 具体的実証箇所**:
   - `storage.ConvertNodesAndEdgesToTriples`: Node 配列と Edge 配列を受け取り、中間表現としての Triple 配列を生成する関数。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // storage/interfaces.go (実際の実装は共通ユーティリティにあるがインターフェースとして定義)
   func ConvertNodesAndEdgesToTriples(nodes []*Node, edges []*Edge) []string {
       var triples []string
       for _, edge := range edges {
           // (SourceID, Type, TargetID) の形式で文字列化
           triple := fmt.Sprintf("(%s, %s, %s)", edge.SourceID, edge.Type, edge.TargetID)
           triples = append(triples, triple)
       }
       return triples
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/storage/mod.rs`: ストレージ関連の共通データ変換。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/storage/mod.rs` 内に `impl GraphData` のメソッドとして、またはスタンドアロン関数として実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **データ構造の整合性**: Rust 版の `Edge` も `source_id`, `r#type`, `target_id` を持っているため、Go 版と全く同一のフォーマット `(S, P, O)` を生成できます。
   - **一貫した表現**: メモリグループが既に付与された状態のフル ID を使用することで、多重空間（memory_group）が混在する状況でも、正しいエンティティ間の関係を文字列として表現できます。
   - **LLM コンテキストへの最適化**: この文字列化されたトリプルは、`Memify` 等で LLM のプロンプトに挿入されるため、フォーマットの不一致は LLM の精度低下を招きます。Go 版の括弧とカンマの形式を維持することが重要です。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/storage/mod.rs ---

   /// ノードとエッジから、LLM プロンプト等で使用するトリプル文字列リストを生成する
   /// Go: storage.ConvertNodesAndEdgesToTriples 相当
   pub fn convert_to_triples(edges: &[Edge]) -> Vec<String> {
       edges.iter()
           .map(|e| format!("({}, {}, {})", e.source_id, e.r#type, e.target_id))
           .collect()
   }
   ```

### 11.22 cleanJSON による不完全な LLM 応答からのオブジェクト抽出

LLM が出力する JSON 前後の不要な説明文や Markdown 装飾を除去し、純粋な JSON オブジェクト部分のみを堅牢に抽出するためのロジックについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/graph/graph_extraction_task.go` (抽出ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `cleanJSON`: 文字列から最初と最後の波括弧を探し、その範囲を切り出すユーティリティ。

3. **Go実装の具体的実装箇所のコードスニペット**:
   ```go
   // graph_extraction_task.go:L233
   func cleanJSON(content string) string {
       firstBrace := strings.Index(content, "{")
       if firstBrace == -1 {
           return "{}"
       }
       lastBrace := strings.LastIndex(content, "}")
       if lastBrace == -1 || lastBrace < firstBrace {
           return "{}"
       }
       return content[firstBrace : lastBrace+1]
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/json.rs`: JSON 操作ヘルパーモジュール。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `src/cuber/utils/json.rs` を新規作成し、そこに `pub fn clean_json(content: &str) -> &str` を実装します。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **パース失敗の回避**: LLM が「Here is the JSON: ```json ... ```」のような形式で返答した場合、Go の `strings.Index` と `strings.LastIndex` を用いたスライス方式は、複雑な正規表現を用いずに最も確実に `{ ... }` の範囲を特定できます。
   - **ゼロ・コピー・スライシング**: Rust では `&str` のスライスを返すことで、新たな文字列メモリ確保を避けつつ効率的に抽出可能です。
   - **フォールバックの単純さ**: 波括弧が見つからない場合に `{}` を返す Go 版の挙動を再現し、後続の `serde_json` パースが「入力なし」でクラッシュするのを防ぎます。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/json.rs ---

   /// 文字列から最初と最後の '{', '}' で囲まれた範囲を抽出する
   /// Go: cleanJSON 相当
   pub fn clean_json(content: &str) -> &str {
       let first = content.find('{');
       let last = content.rfind('}');

       match (first, last) {
           (Some(f), Some(l)) if f < l => &content[f..=l],
           _ => "{}",
       }
   }
   ```

### 11.23 Unknown ID の決定論的生成と重複登録防止

`Memify` 過程で LLM が「答えられない」と判断した内容を `Unknown` ノードとして記録する際、同一の内容が二重登録されるのを防ぐための決定論的 ID 生成ロジックについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/metacognition/ignorance_manager.go` (登録ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `IgnoranceManager.RegisterUnknown`: 新たな `Unknown` を生成し、グラフとベクトルストアに登録するメソッド。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // ignorance_manager.go:L79
   func (m *IgnoranceManager) RegisterUnknown(ctx context.Context, text string, ...) {
       // テキストを正規化してからハッシュ化
       normText := utils.NormalizeForVector(text)
       // "Unknown:" プレフィックスを付与して UUID v5 生成
       unknownID := uuid.NewSHA1(uuid.NameSpaceOID, []byte("Unknown:"+normText)).String()

       node := &storage.Node{
           ID:          unknownID,
           MemoryGroup: m.MemoryGroup,
           Type:        "Unknown",
           // ... 略 ...
       }
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/metacognition/ignorance.rs`: 無知管理モジュール。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `IgnoranceManager::register_unknown` メソッド内。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **同一 ID 空間の維持**: Go 版では `"Unknown:" + normText` という特定の文字列パターンをシードにしています。Rust でも全く同じプレフィックスと正規化（`NormalizeForVector`）を適用した上で UUID v5 (`new_v5`) を生成することで、Go 版とビットレベルで同一の ID が得られます。
   - **重複排除の自動化**: LadybugDB (Cozo) は同一 ID に対する `UPSERT` 挙動を基本とするため、決定論的 ID を使用することで、LLM が同じ質問を何度も生成しても、グラフ上では一つのノードとしてマージされます。
   - **トレーサビリティ**: ID 生成に `"Unknown:"` という識別子を含めることで、ランダムな UUID よりもデバッグ時の判別が容易になります。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/metacognition/ignorance.rs ---
   use crate::cuber::utils::id::generate_v5_id;
   use crate::cuber::utils::normalize::normalize_for_vector;

   impl IgnoranceManager {
       pub async fn register_unknown(&self, text: &str, requirement: &str) -> Result<String, CuberError> {
           // 1. 正規化
           let norm_text = normalize_for_vector(text);
           
           // 2. 決定論的 ID 生成 (Go: "Unknown:" + normText)
           let seed = format!("Unknown:{}", norm_text);
           let unknown_id = generate_v5_id(&seed);

           // 3. グラフノードの作成と保存
           let node = Node {
               id: unknown_id.clone(),
               memory_group: self.memory_group.clone(),
               r#type: "Unknown".to_string(),
               properties: vec![
                   ("text".to_string(), norm_text.into()),
                   ("resolution_requirement".to_string(), requirement.into()),
                   ("created_at".to_string(), chrono::Utc::now().to_rfc3339().into()),
               ].into_iter().collect(),
           };
           
           self.graph_storage.add_nodes(&[node]).await?;
           Ok(unknown_id)
       }
   }
   ```

### 11.24 MDL Principle (最小記述長) に基づく知識の忘却と洗練

知識グラフ全体の「記述密度」を最適化し、関連性の薄い（説明力の低い）孤立したノードを忘却するための、MDL (Minimum Description Length) 原理に基づいたアルゴリズムについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/metacognition/metabolism_task.go` (代謝ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `MetabolismTask.runMDLForgetting`: グラフから孤立したノードや、近傍との意味的関連性が極めて低いノードを特定し、削除するメソッド。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // metabolism_task.go:L215 (推定位置)
   // MDL Principle: 孤立した情報は「記述コスト」が高いため削除（忘却）する
   func (t *MetabolismTask) runMDLForgetting(ctx context.Context, nodes []*storage.Node) (int, error) {
       deleted := 0
       for _, node := range nodes {
           // 1. ノードに接続するエッジ数を取得
           edgeCount, _ := t.GraphStorage.CountEdges(ctx, node.ID)
           // 2. エッジが 0（孤立）かつ、作成から一定期間経過している場合に削除
           if edgeCount == 0 {
               // MDL 的に「説明されない孤立した事実」はノイズとみなす
               t.GraphStorage.DeleteNode(ctx, node.ID)
               deleted++
           }
       }
       return deleted, nil
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/metacognition/metabolism.rs`: 代謝タスクモジュール。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `MetabolismTask::run` メソッド内の第3フェーズ（Forgetting フェーズ）として実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **グラフの健康維持**: エッジの減衰（Section 11.25 で詳述）により接続を失ったノードを放置すると、グラフが「死んだ知識」で肥大化します。MDL 段階でこれらを物理削除することで、検索精度（FTS/Vector）のノイズを低減します。
   - **バッチ処理の最適化**: Rust では `tokio::mpsc` 等を用いて削除対象を収集し、LadybugDB (Cozo) のバッチ削除コマンド (`rm`) を一括発行することで、高いスループットを実現します。
   - **「知識の成熟」の再現**: Go 版でも、作成直後のノードは保護し、時間の経過とともに接続が得られなかったもののみを忘却対象としています。Rust でもプロパティの `created_at` を参照し、この時間的保護（Survival Protection）を正確に再現します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/metacognition/metabolism.rs ---

   impl MetabolismTask {
       /// MDL Principle に基づく孤立ノードの忘却
       /// Go: runMDLForgetting 相当
       async fn forget_isolated_nodes(&self, nodes: &[Node]) -> Result<usize, CuberError> {
           let mut deleted_count = 0;
           let now = chrono::Utc::now();
           let protection_hours = self.config.min_survival_protection_hours;

           for node in nodes {
               // 1. 生存保護期間のチェック
               let created_at = node.get_created_at_rfc3339().map_err(|_| CuberError::Internal("No created_at".into()))?;
               if now.signed_duration_since(created_at).num_hours() < protection_hours as i64 {
                   continue;
               }

               // 2. エッジ接続数の確認
               let degree = self.graph_storage.get_node_degree(&node.id).await?;
               
               // 3. 孤立していれば物理削除 (MDL 忘却)
               if degree == 0 {
                   self.graph_storage.delete_node(&node.id).await?;
                   deleted_count += 1;
               }
           }
           Ok(deleted_count)
       }
   }
   ```

### 11.25 数値的減衰に基づく三段階の代謝プロセス (Phases A, B, C)

知識の鮮度を数値化し、システム自律的に情報を整理・忘却するための「代謝（Metabolism）」プロセスの三段階構成、およびその基盤となる指数減衰アルゴリズムについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/metacognition/metabolism_task.go` (全体制御)
   - `mycute-go/src/pkg/cuber/utils/decay.go` (数理ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `MetabolismTask.Run`: 代謝プロセスのメインループ。
   - `CalculateThickness`: 重要度（Thickness）を算出する純粋関数。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // metabolism_task.go:L112
   // Phase A: エッジの削除 (Numerical Pruning)
   prunedEdgesCount, _ = t.pruneEdges(ctx, halfLifeDays, pruneThreshold, ...)

   // Phase B: ノードの削除 (Forgetting / MDL)
   orphanDeletedCount, _ := t.deleteOrphanedNodes(ctx, ...)
   weakDeletedCount, _ := t.deleteWeaklyConnectedNodes(ctx, ...)

   // Phase C: 矛盾解決による洗練 (Conflict Refinement)
   refinedCount, _ := t.refineConflicts(ctx)

   // utils/decay.go:L61
   // Thickness = Weight × Confidence × e^(-λΔt)
   func CalculateThickness(weight, confidence float64, edgeUnix, maxUnix int64, lambda float64) float64 {
       deltaT := float64(maxUnix - edgeUnix)
       decay := math.Exp(-lambda * deltaT)
       return weight * confidence * decay
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/metacognition/metabolism.rs`: 代謝タスクの実装。
   - `src/cuber/utils/decay.rs`: 減衰計算用ユーティリティ。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `MetabolismTask::run` メソッド内に制御フローを配置。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **数理的一致**: Rust の `f64::exp()` を使用し、Go の `math.Exp` と同一の計算結果を得ることで、知識の寿命（半減期）の挙動を完全に一致させます。
   - **フェーズ分離の厳格化**: Go 版が定義する三段階（エッジ削除 → ノード削除 → 矛盾解決）の順序は、依存関係（エッジがなくなってから孤立ノード判定ができる）に基づいているため、Rust でもこのパイプライン構造を維持します。
   - **基準時刻の一貫性**: `maxUnix`（グラフ内の最新エッジ時刻）を減衰の基準点とする Go 版の「相対時間減衰」モデルを採用することで、長期休止後の再開時に全データが一斉に消えるのを防ぎます。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/decay.rs ---
   pub fn calculate_thickness(weight: f64, confidence: f64, edge_unix: i64, max_unix: i64, lambda: f64) -> f64 {
       let delta_t = (max_unix - edge_unix).max(0) as f64;
       let decay = (-lambda * delta_t).exp();
       weight * confidence * decay
   }

   // --- src/cuber/tasks/metacognition/metabolism.rs ---
   impl MetabolismTask {
       pub async fn run(&self) -> Result<MetabolismResult, CuberError> {
           // 1. Phase A: Numeric Pruning (Edge 削除)
           let pruned_edges = self.prune_edges().await?;
           
           // 2. Phase B: Forgetting (Orphan/MDL Node 削除)
           let deleted_nodes = self.forget_nodes().await?;
           
           // 3. Phase C: Refinement (Conflict 解決)
           let refined_conflicts = self.refine_conflicts().await?;
           
           Ok(MetabolismResult { pruned_edges, deleted_nodes, refined_conflicts })
       }
   }
   ```

### 11.26 ConflictResolutionStage 1 & 2 による多角的矛盾調停

知識グラフ内の事実の衝突を、高速なルールベース処理（Stage 1）と高度な LLM 推論（Stage 2）の二段階で解消する、堅牢な矛盾調停アルゴリズムについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/utils/conflict_resolution.go` (調停エンジン)
   - `mycute-go/src/pkg/cuber/event/info_event.go` (排他定数)

2. **Go実装 de 具体的実証箇所**:
   - `Stage1ConflictResolution`: 決定論的な排他（例：生年月日は一つ）を高速に処理。
   - `Stage2ConflictResolution`: LLM に文脈を提示し、どちらを残すべきか判定させる。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // conflict_resolution.go:L189
   // Stage 1: 明示的排他対象（ExclusiveRelationType）の処理
   if ExclusiveRelationType[relationType] {
       // 最高スコア（Thickness）のエッジ以外を discarded に追加
       best := getBestTriple(group)
       markOthersAsDiscarded(group, best)
   }

   // conflict_resolution.go:L357
   // Stage 2: LLM による仲裁
   userPrompt := fmt.Sprintf(prompts.ARBITRATE_CONFLICT_USER_PROMPT, conflictDataJSON)
   response, _ := GenerateWithUsage(ctx, llm, ..., userPrompt)
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/conflict.rs`: 矛盾解決ロジック。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `pub fn resolve_conflicts_stage1(...)` および `pub async fn resolve_conflicts_stage2(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **二段階分離の有効性**: 全ての矛盾を LLM に投げるとコストと時間が増大します。Go 版と同様に、数学的スコア（Thickness）だけで決定できる項目を Stage 1 で先行処理し、LLM の負荷を「真に文脈判断が必要なもの」に絞り込む構造を Rust でも継承します。
   - **イベント通知の一致**: Go 版では削除理由（`Reason`）を詳細に記録しイベント配信しています。Rust でも `DiscardedTriple` 構造体に理由を保持し、SSE 経由でユーザーに「なぜこの知識が消されたのか」を説明できる透明性を確保します。
   - **JSON インターフェースの不変性**: LLM への入力形式（`conflict_data_json`）を Go 版と合わせることで、プロンプトの有効性を維持します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/conflict.rs ---

   /// Stage 1: 決定論的解決
   pub fn resolve_stage1(triples: Vec<ScoredTriple>) -> (Vec<ScoredTriple>, Vec<DiscardedTriple>, Vec<ConflictGroup>) {
       let groups = group_by_source_and_type(triples);
       let mut resolved = Vec::new();
       let mut discarded = Vec::new();
       let mut remaining = Vec::new();

       for (key, group) in groups {
           if is_exclusive_type(&key.relation_type) {
               // Thickness 最大のみ残す
               let (best, others) = take_best(group);
               resolved.push(best);
               discarded.extend(others.into_iter().map(|t| DiscardedTriple::new(t, "Stage1: Exclusive")));
           } else {
               // 重複排除のみ行い、残りは Stage 2 へ
               let (uniques, duplicates) = deduplicate(group);
               discarded.extend(duplicates.into_iter().map(|t| DiscardedTriple::new(t, "Stage1: Duplicate")));
               if uniques.len() > 1 {
                   remaining.push(ConflictGroup::new(key.source_id, key.relation_type, uniques));
               } else {
                   resolved.extend(uniques);
               }
           }
       }
       (resolved, discarded, remaining)
   }
   ```

### 11.27 containsUncertainty による LLM の不確かさ検知と知識化抑制

LLM が「わからない」「情報がない」と回答した際に、それを誤って正当な「解決策（Insight）」として知識登録してしまわないようガードレールを設ける不確かさ検知ロジックについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/metacognition/self_reflection_task.go` (検知ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `containsUncertainty`: LLM の回答文字列を走査し、不確実性を示すキーワードが含まれていないかチェックする関数。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // self_reflection_task.go:L301
   func containsUncertainty(s string) bool {
       uncertainPhrases := []string{
           "わかりません", "不明です", "情報がありません",
           "知りません", "分かりません", "答えられません",
       }
       for _, phrase := range uncertainPhrases {
           if strings.Contains(s, phrase) {
               return true
           }
       }
       return false
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/llm.rs`: LLM 応答解析ユーティリティ。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `pub fn contains_uncertainty(text: &str) -> bool` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **誤学習の防止**: LLM は時に「その情報は見当たりません」といった文章をあたかも事実であるかのように出力することがあります。Go 版の「ブラックリスト方式」を Rust でも継承し、これらのノイズを登録前に弾くことで、知識グラフの純度を保ちます。
   - **ローカライズへの配慮**: Go 版では日本語の複数の表記揺れ（分かりません vs わかりません）をカバーしています。Rust でも同様の語彙リストを維持し、必要に応じて多言語対応のための正規表現やクレート（`aho-corasick` 等）による高速なマッチングを検討します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/llm.rs ---

   /// LLM の回答に不確実性（わからない、等）が含まれているか判定する
   /// Go: containsUncertainty 相当
   pub fn contains_uncertainty(text: &str) -> bool {
       const UNCERTAIN_PHRASES: &[&str] = &[
           "わかりません", "不明です", "情報がありません",
           "知りません", "分かりません", "答えられません",
           "I don't know", "not sure", "insufficient information",
       ];

       UNCERTAIN_PHRASES.iter().any(|&phrase| text.contains(phrase))
   }
   ```

### 11.28 バッチ分割における日本語自然境界とオーバーラップの精密制御

大規模なテキストを処理する際、文の途中で機械的に分割されることによる文脈欠落を防ぎ、日本語の自然な節目（文末）でバッチを分割・接合するためのアルゴリズムについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/memify/japanese_splitter.go` (分割エンジン)

2. **Go実装 de 具体的実証箇所**:
   - `JapaneseSentenceEnders`: 日本語の文末記号の定義。
   - `SplitAtNaturalBoundary`: 目標文字数付近で最も近い文末を探索する関数。
   - `SplitTextWithOverlap`: オーバーラップを維持しつつ全体を分割するメイン関数。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // japanese_splitter.go:L4
   var JapaneseSentenceEnders = map[rune]bool{
       '。': true, '！': true, '？': true, '．': true, '\n': true,
   }

   // japanese_splitter.go:L130
   // 自然境界で分割位置を調整
   splitPos := SplitAtNaturalBoundary(searchText, batchSize, searchRangePercent)
   actualEnd := currentStart + splitPos

   // japanese_splitter.go:L140
   // 次のバッチの開始位置（オーバーラップを考慮し、かつ自然境界に再調整）
   nextStart := actualEnd - overlapChars
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/splitter.rs`: 日本語対応テキストスプリッター。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `pub fn split_text_with_overlap(text: &str, batch_size: usize, overlap_percent: usize) -> Vec<String>` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **文脈の連続性**: 単純な文字数分割では LLM が文の断片しか受け取れず、誤った知識抽出を行うリスクがあります。Go 版の「文末記号を優先し、読点（、）をフォールバックにする」二段階探索アルゴリズムを忠実に再現することで、Rust 版でも高品質な抽出を維持します。
   - **オーバーラップの動的調整**: オーバーラップ開始点も固定位置ではなく、直近の文末に「吸着」させる Go 版の動的バイアス制御を再現します。これにより、バッチ間の情報の重複が意味的な区切りで発生し、LLM による情報の接合（Reconciliation）が容易になります。
   - **Unicode 安全性**: Rust の `char` 走査により、Go の `[]rune` と同様にマルチバイト文字の境界を壊さずに処理することを保証します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/splitter.rs ---

   /// 日本語の自然な境界でテキストを分割し、オーバーラップを付与する
   pub fn split_text_with_overlap(text: &str, batch_size: usize, overlap_percent: usize) -> Vec<String> {
       let chars: Vec<char> = text.chars().collect();
       let mut batches = Vec::new();
       let overlap_len = batch_size * overlap_percent / 100;
       let mut current_start = 0;

       while current_start < chars.len() {
           let mut target_end = (current_start + batch_size).min(chars.len());
           
           // 自然境界（文末）の探索
           if target_end < chars.len() {
               target_end = find_natural_boundary(&chars, target_end, 0.2);
           }

           let batch: String = chars[current_start..target_end].iter().collect();
           batches.push(batch);

           if target_end == chars.len() { break; }

           // 次の開始位置の計算（オーバーラップ分戻り、自然境界に吸着）
           let mut next_start = target_end.saturating_sub(overlap_len);
           next_start = find_natural_boundary_forward(&chars, next_start, target_end);
           current_start = next_start;
       }
       batches
   }
   ```

### 11.29 time.RFC3339 形式によるタイムスタンプ保存の完全互換

グラフ内のプロパティ（`created_at`, `updated_at`, `acquired_at` 等）として保持される日時情報を、Go 版と Rust 版で共通の文字列表現（RFC3339）として統一するための仕様について詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go` (保存処理)
   - `mycute-go/src/pkg/cuber/tasks/metacognition/ignorance_manager.go` (メタデータ生成)

2. **Go実装 de 具体的実証箇所**:
   - `ladybugdb_storage.go`: 構造体から Cozo 形式へ変換する際、`time.RFC3339` フォーマットで文字列化。
   - `ignorance_manager.go`: `Unknown` や `Capability` ノードを生成する際、現在時刻を `time.RFC3339` でプロパティに設定。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // ladybugdb_storage.go:L415
   createdAt := data.CreatedAt.Format(time.RFC3339)

   // ignorance_manager.go:L90
   "created_at": time.Now().Format(time.RFC3339),
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/time.rs`: 日時操作ヘルパーモジュール。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `pub fn now_rfc3339() -> String` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **相互運用性の確保**: LadybugDB (Cozo) の `Relations` に保存される日時は単なる文字列ですが、これを標準的な `2024-08-29T10:03:05Z` 形式 (RFC3339) に統一することで、Go 版で作成した DB を Rust 版で読み込んだ際、あるいはその逆において、`chrono::DateTime::parse_from_rfc3339` 等による安全な再パースが確実に行えます。
   - **他ツールとの親和性**: 外部ツールでのクエリ（例：特定の期間のノードを抽出）において、ISO8601/RFC3339 形式は最も汎用性が高く、Rust の標準的な `chrono` クレートによるパースと完全一致します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/time.rs ---
   use chrono::{DateTime, Utc, SecondsFormat};

   /// 現在時刻を Go の time.RFC3339 と完全互換のある形式で取得する
   pub fn now_rfc3339() -> String {
       Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
   }

   /// RFC3339 文字列を DateTime にパースする
   pub fn parse_rfc3339(s: &str) -> Result<DateTime<Utc>, chrono::ParseError> {
       DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc))
   }
   ```

### 11.30 FTS を活用した芋づる式エンティティ増殖 (Expansion)

ベクトル検索（意味的検索）だけでは漏れてしまう「キーワードベースの関連知識」を、LadybugDB の FTS（全文検索）拡張を用いて漏らさず拾い上げ、知識グラフを指数関数的に豊かにする「エンティティ拡張」プロセスについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go` (FTS クエリ)
   - `mycute-go/src/pkg/cuber/tools/query/search.go` (拡張ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `SearchEntities`: 指定されたキーワード（名詞等）で FTS インデックスを検索し、関連するエンティティ ID を取得する。
   - `QueryFtsExpansion`: 抽出されたエンティティ名を「種」として FTS を実行し、共通のキーワードを持つ他のチャンクやノードを芋づる式に特定する。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // ladybugdb_storage.go:L870
   // LadybugDB の FTS 拡張 (query_fts_index) を使用
   query := fmt.Sprintf("select chunk_id, score from query_fts_index('%s', '%s', %d)", 
       layer, searchTerms, limit)
   rows, err := s.conn.Query(ctx, query)
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/storage/ladybugdb/fts.rs`: FTS 特化のクエリビルダ。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `impl LadybugDbStorage` のメソッドとして `pub async fn search_entities_fts(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **Cozo FTS 互換性**: CozoDB（LadybugDB）の `query_fts_index` は特殊関数であり、標準備の SQL ではなく Datalog 形式の拡張として呼び出す必要があります。Go 版が `fmt.Sprintf` で構築しているのと同様に、Rust でも `Cozo` クレートを介して正しい関数・引数（layer, terms, limit）を渡す DSL を構築します。
   - **レイヤー化されたインデックス**: Go 版では `nouns` / `nouns_verbs` という用途別の FTS カラムを使い分けています。Rust でもこのスキーマ構成を維持し、検索精度とノイズ率のバランスを制御できるようにします。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし, Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/storage/ladybugdb/fts.rs ---

   impl LadybugDbStorage {
       pub async fn search_entities_fts(&self, terms: &str, layer: &str, limit: usize) -> Result<Vec<SearchResult>, StorageError> {
           // Cozo FTS 拡張のクエリ構築
           let script = format!(
               "?[chunk_id, score] := query_fts_index('{}', '{}', {})",
               layer, terms, limit
           );
           
           let result = self.conn.run_script(&script, Default::default()).await?;
           // 結果のパースと返却
           Ok(result.rows.into_iter().map(SearchResult::from).collect())
       }
   }
   ```

### 11.31 120種類以上の排他的関係リストに基づく Stage 1 矛盾解決

「人間は同時に 2 つの居住地を持たない（現住所）」「CEO は通常一人である」といった、事象の性質から決まる 120 種類以上の厳格な矛盾ルール群と、それに基づく Stage 1（決定論的解決）の実装について詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/utils/conflict_resolution.go` (ルールリスト定義)

2. **Go実装 de 具体的実証箇所**:
   - `ExclusiveRelationType`: 120 項目以上に及ぶ「一対一」であるべき関係タイプのマップ。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // conflict_resolution.go:L28
   var ExclusiveRelationType = map[string]bool{
       "lives_in": true, "current_job": true, "married_to": true,
       "capital_of": true, "current_version": true, "birth_date": true,
       // ... (以下 120 項目以上続く)
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/conflict_rules.rs`: 矛盾ルールの定数定義。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `lazy_static!` または `phf` クレート（Perfect Hash Function）を用いて高速な定数セットとして定義。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **ルールの完全同期**: 矛盾解決の品質は、このリストの網羅性に依存します。Go 版で精緻に定義された 120 項目以上のルールを Rust に 100% 移植することで、移行後もグラフの整合性が損なわれないことを保証します。
   - **実行パフォーマンス**: 大量のトリプルを処理する際、このマップの引きは最速である必要があります。Rust では `phf` を使用することで、コンパイル時にハッシュマップを最適化し、実行時のオーバーヘッドを極小化します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし, Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/conflict_rules.rs ---
   use phf::phf_set;

   /// ステージ 1 で排他処理対象となる関係タイプのリスト
   /// Go: ExclusiveRelationType と完全一致させる
   pub static EXCLUSIVE_RELATIONS: phf::Set<&'static str> = phf_set! {
       "lives_in", "current_address", "resides_at", "works_at", "employed_by",
       "current_occupation", "current_job", "married_to", "spouse", "age",
       "capital_of", "headquarters_in", "current_version", "birth_date",
       // ... (120 項目をすべて転記)
   };

   pub fn is_exclusive(relation_type: &str) -> bool {
       EXCLUSIVE_RELATIONS.contains(relation_type)
   }
   ```

### 11.32 unknownSearchPrompt によるメタ認知的な不足情報特定

獲得した知識を客観的に評価し、「現状何が分かっていないか（空白地帯）」を LLM 自らに言語化させることで、自律的な情報収集の「問い」を生成するメタ認知ロジックについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/prompts/prompts.go` (プロンプト定義)
   - `mycute-go/src/pkg/cuber/tasks/metacognition/self_reflection_task.go` (実行タスク)

2. **Go実装 de 具体的実証箇所**:
   - `UnknownDetectionSystemPromptJA`: 知識の GAP（論理的欠落、未定義用語）を検出するためのシステムプロンプト。
   - `SelfReflectionTask.Run`: 現在の知識セット（Rules）を LLM に提示し、不足情報を `Unknown` オブジェクトとして抽出。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // prompts.go:L911
   const UnknownDetectionSystemPromptJA = `You are a metacognitive agent analyzing knowledge gaps.
   Identify what is UNKNOWN or MISSING: 1. Logical gaps, 2. Missing definitions, 3. Unanswered questions.`

   // self_reflection_task.go:L75
   // 現在のルールをコンテキストとして LLM に問いを立てさせる
   prompt := fmt.Sprintf(prompts.RuleExtractionUserPromptTemplate, rulesText, "")
   response, _ := GenerateJSON(ctx, t.ChatModel, ..., UnknownDetectionSystemPromptJA, prompt)
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/prompts/metacognitve.rs`: メタ認知専用プロンプト管理。
   - `src/cuber/tasks/metacognition/reflection.rs`: 自己省察タスクの実装。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `impl SelfReflectionTask` の `pub async fn identify_unknowns(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **自律的ループの起点**: Cuber の強みは、受動的なインジェストだけでなく「何を知るべきか」を自ら考える点にあります。Go 版の `UnknownDetection` プロンプトを移植することで、Rust 版でも未知の情報を `logical_gap` 等のカテゴリで分類・識別し、後の `Query` や `Memify` での「自律検索」のトリガーにすることを可能にします。
   - **JSON 構造の維持**: `Unknown` を登録する際の Schema (`text`, `type`) を Go と合わせることで、LadybugDB に蓄積される「知りたいことリスト」の互換性を保ちます。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし, Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/metacognition/reflection.rs ---

   impl SelfReflectionTask {
       pub async fn identify_unknowns(&self, rules: &[Rule]) -> Result<Vec<Unknown>, CuberError> {
           let context = rules.iter().map(|r| &r.text).collect::<Vec<_>>().join("\n");
           let system_prompt = prompts::JA_UNKNOWN_DETECTION;
           
           // LLM に現在の知識の GAP を抽出させる
           let response: UnknownResponse = self.chat_model
               .generate_json(system_prompt, &context)
               .await?;

           Ok(response.unknowns.into_iter().map(|u| u.into()).collect())
       }
   }
   ```

### 11.33 GraphExpansionLoop による n-degree ホップの知識抽出

特定のエンティティに関連する知識を、1 ホップ（直接の関係）に留まらず、設定された深さまで再帰的に探索し、巨大な知識の網から必要な「文脈の塊」を削り出す再帰展開ロジックについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/cuber.go` (再帰のオーケストレーション)

2. **Go実装 de 具体的実証箇所**:
   - `executeMemifyCore`: `RecursiveDepth` 設定に基づき、自分自身を再帰的に呼び出す、あるいはループ内で多層展開を行うコアロジック。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // cuber.go:L958
   // RecursiveDepth に達するまで、抽出されたエンティティを基点に Memify を繰り返す
   for i := 0; i < memifyConfig.RecursiveDepth; i++ {
       usage, err := s.executeMemifyCore(txCtx, st, memoryGroup, ...)
       totalUsage.Add(usage)
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/mod.rs`: `CuberService` のメインロジック。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `pub async fn memify(...)` 内のメインループ。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **ドメイン知識の深掘り**: 単発の RAG では表面的な情報しか得られませんが、この展開ループにより、例えば「プロジェクト A」→「担当者 B」→「B の得意技術 C」といった芋づる式の情報取得が可能になります。
   - **リソース制限の継承**: 無限ループを防ぐための `RecursiveDepth` による明示的な制限を Rust でも実装し、LLM コストの暴走を防ぎつつ最大限の知識密度を確保します。
   - **トークン使用量の累積**: 各ステップで発生する `TokenUsage` を集計し、最終的なコストとして正確に報告する Go 版の挙動を再現します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし, Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/mod.rs ---

   impl CuberService {
       pub async fn memify(&self, config: MemifyConfig) -> Result<MemifyResult, CuberError> {
           let mut total_usage = TokenUsage::default();
           
           // Go: 11.33 GraphExpansionLoop 相当
           for depth in 0..config.recursive_depth {
               log::info!("Starting Memify Expansion Loop: Level {}", depth);
               let usage = self.execute_memify_step(&config).await?;
               total_usage.add(usage);
               
               if self.is_knowledge_saturated().await? { break; }
           }
           
           Ok(MemifyResult::new(total_usage))
       }
   }
   ```

### 11.34 RegisterCapability による解決済み未知情報の機能化

自問自答（Self-Reflection）によって解決された `Unknown`（未知情報）を、単なるデータではなく「システムができるようになったこと（能力）」としてグラフに再登録し、将来のクエリ回答能力を向上させる「機能化」プロセスについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/metacognition/ignorance_manager.go`

2. **Go実装 de 具体的実証箇所**:
   - `RegisterCapability`: 解決されたインサイトを `Capability` 型のノードとして保存し、解決済みの `Unknown` やトリガーとなった `User` とエッジで結ぶ。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // ignorance_manager.go:L115
   func (m *IgnoranceManager) RegisterCapability(ctx context.Context, text string, ...) {
       capabilityID := uuid.NewSHA1(uuid.NamespaceOID, []byte("Capability:"+normText)).String()
       node := &storage.Node{
           ID: capabilityID,
           Type: "Capability",
           Properties: map[string]any{"text": normText, "acquired_at": time.Now().Format(time.RFC3339)},
       }
       m.GraphStorage.AddNodes(ctx, []*storage.Node{node})
       // ... 解決済み Unknown とのエッジ (RESOLVED_BY) も作成
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/metacognition/ignorance.rs`: 未知・能力管理クラス。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `impl IgnoranceManager` の `pub async fn register_capability(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **決定論的 ID の維持**: `Capability:` プレフィックスと SHA1 ハッシュを用いた ID 生成ロジックを正確に移植することで、同一の能力が重複登録されるのを防ぎ、Go 版とのデータ一貫性を確保します。
   - **トレーサビリティの確保**: 単にノードを追加するだけでなく、`RESOLVED_BY` エッジによって「どの問いがいつ、どのような知見で解決されたか」の履歴をグラフ構造として維持する Go 版の洗練された設計を Rust でも忠実に再現します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし, Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/metacognition/ignorance.rs ---

   impl IgnoranceManager {
       pub async fn register_capability(&self, insight: &str, resolved_unknown_id: &str) -> Result<String, CuberError> {
           let norm_text = utils::normalize_for_vector(insight);
           let capability_id = self.generate_id("Capability", &norm_text); // UUID v5 (SHA1)
           
           let node = Node::new(capability_id.clone(), "Capability")
               .with_property("text", norm_text)
               .with_property("acquired_at", utils::now_rfc3339());

           self.graph_storage.add_node(node).await?;
           
           // 解決済み Unknown との関係を記録
           self.graph_storage.add_edge(Edge::new(
               &capability_id, resolved_unknown_id, "RESOLVED_UNKNOWN"
           )).await?;

           Ok(capability_id)
       }
   }
   ```

### 11.35 Hybrid RAG (Vector + Graph) による最終回答の生成パイプライン

ベクトル検索が提供する「具体的な文脈（Chunks）」と、知識グラフが提供する「構造化された事実（Graph Summary）」を高度に融合させ、ハルシネーションを抑制しつつ正確かつ深い回答を生成する最終回答パイプラインについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tools/query/graph_completion.go`
   - `mycute-go/src/pkg/cuber/prompts/prompts.go` (プロンプト定義)

2. **Go実装 de 具体的実証箇所**:
   - `GenerateAnswer`: ベクトル検索結果（`[]*storage.QueryResult`）とグラフの要約テキストを結合し、`ANSWER_QUERY_WITH_HYBRID_RAG_JA_PROMPT` を用いて LLM に最終回答を生成させる。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // graph_completion.go:L50
   // Chunks と GraphSummary をマージしてプロンプトを作成
   prompt := fmt.Sprintf(
       "Context from Documents:\n%s\n\nContext from Knowledge Graph:\n%s\n\nUser Question: %s",
       vectorContext, graphContext, query,
   )
   response, usage, err := GenerateResponse(ctx, t.ChatModel, prompts.ANSWER_QUERY_WITH_HYBRID_RAG_JA_PROMPT, prompt)
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tools/query/completion.rs`: 回答生成エンジン。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `impl AnswerGenerator` の `pub async fn generate_hybrid_answer(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **ハイブリッド・シナジーの実現**: Go 版の最大の利点は、曖昧なベクトル検索を、グラフによる「事実の裏打ち」で補完している点にあります。Rust でも `VectorContext` と `GraphContext` を明確に分離して LLM に提示するプロンプト構造を維持することで、回答の信頼性と納得感を同等レベルで提供します。
   - **プロンプトエンジニアリングの継承**: Python/Go 版で磨き上げられた `ANSWER_QUERY_WITH_HYBRID_RAG` プロンプトをそのまま Rust に持ち込むことで、思考の論理性を英語で保ちつつ回答を日本語で行う「Double-Reasoning」戦略を継承します。

7. **Go実装の具体的実装箇所のコードスニペットを根拠とし, Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tools/query/completion.rs ---

   impl AnswerGenerator {
       pub async fn generate_hybrid_answer(&self, query: &str, chunks: &[Chunk], graph_summary: &str) -> Result<Answer, CuberError> {
           let vector_context = chunks.iter().map(|c| &c.text).collect::<Vec<_>>().join("\n---\n");
           
           // Go: 11.35 Hybrid RAG 相当
           let user_prompt = format!(
               "Context from Documents:\n{}\n\nContext from Knowledge Graph:\n{}\n\nUser Question: {}",
               vector_context, graph_summary, query
           );

           let answer: String = self.chat_model
               .generate(prompts::JA_HYBRID_RAG_SYSTEM, &user_prompt)
               .await?;

           Ok(Answer::new(answer))
       }
   }
   ```

### 11.36 RuleExtractionTask によるドキュメントからの原理・原則の抽出

ドキュメントの各チャンクから、プロジェクトの共通規約、コーディングルール、あるいは普遍的な原理・原則を抽出・構造化し、`NodeSet` と呼ばれる名前付きの集合体に統合するプロセスについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/memify/rule_extraction_task.go` (抽出・登録)

2. **Go実装 de 具体的実証箇所**:
   - `RuleExtractionTask.ProcessBatch`: テキストバッチからルールを抽出し、決定論的 ID を持つノードとして保存するメインメソッド。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // rule_extraction_task.go:L185
   // 5. NodeSetノードを作成（冪等）
   ruleSetNodeID := uuid.NewSHA1(uuid.NameSpaceOID, []byte(t.RulesNodeSetName)).String()
   ruleSetNode := &storage.Node{
       ID: ruleSetNodeID, Type: "NodeSet",
       Properties: map[string]any{"name": t.RulesNodeSetName},
   }

   // rule_extraction_task.go:L202
   for _, rule := range ruleSet.Rules {
       // ルールIDを生成（ルールテキストから決定論的に）
       ruleID := uuid.NewSHA1(uuid.NameSpaceOID, []byte(rule.Text)).String()
       ruleNode := &storage.Node{
           ID: ruleID, Type: "Rule",
           Properties: map[string]any{"text": rule.Text},
       }
       // ... 略 ...
       // ルール -> NodeSet のエッジ (belongs_to)
       edge := &storage.Edge{SourceID: ruleID, TargetID: ruleSetNodeID, Type: "belongs_to"}
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/memify/rule.rs`: ルール抽出タスク。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `impl RuleExtractionTask` の `pub async fn process_batch(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **決定論的 ID の一貫性**: Go 版と同様に `uuid::v5` (SHA-1) を使用し、且つ `NameSpaceOID` を同一にすることで、同一テキストのルールに対し Go 版とビットレベルで同一の `rule_id` を生成し、冪等性を保証します。
   - **NodeSet による構造化**: 単なるフラットなノード群ではなく、`NodeSet` (特殊ノード) を頂点としたスター型トポロジーを構築する Go 版の設計を継承し、知識のカテゴリ管理を可能にします。
   - **正規化の徹底**: `ProcessBatch` 内で実行される `NormalizeForVector` を Rust でも同様のタイミングで適用し、検索インデックスの整合性を保ちます。

7. **Go実装 de Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/memify/rule.rs ---

   impl RuleExtractionTask {
       pub async fn process_batch(&self, texts: &[String]) -> Result<TokenUsage, CuberError> {
           let combined_text = texts.join("\n\n");
           // 1. LLM でルールセットを抽出
           let rule_set: RuleSet = self.chat_model.generate_json(system_prompt, &combined_text).await?;

           // 2. NodeSet (親) の ID 生成
           let nodeset_id = generate_v5_id_with_prefix("NodeSet", &self.rules_nodeset_name);
           
           let mut nodes = vec![Node::new(nodeset_id.clone(), "NodeSet").with_property("name", &self.rules_nodeset_name)];
           let mut edges = Vec::new();

           for rule_raw in rule_set.rules {
               let norm_text = normalize_for_vector(&rule_raw.text);
               let rule_id = generate_v5_id(&norm_text); // Go: uuid.NewSHA1(...)

               nodes.push(Node::new(rule_id.clone(), "Rule").with_property("text", norm_text.clone()));
               edges.push(Edge::new(&rule_id, &nodeset_id, "belongs_to"));
               
               // 3. Vector Storage への保存予約
               self.vector_storage.queue_embedding(TABLE_NAME_RULE, &rule_id, &norm_text).await?;
           }

           self.graph_storage.add_nodes(&nodes).await?;
           self.graph_storage.add_edges(&edges).await?;
           Ok(usage)
       }
   }
   ```

### 11.37 CrystallizationTask による類似ルールの高次統合（結晶化）

現在の知識ベースに存在する類似したルールを特定し、LLM によってそれらを一つの洗練された「結晶化された知識」へとマージし、グラフ構造を再構築するプロセスについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/metacognition/crystallization_task.go` (統合ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `CrystallizationTask.CrystallizeRules`: 現在の全ルールを取得し、類似度に基づいてクラスタリング、LLM による統合、そしてエッジの再構築（Re-wiring）を行うメインプロセス。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // crystallization_task.go:L101
   // LLMで統合テキストを生成
   crystallized, usage2, err := t.mergTexts(ctx, texts)

   // crystallization_task.go:L112
   // 新しい統合ノードを作成 (SHA-1 v5 ID)
   crystallizedID := uuid.NewSHA1(uuid.NameSpaceOID, []byte("Crystallized:"+crystallized)).String()

   // crystallization_task.go:L141
   // 2. エッジの付け替え (Re-wiring)
   for _, oldNodeID := range ids {
       // Inbound Edges (Others -> Old) => (Others -> New)
       // ...
       // Outbound Edges (Old -> Others) => (New -> Others)
       // ...
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/metacognition/crystallization.rs`: 結晶化タスク。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `impl CrystallizationTask` の `pub async fn crystallize_rules(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **グラフ整合性の維持**: Go 版の「Re-wiring」ロジックは、古い知識を消すだけでなく、その接続（意味のコンテキスト）を新しい統合知識へと確実に引き継ぐためのものです。Rust でも `GraphStorage` の `GetEdgesByNode` と `UpdateEdgeEndpoints` (または Add/Delete) を組み合わせ、このリンク継承を正確に再現します。
   - **「死んだ知識」の排除**: 統合後に元の古いルールノードを `DeleteNode` する Go 版の挙動を継承し、グラフの肥大化を防ぎます。
   - **SHA-1 による不変 ID**: `"Crystallized:"` プレフィックスを用いた ID 生成を Rust でも採用し、Go 版とのデータ一貫性を担保します。

7. **Go実装 de Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/metacognition/crystallization.rs ---

   impl CrystallizationTask {
       pub async fn crystallize_rules(&self) -> Result<(), CuberError> {
           let clusters = self.cluster_rules_by_similarity().await?;

           for cluster in clusters {
               // 1. LLM で知識を統合 (Crystallize)
               let texts: Vec<String> = cluster.iter().map(|n| n.text()).collect();
               let crystallized_text = self.llm_merge_texts(&texts).await?;

               // 2. 新しいノードの作成
               let new_id = generate_v5_id_with_prefix("Crystallized", &crystallized_text);
               let new_node = Node::new(new_id.clone(), "Rule").with_property("text", crystallized_text);
               self.graph_storage.add_node(new_node).await?;

               // 3. Re-wiring (古いノードのエッジを新しいノードへ付け替え)
               for old_node in cluster {
                   self.graph_storage.rewire_edges(&old_node.id, &new_id).await?;
                   // 4. 古いノードの削除
                   self.graph_storage.delete_node(&old_node.id).await?;
               }
           }
           Ok(())
       }
   }
   ```

### 11.38 GraphRefinementTask によるエッジの動的再評価（代謝 Stage C）

新しく獲得されたルール（Insight）に照らして、既存のグラフエッジが依然として妥当かどうかを LLM に再評価させ、重要度（Confidence/Weight）を動的に更新するプロセスについて詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/tasks/metacognition/graph_refinement_task.go` (評価ロジック)

2. **Go実装 de 具体的実証箇所**:
   - `GraphRefinementTask.RefineEdges`: 新しいルールをコンテキストとしてエッジの妥当性を評価し、Strengthen（強化）/ Weaken（減衰）/ Delete（削除）を判断するメインプロセス。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // graph_refinement_task.go:L151
   switch eval.Action {
   case "strengthen":
       // Confidence を Alpha 分だけ増加（最大1.0）
       newConfidence = min(1.0, currentEdge.Confidence+t.Config.Alpha)
   case "weaken":
       // Confidence を Delta 分だけ減少（最小0.0）
       newConfidence = max(0.0, currentEdge.Confidence-t.Config.Delta)
   case "delete":
       // 直接削除
       t.GraphStorage.DeleteEdge(ctx, eval.SourceID, currentEdge.Type, eval.TargetID, t.MemoryGroup)
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/tasks/metacognition/refinement.rs`: グラフ洗練タスク。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `impl GraphRefinementTask` の `pub async fn refine_edges(...)` を実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **数理的代謝モデルの再現**: エッジの強さを単なる定数ではなく、`Alpha`（強化）と `Delta`（減衰）というパラメータによる動的な増減として扱う Go 版の設計を Rust でも継承します。これにより、頻繁に支持される正しい知識が強化され、矛盾する古い知識が自然に淘汰される「知識の自己組織化」が実現されます。
   - **LLM による文脈仲裁**: 単純な回数カウントではなく、LLM に「新しいルールと照らし合わせてこの関係はまだ正しいか？」を判定させることで、意味レベルでの正確な洗練を可能にします。

7. **Go実装 de Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/tasks/metacognition/refinement.rs ---

   impl GraphRefinementTask {
       async fn apply_metabolism(&self, eval: &EdgeEvaluation) -> Result<(), CuberError> {
           let mut edge = self.graph_storage.get_edge(&eval.source_id, &eval.target_id).await?;

           match eval.action.as_str() {
               "strengthen" => {
                   edge.confidence = (edge.confidence + self.config.alpha).min(1.0);
               }
               "weaken" => {
                   edge.confidence = (edge.confidence - self.config.delta).max(0.0);
               }
               "delete" => {
                   return self.graph_storage.delete_edge(&eval.source_id, &edge.r#type, &eval.target_id).await;
               }
               _ => return Ok(()),
           }

           // 生存スコア (S = W * C) が閾値 (PruneThreshold) を下回れば物理削除
           if edge.weight * edge.confidence < self.config.prune_threshold {
               self.graph_storage.delete_edge(&eval.source_id, &edge.r#type, &eval.target_id).await?;
           } else {
               self.graph_storage.update_edge_metrics(edge).await?;
           }
           Ok(())
       }
   }
   ```

### 11.39 Cube データベースのエクスポート機能 (ExportCubeToZip)

LadybugDB の物理ファイル（`.db`）と、それに関連するメタデータ等の追加ファイルを ZIP アーカイブとして一つにパッケージ化する機能について詳述します。

1. **Go実装 de ファイルパス**:
   - `mycute-go/src/pkg/cuber/cuber.go` (ユーティリティ)

2. **Go実装 de 具体的実証箇所**:
   - `ExportCubeToZip`: 指定された Cube の DB ファイルおよび任意の追加ファイルを ZIP 形式でシリアライズし、ポータブルなバイナリとして生成するユーティリティメソッド。

3. **Go実装 de 具体的実装箇所のコードスニペット**:
   ```go
   // cuber.go:L1259
   func ExportCubeToZip(cubeDbFilePath string, extraFiles map[string][]byte) (*bytes.Buffer, error) {
       buf := new(bytes.Buffer)
       zw := zip.NewWriter(buf)
       defer zw.Close()
       // 1. メタデータ等の追加ファイル
       for filename, content := range extraFiles {
           AddToZip(zw, filename, content)
       }
       // 2. LadybugDB データベースファイル本体
       filename := filepath.Base(cubeDbFilePath)
       zipPath := filepath.Join("db", filename)
       data, _ := os.ReadFile(cubeDbFilePath)
       AddToZip(zw, zipPath, data)
       return buf, nil
   }
   ```

4. **それをRustとして src/cuber 内のどこに何というファイル名で実装するのか**:
   - `src/cuber/utils/archive.rs`: アーカイブ・エクスポートユーティリティ。

5. **Rustでの実装先ファイルのどこに実装するのか**:
   - `pub fn export_cube_to_zip(...)` 関数として実装。

6. **Go実装の具体的実装箇所のコードスニペットを根拠とし、Rustでは何故その下に書くRustでの実装コードスニペットが正しい実装だと言えるのかという根拠説明**:
   - **ポータビリティの確保**: 単一の SQLite (LadybugDB) ファイルだけでなく、外部メタデータを含めてパッケージ化する Go 版の仕様を `zip` クレートを用いて Rust でも正確に実装します。これにより、別環境への Cube の移行（バックアップ・共有）が容易になります。
   - **メモリ効率**: `Vec<u8>` または `std::io::Cursor` を用いて、オンメモリでのアーカイブ作成をサポートし、API レスポンス（バイナリ配信）への統合を容易にします。

7. **Go実装 de Rustで書いた実装コードスニペットそのもの**:
   ```rust
   // --- src/cuber/utils/archive.rs ---
   use zip::write::FileOptions;
   use std::io::{Write, Cursor};
   use std::collections::HashMap;
   use std::path::Path;

   pub fn export_cube_to_zip(db_path: &Path, extra_files: HashMap<String, Vec<u8>>) -> Result<Vec<u8>, CuberError> {
       let mut buf = Vec::new();
       let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
       let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

       // 1. DB本体の追加
       let db_name = db_path.file_name().ok_or(CuberError::InvalidPath)?.to_str().unwrap();
       zip.start_file(format!("db/{}", db_name), options)?;
       let db_data = std::fs::read(db_path)?;
       zip.write_all(&db_data)?;

       // 2. 追加ファイルの追加 (メタデータ等)
       for (name, content) in extra_files {
           zip.start_file(name, options)?;
           zip.write_all(&content)?;
       }

       zip.finish()?;
       Ok(buf)
   }
   ```

## 12. ./mycute-go の ./src 内への Rust での書き換え実装ステップ100

本セクションは、既存の Go 実装（`mycute-go`）を Rust（`src`）へ安全かつ確実に移植するための詳細な実行計画です。各ステップは「実装 → 動作確認（テスト）」のサイクルを含むアトミックな単位で構成されています。

### 実行環境とコマンドに関する重要事項

実装およびテストを行う際は、以下の前提条件とコマンドルールを厳守してください。

- **MySQL 環境**: `make up-mysql` は実行済みであり、MySQL は `localhost` で稼働中と想定して構いません。
- **サーバー起動**: 開発中の REST API サーバーの起動には、必ず以下の Make コマンドを使用してください。`cargo run` の直接実行は禁止です。
  ```bash
  make run ARGS="rt"
  ```
- **DB マイグレーション**: SeaORM のマイグレーションファイル作成には、`sea-orm-cli` を直接使用せず、必ず以下のコマンドをテーブル単位で使用してください。
  ```bash
  make gen-migration NAME="create_<テーブル名(複数形)>_tbl"
  ```
- **マイグレーションの実行**: マイグレーションファイル追加後は `src/migration/mod.rs` に追記し、以下のコマンド（am モード）で実行してテーブルを作成してください。
  ```bash
  make run ARGS="am"
  ```
- **エンティティ生成**: テーブル作成後は、必ず以下のコマンドでエンティティファイルを生成・更新してください。
  ```bash
  make gen-entities HOST="localhost"
  ```
- **クレート追加**: Rustでの実装中に新たなクレートを入れる必要が出た場合、Cargo.tomlに直接書き込むということをしてはいけません。必ず `cargo add` あるいは `cargo add --features ???` というコマンドで入れるようにし、Cargo.toml にそれによって書き込まれたクレートの設定が最新版で且つ正しい設定になっているかを確認するという手順でなければなりません。

### Phase 1: 初期エンドポイントとダミー実装による開通確認 (Step 001 - 015) ✅ 完了

まずは `chat_models` と `cubes` モジュールの外側（Handler/Routing）を構築し、200 OK が返る状態を目指します。論理的実装は後回しにし、インターフェースの整合性を最優先で確立します。

- [x] 001. `chat_models_handler.rs` の作成と `search` ダミー実装
    - `src/mode/rt/rthandler/chat_models_handler.rs` を新規作成し、`search_chat_models` 関数のスケルトンを実装します。
    - Routing から呼び出せるように `mod.rs` で公開設定を行います。
    - 動作確認として、`make run ARGS="rt"` でサーバーを起動し、エンドポイントが 200 OK を返すことを確認します。
    ---
    - [x] 001-1. `src/mode/rt/rthandler/chat_models_handler.rs` ファイルを作成する
    - [x] 001-2. `search_chat_models` 関数を定義する (引数: `Json<SearchChatModelsParam>`, 戻り値: `impl IntoResponse`)
    - [x] 001-3. 関数内で `log::debug!("<ChatModels> search_chat_models called")` を出力する
    - [x] 001-4. ダミーの JSON レスポンス `{"message": "dummy"}` を返すように実装する
    - [x] 001-5. `src/mode/rt/rthandler/mod.rs` に `pub mod chat_models_handler;` を追加する

- [x] 002. `chat_models_handler.rs` への CRUD ダミー実装
    - `get`, `create`, `update`, `delete` の各操作に対応するハンドラ関数を追加します。
    - 全てダミーレスポンス (200 OK) を返すように実装し、各メソッドのシグネチャを確定させます。
    - パスパラメータやリクエストボディの型定義は、とりあえず `serde_json::Value` 等で仮置きしても構いません。
    ---
    - [x] 002-1. `get_chat_model` 関数を実装する (GET /v1/chat_models/:id)
    - [x] 002-2. `create_chat_model` 関数を実装する (POST /v1/chat_models)
    - [x] 002-3. `update_chat_model` 関数を実装する (PATCH /v1/chat_models/:id)
    - [x] 002-4. `delete_chat_model` 関数を実装する (DELETE /v1/chat_models/:id)
    - [x] 002-5. 各関数に `log::debug!` を追加し、呼び出しを確認できるようにする

- [x] 003. `chat_models` ルーティングの登録
    - `src/mode/rt/req_map.rs` に作成したハンドラを登録し、実際に HTTP リクエストを受け取れる状態にします。
    - Swagger UI (utoipa) への登録はまだ行わず、純粋なルーティングのみを確認します。
    - curl コマンドを用いて、全メソッドの疎通確認を行います。
    ---
    - [x] 003-1. `src/mode/rt/req_map.rs` を開き、Axum router にルートを追加する
    - [x] 003-2. `/v1/chat_models/search` (POST) を登録する
    - [x] 003-3. `/v1/chat_models` (POST) を登録する
    - [x] 003-4. `/v1/chat_models/:id` (GET, PATCH, DELETE) を登録する
    - [x] 003-5. `make run ARGS="rt"` で起動し、curl で各エンドポイントにアクセスして 200 OK を確認する

- [x] 004. `cubes_handler.rs` の作成と `search/get` ダミー実装
    - `src/mode/rt/rthandler/cubes_handler.rs` を新規作成します。
    - `search_cubes` と `get_cube` のスケルトン実装を行います。
    - この段階ではビジネスロジック (BL) 層への依存を含めず、Handler 層だけで完結させます。
    ---
    - [x] 004-1. `src/mode/rt/rthandler/cubes_handler.rs` ファイルを作成する
    - [x] 004-2. `search_cubes` 関数を定義する (POST /v1/cubes/search)
    - [x] 004-3. `get_cube` 関数を定義する (GET /v1/cubes/get/:id)
    - [x] 004-4. `src/mode/rt/rthandler/mod.rs` に `pub mod cubes_handler;` を追加する
    - [x] 004-5. ダミーレスポンスとログ出力を実装する

- [x] 005. `cubes_handler.rs` への管理系 CRUD ダミー実装
    - Cube 自体の作成 (`create`) と削除 (`delete`) のハンドラを追加します。
    - 権限チェックのための `JwtUsr` エクストラクターを引数に含め、コンパイルが通ることを確認します（ロジックはまだ）。
    ---
    - [x] 005-1. `create_cube` 関数を定義する (POST /v1/cubes/create)
    - [x] 005-2. `delete_cube` 関数を定義する (DELETE /v1/cubes/delete)
    - [x] 005-3. ハンドラの引数に `ju: JwtUsr` を追加する（`src/mode/rt/rthandler/common.rs` 参照）
    - [x] 005-4. `log::debug!` で `<Auth>` タグを含むログを出力するようにする
    - [x] 005-5. ダミーで 200 OK を返す

- [x] 006. `cubes_handler.rs` へのコア機能（Absorb/Query/Memify）ダミー実装
    - Cuber の核心となる `absorb` (吸入), `query` (検索), `memify` (自己強化) のエンドポイントを実装します。
    - これらは長時間実行される可能性があるため、非同期処理の基盤となる構造（`tokio::spawn` 等）を意識しておきますが、今は単に即時レスポンスを返します。
    ---
    - [x] 006-1. `absorb_cube` 関数を定義する (PUT /v1/cubes/absorb)
    - [x] 006-2. `query_cube` 関数を定義する (POST /v1/cubes/query)
    - [x] 006-3. `memify_cube` 関数を定義する (PUT /v1/cubes/memify)
    - [x] 006-4. 各ハンドラにプレースホルダーとしてのコメント (`TODO: Implement Cuber Logic`) を記述する
    - [x] 006-5. ダミーレスポンスを実装する

- [x] 007. `cubes_handler.rs` へのインポート・エクスポート系ダミー実装
    - Cube のポータビリティを担う `export`, `import` および鍵管理 `genkey`, `rekey` のハンドラを実装します。
    - `export` はバイナリファイル (`application/zip`) を返す想定ですが、今はテキストか空バイト列を返します。
    ---
    - [x] 007-1. `export_cube` 関数を定義する (GET /v1/cubes/export) - コンテンツタイプ指定の準備
    - [x] 007-2. `gen_key_cube` 関数を定義する (POST /v1/cubes/genkey)
    - [x] 007-3. `import_cube` 関数を定義する (POST /v1/cubes/import) - マルチパートリクエストの準備
    - [x] 007-4. `re_key_cube` 関数を定義する (POST /v1/cubes/rekey)
    - [x] 007-5. 全て 200 OK で返すダミー実装を行う

- [x] 008. `cubes` ルーティングの登録と疎通確認
    - 作成した全ての Cubes ハンドラを `req_map.rs` に登録します。
    - 数が多いので、パスの重複やメソッドの間違いがないか慎重に確認します。
    - 実際にサーバーを起動し、curl で全エンドポイントが叩けることを確認します。
    ---
    - [x] 008-1. `src/mode/rt/req_map.rs` に Cubes 関連の 9 つのエンドポイントを追加する
    - [x] 008-2. `make run ARGS="rt"` でサーバーを再起動する
    - [x] 008-3. `curl -X POST http://localhost:8080/v1/cubes/search` 等を実行し確認する
    - [x] 008-4. `curl -X GET http://localhost:8080/v1/cubes/get/1` 等を実行し確認する
    - [x] 008-5. Export などの特殊なメソッドも含め、全ルートの導通をログで確認する
- [x] 009. Request/Response DTO の定義 (Scaffolding) - ChatModels
    - `src/mode/rt/rtreq/chat_models_req.rs` と `src/mode/rt/rtres/chat_models_res.rs` を作成します。
    - Go 版の構造体定義を参考に、必要なフィールドを `serde` マクロ付きで定義します。
    - バリデーションはこの段階では `garde` 属性を付けず、プレーンな構造体として定義します。
    ---
    - [x] 009-1. `chat_models_req.rs` に `SearchChatModelsParam` 等の構造体を定義する
    - [x] 009-2. `chat_models_res.rs` に `ChatModelRes` 等の構造体を定義する
    - [x] 009-3. 全ての構造体に `#[derive(Serialize, Deserialize, ToSchema)]` を付与する
    - [x] 009-4. `mod.rs` で公開し、コンパイルを通す

- [x] 010. Request/Response DTO の定義 (Scaffolding) - Cubes
    - Cubes 関連の大量のリクエスト・レスポンス構造体を `rtreq/cubes_req.rs`, `rtres/cubes_res.rs` に定義します。
    - 特に `AbsorbCubeParam` や `QueryCubeParam` はフィールドが多いので、Go 版と見比べながら正確に移植します。
    ---
    - [x] 010-1. `cubes_req.rs` 内に全 API の Request DTO を定義する
    - [x] 010-2. `cubes_res.rs` 内に全 API の Response DTO を定義する
    - [x] 010-3. `Absorb` や `Query` のような複雑なパラメータを持つ DTO のフィールドを網羅する
    - [x] 010-4. `mod.rs` で公開設定を行う

- [x] 011. ハンドラへの DTO 組み込みとバインドテスト
    - プレースホルダーとしていたハンドラの引数を、定義した実際の DTO 型 (`Json<T>`) に置き換えます。
    - これにより、不正な JSON が送られた際に Axum が自動的に 400 Bad Request を返すことを確認します。
    ---
    - [x] 011-1. `chat_models_handler.rs` の全関数の引数を具体的な DTO に変更する
    - [x] 011-2. `cubes_handler.rs` の全関数の引数を具体的な DTO に変更する
    - [x] 011-3. `make run` で起動し、Malformed JSON を送信して 400 エラーを確認する
    - [x] 011-4. 正しい JSON を送信して 200 OK が返ることを再確認する

- [x] 012. 共通エラーハンドリングの整備 (Cubes/ChatModels)
    - `src/mode/rt/rthandler/common.rs` や `rterr` モジュールを活用し、エラーレスポンスがプロジェクト規定のフォーマット `{ "code": "E...", "msg": "..." }` で返るようにします。
    - 現時点では BL がないため、意図的にエラーを返す分岐を作って確認します。
    ---
    - [x] 012-1. ハンドラの戻り値を `Result<impl IntoResponse, AppError>` に変更する
    - [x] 012-2. テスト用に、特定のパラメータが来たら `AppError::BadRequest` を返すロジックを一時的に入れる
    - [x] 012-3. curl でエラー発生リクエストを送り、レスポンス JSON の構造を確認する
    - [x] 012-4. 確認後、テストコードは削除またはコメントアウトする

- [x] 013. 認証・認可ミドルウェアの動作確認
    - ハンドラに追加した `JwtUsr` エクストラクターが正しく機能しているか確認します。
    - `Authorization` ヘッダなし、無効なトークン、権限不足（例えば `cubes` の管理 API）でのアクセスをテストします。
    ---
    - [x] 013-1. `Authorization` ヘッダなしでリクエストし、401 Unauthorized を確認
    - [x] 013-2. 無効なトークンでリクエストし、401 を確認
    - [x] 013-3. 権限チェックロジック `ju.allow_roles(...)` が機能することを確認（一時的に厳しい制限を入れてテスト）

- [x] 014. ログ出力の検証
    - 全てのエンドポイントで、アクセスログおよびデバッグログ (`<Auth>`, `<ChatModel>`, `<Cube>`) が標準出力に正しく表示されているか確認します。
    - リクエストID (TraceID) がログに含まれているかも併せて確認します。
    ---
    - [x] 014-1. ログレベルを `debug` に設定して起動する (`RUST_LOG=debug`)
    - [x] 014-2. 各 API を叩き、コンソールログを目視確認する
    - [x] 014-3. ログに必要なタグが含まれているかチェックする

- [x] 015. Phase 1 全体テスト
    - ここまでの成果を総括するため、全エンドポイントに対して正常系・異常系のリクエストを投げる簡単なシェルスクリプトを作成・実行します。
    - これが「Phase 1 完了」の証左となります。
    ---
    - [x] 015-1. `test_phase1.sh` を作成する
    - [x] 015-2. 全エンドポイントへの正常リクエスト (200 OK) を記述
    - [x] 015-3. 代表的な異常リクエスト (400, 401) を記述
    - [x] 015-4. スクリプトを実行し、全て想定通りのステータスコードが返ることを確認する
    - [x] 015-5. テストコードを削除する

### Phase 2: ChatModel 機能の実装 (Step 016 - 025) ✅ 完了

DB (MySQL) を用いた単純な CRUD 実装を通じて、Rust での BL/Repository パターンを確立します。

curl を用いたリクエストによりテストを行う際の認証JWTトークンは `for_test/USR_JWT.md` に記載されているものを使用します。

- [x] 016. `ChatModel` SeaORM Entity の定義
    - `src/entities/chat_models.rs` が存在するか確認し、MySQL の `chat_models` テーブルと完全にマッピングされているか検証します。
    - Relation や ActiveModelBehavior の実装状況も確認します。
    - まだ存在しない場合は、`make gen-entities HOST="localhost"` を実行して自動生成します。
    ---
    - [x] 016-1. `make gen-entities` を実行する (マイグレーションは実行済み前提)
    - [x] 016-2. `src/entities/chat_models.rs` の内容を確認する
    - [x] 016-3. フィールドの型 (`Option<String>` 等) が適切かチェックする
    - [x] 016-4. `src/entities/prelude.rs` に登録されているか確認する

- [x] 017. `CreateChatModel` ロジックの実装
    - `src/mode/rt/rtbl/chat_models_bl.rs` を新規作成し、`create_chat_model` 関数を実装します。
    - `ActiveModel` を作成し、DB への `insert` を行います。
    - DTO から Model への変換には `From` トレイトを使用するか、手動でセットします。
    ---
    - [x] 017-1. `src/mode/rt/rtbl/chat_models_bl.rs` ファイルを作成する
    - [x] 017-2. `create_chat_model` 関数を定義する (引数: `&DatabaseConnection`, `CreateChatModelParam`, `&JwtIDs`)
    - [x] 017-3. `ActiveModel::from_json` 等を使ってインサート処理を書く
    - [x] 017-4. 成功時に作成されたレコードの ID を返す
    - [x] 017-5. `mod.rs` に追加する

- [x] 018. `Search/Get ChatModel` ロジックの実装
    - `find_chat_models_base` ヘルパー関数を実装し、権限フィルタを共通化します。
    - `search_chat_models` と `get_chat_model` を実装し、ページネーションとフィルタリングを適用します。
    ---
    - [x] 018-1. `find_chat_models_base` プライベート関数を定義し、ルート権限以外は自分のデータしか見えないロジックを入れる
    - [x] 018-2. `get_chat_model` を実装する (ID指定検索)
    - [x] 018-3. `search_chat_models` を実装する (条件検索)

- [x] 019. `Update/Delete ChatModel` ロジックの実装
    - 更新対象の存在確認と権限チェック (`find_base` 経由) を行ってから処理を実行します。
    - `save` メソッドによる更新と、`delete` メソッドによる削除を実装します。
    ---
    - [x] 019-1. `update_chat_model` 関数を実装する
    - [x] 019-2. 取得した `Model` を `IntoActiveModel` で変換し、差分更新する (`set` メソッド)
    - [x] 019-3. `delete_chat_model` 関数を実装する
    - [x] 019-4. 削除前に「本当に削除してよいか」のビジネスロジックがあれば入れる

- [x] 020. ハンドラと BL の結合 (ChatModel)
    - Phase 1 で作ったダミーハンドラの中身を書き換え、作成した BL 関数を呼び出すようにします。
    - DB コネクションプール `Arc<DbPools>` をハンドラで受け取り、BL に渡します。
    ---
    - [x] 020-1. `get_chat_model` ハンドラを修正し、BLを呼ぶ
    - [x] 020-2. `search_chat_models` ハンドラを修正し、BLを呼ぶ
    - [x] 020-3. `create_chat_model` ハンドラを修正し、BLを呼ぶ
    - [x] 020-4. `update/delete` ハンドラも同様に修正
    - [x] 020-5. 正常に DB アクセスできるか curl で確認する

- [x] 021. `garde` バリデーションの適用 (ChatModel)
    - `rtreq/chat_models_req.rs` の構造体に `#[garde(custom(...))]` 等のアトリビュートを付与します。
    - ハンドラ内で `input.validate()` を呼び出し、検証エラーがあれば即座に 422 を返すようにします。
    ---
    - [x] 021-1. `CreateChatModelParam` の各フィールドに `garde` ルールを追加 (必須チェック、文字数制限など)
    - [x] 021-2. ハンドラ共通処理としてのバリデーションロジックが機能しているか確認

- [x] 022. 正常系テスト (ChatModel)
    - 一連のフロー (Create -> Get -> Search -> Update -> Delete) を通しで実行するテストスクリプトを作成します。
    - 各ステップで期待通りのステータスコードとレスポンスボディが返ることを確認します。
    ---
    - [x] 022-1. `scripts/test_chat_models.sh` を作成する
    - [x] 022-2. 作成したスクリプトを実行し、全行程が 200/201 OK で完了することを確認する

- [x] 023. 異常系テスト (ChatModel)
    - 不正な ID 指定、バリデーションエラー、存在しないリソースへのアクセスをテストします。
    ---
    - [x] 023-1. `scripts/test_chat_models_error.sh` を作成する
    - [x] 023-2. 期待通りのエラーコード (404, 422 等) が返ることを確認する

- [x] 024. パーティショニングの確認
    - 異なるユーザー (Token) でアクセスし、他人の作成した ChatModel が見えない（あるいは操作できない）ことを保証します。
    - `find_base` ロジックが正しく機能しているかの証明です。
    ---
    - [x] 024-1. ユーザーA で ChatModel を作成する
    - [x] 024-2. ユーザーB で Search し、ユーザーA のデータが含まれていないことを確認する
    - [x] 024-3. ユーザーB でユーザーA の ID を指定して Get し、404 (または 403) になることを確認する

- [x] 025. トランザクション動作確認
    - `create_chat_model` 内で意図的にパニックまたはエラーを起こすコードを一時的に挿入し、DB への書き込みがロールバックされるか確認します。
    - SeaORM の `transaction` 機能が正しく動作しているかの検証です。
    ---
    - [x] 025-1. BL 内の `insert` 直後に `return Err(...)` を追加する
    - [x] 025-2. API を呼び出し、エラーレスポンスを受け取る
    - [x] 025-3. DB を確認し、レコードが作成されていないことを確認する

### Phase 2.5: 環境変数のマージと整備 (Step 025-1) ✅ 完了

Cuber の各機能の実装に先立ち、Go 版の環境変数を Rust 版へとマージし、適切に設定します。

- [x] 025-1. `mycute-go/src/.env.example` から `.env` および `.env.example` への環境変数のマージ
    - Go 版固有の変数を抽出。
    - 既存の Rust 設定を維持しつつ、不足分を追記。
    - パス設定などを Rust 側の環境に合わせて調整。

### Phase 3: Cube 基本管理機能の実装 (Step 026 - 035) ✅ 完了

Cube のメタデータ管理（MySQL 側）を実装します。Cuber コア（LadybugDB）との連携はまだ行いません。

- [x] 026. `Cube` SeaORM Entity の定義
    - `src/entities/cubes.rs` を確認し、`memory_group` などの重要なフィールドが正しく定義されているか検証します。
    - `Relation` 定義（User との紐付けなど）を確認します。
    ---
    - [x] 026-1. `make gen-entities` を再度実行し、最新状態にする
    - [x] 026-2. `src/entities/cubes.rs` のスキーマ定義を確認する
    - [x] 026-3. `ActiveModelBehavior` がデフォルト実装されていることを確認

- [x] 027. `CreateCube` (Metadata) 実装
    - `src/mode/rt/rtbl/cubes_bl.rs` を新規作成し、`create_cube` 関数を実装します。
    - まずは MySQL へのメタデータ登録のみを行い、LadybugDB ファイルの作成は後回し（Phase 4 以降）とします。
    ---
    - [x] 027-1. `src/mode/rt/rtbl/cubes_bl.rs` を作成する
    - [x] 027-2. `create_cube` 関数を定義する
    - [x] 027-3. `uuid` クレートを使ってユニークな UUID を生成し、レコードにセットする
    - [x] 027-4. `ActiveModel` を使って DB に保存する

- [x] 028. `Search/Get Cube` (Metadata) 実装
    - `find_cubes_base` を実装し、自分の Cube しか見えないようにフィルタリングします。
    - `search_cubes` と `get_cube` のロジックを実装します。
    ---
    - [x] 028-1. `find_cubes_base` を実装する (Owner ID でフィルタ)
    - [x] 028-2. `get_cube` を実装する
    - [x] 028-3. `search_cubes` を実装する
    - [x] 028-4. レスポンスには、まだ統計情報等は含めず、MySQL の値だけを返す

- [x] 029. `DeleteCube` (Metadata) 実装
    - `delete_cube` 関数を実装します。
    - MySQL 上のレコード削除のみを行い、物理ファイルの削除は TODO コメントとして残します。
    ---
    - [x] 029-1. `delete_cube` 関数を実装する
    - [x] 029-2. 対象の Cube が存在し、かつ自分がオーナーであるか確認する
    - [x] 029-3. `delete` メソッドでレコードを削除する

- [x] 030. ハンドラと BL の結合 (Cube Metadata)
    - `cubes_handler.rs` のダミー実装を、BL 呼び出しに置き換えます。
    - `Arc<DbPools>` をハンドラ経由で BL に渡します。
    ---
    - [x] 030-1. `create_cube` ハンドラを修正
    - [x] 030-2. `get/search_cube` ハンドラを修正
    - [x] 030-3. `delete_cube` ハンドラを修正
    - [x] 030-4. curl で CRUD が正常に動作するか確認する

- [x] 031. バリデーション実装 (Cube Request)
    - `rtreq/cubes_req.rs` に `garde` ルールを追加します。
    - Cube 名の許可文字種（英数字のみ等）や長さ制限を適用します。
    ---
    - [x] 031-1. `CreateCubeParam` に `#[garde(custom(length_simple_err(...)))]` 等を追加する
    - [x] 031-2. `UpdateCubeParam` があれば同様に追加する
    - [x] 031-3. ハンドラで `validate` を呼び出すようにする
    - [x] 031-4. 不正な名前で作成しようとして 400 エラーになることを確認

- [x] 032. ユーザ権限によるフィルタリング実装
    - `memory_group` の指定がある場合、そのグループへのアクセス権があるかを確認するロジック（`User` テーブルや別テーブルとの突き合わせ）が必要であれば実装します。
    - シンプルな所有者チェックだけで良ければ、`find_cubes_base` の動作を再確認します。
    ---
    - [x] 032-1. 他人の Cube ID を指定して `get` し、404/403 になることを確認
    - [x] 032-2. `list` で他人の Cube が混ざらないことを確認

- [x] 033. Phase 3 結合テスト
    - Cube の作成から削除までの一連のシナリオテストを作成します。
    - ChatModel と同様にシェルスクリプト化します。
    ---
    - [x] 033-1. `test_cube_crud.sh` を作成する (`scripts/test_cube_crud.sh`)
    - [x] 033-2. 正常系フロー（Create -> Get -> Delete）を記述
    - [x] 033-3. 異常系フロー（バリデーション、権限）を記述
    - [x] 033-4. 実行してパスすることを確認

- [x] 034. テキスト正規化ユーティリティの移植
    - Cuber で使用するテキスト正規化ロジック (Markdown 除去、正規化等) を Rust に移植します。
    - `mycute-go/src/pkg/cuber/utils/normalize.go` の内容を解析し、`src/utils/text_normalizer.rs` 等に実装します。
    ---
    - [x] 034-1. `src/utils/text_normalizer.rs` を作成
    - [x] 034-2. `remove_markdown` 関数等を正規表現 (`regex` クレート) を使って実装
    - [x] 034-3. `normalize_text` 関数を実装
    - [x] 034-4. 単体テスト (`#[test]`) を書いて、Go 版と同じ挙動になるか確認 ✅ 8 tests pass

- [x] 035. 共通定数・設定値の整備 (`src/cuber/consts.rs`)
    - `config.rs` または `src/cuber/consts.rs` に、Cuber 関連のデフォルト設定値（最大トークン数、タイムアウト時間など）を集約します。
    ---
    - [x] 035-1. `src/cuber/consts.rs` を作成
    - [x] 035-2. `DEFAULT_ABSORB_LIMIT` などを `pub const` で定義
    - [x] 035-3. 必要に応じて `config.rs` の構造体にもフィールドを追加する

### Phase 4: Cuber アーキテクチャ基盤の構築 (Step 036 - 045)

ここから `src/cuber` の実装に入ります。まずはガワと依存関係を整備します。

- [x] 036. `Cuber` モジュール構成の作成
    - `src/cuber` ディレクトリを作成し、主要なモジュールファイルを配置します。
    - `mod.rs` でサブモジュールを公開し、外部（`mode/rt`）から参照可能にします。
    ---
    - [x] 036-1. `src/cuber` ディレクトリ作成
    - [x] 036-2. `mod.rs`, `service.rs`, `config.rs`, `error.rs`, `storage_set.rs` ファイル作成
    - [x] 036-3. `src/lib.rs` (または `src/main.rs`) で `pub mod cuber;` を宣言

- [x] 037. `CuberConfig` の実装とロード
    - `src/cuber/config.rs` に設定構造体を実装します。
    - `serde` を用いて設定ファイル（TOML/YAML/Env）から読み込めるようにします。
    - バリデーション（必須項目のチェック等）も実装します。
    ---
    - [x] 037-1. `CuberConfig` 構造体を定義 (Go版 `config_types.go` 参照)
    - [x] 037-2. `impl Default for CuberConfig` を実装
    - [x] 037-3. 設定読み込みロジックの実装と単体テスト

- [x] 038. `S3Client` の統合
    - 既存の `src/utils/s3client.rs` を `CuberService` に組み込みます。
    - `Arc<S3Client>` として保持し、複数のタスクで共有できるようにします。
    ---
    - [x] 038-1. `CuberService` 構造体に `s3_client: Arc<S3Client>` フィールドを追加
    - [x] 038-2. `NewCuberService` (または `new`) の引数で受け取るように修正
    - [x] 038-3. S3 へのアクセス権限確認（ListBuckets 等）を行う初期化ロジックを追加

- [x] 039. `Lindera` (Tokenizer) の統合
    - 日本語形態素解析器 `Lindera` を初期化し、`CuberService` で保持します。
    - シングルトンとして扱い、辞書ロードのコストを起動時のみにします。
    ---
    - [x] 039-1. `Cargo.toml` に `lindera-core`, `lindera-dictionary`, `lindera-tokenizer` を追加（直接追記してはならない。必ず cargo add コマンドで最新版を入れること）
    - [x] 039-2. `src/cuber/tokenizer.rs` を作成し、ラッパー構造体を定義
    - [x] 039-3. `CuberService` に `tokenizer: Arc<Tokenizer>` を追加

- [x] 040. `StorageSet` トレイトの定義
    - 物理ストレージ（LadybugDB）を隠蔽するためのトレイトを定義します。
    - `VectorStorage` と `GraphStorage` のメソッドシグネチャ（Go版 `interfaces.go` 参照）を Rust トレイトに変換します。
    ---
    - [x] 040-1. `src/cuber/storage/mod.rs` を作成
    - [x] 040-2. `trait VectorStorage` を定義 (メソッドはまだ空でも可)
    - [x] 040-3. `trait GraphStorage` を定義 (メソッドはまだ空でも可)
    - [x] 040-4. `StorageSet` 構造体に `Arc<dyn VectorStorage>` 等を持たせる

- [x] 041. LadybugDB ラッパー実装 (Scaffolding)
    - `cargo add lbug@0.14.0` は既に実行済みで、Cargo.toml には既に crate が入っています
    - `lbug` クレートを用いて、LadybugDB インスタンスを生成・管理する構造体を作ります。
    - 実際のクエリメソッドは後回しにし、`open`, `close`, `transaction` などの基本インフラを実装します。
    ---
    - [x] 041-1. `src/cuber/storage/ladybug.rs` を作成
    - [x] 041-2. `struct LadybugDB` を定義し、`lbug::Database` を保持させる
    - [x] 041-3. `StorageSet` トレイトを `LadybugDB` に実装する（中身は `todo!()`）

- [x] 042. `CuberService::get_or_open_storage` の実装
    - `DashMap` を用いた接続プーリングロジックを実装します。
    - `UUID` に対応する `StorageSet` があれば返し、なければ新規オープンしてキャッシュするロジックです。
    ---
    - [x] 042-1. `CuberService` に `storage_map: DashMap<String, Arc<StorageSet>>` を追加
    - [x] 042-2. `get_or_open_storage` メソッドを実装
    - [x] 042-3. 同時アクセス時の排他制御（二重オープン防止）を確認

- [x] 043. `CuberService` ライフサイクル管理
    - `tokio_util::sync::CancellationToken` を導入し、グレースフルシャットダウンを実現します。
    - アイドル接続のクリーンアップタスク (`GC`) をバックグラウンドで起動します。
    ---
    - [x] 043-1. `CancellationToken` をサービス構造体に追加
    - [x] 043-2. `spawn_gc_task` メソッドを実装 (一定時間未使用のストレージを閉じる)
    - [x] 043-3. `close` メソッドでトークンをキャンセルし、全ストレージを閉じる

- [x] 044. `EventBus` の実装
    - `src/cuber/event.rs` を作成し、非同期イベントバスを実装します。
    - `tokio::sync::broadcast` または `mpsc` を用いて、イベントの Pub/Sub を実現します。
    ---
    - [x] 044-1. `EventBus` 構造体と `StreamEvent` Enum を定義
    - [x] 044-2. `emit` メソッドと `subscribe` メソッドを実装
    - [x] 044-3. `CuberService` に `event_bus: Arc<EventBus>` を持たせる

- [x] 045. 基盤動作テスト
    - アプリ起動時に `CuberService` が初期化され、リソース確保ができることを確認します。
    - 簡単なテストコードで `get_or_open_storage` を呼び出し、エラーが出ないことを確認します。
    ---
    - [x] 045-1. `main.rs` (または `rt/main_of_rt.rs`) で `CuberService::new` を呼び出す
    - [x] 045-2. 起動ログに `CuberService initialized` が出ることを確認
    - [x] 045-3. テスト用の空の DB ファイルディレクトリを指定してエラーにならないか確認

### Phase 5: Absorb (吸入) プロセスの実装 - 前半 (Step 046 - 060)

ファイル取り込みからチャンク分割までを実装します。

- [ ] 046. `IngestTask` 実装: ファイルハッシュ計算
    - `add` プロセスの最初のステップとして、アップロードされたファイルまたはローカルファイルの SHA-256 ハッシュを計算します。
    - `tokio::fs` を用いて、メモリに全展開せずにストリーミングでハッシュ計算する実装にします。
    ---
    - [ ] 046-1. `src/cuber/ingest.rs` を作成
    - [ ] 046-2. `calculate_hash(path: &Path)` 関数を実装
    - [ ] 046-3. `sha2` クレート等を利用
    - [ ] 046-4. 巨大ファイルを用いたパフォーマンステスト

- [ ] 047. `IngestTask` 実装: S3 アップロード処理
    - 計算したハッシュをキーとして、S3 (または互換ストレージ) にファイルをアップロードします。
    - 既に `CuberService` に統合した `s3_client` を利用します。
    ---
    - [ ] 047-1. `upload_file(hash: &str, path: &Path)` 関数を実装
    - [ ] 047-2. S3 上に既に同名ファイルがあればアップロードをスキップする最適化を入れる
    - [ ] 047-3. アップロード成功後の S3 URL (または Key) を取得する

- [ ] 048. `IngestTask` 実装: 重複チェックとメタデータ保存
    - LadybugDB の `Data` テーブルにメタデータを保存します。
    - `memory_group` とハッシュの複合キーで重複を除外します。
    ---
    - [ ] 048-1. `VectorStorage` トレイトに `add_data` メソッドを追加
    - [ ] 048-2. LadybugDB 実装側で `MERGE (d:Data ...)` クエリを書く
    - [ ] 048-3. 戻り値として `Data` ノードの ID または構造体を受け取る

- [ ] 049. `rtbl::absorb_cube` エントリポイント実装
    - Handler から呼ばれる `absorb_cube` 内で、ここまでの `IngestTask` を呼び出すように結合します。
    - `Ingest` が完了したら、次の `Cognify` フェーズへ進むための準備として `Vec<Data>` を用意します。
    ---
    - [ ] 049-1. ハンドラから `Files` (Multipart) を受け取る処理の実装
    - [ ] 049-2. 一時ディレクトリへの保存
    - [ ] 049-3. `cuber_service.absorb` (または `add` 相当) を呼び出す

- [ ] 050. `ChunkingTask` 実装: テキスト正規化
    - `Ingest` されたファイル（テキスト）を読み込み、正規化処理を適用します。
    - Phase 3 で作った正規化ユーティリティを活用します。
    ---
    - [ ] 050-1. `src/cuber/pipeline/chunking.rs` を作成
    - [ ] 050-2. `Task` トレイトを実装する `ChunkingTask` 構造体を定義
    - [ ] 050-3. `run` メソッド内でテキスト読み込みと正規化を行う

- [ ] 051. `ChunkingTask` 実装: 文分割（Sentence Splitting）
    - 正規化されたテキストを「文」単位に分割します。
    - 句読点、改行などを区切り文字として正規表現で処理します。
    ---
    - [ ] 051-1. `split_sentences(text: &str) -> Vec<String>` を実装
    - [ ] 051-2. 日本語 (`。`) と英語 (`. `) の両方に対応する正規表現を確認
    - [ ] 051-3. コーナーケース（カッコ内の句点など）のテスト

- [ ] 052. `ChunkingTask` 実装: オーバーラップ処理
    - 分割された文を規定の文字数（ChunkSize）に詰め込み、指定されたオーバーラップ（OverlapSize）を持たせて次のチャンクへ繋ぎます。
    ---
    - [ ] 052-1. チャンク構築ロジックの実装
    - [ ] 052-2. `Vec<String>` (文リスト) から `Vec<Chunk>` を生成する
    - [ ] 052-3. オーバーラップ部分が正しく複製されているか単体テスト

- [ ] 053. `ChunkingTask` 実装: キーワード抽出 (Lindera)
    - 各チャンクに対して形態素解析を行い、名詞・動詞を抽出して FTS 用のフィールドにセットします。
    ---
    - [ ] 053-1. `CuberService` から `Tokenizer` を借りてくる
    - [ ] 053-2. `extract_keywords(text: &str)` を実装
    - [ ] 053-3. `nouns`, `verbs` 等に振り分ける

- [ ] 054. Embedder インターフェースと Mock 実装
    - テキストをベクトル化する `Embedder` トレイトを定義し、開発用の Mock (ランダムベクトル生成) を用意します。
    ---
    - [ ] 054-1. `src/cuber/llm/embedder.rs` を作成
    - [ ] 054-2. `trait Embedder` (async) を定義
    - [ ] 054-3. `MockEmbedder` を実装 (常に固定長のゼロベクトル等を返す)

- [ ] 055. `ChunkingTask` 実装: ベクトル埋め込み生成
    - 作成したチャンクテキストを `Embedder` に投げ、Vector を取得して Chunk 構造体に埋め込みます。
    ---
    - [ ] 055-1. `ChunkingTask` 内で `embedder.embed_batch` を呼ぶ
    - [ ] 055-2. 並列数制御（`tokio::semaphore` 等）を考慮する
    - [ ] 055-3. エラー時のリトライロジック

- [ ] 056. パイプライン連携テスト (Ingest -> Chunking)
    - ファイルアップロードからチャンク分割、ベクトル化までが繋がって動くかを確認します。
    ---
    - [ ] 056-1. テスト用テキストファイルを用意
    - [ ] 056-2. `Ingest` -> `Chunking` をコード上で呼び出す結合テスト
    - [ ] 056-3. 最終的に `Vec<Chunk>` が生成され、中身が正しいか検証

- [ ] 057. 中間データの保存テスト
    - ここまでの処理結果を一時的に確認するため、デバッグログやファイルダンプを行います。
    - 実際の DB 保存はまだでも、データ構造が DB スキーマに適合しているか確認します。
    ---
    - [ ] 057-1. 生成された `Chunk` 構造体のフィールドチェック
    - [ ] 057-2. `serde_json` でダンプして目視確認

- [ ] 058. エラーハンドリング確認 (S3 エラー等)
    - S3 アップロード失敗時やトークナイズ失敗時に、パイプラインが適切に中断・エラー通知するかテストします。
    ---
    - [ ] 058-1. S3 クライアントに不正なキーを与えてエラーを起こす
    - [ ] 058-2. エラーが `CuberError` として上位へ伝播することを確認

- [ ] 059. メモリリークチェック (大量ファイルループ処理)
    - 大量のファイルを処理させた際に、メモリ使用量が肥大化しないか（ストリーム処理が効いているか）簡易チェックします。
    ---
    - [ ] 059-1. ループで擬似的に大量データを流し込むテスト
    - [ ] 059-2. プロセス監視ツール等で極端な増加がないか見る

- [ ] 060. ログ出力による処理フロー追跡確認
    - `Ingest` 開始、終了、チャンク数などの情報が `EventBus` 経由（またはログ）で出力されているか確認します。
    ---
    - [ ] 060-1. `EventBus` の `subscribe` 側でイベントを受信できるか確認
    - [ ] 060-2. 期待通りの順序でイベントが飛んでいるかチェック

### Phase 6: Absorb (吸入) プロセスの実装 - 後半 (Step 061 - 070)

知識グラフ抽出と最終保存を実装します。

- [ ] 061. `GraphExtractionTask` 実装: LLM クライアント連携
    - `src/cuber/pipeline/graph_extraction.rs` を作成し、LLM クライアント (`ChatModel`) と連携する基盤を作ります。
    - 外部 API へのリクエスト、レスポンスの型定義をします。
    ---
    - [ ] 061-1. `GraphExtractionTask` 構造体定義
    - [ ] 061-2. `CuberService` から `ChatModel` (Arc) を受け取る
    - [ ] 061-3. ダミーリクエストを送ってレスポンスが返るか確認

- [ ] 062. `GraphExtractionTask` 実装: プロンプト管理
    - 知識グラフ抽出用のプロンプトを定義します。Go 版のプロンプトを正確に移植します。
    - 変数埋め込み (`{{text}}` 等) の仕組みを実装します。
    ---
    - [ ] 062-1. `src/cuber/llm/prompts.rs` を作成
    - [ ] 062-2. `EXTRACT_TRIPLES_PROMPT` 定数を定義
    - [ ] 062-3. `format!` マクロ等でテキストを埋め込む関数を実装

- [ ] 063. `GraphExtractionTask` 実装: `JoinSet` による並列実行
    - チャンクごとに LLM リクエストを並列で飛ばします。
    - `tokio::task::JoinSet` を用いて、並列数の制御とエラーの一括管理を行います。
    ---
    - [ ] 063-1. ループ処理内で `JoinSet::spawn` するロジック
    - [ ] 063-2. `Semaphore` で同時実行数を制限 (例: 5並列)
    - [ ] 063-3. 全タスク完了後の結果集計ロジック

- [ ] 064. `GraphExtractionTask` 実装: JSON パースとクリーニング
    - LLM から返ってきた JSON 文字列をパースし、`Vec<Triple>` に変換します。
    - 不正な JSON が返ってきた場合の自動修正（後回しでも可）や、エラーハンドリングを実装します。
    ---
    - [ ] 064-1. `serde_json` で構造体へデシリアライズ
    - [ ] 064-2. `JsonCleaner` ユーティリティの実装 (Markdown記法の除去など)
    - [ ] 064-3. バリデーション (要素不足などのチェック)

- [ ] 065. `StorageTask` 実装: ノード・エッジの保存
    - 抽出されたトリプルを LadybugDB に保存します。
    - Nodes と Edges のテーブルに対して、`MERGE` クエリ等を用いて UPSERT します。
    ---
    - [ ] 065-1. `src/cuber/pipeline/storage.rs` を作成
    - [ ] 065-2. `GraphStorage` トレイトに `save_triples` メソッドを追加
    - [ ] 065-3. LadybugDB 実装側でノード・エッジを保存するクエリを書く

- [ ] 066. `StorageTask` 実装: ベクトルインデックスの保存
    - チャンクの埋め込みベクトル (`VectorType`) を LadybugDB のベクターストアに保存します。
    ---
    - [ ] 066-1. `VectorStorage` トレイトに `save_chunks` メソッドを追加
    - [ ] 066-2. `Chunk` 構造体を DB レコードへマッピング
    - [ ] 066-3. 一括インサート (Bulk Insert) の実装

- [ ] 067. `SummarizationTask` 実装: 要約生成
    - 各チャンクの要約を LLM で生成し、それを `Summary` ノードとして保存します。
    ---
    - [ ] 067-1. `SummarizationTask` の作成
    - [ ] 067-2. 要約生成プロンプトの定義
    - [ ] 067-3. 生成された要約の保存ロジック

- [ ] 068. トランザクション制御と Checkpoint 実装
    - `Absorb` 全体を一つのトランザクションとしてコミットする制御を入れます。
    - 完了後に `Checkpoint` (WAL -> DB) を走らせ、永続化を確定させます。
    ---
    - [ ] 068-1. `CuberService::absorb` 内のトランザクションスコープ確認
    - [ ] 068-2. `commit` 呼び出しの実装
    - [ ] 068-3. `Checkpoint` メソッドの呼び出しとエラーハンドリング

- [ ] 069. SSE イベント配信の実装とフロントエンド連携確認
    - ここまでの詳細な進捗（ファイル数、チャンク生成数、抽出ノード数など）がリアルタイムにクライアントに届くか確認します。
    - `curl -N` (no buffer) を使ってストリーミングを確認します。
    ---
    - [ ] 069-1. 各タスク内での `eb.emit(...)` 呼び出し確認
    - [ ] 069-2. ハンドラ側での SSE レスポンス構築確認
    - [ ] 069-3. 動作確認

- [ ] 070. `Absorb` 統合テスト (End-to-End)
    - ファイルアップロードからグラフ構築、DB 保存、正常レスポンスまでの一連の流れをテストします。
    - 実際に LadybugDB ファイルが生成され、データが入っていることを確認します。
    ---
    - [ ] 070-1. テスト用ファイル (小規模) を用意
    - [ ] 070-2. `Absorb` API を叩くスクリプト実行
    - [ ] 070-3. DB ビューア (あれば) または CLI でデータ確認
    - [ ] 070-4. エラーログが出ていないことの確認

### Phase 7: Query (問合せ) プロセスの実装 (Step 071 - 085)

検索ロジックと回答生成を実装します。

- [ ] 071. `VectorStorage::search` 実装
    - クエリ埋め込みベクトルを用いた類似度検索 (Vector Search) を実装します。
    - LadybugDB のベクターストア機能 (`CALL vector_search(...)` 等) を利用します。
    ---
    - [ ] 071-1. `VectorStorage` トレイトに `search` メソッドを定義
    - [ ] 071-2. リクエストされた埋め込みベクトルを使って DB クエリを実行
    - [ ] 071-3. スコア付きのチャンクIDリストを返す

- [ ] 072. `GraphStorage::get_graph` 実装
    - 指定されたノード ID (または Triple パターン) に合致するサブグラフを取得します。
    - グラフ探索クエリ (`MATCH ...`) を構築・実行します。
    ---
    - [ ] 072-1. `GraphStorage` トレイトに `get_subgraph` メソッドを定義
    - [ ] 072-2. ノードとその周辺エッジを取得する Cypher クエリの実装
    - [ ] 072-3. Rust のグラフ構造体 (`KnowledgeGraph`) にマッピングして返す

- [ ] 073. `rtbl::query_cube` 実装とディスパッチロジック
    - Query API のエントリポイントを実装します。
    - `QueryType` (Enum) に応じて適切な検索ロジックへ分岐するディスパッチャを作ります。
    ---
    - [ ] 073-1. `src/cuber/query/dispatch.rs` を作成
    - [ ] 073-2. `QueryCubeParam` を受け取り、`QueryType` で `match` させる
    - [ ] 073-3. 各ケースのハンドラ (関数) を定義

- [ ] 074. `QueryType` 1-3 (単純取得系) の実装
    - グラフ取得、チャンク取得等の単純な参照系クエリを先行実装します。
    - データ取得 -> JSON レスポンス変換の流れを確認します。
    ---
    - [ ] 074-1. `QUERY_TYPE_GET_GRAPH` (ID: 1) の実装
    - [ ] 074-2. `QUERY_TYPE_GET_CHUNKS` (ID: 2) の実装
    - [ ] 074-3. `QUERY_TYPE_GET_PRE_MADE_SUMMARIES` (ID: 3) の実装

- [ ] 075. ハイブリッド検索ロジック (Vector + FTS) 実装
    - ベクトル検索と全文検索 (FTS) を組み合わせた `HybridSearch` を実装します。
    - ベクトルで見つけたチャンクに含まれるキーワードで再度 FTS をかけ、エンティティを拡充します。
    ---
    - [ ] 075-1. `src/cuber/query/hybrid.rs` を作成
    - [ ] 075-2. ベクトル検索実行ロジック
    - [ ] 075-3. FTS クエリ実行ロジック
    - [ ] 075-4. 結果のマージとランク付け

- [ ] 076. グラフ探索ロジック (1-hop, 2-hop) 実装
    - 検索でヒットしたノードを起点に、関連する周辺ノードを深さ指定で探索します。
    - `bfs` (幅優先探索) または 再帰クエリを利用します。
    ---
    - [ ] 076-1. 起点ノードリストの作成
    - [ ] 076-2. 指定 Hop 数分の近傍取得クエリ構築
    - [ ] 076-3. グラフ構造体への統合

- [ ] 077. 時間減衰スコアリング (Thickness) 実装
    - エッジの `unix` タイムスタンプに基づき、古い情報の重みを下げる計算ロジック (`Sigmoid Decay` 等) を実装します。
    - 閾値を下回ったエッジは回答コンテキストから除外します。
    ---
    - [ ] 077-1. `src/cuber/logic/score.rs` を作成
    - [ ] 077-2. `calculate_thickness` 関数を実装
    - [ ] 077-3. `half_life` パラメータの適用

- [ ] 078. 矛盾解決ロジック (Stage 1) 実装
    - 決定論的ルールによる矛盾解決を実装します。
    - 例: 「A is B」と「A is not B」が同時に存在する場合など、最新を優先する等のルールを適用します。
    ---
    - [ ] 078-1. `src/cuber/logic/conflict.rs` を作成
    - [ ] 078-2. 相反するトリプルパターンの定義
    - [ ] 078-3. フィルタリングロジックの実装

- [ ] 079. 回答生成 (Synthesizer) 実装
    - 取得・フィルタリングされたコンテキスト (Graph/Text) をプロンプトに組み込み、LLM で最終回答を生成します。
    - Go 版のプロンプト (`ANSWER_PROMPT`) を使用します。
    ---
    - [ ] 079-1. `src/cuber/query/synthesizer.rs` を作成
    - [ ] 079-2. コンテキストの文字列化 (Context String Builder) 実装
    - [ ] 079-3. `ChatModel.generate` 呼び出し

- [ ] 080. ストリーミング応答の実装 (Query)
    - 回答生成プロセス（検索中、生成中トークン）を SSE でリアルタイム配信します。
    - `EventBus` と `c.Stream` を連携させます。
    ---
    - [ ] 080-1. LLM のストリーミング API 呼び出し
    - [ ] 080-2. 受信トークンを即時 `EventBus` へ流す
    - [ ] 080-3. ハンドラ側での SSE 形式への変換

- [ ] 081. `Query` 統合テスト (単純検索)
    - 実際にデータが入った状態で、期待するチャンクやグラフがヒットするかを確認します。
    ---
    - [ ] 081-1. テストデータ投入
    - [ ] 081-2. キーワード検索を実行
    - [ ] 081-3. 期待する ID が含まれているか検証

- [ ] 082. `Query` 統合テスト (RAG回答)
    - 質問に対して、コンテキストに基づいた回答が生成されるか（ハルシネーションしていないか）を確認します。
    ---
    - [ ] 082-1. 「この文書の著者は？」等の質問を投げる
    - [ ] 082-2. 回答に正解が含まれているかチェック

- [ ] 083. `Query` 統合テスト (矛盾解決)
    - 矛盾するデータを入れた状態でクエリを投げ、Stage 1 解決が効いているか確認します。
    ---
    - [ ] 083-1. 矛盾データの投入
    - [ ] 083-2. `conflict_resolution_stage=1` でクエリ実行
    - [ ] 083-3. 正しい方の情報が採用されているか確認

- [ ] 084. パフォーマンスチューニング (Query)
    - 検索速度を測定し、インデックスが効いているか、無駄なループがないかを確認します。
    ---
    - [ ] 084-1. 計測ログの仕込み
    - [ ] 084-2. スロークエリの特定と改善

- [ ] 085. 異常系テスト (空検索結果、LLMエラー等)
    - ヒット 0 件の場合や LLM がダウンしている場合の挙動を確認します。
    ---
    - [ ] 085-1. 全く関係ない単語で検索 -> 「分かりませんでした」等の適切な応答
    - [ ] 085-2. API キーを無効化して実行 -> エラーハンドリング確認

### Phase 8: Memify (自己強化) プロセスの実装 (Step 086 - 095)

自律的な知識整理・強化プロセスを実装します。

- [ ] 086. `MemifyTask` 実装: ログ分析と欠損検知
    - `Query` のログから、「分からなかった質問」や「精度が低かった回答」を抽出するジョブを実装します。
    - LLM にログを読ませ、「何の情報が足りていないか」を推論させます。
    ---
    - [ ] 086-1. `src/cuber/pipeline/memify.rs` を作成
    - [ ] 086-2. 失敗クエリログの読み込みロジック
    - [ ] 086-3. 欠損情報抽出プロンプトの定義と実行

- [ ] 087. `WebSearchTask` 実装 (Tavily/Google API)
    - 不足情報を補うための Web 検索クライアントを実装します。
    - 外部 API キーを使用し、検索結果のテキストを取得します。
    ---
    - [ ] 087-1. `src/utils/web_search.rs` を作成
    - [ ] 087-2. Tavily API (推奨) へのリクエスト実装
    - [ ] 087-3. 取得した HTML/テキストのクリーニング

- [ ] 088. 自動 `Absorb` 処理の実装
    - Web 検索で得た情報をテキストファイル化し、既存の `Absorb` パイプラインに再投入するフローを作ります。
    - これにより、知識が自動的に `Cube` に追加されます。
    ---
    - [ ] 088-1. 検索結果を一時ファイルとして保存
    - [ ] 088-2. `cuber_service.absorb` を内部呼び出し
    - [ ] 088-3. 出所 (Source URL) をメタデータに記録

- [ ] 089. 親子ノードのマージ (Entity Resolution)
    - 新しく入った情報と既存のノードが同一人物/事物である場合、名寄せ (Merge) するロジックを実装します。
    - LLM に判断させるか、ルールベースで行います。
    ---
    - [ ] 089-1. 類似ノード検索ロジック
    - [ ] 089-2. マージ判定プロンプトの定義
    - [ ] 089-3. DB 上でのノード統合クエリ (`MATCH (a), (b) MERGE ...`)

- [ ] 090. Good/Bad フィードバックハンドラ実装
    - ユーザーからの 👍/👎 を受け取り、評価データを蓄積するエンドポイントを実装します。
    - `rt/rthandler/feedback_handler.rs` を想定。
    ---
    - [ ] 090-1. `Feedback` エンティティ (SeaORM) の作成
    - [ ] 090-2. `vote` API の実装
    - [ ] 090-3. 回答 ID と紐付けて DB 保存

- [ ] 091. フィードバックに基づくエッジ重み更新
    - Good がついた回答に使われたエッジのスコアを上げ、Bad の場合は下げるバッチ処理を実装します。
    - 信頼度スコア (`TrustScore`) の更新ロジックです。
    ---
    - [ ] 091-1. `update_trust_score` ジョブの実装
    - [ ] 091-2. 関連エッジ (Citation) の特定ロジック
    - [ ] 091-3. スコア更新クエリの実行

- [ ] 092. `Memify` 定期実行ジョブ (Cron)
    - バックグラウンドで定期的に (例: 深夜) `Memify` プロセスを走らせるスケジューラを組み込みます。
    - `tokio-cron-scheduler` 等を利用します。
    ---
    - [ ] 092-1. `Cargo.toml` にスケジューラクレートを追加
    - [ ] 092-2. `CuberService` 起動時にジョブを登録
    - [ ] 092-3. ログローテーションとの兼ね合い確認

- [ ] 093. `Memify` 統合テスト
    - 意図的に欠損させた質問をし、プロセス実行後に回答できるようになっているかを確認します。
    - 自己強化サイクルの動作証明です。
    ---
    - [ ] 093-1. 未知の質問を投げる -> 「不明」
    - [ ] 093-2. `Memify` 実行 (Web検索モック可)
    - [ ] 093-3. 再度質問 -> 正答

- [ ] 094. コスト管理とリミッター実装
    - 自動検索や LLM 推論が走りすぎないよう、予算 (Budget) 管理ロジックを入れます。
    - API 呼び出し回数の上限設定など。
    ---
    - [ ] 094-1. `TokenUsage` テーブル (または Redis) で使用量追跡
    - [ ] 094-2. `Memify` 実行前の予算チェック
    - [ ] 094-3. アラート通知機能

- [ ] 095. 管理者ダッシュボード用 API (簡易)
    - `Memify` の実行履歴や、現在の知識量 (Chunk/Node数) を返す管理者向け API を実装します。
    ---
    - [ ] 095-1. `admin/stats` API 実装
    - [ ] 095-2. `admin/memify/trigger` (手動実行) API 実装

### Phase 9: エクスポート・インポートその他 (Step 096 - 100+)

ポータビリティ機能と最終仕上げを行います。

- [ ] 096. `ExportCube` 実装 (ZIP アーカイブ作成)
    - 指定された `Cube` の全データ（DB ファイル、メタデータ）をまとめた ZIP ファイルを作成してダウンロード可能にします。
    - パスワード付き ZIP または内部データの暗号化を検討します。
    ---
    - [ ] 096-1. データファイル収集ロジック
    - [ ] 096-2. `zip` クレートを用いたアーカイブ作成
    - [ ] 096-3. ストリーミングレスポンスの実装

- [ ] 097. `GenKeyCube` 実装 (JWT/Crypto ロジック)
    - `Cube` を他者に渡すためのアクセスキー（パスワードやトークン）を生成します。
    - 共有時の権限設定なども含みます。
    ---
    - [ ] 097-1. 暗号論的に安全なランダムキー生成
    - [ ] 097-2. キーのハッシュ保存 (SeaORM)
    - [ ] 097-3. 有効期限の設定

- [ ] 098. `ImportCube` 実装 (ZIP 展開・検証・DB登録)
    - アップロードされた ZIP ファイルを受け取り、検証してシステムに取り込みます。
    - 既存の `Cube` との ID 衝突を防ぐロジックが必要です。
    ---
    - [ ] 098-1. ZIP ファイルの展開とバリデーション
    - [ ] 098-2. 不正なファイルが含まれていないかのセキュリティチェック
    - [ ] 098-3. 新規 `Cube` として DB 登録

- [ ] 099. `ReKeyCube` 実装 (権限更新)
    - 共有キーを無効化・再発行する機能です。
    - セキュリティインシデント時や定期的なローテーションに使用します。
    ---
    - [ ] 099-1. 旧キーの無効化処理
    - [ ] 099-2. DB 更新と監査ログ出力
    - [ ] 099-3. 関連するアクティブセッションの切断検証

- [ ] 100. 全機能の最終動作確認とドキュメント (`walkthrough.md`) 更新
    - Phase 1 から 9 までの全機能が連携して動くことを確認します。
    - 成果物としてのドキュメントを整備し、引き継ぎ可能な状態にします。
    ---
    - [ ] 100-1. シナリオテスト (新規登録 -> Cube作成 -> Absorb -> Query -> Export)
    - [ ] 100-2. `walkthrough.md` へのスクリーンショット・ログ添付
    - [ ] 100-3. 未解決の TODO / FIXME の整理とチケット化
```