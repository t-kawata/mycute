# REST API 実装構造とルール

本プロジェクトにおけるREST API（RTモード）の実装構造と開発ルールについて記述します。
本プロジェクトでは、責務の分離を徹底し、保守性と可読性を高めるために厳格なレイヤードアーキテクチャを採用しています。

## 1. 全体アーキテクチャ概要

サーバーの起動フローは以下の通りです。

1.  **Entry Point**: `src/main.go`
    *   コマンドライン引数に応じてモードを決定します。
    *   モード定義は `src/enum/mode/mode.go` に集約されています。
        *   `rt`: REST APIサーバー起動モード
        *   `am`: オートマイグレーション実行モード
2.  **RT Mode Entry**: `src/mode/rt/main_of_rt.go` (`MainOfRT`)
    *   RT（REST API）モードのエントリーポイントです。
    *   環境変数のロード、DB接続、Logger初期化、Ginエンジンのセットアップを行います。
3.  **Routing**: `src/mode/rt/request_mapper.go` (`MapRequest`)
    *   URLルーティング定義を一元管理します。
    *   ミドルウェア（認証など）の適用を行い、リクエストを `rthandler` に振り分けます。

## 2. 5つのプログラム層と責務

リクエスト処理は、以下の5つのパッケージに役割分担されています。

| パッケージ | パス | 主な責務 |
| :--- | :--- | :--- |
| **rthandler** | `src/mode/rt/rthandler/hv1` | **ハンドラー層**。<br>HTTPリクエストの入り口。Swagger定義、権限チェック呼び出し、バインディング呼び出し、ビジネスロジック呼び出しを行う。処理ロジックは書かない。 |
| **rtparam** | `src/mode/rt/rtparam` | **パラメータ定義層**。<br>リクエストボディ（JSON）の構造体定義を行う。Swagger用のexampleやバリデーションタグを記述する。 |
| **rtreq** | `src/mode/rt/rtreq` | **リクエスト処理層**。<br>GinのContextからデータを抜き出し、バリデーションを行い、整形されたリクエスト構造体を返す。特別なバリデーションがある場合にもここに実装をまとめる。 |
| **rtres** | `src/mode/rt/rtres` | **レスポンス定義層**。<br>クライアントに返すレスポンスJSONの構造体（DataとErrors）を定義する。 |
| **rtbl** | `src/mode/rt/rtbl` | **ビジネスロジック層**。<br>実際の処理（DB操作、計算、外部API連携など）を行う。最終的なレスポンスの書き込みも担当する。 |

---

## 3. 各層の実装詳細とルール

### 3.1. rthandler (Handlers)
パス: `src/mode/rt/rthandler/hv1/*.go`

HTTPリクエストを受け取る最初の層です。

*   **命名規則**: `機能名_handler.go` (例: `usrs_handler.go`)
*   **責務**:
    1.  **Swaggerコメントの記述**: API仕様書となるコメントを関数ヘッダに詳細に記述します。
    2.  **権限チェック**: `rtbl.RejectUsr` 等を使用して、アクセス権限がないユーザーを弾きます。
    3.  **リクエストバインド**: `rtreq`層のBind関数を呼び出し、入力を検証・取得します。
    4.  **ロジック委譲**: 正常なリクエストであれば `rtbl`層の関数へ処理を委譲します。
    5.  **エラーハンドリング**: バインドエラー時は `rtbl.BadRequest` を呼び出します。
*   **禁止事項**:
    *   この層にビジネスロジック（DB操作など）を記述してはいけません。
    *   条件分岐は「権限チェック」や「バインド結果」程度に留めてください。

**実装例**:
```go
// @Summary ユーザを作成する
// ... Swagger annotation ...
func CreateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
    // 1. 権限チェック
    if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.USR}) {
        return
    }
    // 2. リクエストバインド
    if req, res, ok := rtreq.CreateUsrReqBind(c, u, ju); ok {
        // 3. ロジック実行
        rtbl.CreateUsr(c, u, ju, &req, &res)
    } else {
        // 4. バインドエラー
        rtbl.BadRequest(c, &res)
    }
}
```

