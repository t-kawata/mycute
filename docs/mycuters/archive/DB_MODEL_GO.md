以下は、Go言語のGorm用に書かれた構造体です。
今回のRustでの実装においては Key は BD という名前に変更してあります。

Makefile に定義してある `make gen-migration NAME="???"` というコマンドを使用してSeaORM用のマイグレーションファイルを作成して適切なカラムやインデックスの設定を書き込んだ後、`cargo run ARGS="am"` コマンドでマイグレーションを実行できるように改修の上、マイグレーションを実行しなければなりません。

また、マイグレーションにより新たなテーブルが作成されたら `make gen-entities HOST="localhost"` コマンドによってエンティティファイルを作成しなければなりません。

```go
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
	FlushDays uint    `gorm:"not null;default:0"`          // 現金プールを現金分配実行するためのサイクルとなる日数
	Badged    uint    `gorm:"not null;default:0"`          // 法人が、授与した Badge の累積数
	Rate      float64 `gorm:"type:decimal(5,5);default:0"` // 法人が、自分に所属するユーザーに対して付与する割増ポイント率
	// --------- 法人だけの項目 end
	// --------- VDR だけの項目 bgn
	TotalBadged  uint    `gorm:"not null;default:0"`          // Vdr内のBadgedの合計（Badgedが占める割合を計算するために用いる）
	TotalBadges  uint    `gorm:"not null;default:0"`          // Vdr内のバッジ保有総数（バッジ数によるポイント割増率を計算するために用いる）
	BasePoint    uint    `gorm:"not null;default:0"`          // バッジ授与時に授与者である個人に付与される基本ポイント数
	BelongRate   float64 `gorm:"type:decimal(5,5);default:0"` // 所属によるポイント割増率
	MaxWorks     uint    `gorm:"not null;default:0"`          // Vdr内の個人が就労できる最大数
	FlushFeeRate float64 `gorm:"type:decimal(5,5);default:0"` // 現金プールを現金分配実行する時に、事務コストを賄うために Pool から引かれる割合
	// --------- VDR だけの項目 end
	IsStaff bool      `gorm:"default:false;column:is_staff"`
	BgnAt   time.Time `gorm:"default:null"`
	EndAt   time.Time `gorm:"default:null"`
	ApxID   *uint     `gorm:"uniqueIndex:usr_apxid_vdrid_email_unique"`
	VdrID   *uint     `gorm:"uniqueIndex:usr_apxid_vdrid_email_unique"`
}

func (Usr) TableName() string {
	return "usrs"
}

type Crypto struct {
	gorm.Model
	Key   string `gorm:"size:50;not null;default:'';unique;index:crypto_apxid_vdrid_key_dx"`
	Value string `gorm:"size:1024;not null;default:''"`
	ApxID *uint  `gorm:"index:crypto_apxid_vdrid_key_dx"`
	VdrID *uint  `gorm:"index:crypto_apxid_vdrid_key_dx"`
}

func (Crypto) TableName() string {
	return "cryptos"
}

// 求人情報
type Job struct {
	gorm.Model
	CorpID       uint       `gorm:"not null;default:0;index:job_apxid_vdrid_corpid_idx"` // 求人を発行した法人の UsrID
	Name         string     `gorm:"size:100;not null;default:''"`
	Description  string     `gorm:"size:1000;not null;default:''"`
	Max          uint       `gorm:"not null;default:0"`            // 求人する人数
	Filled       uint       `gorm:"not null;default:0"`            // 充足した人数（当該 Job の Work の数に連動する）
	HourPrice    uint       `gorm:"not null;default:0"`            // 時給
	Requirements string     `gorm:"size:1000;not null;default:''"` // 必要条件
	Benefits     string     `gorm:"size:1000;not null;default:''"` // 働くメリット
	Location     string     `gorm:"size:128;not null;default:''"`  // 働く場所
	Phone        string     `gorm:"size:15;not null;default:''"`   // 働く場所の電話番号
	MaxBadges    uint       `gorm:"not null;default:0"`            // このお仕事でもらえる可能性のあるバッジの最大数
	WorkBgnAt    *time.Time `gorm:"default:null"`                  // 就業開始日時
	WorkEndAt    *time.Time `gorm:"default:null"`                  // 就業終了日時
	OpenAt       *time.Time `gorm:"default:null"`                  // 求人開始日時
	CloseAt      *time.Time `gorm:"default:null"`                  // 求人終了日時
	ApxID        uint       `gorm:"index:job_apxid_vdrid_corpid_idx"`
	VdrID        uint       `gorm:"index:job_apxid_vdrid_corpid_idx"`
}

func (Job) TableName() string {
	return "jobs"
}

// 個人に対して行なった求人のアプローチ情報
type Match struct {
	gorm.Model
	JobID         uint    `gorm:"not null;default:0;index:match_apxid_vdrid_jobid_idx"` // 求人の JobID
	From          uint    `gorm:"not null;default:0"`                                   // 求人を発行した法人の UsrID
	To            uint    `gorm:"not null;default:0"`                                   // 求人のアプローチを受けた個人の UsrID
	Status        uint8   `gorm:"not null;default:0"`                                   // 1:アプローチ, 2:面談設定, 3:面談実行, 4:採用成功
	PriorityScore float64 `gorm:"type:decimal(10,4);default:0"`                         // バッジ数による優先度スコア記録
	BadgeCount    uint    `gorm:"not null;default:0"`                                   // マッチング時点での個人のバッジ数記録
	MatchReason   uint8   `gorm:"not null;default:0"`                                   // マッチング理由記録（数値化した "high_badge", "random" etc.）
	ApxID         uint    `gorm:"index:match_apxid_vdrid_jobid_idx"`
	VdrID         uint    `gorm:"index:match_apxid_vdrid_jobid_idx"`
}

func (Match) TableName() string {
	return "matches"
}

// 求人のアプローチ情報のステータスを時系列で終えるようにするためのテーブル
type MatchStatus struct {
	gorm.Model
	JobID   uint  `gorm:"not null;default:0;index:matchstatus_apxid_vdrid_jobid_machid_idx"` // 求人の JobID
	MatchID uint  `gorm:"not null;default:0;index:matchstatus_apxid_vdrid_jobid_machid_idx"` // 求人のアプローチ情報の MatchID
	From    uint  `gorm:"not null;default:0"`                                                // 求人を発行した法人の UsrID
	To      uint  `gorm:"not null;default:0"`                                                // 求人のアプローチを受けた個人の UsrID
	Status  uint8 `gorm:"not null;default:0"`                                                // 1:アプローチ, 2:面談設定, 3:面談実行, 4:採用成功
	IsTmp   bool  `gorm:"not null;default:true"`                                             // 個人によって「仮予定」とした場合 true、確定したら false
	ApxID   uint  `gorm:"index:matchstatus_apxid_vdrid_jobid_machid_idx"`
	VdrID   uint  `gorm:"index:matchstatus_apxid_vdrid_jobid_machid_idx"`
}

func (MatchStatus) TableName() string {
	return "match_statuses"
}

type Work struct {
	ID            uint       `gorm:"primarykey"`
	JobID         uint       `gorm:"not null;default:0"`                                 // 求人の JobID
	MatchID       uint       `gorm:"not null;default:0"`                                 // 求人のアプローチ情報の MatchID
	From          uint       `gorm:"not null;default:0;index:work_apxid_vdrid_from_idx"` // 求人を発行した法人の UsrID
	To            uint       `gorm:"not null;default:0;index:work_apxid_vdrid_to_idx"`   // 求人のアプローチを受けた個人の UsrID（就業する人間）
	WorkBgnAt     *time.Time `gorm:"default:null"`                                       // 就業開始日時予定
	WorkEndAt     *time.Time `gorm:"default:null"`                                       // 就業終了日時予定
	RealWorkBgnAt *time.Time `gorm:"default:null"`                                       // 就業開始日時実績（タイムカードを兼ねる）
	RealWorkEndAt *time.Time `gorm:"default:null"`                                       // 就業終了日時実績（タイムカードを兼ねる）
	ApxID         uint       `gorm:"index:work_apxid_vdrid_from_idx;index:work_apxid_vdrid_to_idx"`
	VdrID         uint       `gorm:"index:work_apxid_vdrid_from_idx;index:work_apxid_vdrid_to_idx"`
}

func (Work) TableName() string {
	return "works"
}

type Belong struct {
	ID      uint       `gorm:"primarykey"`
	CorpID  uint       `gorm:"not null;default:0;index:belong_apxid_vdrid_corpid_idx"` // 所属先法人の UsrID
	UsrID   uint       `gorm:"not null;default:0;index:belong_apxid_vdrid_usrid_idx"`  // 個人の UsrID
	OpenAt  *time.Time `gorm:"default:null"`                                           // 所属開始
	CloseAt *time.Time `gorm:"default:null"`                                           // 所属終了
	ApxID   uint       `gorm:"index:belong_apxid_vdrid_usrid_idx;index:belong_apxid_vdrid_corpid_idx"`
	VdrID   uint       `gorm:"index:belong_apxid_vdrid_usrid_idx;index:belong_apxid_vdrid_corpid_idx"`
}

func (Belong) TableName() string {
	return "belongs"
}

// バッジ情報（法人だけが発行できる）
type Badge struct {
	gorm.Model
	CorpID      uint   `gorm:"not null;default:0;index:badge_apxid_vdrid_corpid_idx"` // Badgeを作った法人の UsrID
	Name        string `gorm:"size:50;not null;default:''"`
	ShortName   string `gorm:"size:20;not null;default:''"` // バッジを fe で表示する時の短い名前
	Description string `gorm:"size:255;not null;default:''"`
	ApxID       uint   `gorm:"index:badge_apxid_vdrid_corpid_idx"`
	VdrID       uint   `gorm:"index:badge_apxid_vdrid_corpid_idx"`
}

func (Badge) TableName() string {
	return "badges"
}

// バッジ認定情報
type UsrBadge struct {
	gorm.Model
	BadgeID uint   `gorm:"not null;default:0"`
	CorpID  uint   `gorm:"not null;default:0;index:usrbadge_apxid_vdrid_corpid_idx"` // Badgeを作った法人の UsrID
	From    uint   `gorm:"not null;default:0"`                                       // Badgeをあげた認定マン UsrID
	To      uint   `gorm:"not null;default:0"`                                       // Badgeをもらったユーザー UsrID
	Title   string `gorm:"size:100;not null;default:''"`                             // メッセージの件名
	Message string `gorm:"size:500;not null;default:''"`                             // メッセージ本体
	Type    uint8  `gorm:"not null;default:0"`                                       // 1: 法人による授与, 2: 個人による授与 （バッジをあげた側の Usr のタイプ）
	ApxID   uint   `gorm:"index:usrbadge_apxid_vdrid_corpid_idx"`
	VdrID   uint   `gorm:"index:usrbadge_apxid_vdrid_corpid_idx"`
}

func (UsrBadge) TableName() string {
	return "usr_badges"
}

// ポイント履歴
type Point struct {
	gorm.Model
	BadgeID uint `gorm:"not null;default:0;index:point_apxid_vdrid_badgeid_idx"`
	CorpID  uint `gorm:"not null;default:0;index:point_apxid_vdrid_corpid_idx"` // Badgeを作った法人の UsrID
	From    uint `gorm:"not null;default:0"`                                    // Badgeをあげた認定マン UsrID（個人）
	To      uint `gorm:"not null;default:0;index:point_apxid_vdrid_to_idx"`     // Badgeをもらったユーザー UsrID（個人）
	Point   uint `gorm:"not null;default:0"`                                    // 付与された基本ポイント
	Extra   uint `gorm:"not null;default:0"`                                    // 付与された割増分のポイント
	ApxID   uint `gorm:"index:point_apxid_vdrid_to_idx;index:point_apxid_vdrid_corpid_idx;index:point_apxid_vdrid_badgeid_idx"`
	VdrID   uint `gorm:"index:point_apxid_vdrid_to_idx;index:point_apxid_vdrid_corpid_idx;index:point_apxid_vdrid_badgeid_idx"`
}

func (Point) TableName() string {
	return "points"
}

type Payment struct {
	gorm.Model
	CorpID uint   `gorm:"not null;default:0;index:payment_apxid_vdrid_corpid_idx"` // 支払った法人 UsrID
	Type   uint8  `gorm:"not null;default:0"`                                      // 1:面談フィー, 2:採用紹介料, etc.
	Amount uint   `gorm:"not null;default:0"`                                      // 支払金額
	Fee    uint   `gorm:"not null;default:0"`                                      // 運営費控除分
	Net    uint   `gorm:"not null;default:0"`                                      // プール流入額（Amount - Fee）
	Note   string `gorm:"size:255;not null;default:''"`                            // メモ
	ApxID  uint   `gorm:"index:payment_apxid_vdrid_corpid_idx"`
	VdrID  uint   `gorm:"index:payment_apxid_vdrid_corpid_idx"`
}

func (Payment) TableName() string {
	return "payments"
}

// Vdr単位の現金プール
type Pool struct {
	gorm.Model
	ApxID    uint `gorm:"index:pool_apxid_vdrid_idx"`
	VdrID    uint `gorm:"not null;default:0;index:pool_apxid_vdrid_idx"` // この現金プールの所属Vdr
	Remain   uint `gorm:"not null;default:0"`                            // 現在の現金プール残高
	TotalIn  uint `gorm:"not null;default:0"`                            // 過去全期間の流入総額
	TotalOut uint `gorm:"not null;default:0"`                            // 過去全期間の分配総額
}

func (Pool) TableName() string {
	return "pools"
}

// Vdr単位の現金プールの分配実行履歴
type Flush struct {
	gorm.Model
	PoolID       uint    `gorm:"not null;default:0;index:flush_apxid_vdrid_poolid_idx"` // 分配元の PoolID
	Total        uint    `gorm:"not null;default:0"`                                    // 分配実行額（多くの場合、その時点の Pool.Remain）
	FlushFeeRate float64 `gorm:"type:decimal(5,5);default:0"`                           // 分配実行時の事務費用割引率記録
	Points       uint    `gorm:"not null;default:0"`                                    // 分配実行時の Vdr 内全ユーザーの Usr.Points 合計
	ApxID        uint    `gorm:"index:flush_apxid_vdrid_poolid_idx"`
	VdrID        uint    `gorm:"not null;default:0;index:flush_apxid_vdrid_poolid_idx"`
}

func (Flush) TableName() string {
	return "flushes"
}

// Usr（個人）単位の現金プール分配実行履歴
type Payout struct {
	gorm.Model
	PoolID  uint    `gorm:"not null;default:0"`                                    // 分配元の PoolID
	FlushID uint    `gorm:"not null;default:0"`                                    // 分配元の FlushID
	UsrID   uint    `gorm:"not null;default:0;index:payout_apxid_vdrid_usrid_idx"` // 還元対象 個人 UsrID
	Points  uint    `gorm:"not null;default:0"`                                    // 分配時の個人ポイント残高（Usr.Points）
	Share   float64 `gorm:"type:decimal(5,5);default:0"`                           // 分配総額に対する自分の取り分割合（Payout.Points / Flush.Points）
	Amount  uint    `gorm:"not null;default:0"`                                    // 現金化された分配金額
	ApxID   uint    `gorm:"index:payout_apxid_vdrid_usrid_idx"`
	VdrID   uint    `gorm:"index:payout_apxid_vdrid_usrid_idx"`
}

func (Payout) TableName() string {
	return "payouts"
}
```