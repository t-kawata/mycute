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

---

## 3. 実行手順
1. 指示書 `docs/DIRECTONS-PHASE-11*.md` に従い、Phase-11A から順に実装する。
2. ユーザーの明示的な指示があるまで実装は開始しない。
