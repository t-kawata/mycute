package config

const VERSION = "v0.2.1"

const TIME_ZONE = "Asia/Tokyo"

const DEFAULT_SKEY = "iJsfNZwZgc4VvDZyvhebvjVz/+J3IkKpvkb++HYc39Y/="

const DEFAULT_CRYPTO_KEY = "o?Hb*S+!iT-p-wYeMFuVJiLWGud-_tez"

const PW_REGEXP = "^[0-9a-zA-Z!?-_@]{6,20}$"

const REST_PORT = 8888

const NODES_DB_NAME = "mycute"

const S3C_LOCAL_ROOT = "/home/asterisk/s3" // s3c をローカルで使用する時のファイル保管ルートディレクトリ

const DL_LOCAL_ROOT = "/home/asterisk/dl" // s3c で Down を実行する時にダウンロード先になるルートディレクトリ

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
