# Phase-11A: Create Cube API Implementation

## 1. 概要 (Overview)
`POST /v1/cubes/create` エンドポイントを実装し、ユーザーが新しい空の Cube を作成できるようにします。
ここでは、Cube システムの根幹となるデータベースモデル定義 (`Cube`, `CubeStat` 等) と、初期 creation ロジックを実装します。

## 2. 実装要件 (Requirements)
*   **エンドポイント**: `POST /v1/cubes/create`
*   **権限**: `usrtype.USR` のみ実行可能。
*   **データパーティショニング**: DB操作時は必ず `ApxID`, `VdrID` を指定して行うこと。
*   **物理配置**: `filepath.Join(os.Getenv("DB_DIR_PATH"), fmt.Sprintf("%d-%d-%d/%s.db", apxID, vdrID, usrID, cube.UUID))`
*   **初期権限**: 作成者は「神権限 (Limit = 0: 無制限)」を持つ。

> [!NOTE]
> **MemoryGroup 関連**
> 
> `create` エンドポイントでは `memory_group` パラメータは不要です。
> MemoryGroup は Absorb/Memify/Search 時に指定されるもので、Cube 作成時には存在しません。
> MemoryGroup の詳細は `docs/DIRECTONS-PHASE-11.md` セクション 2.4 を参照。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: モデル定義 (`model/db_model.go`)

**【解説】**
`gorm.io/datatypes` を使用する理由は、MySQL の JSON 型を Go の構造体やマップとして扱いやすくするためです。
`CubePermission` は JSON として保存されますが、Go 上では構造体として扱うことで、フィールド名のタイポを防ぎ、型安全に数値を扱えます。
各フィールドには、開発者が迷わないよう詳細な日本語コメントを付与します。

**【実装コードスニペット】**

```go
package model

import (
	"time"
	"gorm.io/datatypes"
)

// CubePermission は Cube の permissions カラム（JSON）に格納される詳細権限情報です。
// 回数制限のロジック:
//   0: 無制限 (Unlimited)。何度でも実行可能。
// > 0: 残り回数 (Remaining)。実行ごとに減算。
// < 0: 禁止/終了 (Forbidden/Finished)。実行不可。
type CubePermission struct {
	ExportLimit int `json:"export_limit"` // エクスポート可能回数
	RekeyLimit  int `json:"rekey_limit"`  // 鍵更新可能回数
	GenKeyLimit int `json:"genkey_limit"` // 子鍵発行可能回数
	AbsorbLimit int `json:"absorb_limit"` // 知識取込可能回数
	MemifyLimit int `json:"memify_limit"` // 自己強化可能回数
	SearchLimit int `json:"search_limit"` // 検索利用可能回数

	AllowStats  bool `json:"allow_stats"`  // 統計情報の閲覧可否 (true: 許可)
	// AllowDelete は廃止されました（所有者は常に削除可能）

	// Config制限など拡張用
	// Memify 実行時の epoch 数などの上限を設定します。nil または空マップの場合はデフォルト上限が適用されます。
	MemifyConfigLimit map[string]interface{} `json:"memify_config_limit"`
	// Search 実行時に指定可能な search_type のリスト。nil または空リストの場合はデフォルトのみ許可等のポリシーに従います。
	SearchTypeLimit   []string               `json:"search_type_limit"`
}

type Cube struct {
	ID          uint   `gorm:"primarykey"` // 内部ID
	UUID        string `gorm:"size:36;index:cube_apxid_vdrid_uuid_idx"` // Cubeの一意な識別子 (UUID)
	UsrID       string `gorm:"size:36;index:cube_apxid_vdrid_usrid_idx"` // 所有者のユーザーID
	Name        string `gorm:"size:50;not null;default:''"` // Cubeの表示名
	Description string `gorm:"size:255;not null;default:''"` // Cubeの説明文

	// 制限情報。ImportされたCubeの場合に鍵の情報がセットされます。
	// Create時は nil (無期限) です。
	ExpireAt    *time.Time `gorm:"default:null"` 
	
	// 詳細な権限設定を保持するJSONカラム。
	// datatypes.JSON を使用することで GORM が自動的に Serialize/Deserialize します。
	Permissions datatypes.JSON `gorm:"default:null"` 

	ApxID       uint   `gorm:"index:cube_apxid_vdrid_usrid_idx;index:cube_apxid_vdrid_uuid_idx"` // アプリケーションID (パーティションキー)
	VdrID       uint   `gorm:"index:cube_apxid_vdrid_usrid_idx;index:cube_apxid_vdrid_uuid_idx"` // ベンダーID (パーティションキー)
	CreatedAt   time.Time
	UpdatedAt   time.Time
}

// CubeModelStat は、Cube の使用モデルごとのトークン消費量を記録します。
// **MemoryGroup を最上位の粒度として含む**ことで、「どの専門分野に」「どのモデルが」「どれだけ使われたか」を把握できます。
// Search と Training (Absorb/Memify) を区別して集計します。
// Export 時に stats_usage.json として書き出され、Import 時に復元されます。
//
// 統計階層: MemoryGroup → ActionType → ModelName
type CubeModelStat struct {
	ID        uint   `gorm:"primarykey"`
	CubeID    uint   `gorm:"index:model_stat_cube_idx;not null"`
	
	MemoryGroup string `gorm:"size:64;not null;index:idx_cube_mg_model_action,unique"` // e.g. "legal_expert", "medical_expert"
	ModelName   string `gorm:"size:100;not null;index:idx_cube_mg_model_action,unique"` // e.g. "gpt-4", "text-embedding-3-small"
	ActionType  string `gorm:"size:20;not null;index:idx_cube_mg_model_action,unique"`  // "search" or "training"
	
	InputTokens  int64 `gorm:"default:0"` // 累積入力トークン
	OutputTokens int64 `gorm:"default:0"` // 累積出力トークン

	ApxID     uint `gorm:"index:model_stat_apxid_vdrid_idx;not null"`
	VdrID     uint `gorm:"index:model_stat_apxid_vdrid_idx;not null"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

