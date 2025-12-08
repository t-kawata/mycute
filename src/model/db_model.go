package model

import (
	"time"

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
