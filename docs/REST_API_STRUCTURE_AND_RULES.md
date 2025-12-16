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
*   **厳格な実装ルール (Strict Rules)**:
    1.  **ハンドラーの実装パターン**: 以下のif-elseパターンを厳守してください。ロジックの分岐やエラーハンドリングを独自に書かないでください。
        ```go
        if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
            return
        }
        if req, res, ok := rtreq.XxxReqBind(c, u); ok {
            rtbl.Xxx(c, u, ju, &req, &res)
        } else {
            rtbl.BadRequest(c, &res)
        }
        ```
    2.  **RejectUsrの使用**: `rtbl.RejectUsr` の第4引数には、アクセスを**拒否**するユーザータイプ (`usrtype.UsrType` のスライス) を明示的に渡してください。
        *   例: `[]usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}` (USRのみ許可)
    3.  **無駄な改行の禁止**: 関数内の各ステートメント間に無駄な改行を入れないでください。
    4.  **rtreqへの完全委譲**: `c.ShouldBindJSON` などのバインド処理やバリデーションは、必ず `rtreq` 層の `ReqBind` 関数に委譲してください。

**実装例**:
```go
// @Summary Cubeを検索
// @Tags v1 Cube
// @Router /v1/cubes/search [post]
// @Summary Cubeを検索
// @Description - USR によってのみ使用できる
// @Description - 条件に一致するCubeの詳細情報を一覧取得する
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param params body rtparam.SearchCubesParam true "Search Params"
// @Success 200 {object} rtres.SearchCubesRes "Success"
// @Failure 400 {object} rtres.ErrRes "Validation Error"
// @Failure 401 {object} rtres.ErrRes "Unauthorized"
// @Failure 500 {object} rtres.ErrRes "Internal Server Error"
func SearchCubes(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
    // 1. 権限チェック (拒否したいTypeを列挙)
    if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
        return
    }
    // 2. リクエストバインド & ロジック実行
    if req, res, ok := rtreq.SearchCubesReqBind(c, u); ok {
        rtbl.SearchCubes(c, u, ju, &req, &res)
    } else {
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
    1.  **バインディング用構造体の定義**: Param構造体と似ていますが、こちらは `binding` タグ（Gin/validator）をフル活用してバリデーションルールを記述します。パスパラメータやクエリパラメータもこの構造体にマッピングします。タグによるバリデーションができなかったり、合理的ではないと判断する場合には、ここにバリデーション処理を記述してまとめることにより、バリデーションはReq内に納まり、Reqを抜けるタイミング（BLのビジネスロジックに入るタイミング）では、データ自体の検証は全て済んだ後であることを保証しなければなりません。
    2.  **Bind関数の実装**:
        *   `c.ShouldBind` や `c.ShouldBindJSON` を使用。
        *   Path Parameter (`c.Param`) の取得と型変換。
        *   カスタムバリデーション（DBを使った重複チェックなど）の実行。
        *   エラー時のレスポンスオブジェクト (`rtres`) の作成（エラーメッセージ詰め込み）。
        *   **厳格なルール**: バリデーションエラー時は必ず `false` を返し、ビジネスロジックへ不正なデータを渡さないでください。
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
*   **厳格な実装ルール (Strict Rules)**:
    *   **`Of` 関数の実装**: `Search` 系や `Get` 系のAPIレスポンスでは、必ず `Of` 関数を作成し、GORMのModelからResponseDataへの変換ロジックを分離してください。
    *   `Of` 関数は、`ResponseData` のポインタレシーバとして定義し、引数に `Model` (またはそのSlice) を受け取ります。

**実装例**:
```go
type SearchAskActsResData struct {
	ID            uint   `json:"id" swaggertype:"integer" format:"" example:"1"`
	OwnerID       uint   `json:"owner_id" swaggertype:"integer" format:"" example:"1"`
	Name          string `json:"name" swaggertype:"string" format:"" example:"AskAct01"`
	Description   string `json:"description" swaggertype:"string" format:"" example:"This act is used for..."`
	CttsHost      string `json:"ctts_host" swaggertype:"string" format:"" example:"p00-cv001a.samle.com"`
	CttsIdent     string `json:"ctts_ident" swaggertype:"string" format:"" example:"man-01"`
	Words         string `json:"words" swaggertype:"string" format:"" example:"Hello world."`
	Try           uint16 `json:"try" swaggertype:"number" format:"" example:"1"`
	IsEditing     bool   `json:"is_editing" swaggertype:"boolean" format:"" example:"false"`
	IsReady       bool   `json:"is_ready" swaggertype:"boolean" format:"" example:"false"`
	IsReadyFailed bool   `json:"is_ready_failed" swaggertype:"boolean" format:"" example:"false"`
	Data          string `json:"data"`
	BgnAt         string `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt         string `json:"end_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
} // @name SearchAskActsResData

