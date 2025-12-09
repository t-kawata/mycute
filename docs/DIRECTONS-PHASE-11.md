# mycute / cube Implementation: Phase-11 Detailed Development Directives
# Cubes API & Service Implementation (Phase-11)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-11: Cubes API & Service Implementation** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。
Phase-11では、`docs/ABOUT_MYCUTE_SERVICE.md` で定義された「暗号化された知識キューブ（cube）」独自の概念を実現し、それを操作するための API エンドポイントを実装します。

> [!IMPORTANT]
> **Phase-10の完了確認**
> Phase-10までの実装（KuzuDBの統合、DBモード切替など）が完了していることが前提です。

> [!CRITICAL]
> **API実装ルールの厳守**
> すべての実装は、`docs/REST_API_STRUCTURE_AND_RULES.md` に記載された **REST API 実装ルール（5層構造: Handler, Param, Req, Res, BL）** に完全に準拠しなければなりません。

> [!CRITICAL]
> **データパーティショニングの厳守**
> すべてのDBモデル定義には `ApxID` と `VdrID` が必須です。
> すべてのDB操作で `ApxID` と `VdrID` をWHERE句に含めることを徹底してください。

> [!CRITICAL]
> **厳格なトークン管理**
> トークン集計は Cube の市場価値に直結するため、`search`, `absorb`, `memify` におけるトークンカウント（IN/OUT）は **OpenAI形式で正確に** 行わなければなりません。
> 万が一トークン数が取得できない場合は、処理をエラーとして中断するほどの厳格さが求められます。

---

## 1. 全体スケジュールとサブフェーズ (Schedule & Sub-phases)

Phase-11は、対象となるAPIエンドポイントごとに以下のサブフェーズに分割して進行します。

| Sub-phase | 対応エンドポイント | HTTP Method | 主なタスク | 開発指示書 |
|-----------|-------------------|-------------|-----------|------------|
| **Phase-11A** | `create` | `POST` | Cube/Export/BurnedKeyモデル定義 (gorm.io/datatypes), `POST /v1/cubes/create` 実装 | `docs/DIRECTONS-PHASE-11A.md` |
| **Phase-11B** | `import` | `POST` | `.cube` 取込, 鍵消費(Burn), 権限設定, `POST /v1/cubes/import` 実装 | `docs/DIRECTONS-PHASE-11B.md` |
| **Phase-11C** | `export` | `GET` | Export記録作成, 新UUID付与, `GET /v1/cubes/export` 実装 | `docs/DIRECTONS-PHASE-11C.md` |
| **Phase-11D** | `rekey` | `PUT` | 鍵更新(Burn), `PUT /v1/cubes/rekey` 実装 | `docs/DIRECTONS-PHASE-11D.md` |
| **Phase-11E** | `absorb` | `PUT` | 知識取り込み, 権限チェック, `PUT /v1/cubes/absorb` 実装 | `docs/DIRECTONS-PHASE-11E.md` |
| **Phase-11F** | `memify` | `PUT` | 自己強化, 権限チェック, `PUT /v1/cubes/memify` 実装 | `docs/DIRECTONS-PHASE-11F.md` |
| **Phase-11G** | `search` | `GET` | 知識検索, 権限チェック, `GET /v1/cubes/search` 実装 | `docs/DIRECTONS-PHASE-11G.md` |
| **Phase-11H** | `delete` | `DELETE` | 物理削除, 権限チェック, `DELETE /v1/cubes/delete` 実装 | `docs/DIRECTONS-PHASE-11H.md` |
| **Phase-11I** | `stats` | `GET` | 統計情報取得, 権限チェック, `GET /v1/cubes/stats` 実装 | `docs/DIRECTONS-PHASE-11I.md` |
| **Phase-11J** | `genkey` | `POST` | 鍵発行, 権限チェック, `POST /v1/cubes/genkey` 実装 | `docs/DIRECTONS-PHASE-11J.md` |

---

## 2. 共通要件と注意事項 (Common Requirements)