**Swagger annotation**
以下にSwaggerに出力されるアノテーションの書き方の例を示します。@Tagsが一番上、@Routerが次に、そして@Summary・・・のように書く順番も以下に倣ってください。@DescriptionはMarkdownの1行として扱われます。可能な限り丁寧にわかりやすく詳細に@Descriptionを書くようにしてください。
```
// @Tags v1 User
// @Router /v1/usrs/ [post]
// @Summary ユーザを作成する。
// @Description - Key で取得した token では Apx のみを作成できる
// @Description - Apx で取得した token では Vdr のみを作成できる
// @Description - Vdr で取得した token では Usr のみを作成できる
// @Description - Usr は Usr を作れない
// @Description ### パラメータについて
// @Description - type: 1: 法人, 2: 個人 (VDR作成時は無視される)
// @Description - base_point: VDRのみ必須 (バッジ授与時に授与者である個人に付与される基本ポイント数)
// @Description - belong_rate: VDRのみ必須 (所属によるポイント割増率)
// @Description - max_works: VDRのみ必須 (Vdr内の個人が就労できる最大数)
// @Description - flush_fee_rate: VDRのみ必須 (現金プールを現金分配実行する時に、事務コストを賄うために Pool から引かれる割合)
// @Description - flush_days: 法人のみ必須 (現金プールを現金分配実行するためのサイクルとなる日数)
// @Description - rate: 法人のみ必須 (法人が、自分に所属するユーザーに対して付与する割増ポイント率)
// @Description - VDR作成時以外にVDR用項目を送信するとエラーとなる
// @Description - 法人作成時以外に法人用項目を送信するとエラーとなる
// @Description ### name について
// @Description - type=2 (個人) の場合、姓名の間にスペース（半角・全角問わず）が必須
// @Description - 全角スペースは半角スペースに変換され、連続するスペースは1つにまとめられる
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body CreateUsrParam true "json"
// @Success 200 {object} CreateUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func CreateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
```

### 3.2. rtparam (Parameters)
パス: `src/mode/rt/rtparam/*.go`

POST/PUT/PATCH等のリクエストボディ（JSON）を受け取るための構造体を定義します。

*   **命名規則**: `機能名_param.go` (例: `usrs_param.go`)
*   **構造体名**: `ActionNameParam` (例: `CreateUsrParam`)
*   **責務**:
    *   `json` タグ: JSONキーの指定。
    *   `swaggertype`: Swaggerでの型指定。
    *   `example`: Swaggerでの入力例。
    *   `format`: 日付やメールアドレスなどのフォーマット指定。
*   **ルール**:
    *   バリデーションタグ（`binding`）はここでは記述せず、**rtreq内で定義する構造体**にて記述するケースもありますが、基本的にはJSONパース用の純粋な構造体として扱います。

