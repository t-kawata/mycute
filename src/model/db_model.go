package model

import (
	"time"

	"gorm.io/datatypes"
	"gorm.io/gorm"
)

type IDs struct {
	ApxID *uint
	VdrID *uint
	UsrID *uint
}

type Key struct {
	gorm.Model
	Hash  string    `gorm:"size:60;not null;default:''"`
	BgnAt time.Time `gorm:"default:null"`
	EndAt time.Time `gorm:"default:null"`
}

func (Key) TableName() string {
	return "keys"
}

// Usr構造体（法人・個人共通）
// APX: 頂点となるシステム管理者（ApxID == nil && VdrID == nil）
// VDR: サービスベンダー（ApxID > 0 && VdrID == nil）
// USR: ユーザー（法人 or 個人）（ApxID > 0 && VdrID > 0）
// Badged:
//   - Vdr内全法人の合計に対する割合でゲーム優位性が変わる
//   - 「採用アプローチ個人母数」に対して、
//   - Badged率を何らかの形で掛け算することで、
//   - 実際のアプローチ数が具体的に決定されて法人に表示される仕組み
//   - 「法人が積極的にバッジを発行し、積極的に授与する」力学発生のため
type Usr struct {
	gorm.Model
	Name   string `gorm:"size:50;not null;default:''"`
	Type   uint8  `gorm:"not null;default:0"` // 1: 法人, 2: 個人
	Points uint   `gorm:"not null;default:0"` // 現在の保有ポイント（現金変換してない分）
	SumP   uint   `gorm:"not null;default:0"` // 過去に現金変換したポイントの累積値
	SumC   uint   `gorm:"not null;default:0"` // 過去に現金変換した現金の累積値
	// --------- シンプル認証用 bgn
	Email    string `gorm:"size:100;not null;default:'';uniqueIndex:usr_apxid_vdrid_email_unique"` // ログインID (ZITADEL連携時も使用)
	Password string `gorm:"size:255;not null;default:''"`                                          // パスワードハッシュ (ZITADEL連携時は空文字可、フォールバック用)
	// --------- シンプル認証用 end
	// --------- ZITADEL連携用 bgn
	ZitadelID     string `gorm:"size:100;index"` // ZITADELのsub (ユーザーID)
	EmailVerified bool   `gorm:"default:false"`  // メール検証済みフラグ
	// --------- ZITADEL連携用 end
	// --------- 法人だけの項目 bgn
	// --------- 法人だけの項目 end
	IsStaff bool      `gorm:"default:false;column:is_staff"`
	BgnAt   time.Time `gorm:"default:null"`
	EndAt   time.Time `gorm:"default:null"`
	ApxID   *uint     `gorm:"uniqueIndex:usr_apxid_vdrid_email_unique"`
	VdrID   *uint     `gorm:"uniqueIndex:usr_apxid_vdrid_email_unique"`
}

func (Usr) TableName() string {
	return "usrs"
}

// ========================================
// Cube 関連モデル (Phase-11A)
// ========================================

// CubePermission は Cube の permissions カラム（JSON）に格納される詳細権限情報です。
// 回数制限のロジック:
//
//	0: 無制限 (Unlimited)。何度でも実行可能。
//
// > 0: 残り回数 (Remaining)。実行ごとに減算。
// < 0: 禁止/終了 (Forbidden/Finished)。実行不可。
type CubePermission struct {
	ExportLimit int `json:"export_limit"` // エクスポート可能回数
	RekeyLimit  int `json:"rekey_limit"`  // 鍵更新可能回数
	GenKeyLimit int `json:"genkey_limit"` // 子鍵発行可能回数
	AbsorbLimit int `json:"absorb_limit"` // 知識取込可能回数
	MemifyLimit int `json:"memify_limit"` // 自己強化可能回数
	QueryLimit  int `json:"query_limit"`  // クエリ利用可能回数

	AllowStats bool `json:"allow_stats"` // 統計情報の閲覧可否 (true: 許可)

	// Memify 実行時の epoch 数などの上限を設定します。
	MemifyConfigLimit map[string]any `json:"memify_config_limit"`
	// Query 実行時に指定可能な query_type のリスト。
	QueryTypeLimit []string `json:"query_type_limit"`
}