### 2.1 データモデルと permissions
- **GORM Type**: JSONカラムには `gorm.io/datatypes` の `datatypes.JSON` を使用すること。
- **Permission Limit Logic**:
    - **0**: 無制限 (Unlimited)。
    - **正の整数 (>0)**: 残り回数 (Remaining)。
    - **負の数 (<0)**: 禁止/終了 (Forbidden/Finished)。
    - **状態遷移**: `1` (残り1回) を消費すると、`0` (無制限) になってはいけないため、**`-1` (終了)** に更新するロジックを実装すること。

- **`allow_delete` 廃止**: Import した Cube も不要になれば削除できるべきであるため、削除権限フラグは廃止する。

### 2.2 鍵管理と Export
- **ExportとGenKeyの分離**:
    - `export` は物理的なパッケージ生成とレコード作成のみを行う。
    - `genkey` は、手持ちの Cube または **Export済み (Export Recordが存在する) Cube** に対して、何度でも鍵を発行できる。
    - これにより、一度 Export したパッケージに対して、後から異なる権限（期限付き、無期限など）の鍵を販売・配布することが可能になる。

- **Lineage**: エクスポート時のタイムスタンプ (`ExportedAt` ms) を含めること。

### 2.3 トークン詳細集計
- `pkg/cuber` を改修し、OpenAI互換のレスポンス形式から正確な `usage.prompt_tokens`, `usage.completion_tokens` を取得する。
- 取得に失敗した場合はエラーとし、不正確な記録を残さない。

### 2.3.1 統計情報の階層構造 (Stats Hierarchy)

> [!IMPORTANT]
> **MemoryGroup を最上位の粒度とした統計設計**
> 
> 統計情報は、人間が「この Cube はどの分野にどれだけ詳しいか」を判断できる形式で提供されなければなりません。

**背景と目的**:

ユーザーが Cube を評価する際、最も重要な観点は「**どの専門分野 (MemoryGroup) に、どれだけの知識が蓄積されているか**」です。

例えば、あるCubeの統計情報を見たときに、以下のような判断ができる必要があります:
- 「この Cube は `legal_expert` 分野に 10人が 500万トークンを使って育てたので、法律には詳しいはずだ」
- 「しかし `medical_expert` 分野には 1人が 5万トークンしか使っていないので、医療にはあまり詳しくなさそうだ」

**統計モデルの階層構造**:

```
Cube
├── MemoryGroup: "legal_expert"           ← 専門分野単位
│   ├── CubeModelStat (Training)
│   │   ├── gpt-4: 入力100万, 出力50万
│   │   └── text-embedding-3-small: 入力200万
│   ├── CubeModelStat (Search)
│   │   └── gpt-4: 入力20万, 出力10万
│   └── CubeContributor (Training のみ)
│       ├── 山田太郎: gpt-4 で 入力30万, 出力15万
│       └── 鈴木花子: gpt-4 で 入力70万, 出力35万
│
├── MemoryGroup: "medical_expert"
│   └── ...
│
└── Lineage (系譜情報 - MemoryGroup に依存しない)
```

**モデル定義への影響**:

`CubeModelStat` と `CubeContributor` の両方に `MemoryGroup` フィールドを追加し、複合ユニークインデックスを設定します（詳細は Phase-11A 参照）。

## 2.4 メモリグループ (MemoryGroup) 概念

> [!IMPORTANT]
> **Phase-11 設計変更: `user` パラメータの廃止と `memoryGroup` の導入**
> 
> この設計変更は、Cube 内の知識を「分野」や「専門領域」ごとに分離・管理するための柔軟性を提供します。

### 2.4.1 背景と動機

従来の実装では、`pkg/cuber` の主要関数 (`Absorb`, `Memify`, `Search`) において `user string` パラメータを受け取り、内部で `memoryGroup := user + "-" + cubeUUID` という形式でグループIDを生成していました。

この方式には以下の問題がありました:

1. **柔軟性の欠如**: グループIDがユーザーIDに紐づくため、ユーザーが「知識の分野」を自由に指定できない
2. **マルチテナント対応の困難**: 同一Cube内で複数の専門家（例: 「法律専門家」「医療専門家」）を並存させることができない
3. **API設計との乖離**: REST API で「どの知識グループに対して操作するか」を明示的に指定できない