**実装例**:
```go
type CreateUsrParam struct {
    Name     string    `json:"name" swaggertype:"string" example:"User01"`
    Email    string    `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
    // ...
}
```

### 3.3. rtreq (Requests)
パス: `src/mode/rt/rtreq/*.go`

リクエストの検証と整形を担当します。

*   **命名規則**: `機能名_req.go` (例: `usrs_req.go`)
*   **構造体名**: `ActionNameReq` (例: `CreateUsrReq`)
*   **関数名**: `ActionNameReqBind` (例: `CreateUsrReqBind`)
*   **責務**:
    1.  **バインディング用構造体の定義**: Param構造体と似ていますが、こちらは `binding` タグ（Gin/validator）をフル活用してバリデーションルールを記述します。パスパラメータやクエリパラメータもこの構造体にマッピングします。タグによるバリデーションができなかったり、合理的ではないと判断する場合には、ここにバリデーション処理を記述してまとめることにより、バリデーションはReq内に納まり、Reqを抜けるタイミング（BLのビジネスロジックに入るタイミング）では、データ自体の検証は全て済んだ後であることを保証する。
    2.  **Bind関数の実装**:
        *   `c.ShouldBind` や `c.ShouldBindJSON` を使用。
        *   Path Parameter (`c.Param`) の取得と型変換。
        *   カスタムバリデーション（DBを使った重複チェックなど）の実行。
        *   エラー時のレスポンスオブジェクト (`rtres`) の作成（エラーメッセージ詰め込み）。
*   **戻り値**: `(リクエスト構造体, レスポンス構造体, 成功フラグbool)`

**実装例**:
```go
type CreateUsrReq struct {
    Name  string `json:"name" binding:"required,max=50"`
    // ...
}

func CreateUsrReqBind(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) (CreateUsrReq, rtres.CreateUsrRes, bool) {
    var req CreateUsrReq
    var res rtres.CreateUsrRes
    
    // JSONバインド & シンプルなタグベースバリデーション
    // Errorがnilの場合のみ詳細なバリデーションへ進む
    if err := c.ShouldBindJSON(&req); err == nil {
        // カスタムバリデーション 1: メールアドレスの重複チェック
        // cutilやcommonなどのヘルパー関数を使用してDBアクセスを伴う検証を行う
        if !cutil.IsUniqueByUsr(u.DB, req.Email) {
            res.Errors = append(res.Errors, rtres.Err{Field: "email", Code: rterr.Unique.Code(), Message: "Email already exists"})
        }

        // カスタムバリデーション 2: 特定条件下の必須チェック
        // タグでは表現しきれない複合条件などを記述する
        if req.Type == 1 && req.CorporateName == "" {
             res.Errors = append(res.Errors, rtres.Err{Field: "corporate_name", Code: rterr.Required.Code(), Message: "Corporate Name is required"})
        }

        // カスタムバリデーション 3: 日付の前後関係チェック
        if req.EndAt.Before(req.BgnAt) {
             res.Errors = append(res.Errors, rtres.Err{Field: "end_at", Code: rterr.Invalid.Code(), Message: "EndAt must be after BgnAt"})
        }

        // エラーが1つでもあれば失敗として返す
        if len(res.Errors) > 0 {
            return req, res, false
        }
        
        // 全てのバリデーションを通過
        return req, res, true

    } else {
        // Ginのバインドエラー（型不一致やbindingタグ違反）の処理
        res.Errors = u.GetValidationErrs(err)
        return req, res, false
    }
}
```

### 3.4. rtres (Responses)
パス: `src/mode/rt/rtres/*.go`

レスポンスフォーマットを定義します。

*   **命名規則**: `機能名_res.go` (例: `usrs_res.go`)
*   **構造体名**:
    *   `ActionNameResData`: 成功時に `data` フィールドに入るオブジェクト。
    *   `ActionNameRes`: API全体のレスポンス形式（`Data` と `Errors` を持つ）。
*   **責務**:
    *   APIレスポンスの型定義。
    *   ModelからResponseDataへの変換メソッド (`Of` メソッドなど) の実装。

**実装例**:
```go
type CreateUsrResData struct {
    ID uint `json:"id"`
}
type CreateUsrRes struct {
    Data   CreateUsrResData `json:"data"`
    Errors []Err            `json:"errors"`
}
```

### 3.5. rtbl (Business Logic)
パス: `src/mode/rt/rtbl/*.go`

アプリケーションの核心となる処理を行います。以下の厳格なコーディング規約に従ってください。

*   **命名規則**: `機能名_bl.go` (例: `usrs_bl.go`)
*   **関数名**: `ActionName` (例: `CreateUsr`)
*   **責務**:
    *   DBトランザクション管理。
    *   CRUD操作。
    *   複雑な計算や外部API呼び出し。
    *   最終的なHTTPレスポンスの送信（`OK`, `InternalServerError`, `NotFound` などのヘルパー関数を使用）。
*   **厳格な実装ルール (Strict Rules)**:
    1.  **関数シグネチャ**: 必ず以下の形式で統一します。
        `func FunctionName(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.RequestStruct, res *rtres.ResponseStruct) bool`
    2.  **ID取得**: `ApxID/VdrID` を使用する際は、必ず `ids := ju.IDs(true or false)` で取得した `ids` 構造体を使用してください。ju.IsApx() == true || ju.IsKey() == true なら ju.IDs(false), さもなくば ju.IDs(true) を使用します。
    3.  **ロギング**: 必ず `u.Logger` (*zap.Logger) を使用し、メッセージの先頭は**大文字**で始めてください。
        *   OK: `u.Logger.Info("Creating new cube", ...)`
        *   NG: `u.Logger.Info("creating new cube", ...)`
    4.  **DB操作 (GORM)**:
        *   可能な限りメソッドチェーンを改行せず **1行** で記述し、`.Error` をチェックします。
        *   カラム名やテーブル名を文字列で指定する場合は、MySQLの作法に従い **バッククォート** で囲みます。
        *   例: `if err := u.DB.Where("`apx_id` = ?", ids.ApxID).First(&model).Error; err != nil { ... }`
    5.  **共通ユーティリティ**: `src/lib/common` 以下の関数 (`common.GetNowUnixMilli()`, `common.ToJson()`等) を積極的に利用してください。

*   **戻り値**: 基本的に `bool` (処理成功か否か、ただしレスポンス書き込み済みなので呼び出し元で細かく判定することは少ない)。

**実装例**:
```go
func CreateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateUsrReq, res *rtres.CreateUsrRes) bool {
    // 処理
    usr := model.Usr{Name: req.Name, ...}
    if err := u.DB.Create(&usr).Error; err != nil {
        return InternalServerError(c, res)
    }
    
    // レスポンス作成
    data := rtres.CreateUsrResData{ID: usr.ID}
    return OK(c, &data, res)
}
```

---

## 4. 開発フローのまとめ

新しいAPIエンドポイント（例: `POST /items`）を追加する場合の手順：

1.  **rtres**: レスポンス定義 (`CreateItemRes`, `CreateItemResData`) を作成。
2.  **rtparam**: パラメータ定義 (`CreateItemParam`) を作成（Swagger用）。
3.  **rtreq**: リクエスト定義 (`CreateItemReq`) とバインド関数 (`CreateItemReqBind`) を作成。ここで入力チェックを実装。
4.  **rtbl**: ビジネスロジック (`CreateItem`) を作成。実際の登録処理を実装。
5.  **rthandler**: ハンドラー (`CreateItem`) を作成。Swaggerを書き、BindとBLを繋ぐ。
6.  **request_mapper**: ルーティングを追加 (`v1.POST("/items", hv1.CreateItem)`).

この構造を守ることで、APIの実装パターンが統一され、どのファイルを見れば何が書いてあるかが明確になります。

## 5. データパーティショニングの絶対ルール

本システムのデータベース設計および操作において、以下のルールは**絶対厳守**です。

### 5.1. ApxIDとVdrIDの必須化
全ての GORM モデル（構造体）は、必ず以下のフィールド定義を持たなければなりません。

```go
ApxID uint `gorm:"index:idx_apxid_vdrid_unique;priority:1"`
VdrID uint `gorm:"index:idx_apxid_vdrid_unique;priority:2"`
```
※ インデックス名や複合インデックスの構成はモデルごとに異なりますが、**`ApxID` と `VdrID` カラム自体は必須**です。

### 5.2. データ操作時の厳密なパーティショニング
データベースへのクエリ（SELECT, UPDATE, DELETE 等）を発行する際は、必ず `ApxID` と `VdrID` を条件に含め、データパーティションを厳密に区切る必要があります。
これにより、異なる組織やベンダー間のデータ混在事故をシステムレベルで防ぎます。

**悪い例:**
```go
// IDのみで検索（NG: 他のApx/Vdrのデータにアクセスできてしまう可能性がある）
db.First(&user, "id = ?", id)
```

**良い例:**
```go
// ApxIDとVdrIDを常にAND条件に含める（OK）
db.Where("apx_id = ? AND vdr_id = ?", ids.ApxID, ids.VdrID).First(&user, "id = ?", id)
```

このルールは、`rtbl` 層での実装時に特に注意して適用してください。
`rtreq` でのカスタムバリデーション（Uniqueness Checkなど）においても同様です。

