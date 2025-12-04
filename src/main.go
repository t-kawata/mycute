package main

import (
	"context"
	"database/sql"
	_ "embed"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"

	cozo "github.com/cozodb/cozo-lib-go"
	_ "github.com/duckdb/duckdb-go/v2"
	"github.com/joho/godotenv"
	"github.com/t-kawata/mycute/pkg/cognee"
)

//go:embed pkg/cognee/db/duckdb/extensions/v1.4.2/darwin_arm64/vss.duckdb_extension
var duckDbVssDarwinArm64 []byte

//go:embed pkg/cognee/db/duckdb/extensions/v1.4.2/linux_amd64/vss.duckdb_extension
var duckDbVssLinuxAmd64 []byte

func main() {
	/*********************************************
	 * 環境変数の読み込み（.envがあれば）
	 ********************************************/
	godotenv.Load()

	/*********************************************
	 * DEBUGモードの設定
	 ********************************************/
	if os.Getenv("COGNEE_DEBUG") == "true" {
		log.SetFlags(log.LstdFlags | log.Lshortfile)
	}

	/*********************************************
	 * DuckDB の初期化
	 ********************************************/
	// DuckDBデータディレクトリの設定
	duckdbDataDir := os.Getenv("DUCKDB_DATA_DIR")
	if duckdbDataDir == "" {
		duckdbDataDir = "./data"
	}
	// DuckDBの初期化
	duckdb, err := sql.Open("duckdb", fmt.Sprintf("%s%s", filepath.Join(duckdbDataDir, "vectors.duckdb"), "?access_mode=READ_WRITE"))
	if err != nil {
		log.Fatalf("Failed to open DuckDB: %v", err)
	}
	defer duckdb.Close()
	// DuckDB VSS拡張のロード
	extensionPath, err := getDuckDBVSSExtensionPath() // プラットフォームに応じた拡張ファイルパスを取得
	if err != nil {
		log.Fatalf("Failed to get DuckDB VSS extension path: %v", err)
	}
	defer os.Remove(extensionPath)                                                 // 終了時に一時ファイルを削除
	query := fmt.Sprintf("INSTALL '%s'; LOAD '%s';", extensionPath, extensionPath) // ローカルの拡張バイナリを直接ロード
	if _, err = duckdb.Exec(query); err != nil {
		log.Fatalf("Failed to load DuckDB VSS extension: %v", err)
	}
	log.Println("DuckDB VSS extension loaded successfully")

	/*********************************************
	 * CozoDB の初期化
	 ********************************************/
	// CozoDBデータディレクトリの設定
	cozoDBDataDir := os.Getenv("COZODB_DATA_DIR")
	if cozoDBDataDir == "" {
		cozoDBDataDir = "./data"
	}
	// RocksDB バックエンド + 永続化
	cozodb, err := cozo.New("rocksdb", filepath.Join(cozoDBDataDir, "graph.cozodb"), nil)
	if err != nil {
		log.Fatalf("Failed to open CozoDB: %v", err)
	}
	defer cozodb.Close()

	/*********************************************
	 * テスト
	 ********************************************/
	ctx := context.Background()

	// Add機能のテスト
	// Ensure test_data directory exists
	if _, err := os.Stat("test_data"); os.IsNotExist(err) {
		os.Mkdir("test_data", 0755)
	}
	// Create sample file if it doesn't exist
	if _, err := os.Stat("test_data/sample.txt"); os.IsNotExist(err) {
		os.WriteFile("test_data/sample.txt", []byte("これはテスト用のサンプルテキストです。"), 0644)
	}

	err = cognee.Add(ctx, []string{"test_data/sample.txt"}, "test_dataset", "user1")
	if err != nil {
		log.Fatalf("❌ Add failed: %v", err)
	}

	log.Println("✅ Milestone 1: Add機能が正常に動作しました")

	// Cognify機能のテスト
	log.Println("🧠 Step 2: グラフ構築...")
	if err := cognee.Cognify(ctx, "test_dataset", "user1"); err != nil {
		log.Fatalf("❌ Cognify failed: %v", err)
	}

	log.Println("✅ Milestone 2: Cognify機能が正常に動作しました")

	// Search機能のテスト
	log.Println("🔍 Step 3: 検索実行...")
	result, err := cognee.Search(ctx, "サンプルテキストについて教えてください", cognee.SearchTypeGraphCompletion, "user1")
	if err != nil {
		log.Fatalf("❌ Search failed: %v", err)
	}

	log.Printf("✅ 検索結果:\n%s\n", result)
	log.Println("🎉 Milestone 3: 全機能が正常に動作しました！")
}

func getDuckDBVSSExtensionPath() (string, error) {
	platform := fmt.Sprintf("%s-%s", runtime.GOOS, runtime.GOARCH)

	var data []byte
	switch platform {
	case "darwin-arm64":
		data = duckDbVssDarwinArm64
	case "linux-amd64":
		data = duckDbVssLinuxAmd64
	default:
		return "", fmt.Errorf("unsupported platform: %s", platform)
	}

	// 一時ファイルとして書き出す
	tmpDir := os.TempDir()
	extPath := filepath.Join(tmpDir, "vss.duckdb_extension")

	if err := os.WriteFile(extPath, data, 0755); err != nil {
		return "", fmt.Errorf("failed to write extension file: %w", err)
	}

	return extPath, nil
}
