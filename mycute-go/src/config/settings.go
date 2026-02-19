package config

const VERSION = "v0.3.8"

const TIME_ZONE = "Asia/Tokyo"

const DEFAULT_SKEY = "iJsfNZwZgc4VvDZyvhebvjVz/+J3IkKpvkb++HYc39Y/="

const DEFAULT_CRYPTO_KEY = "o?Hb*S+!iT-p-wYeMFuVJiLWGud-_tez"

const PW_REGEXP = "^[0-9a-zA-Z!?-_@]{6,20}$"

const REST_PORT = 8888

const NODES_DB_NAME = "mycute"

const S3C_LOCAL_ROOT = "/home/asterisk/s3" // s3c をローカルで使用する時のファイル保管ルートディレクトリ

const DL_LOCAL_ROOT = "/home/asterisk/dl" // s3c で Down を実行する時にダウンロード先になるルートディレクトリ

// MDL_REDUCTION_BENEFIT は、ノード削除によるストレージ削減の固定ベネフィット値です。
const MDL_REDUCTION_BENEFIT float64 = 0.1

// MDL_K_NEIGHBORS は、MDL判定時に考慮する近傍ノード数です。
const MDL_K_NEIGHBORS int = 5

// DEFAULT_HALF_LIFE_DAYS は、エッジの価値が半減するデフォルト日数です。
const DEFAULT_HALF_LIFE_DAYS float64 = 30.0

// DEFAULT_PRUNE_THRESHOLD は、Thickness閾値のデフォルト値です。
const DEFAULT_PRUNE_THRESHOLD float64 = 0.1

// DEFAULT_MIN_SURVIVAL_PROTECTION_HOURS は、新規知識の最低生存保護期間（時間）のデフォルト値です。
const DEFAULT_MIN_SURVIVAL_PROTECTION_HOURS float64 = 72.0

// DEFAULT_THICKNESS_THRESHOLD は、Query時のエッジ足切り閾値のデフォルト値です。
const DEFAULT_THICKNESS_THRESHOLD float64 = 0.3

// METABOLISM_PAGE_SIZE は、Metabolism処理時にページングで取得するノード数です。
const METABOLISM_PAGE_SIZE int = 1000

// METABOLISM_OVERLAP_SIZE は、refineConflicts処理時のオーバーラップノード数です。
// ページ境界での矛盾見逃しを防ぐために使用されます。
const METABOLISM_OVERLAP_SIZE int = 100

type DbInfo struct {
	Host     string
	Port     string
	Username string
	Password string
}

type Env struct {
	Name         string
	Empty        bool
	AsteriskRwDb DbInfo
	NodesMDb     DbInfo
	NodesRDbs    []DbInfo
}

var (
	LocalEnv Env = Env{
		Name:         "local",
		Empty:        false,
		AsteriskRwDb: DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesMDb:     DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesRDbs: []DbInfo{
			{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		},
	}
	DevEnv Env = Env{
		Name:         "dev",
		Empty:        false,
		AsteriskRwDb: DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesMDb:     DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesRDbs: []DbInfo{
			{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		},
	}
	StgEnv Env = Env{
		Name:         "stg",
		Empty:        false,
		AsteriskRwDb: DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesMDb:     DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesRDbs: []DbInfo{
			{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		},
	}
	ProdEnv Env = Env{
		Name:         "prod",
		Empty:        false,
		AsteriskRwDb: DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesMDb:     DbInfo{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		NodesRDbs: []DbInfo{
			{Host: "127.0.0.1", Port: "3306", Username: "asterisk", Password: "yu51043chie3"},
		},
	}
)

func GetEnv(e string) *Env {
	switch e {
	case LocalEnv.Name:
		return &LocalEnv
	case DevEnv.Name:
		return &DevEnv
	case StgEnv.Name:
		return &StgEnv
	case ProdEnv.Name:
		return &ProdEnv
	default:
		return &Env{Empty: true}
	}
}