// Cube は Cuber システムの知識ベースを表します。
type Cube struct {
	ID          uint   `gorm:"primarykey"`
	UUID        string `gorm:"size:36;index:cube_apxid_vdrid_uuid_idx"`
	UsrID       uint   `gorm:"size:36;index:cube_apxid_vdrid_usrid_idx"` // Cubeの現在の所有者UsrID
	Name        string `gorm:"size:50;not null;default:''"`
	Description string `gorm:"size:255;not null;default:''"`

	ExpireAt    *time.Time     `gorm:"default:null"`
	Permissions datatypes.JSON `gorm:"default:null"`

	SourceExportID *uint `gorm:"default:null"` // Link to Export record for ReKey

	ApxID     uint `gorm:"index:cube_apxid_vdrid_usrid_idx;index:cube_apxid_vdrid_uuid_idx"`
	VdrID     uint `gorm:"index:cube_apxid_vdrid_usrid_idx;index:cube_apxid_vdrid_uuid_idx"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

func (Cube) TableName() string {
	return "cubes"
}

// CubeModelStat は Cube のモデルごとのトークン消費量を記録します。
// MemoryGroup を最上位の粒度として含み、「どの専門分野に」「どのモデルで」「どれだけ使われたか」を把握できます。
type CubeModelStat struct {
	ID     uint `gorm:"primarykey" json:"id"`
	CubeID uint `gorm:"index:model_stat_cube_idx;not null;index:idx_cube_mg_model_action,unique,priority:1" json:"cube_id"`

	MemoryGroup string `gorm:"size:64;not null;index:idx_cube_mg_model_action,unique,priority:2" json:"memory_group"` // e.g. "legal_expert"
	ModelName   string `gorm:"size:100;not null;index:idx_cube_mg_model_action,unique,priority:3" json:"model_name"`
	ActionType  string `gorm:"size:6;not null;index:idx_cube_mg_model_action,unique,priority:4" json:"action_type"` // "absorb", "memify", "query"

	InputTokens  int64 `gorm:"default:0" json:"input_tokens"`
	OutputTokens int64 `gorm:"default:0" json:"output_tokens"`

	ApxID     uint      `gorm:"index:model_stat_apxid_vdrid_idx;not null" json:"apx_id"`
	VdrID     uint      `gorm:"index:model_stat_apxid_vdrid_idx;not null" json:"vdr_id"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

func (CubeModelStat) TableName() string {
	return "cube_model_stats"
}

// CubeContributor は Cube の成長に貢献したユーザーの記録です。
// MemoryGroup を最上位の粒度として含み、「どの専門分野に」「誰が」「どれだけ貢献したか」を把握できます。
type CubeContributor struct {
	ID     uint `gorm:"primarykey" json:"id"`
	CubeID uint `gorm:"index:contrib_cube_idx;not null;index:idx_cube_mg_contrib_model,unique,priority:1" json:"cube_id"`

	MemoryGroup     string `gorm:"size:64;not null;index:idx_cube_mg_contrib_model,unique,priority:2" json:"memory_group"` // e.g. "legal_expert"
	ContributorName string `gorm:"size:100;not null;index:idx_cube_mg_contrib_model,unique,priority:3" json:"contributor_name"`
	ModelName       string `gorm:"size:100;not null;index:idx_cube_mg_contrib_model,unique,priority:4" json:"model_name"`

	InputTokens  int64 `gorm:"default:0" json:"input_tokens"`
	OutputTokens int64 `gorm:"default:0" json:"output_tokens"`

	ApxID     uint      `gorm:"index:contrib_apxid_vdrid_idx;not null" json:"apx_id"`
	VdrID     uint      `gorm:"index:contrib_apxid_vdrid_idx;not null" json:"vdr_id"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

func (CubeContributor) TableName() string {
	return "cube_contributors"
}

// CubeLineage は Cube の系譜（祖先情報）を保持します。
type CubeLineage struct {
	ID     uint `gorm:"primarykey" json:"id"`
	CubeID uint `gorm:"index:lineage_cube_idx;not null" json:"cube_id"`

	AncestorUUID  string `gorm:"size:36;not null" json:"ancestor_uuid"`
	AncestorOwner string `gorm:"size:50;not null" json:"ancestor_owner"`
	ExportedAt    int64  `gorm:"not null" json:"exported_at"`
	Generation    int    `gorm:"not null" json:"generation"`

	ApxID     uint      `gorm:"index:lineage_apxid_vdrid_idx;not null" json:"apx_id"`
	VdrID     uint      `gorm:"index:lineage_apxid_vdrid_idx;not null" json:"vdr_id"`
	CreatedAt time.Time `json:"created_at"`
	UpdatedAt time.Time `json:"updated_at"`
}

func (CubeLineage) TableName() string {
	return "cube_lineages"
}

// Export は Cube がファイルとして書き出された記録です。
type Export struct {
	ID      uint   `gorm:"primarykey"`
	CubeID  uint   `gorm:"index:export_cube_idx;not null"`
	NewUUID string `gorm:"size:36;index:export_new_uuid_idx;not null"`
	Hash    string `gorm:"size:64;not null"`

	PrivateKey string `gorm:"type:text"` // RSA Private Key (PEM)

	ApxID     uint `gorm:"index:export_apxid_vdrid_idx;not null"`
	VdrID     uint `gorm:"index:export_apxid_vdrid_idx;not null"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

func (Export) TableName() string {
	return "exports"
}

// BurnedKey は使用済みの鍵（Burn-on-use）を記録します。
type BurnedKey struct {
	ID    uint   `gorm:"primarykey"`
	KeyID string `gorm:"size:36;index:burned_key_id_idx;unique"`

	UsedByUsrID     string `gorm:"size:36;not null"`
	UsedForCubeUUID string `gorm:"size:36;not null"`
	BurnType        string `gorm:"size:6;not null"` // "import" or "rekey"

	ApxID     uint `gorm:"index:burned_apxid_vdrid_idx;not null"`
	VdrID     uint `gorm:"index:burned_apxid_vdrid_idx;not null"`
	CreatedAt time.Time
	UpdatedAt time.Time
}

func (BurnedKey) TableName() string {
	return "burned_keys"
}