func (d *SearchAskActsResData) Of(ms *[]model.Act) *[]SearchAskActsResData {
	data := []SearchAskActsResData{}
	for _, m := range *ms {
		data = append(data, SearchAskActsResData{
			ID:            m.ID,
			OwnerID:       m.OwnerID,
			Name:          m.Name,
			Description:   m.Description,
			CttsHost:      m.CttsHost,
			CttsIdent:     m.CttsIdent,
			Words:         m.Words,
			Try:           m.Try,
			IsEditing:     m.IsEditing,
			IsReady:       m.IsReady,
			IsReadyFailed: m.IsReadyFailed,
			Data:          m.Data,
			BgnAt:         common.ParseDatetimeToStr(&m.BgnAt),
			EndAt:         common.ParseDatetimeToStr(&m.EndAt),
		})
	}
	return &data
}

type SearchAskActsRes struct {
	Data   []SearchAskActsResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name SearchAskActsRes

type GetCubeResData struct {
	ID          uint   `json:"id" swaggertype:"integer" example:"1"`
	UUID        string `json:"uuid" swaggertype:"string" example:"550e8400-e29b-41d4-a716-446655440000"`
	UsrID       string `json:"usr_id" swaggertype:"string" example:"user-uuid"`
	Name        string `json:"name" swaggertype:"string" example:"My Cube"`
	Description string `json:"description" swaggertype:"string" example:"Knowledge base"`
	ExpireAt    string `json:"expire_at,omitempty" swaggertype:"string" format:"date-time"`
	ApxID       uint   `json:"apx_id" swaggertype:"integer"`
	VdrID       uint   `json:"vdr_id" swaggertype:"integer"`
	CreatedAt   string `json:"created_at" swaggertype:"string" format:"date-time"`
	UpdatedAt   string `json:"updated_at" swaggertype:"string" format:"date-time"`
} // @name GetCubeResData

func (d *GetCubeResData) Of(m *model.Cube) *GetCubeResData {
	expireStr := ""
	if m.ExpireAt != nil {
		expireStr = common.ParseDatetimeToStr(m.ExpireAt)
	}
	data := GetCubeResData{
		ID:          m.ID,
		UUID:        m.UUID,
		UsrID:       m.UsrID,
		Name:        m.Name,
		Description: m.Description,
		ExpireAt:    expireStr,
		ApxID:       m.ApxID,
		VdrID:       m.VdrID,
		CreatedAt:   common.ParseDatetimeToStr(&m.CreatedAt),
		UpdatedAt:   common.ParseDatetimeToStr(&m.UpdatedAt),
	}
	return &data
}

type GetCubeRes struct {
	Data   GetCubeResData `json:"data"`
	Errors []Err          `json:"errors"`
} // @name GetCubeRes
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
        *   **UUIDの生成**: `uuid.New().String()` ではなく必ず **`common.GenUUID()`** を使用してください。
        *   **JSON処理**:
            *   `string` <-> `struct` 変換: `common.ParseJson`, `common.ToJson` を使用してください。
            *   `datatypes.JSON` <-> `struct` 変換: `common.ParseDatatypesJson`, `common.ToJsonForDatatypesJson` (または `common.ToJson` の結果をキャスト) を使用してください。
    6.  **環境変数へのアクセス**: `rthandler`, `rtbl` 層やそれ以下の層で `os.Getenv` を直接使用することは **禁止** です。
        *   必要な環境変数は `src/mode/rt/main_of_rt.go` で `RTFlags` に格納し、`MapRequest` -> `AuthMiddleware` -> `rtutil.RtUtil` の順にパススルーし、必ず `u *rtutil.RtUtil` 経由でアクセスできるようにしてください。
    7.  **無駄な改行の禁止**: 関数内の各ステートメント間に無駄な改行を入れないでください。可読性向上のための論理的なブロック分けには、空行ではなく「コメント (`// <説明文>` 形式)」を使用してください。空行は徹底的に排除してください。

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

---

## 6. Swagger アノテーションの厳格なルール

Swagger（OpenAPI）ドキュメントの自動生成における重要なルールです。

### 6.1. リクエストボディの型指定

ハンドラー関数のSwaggerアノテーションで `@Param json body` を指定する場合、**必ず `rtparam` パッケージで定義した構造体を使用**してください。

> [!CAUTION]
> **`rtreq` の構造体を `@Param json body` に指定してはいけません。**
> `rtreq` は内部処理用の構造体であり、Swagger生成ツール (`swag`) が正しくパースできない場合があります。

**正しい例:**
```go
// @Param json body AbsorbCubeParam true "json"  ← rtparam.AbsorbCubeParam を使用
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
```

**誤った例:**
```go
// @Param json body AbsorbCubeReq true "json"  ← rtreq.AbsorbCubeReq を使用（NG）
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
```

### 6.2. rtparam と rtreq の役割分担

| パッケージ | 用途 | Swagger で使用 |
| :--- | :--- | :---: |
| **rtparam** | Swagger ドキュメント生成用のリクエストパラメータ定義 | ✅ 可 |
| **rtreq** | 内部バインド・バリデーション処理用のリクエスト構造体 | ❌ 不可 |

新しいAPIエンドポイントを作成する際は、必ず `rtparam` に対応する構造体を作成してください。

---

## 7. ビルド成功の検証手順

プロジェクトの変更後、以下の **3つのコマンドを順番に実行** し、全てが成功することを確認してください。

> [!IMPORTANT]
> **全てのコマンドがエラーなく完了するまで、ビルド成功とは判断できません。**
> 1つでも失敗した場合は、問題を解決してから再度全てのコマンドを実行してください。

### 7.1. 実行コマンド（プロジェクトルートディレクトリで実行）

```bash
# 1. Swagger ドキュメント生成
#    - rtparam/rtres の構造体定義が正しいか検証
#    - アノテーションのパースエラーをチェック
make swag

# 2. macOS (darwin/arm64) 向けビルド
#    - Go コードのコンパイルエラーをチェック
make build

# 3. Linux (linux/amd64) 向けクロスコンパイル
#    - クロスコンパイル環境での互換性をチェック
make build-linux-amd64
```

### 7.2. 各コマンドで検出される問題の種類

| コマンド | 検出される問題 |
| :--- | :--- |
| `make swag` | Swagger アノテーションの誤り、`rtparam`/`rtres` 構造体の型エラー |
| `make build` | Go 構文エラー、インポートエラー、型不一致 |
| `make build-linux-amd64` | クロスコンパイル固有の問題、CGO リンクエラー |

### 7.3. よくあるエラーと対処法

1. **`Error parsing type definition 'rtparam.XXX': [field]: uint is not basic types`**
   - 原因: `swaggertype:"uint"` は無効な型指定
   - 対処: `swaggertype:"integer"` に変更する

2. **`Skipping 'rtparam.XXX', recursion detected.`**
   - 原因: 構造体定義に問題がある
   - 対処: `swaggertype` タグの型指定を見直す

3. **`undefined: rtparam.XXXParam`**
   - 原因: ハンドラーで参照している構造体が `rtparam` に存在しない
   - 対処: `rtparam` パッケージに構造体を作成する