### 2.4.2 新しい設計: MemoryGroup

**変更後**:
- `user string` パラメータを廃止
- `memoryGroup string` パラメータを新規追加
- クライアントが指定した `memoryGroup` がそのまま KuzuDB 内の `memory_group` として使用される

**概念図**:
```
Cube (UUID: abc-123)
├── MemoryGroup: "legal_expert"     ← 法律知識
│   ├── DocumentChunks...
│   ├── Nodes/Edges (グラフ)
│   └── Summaries...
│
├── MemoryGroup: "medical_expert"   ← 医療知識
│   ├── DocumentChunks...
│   ├── Nodes/Edges (グラフ)
│   └── Summaries...
│
└── MemoryGroup: "general"          ← 汎用知識
    └── ...
```

### 2.4.3 CuberService 関数シグネチャの変更

**変更前 (旧)**:
```go
func (s *CuberService) Absorb(ctx context.Context, cubeDbFilePath string, user string, filePaths []string) (types.TokenUsage, error)
func (s *CuberService) Search(ctx context.Context, cubeDbFilePath string, searchType search.SearchType, query string, user string) (string, types.TokenUsage, error)
func (s *CuberService) Memify(ctx context.Context, cubeDbFilePath string, user string, config *MemifyConfig) (types.TokenUsage, error)
```

**変更後 (新)**:
```go
// Absorb: memoryGroup が memoryGroup としてそのまま使用される
func (s *CuberService) Absorb(ctx context.Context, cubeDbFilePath string, memoryGroup string, filePaths []string) (types.TokenUsage, error)

// Search: 指定した memoryGroup 内を検索
func (s *CuberService) Search(ctx context.Context, cubeDbFilePath string, memoryGroup string, searchType search.SearchType, query string) (string, types.TokenUsage, error)

// Memify: 指定した memoryGroup の知識を強化
func (s *CuberService) Memify(ctx context.Context, cubeDbFilePath string, memoryGroup string, config *MemifyConfig) (types.TokenUsage, error)
```

### 2.4.4 REST API リクエストパラメータ

`absorb`, `memify`, `search` エンドポイントでは、`MemoryGroup` が**必須パラメータ**となります:

```go
// rtparam/cubes_param.go

// AbsorbRequest (PUT /v1/cubes/absorb)
type AbsorbBody struct {
    CubeID      uint   `json:"cube_id" binding:"required"`             // ← CubeIDで指定
    MemoryGroup string `json:"memory_group" binding:"required"`        // ← 必須
    Content     string `json:"content" binding:"required"`
}

// MemifyRequest (PUT /v1/cubes/memify)
type MemifyBody struct {
    CubeID      uint   `json:"cube_id" binding:"required"`             // ← CubeIDで指定
    MemoryGroup string `json:"memory_group" binding:"required"`        // ← 必須
    // ... config params
}

// SearchQuery (GET /v1/cubes/search)
type SearchQuery struct {
    CubeID      uint   `form:"cube_id" binding:"required"`             // ← CubeIDで指定
    MemoryGroup string `form:"memory_group" binding:"required"`        // ← 必須
    Q           string `form:"q" binding:"required"`
    SearchType  string `form:"search_type"`
}
```

### 2.4.5 MemoryGroup の命名規則

- **推奨形式**: 英数字とアンダースコア/ハイフンのみ (`^[a-zA-Z0-9_-]+$`)
- **最大長**: 64文字
- **禁止文字**: スペース、特殊文字、日本語（KuzuDB/SQL互換性のため）
- **例**: `legal_expert`, `medical-v2`, `user123_private`, `general`

> [!WARNING]
> `MemoryGroup` はユーザー入力をそのまま `memory_group` として使用するため、不正な文字列が混入しないようバリデーションを厳格に行うこと。

---

## 3. 実行手順
1. 指示書 `docs/DIRECTONS-PHASE-11*.md` に従い、Phase-11A から順に実装する。
2. ユーザーの明示的な指示があるまで実装は開始しない。