// CubeContributor は、Cube の成長（Training）に貢献したユーザーとモデルごとの記録です。
// **MemoryGroup を最上位の粒度として含む**ことで、「どの専門分野に」「誰が」「どれだけ貢献したか」を把握できます。
// Search の使用量はここには含めません（貢献ではなく利用のため）。
// Export 時に stats_contributors.json として書き出され、Import 時に復元されます。
//
// 統計階層: MemoryGroup → ContributorName → ModelName
type CubeContributor struct {
	ID        uint   `gorm:"primarykey"`
	CubeID    uint   `gorm:"index:contrib_cube_idx;not null"`
	
	MemoryGroup     string `gorm:"size:64;not null;index:idx_cube_mg_contrib_model,unique"` // e.g. "legal_expert"
	ContributorName string `gorm:"size:100;not null;index:idx_cube_mg_contrib_model,unique"` // 貢献者名 (Usr.Name の平文)
	ModelName       string `gorm:"size:100;not null;index:idx_cube_mg_contrib_model,unique"` // 使用モデル
	
	InputTokens  int64 `gorm:"default:0"` // 累積入力トークン
	OutputTokens int64 `gorm:"default:0"` // 累積出力トークン

	ApxID     uint `gorm:"index:contrib_apxid_vdrid_idx;not null"`
	VdrID     uint `gorm:"index:contrib_apxid_vdrid_idx;not null"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

// CubeLineage は Cube の系譜（祖先情報）を保持します。
// Export 時に現在の情報が metadata.json の一部として引き継がれます。
type CubeLineage struct {
	ID            uint   `gorm:"primarykey"`
	CubeID        uint   `gorm:"index:lineage_cube_idx;not null"`
	
	AncestorUUID  string `gorm:"size:36;not null"` // 祖先のUUID
	AncestorOwner string `gorm:"size:50;not null"` // 祖先の所有者名
	ExportedAt    int64  `gorm:"not null"`         // エクスポートされた時刻 (UnixMilli)
	Generation    int    `gorm:"not null"`         // 世代 (1始まり)

	ApxID     uint `gorm:"index:lineage_apxid_vdrid_idx;not null"`
	VdrID     uint `gorm:"index:lineage_apxid_vdrid_idx;not null"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

// Export は Cube がファイルとして書き出された記録です。
// GenKey の対象として使用されます。
type Export struct {
	ID      uint   `gorm:"primarykey"`
	CubeID  uint   `gorm:"index:export_cube_idx;not null"` // どのCubeから作られたか
	NewUUID string `gorm:"size:36;index:export_new_uuid_idx;not null"` // 書き出されたファイルのUUID
	Hash    string `gorm:"size:64;not null"` // SHA256ハッシュ

	ApxID     uint `gorm:"index:export_apxid_vdrid_idx;not null"`
	VdrID     uint `gorm:"index:export_apxid_vdrid_idx;not null"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

// BurnedKey は使用済みの鍵（Burn-on-use）を記録します。
// KeyID の一意性を保証するために重要です。
type BurnedKey struct {
	ID              uint   `gorm:"primarykey"`
	KeyID           string `gorm:"size:36;index:burned_key_id_idx;unique"` // 使用された鍵ID (Unique)
	
	UsedByUsrID     string `gorm:"size:36;not null"` // 誰が使ったか
	UsedForCubeUUID string `gorm:"size:36;not null"` // どのCubeに使ったか（Import時はNewUUID, Rekey時はTargetUUID）
	ActionType      string `gorm:"size:20;not null"` // "import" or "rekey"

	ApxID     uint `gorm:"index:burned_apxid_vdrid_idx;not null"`
	VdrID     uint `gorm:"index:burned_apxid_vdrid_idx;not null"`
	CreatedAt time.Time
	UpdatedAt time.Time
}
```

### Step 2: リクエスト構造体 (`rtreq` / `rtparam`)

**【解説】**
ユーザーからの入力は `Name` と `Description` のみです。
UUID や初期権限はサーバー側で決定するため、リクエストには含めません。
`Name` は必須項目とします。

**【実装コードスニペット】**

```go
// rtparam/cubes_param.go
type CreateCubeBody struct {
	Name        string `json:"name" binding:"required" example:"My Cube"`
	Description string `json:"description" example:"Knowledge about Go"`
}
```

### Step 3: ビジネスロジック (`rtbl/cubes_bl.go`)

**【解説】**
1. UUID生成: `google/uuid` 等を使用。
2. ディレクトリ作成: 指定されたパスルールに従い、`os.MkdirAll` で作成。失敗時はエラー。
3. 初期化: `cuber.InitNewDB(path)` (仮) を呼び出し、KuzuDB の初期セットアップを行う。
4. DB保存: `Cube` レコードを作成。`Permissions` は初期値として「無制限 (全て0)」の JSON をセットします。

**【実装コードスニペット】**

```go
func CreateCube(ctx context.Context, apxID, vdrID uint, usrID string, body rtparam.CreateCubeBody) (*rtres.CubeRes, error) {
	// 1. UUID 生成
	newUUID := uuid.NewString()

	// 2. パス決定
	dbBaseDir := os.Getenv("DB_DIR_PATH")
	// パス形式: db_dir/apx-vdr-usr/uuid.db
	// ディレクトリ自体が .db という名称になる KuzuDB の仕様を想定
	cubePath := filepath.Join(dbBaseDir, fmt.Sprintf("%d-%d-%d", apxID, vdrID, usrID), newUUID+".db")

	// 3. 親ディレクトリ作成
	if err := os.MkdirAll(filepath.Dir(cubePath), 0755); err != nil {
		return nil, err
	}

	// 4. KuzuDB 初期化 (pkg/cuber 呼び出し)
	// cuber パッケージ側で実際のディレクトリ作成とスキーマ初期化を行う想定
	if err := cuber.CreateCubeDB(cubePath); err != nil {
		return nil, err
	}

	// 5. 初期権限設定 (All Unlimited)
	initialPerm := model.CubePermission{
		ExportLimit: 0, RekeyLimit: 0, GenKeyLimit: 0,
		AbsorbLimit: 0, MemifyLimit: 0, SearchLimit: 0,
		AllowStats: true, 
		// AllowDelete は廃止
	}
	permJSON, _ := json.Marshal(initialPerm)

	// 6. DBレコード作成
	newCube := model.Cube{
		UUID:        newUUID,
		UsrID:       usrID,
		Name:        body.Name,
		Description: body.Description,
		ExpireAt:    nil, // 無期限
		Permissions: datatypes.JSON(permJSON),
		ApxID:       apxID,
		VdrID:       vdrID,
	}

	if err := db.Create(&newCube).Error; err != nil {
		// DB保存失敗時は作成した物理ファイルを削除してゴミを残さない (Cleanup)
		os.RemoveAll(cubePath)
		return nil, err
	}

	return rtres.ToCubeRes(newCube), nil
}
```

### Step 4: ハンドラー (`rthandler`)

**【解説】**
標準的な Handler 実装です。`rtmiddleware` からコンテキスト変数 (`UsrID` 等) を取得し、BL を呼び出します。
エラーハンドリングは `rtres.ErrorResponse` を使用します。

---
**注意点**:
*   `db_model.go` で `import "gorm.io/datatypes"` が必要になるため、`go get gorm.io/datatypes` を忘れないこと（自動で行われるはずですが）。
*   パス区切り文字やパーミッション (0755) は環境に合わせて適切に。
