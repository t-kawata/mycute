package main

import (
	"context"
	"database/sql"
	_ "embed"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"

	cozo "github.com/cozodb/cozo-lib-go"
	_ "github.com/duckdb/duckdb-go/v2"
	"github.com/joho/godotenv"
	"github.com/t-kawata/mycute/pkg/cognee"
	"github.com/t-kawata/mycute/pkg/cognee/db/cozodb"
	duckdbRepo "github.com/t-kawata/mycute/pkg/cognee/db/duckdb"
	"github.com/t-kawata/mycute/pkg/cognee/tools/benchmark"
	"github.com/t-kawata/mycute/pkg/cognee/tools/search"
	"github.com/tmc/langchaingo/llms/openai"
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
	if key := os.Getenv("OPENAI_API_KEY"); key != "" {
		log.Println("OPENAI_API_KEY is set")
	} else {
		log.Println("WARNING: OPENAI_API_KEY is NOT set")
	}

	/*********************************************
	 * DEBUGモードの設定
	 ********************************************/
	if os.Getenv("COGNEE_DEBUG") == "true" {
		log.SetFlags(log.LstdFlags | log.Lshortfile)
	}

	// Default data directory
	dataDir := "./data"
	if err := os.MkdirAll(dataDir, 0755); err != nil {
		log.Fatalf("Failed to create data directory: %v", err)
	}

	/*********************************************
	 * DuckDB の初期化
	 ********************************************/
	// DuckDBデータディレクトリの設定
	duckdbDataDir := os.Getenv("DUCKDB_DATA_DIR")
	var duckDBConn *sql.DB
	var err error
	if duckdbDataDir == "" {
		duckdbPath := filepath.Join(dataDir, "cognee_v2.db")
		log.Printf("DuckDB Path: %s", duckdbPath)
		duckDBConn, err = sql.Open("duckdb", fmt.Sprintf("%s?access_mode=READ_WRITE&hnsw_enable_experimental_persistence=true", duckdbPath))
	} else {
		duckdbPath := filepath.Join(duckdbDataDir, "vectors.duckdb")
		log.Printf("DuckDB Path: %s", duckdbPath)
		duckDBConn, err = sql.Open("duckdb", fmt.Sprintf("%s?access_mode=READ_WRITE&hnsw_enable_experimental_persistence=true", duckdbPath))
	}

	if err != nil {
		log.Fatalf("Failed to open DuckDB: %v", err)
	}
	// defer duckDBConn.Close() // Explicit close at end
	// DuckDB VSS拡張のロード
	extensionPath, err := getDuckDBVSSExtensionPath() // プラットフォームに応じた拡張ファイルパスを取得
	if err != nil {
		log.Fatalf("Failed to get DuckDB VSS extension path: %v", err)
	}
	defer os.Remove(extensionPath)                                                 // 終了時に一時ファイルを削除
	query := fmt.Sprintf("INSTALL '%s'; LOAD '%s';", extensionPath, extensionPath) // ローカルの拡張バイナリを直接ロード
	if _, err = duckDBConn.Exec(query); err != nil {
		log.Fatalf("Failed to load DuckDB VSS extension: %v", err)
	}
	// DuckDB VSS extension loaded successfully
	log.Println("DuckDB VSS extension loaded successfully")

	// Apply DuckDB Schema
	schemaContent, err := os.ReadFile("pkg/cognee/db/duckdb/schema.sql")
	if err != nil {
		log.Fatalf("Failed to read schema.sql: %v", err)
	}
	if _, err := duckDBConn.Exec(string(schemaContent)); err != nil {
		log.Fatalf("Failed to apply DuckDB schema: %v", err)
	}
	log.Println("✅ DuckDB Schema applied")

	/*********************************************
	 * CozoDB の初期化
	 ********************************************/
	// CozoDBデータディレクトリの設定
	cozoDBDataDir := os.Getenv("COZODB_DATA_DIR")
	if cozoDBDataDir == "" {
		cozoDBDataDir = "./data"
	}
	// RocksDB バックエンド + 永続化
	cozodbInstance, err := cozo.New("rocksdb", filepath.Join(cozoDBDataDir, "graph.cozodb"), nil)
	if err != nil {
		log.Fatalf("Failed to open CozoDB: %v", err)
	}
	defer cozodbInstance.Close()

	// Apply CozoDB Schema
	ctx := context.Background()
	cozoStorage := cozodb.NewCozoStorage(&cozodbInstance)
	if err := cozoStorage.EnsureSchema(ctx); err != nil {
		log.Fatalf("Failed to apply CozoDB schema: %v", err)
	}
	log.Println("✅ CozoDB Schema applied")

	/*********************************************
	 * Checkpoint 1 Verification
	 ********************************************/
	log.Println("--- Checkpoint 1 Verification ---")

	// DuckDB Verification
	rows, err := duckDBConn.Query("SHOW TABLES")
	if err != nil {
		log.Fatalf("DuckDB Verification Failed: %v", err)
	}
	defer rows.Close()
	log.Println("DuckDB Tables:")
	for rows.Next() {
		var name string
		rows.Scan(&name)
		log.Printf(" - %s", name)
	}

	// CozoDB Verification
	res, err := cozodbInstance.Run("::relations", nil)
	if err != nil {
		log.Fatalf("CozoDB Verification Failed: %v", err)
	}
	log.Printf("CozoDB Relations: %+v", res)

	log.Println("--- Verification Complete ---")

	// Initialize Embedder (OpenAI)
	llm, err := openai.New()
	if err != nil {
		log.Fatalf("Failed to initialize OpenAI LLM: %v", err)
	}
	embedder := search.NewOpenAIEmbedderAdapter(llm)

	// Initialize Cognee Service
	cogneeService := cognee.NewCogneeService(
		duckdbRepo.NewDuckDBStorage(duckDBConn),
		cozodb.NewCozoStorage(&cozodbInstance),
		embedder,
	)

	// CLI Command Handling
	args := os.Args[1:]
	if len(args) == 0 {
		log.Println("No command specified. Usage: mycute <command> [args]")
		return
	}

	command := args[0]
	switch command {
	case "version":
		log.Println("Version: v0.0.3")
	case "add":
		// Example: add -f test_data/sample.txt
		// Simple parsing for verification
		log.Println("--- Phase 2B Verification: Add Pipeline ---")
		if _, err := os.Stat("test_data/sample.txt"); os.IsNotExist(err) {
			os.MkdirAll("test_data", 0755)
			os.WriteFile("test_data/sample.txt", []byte("This is a sample text for Cognee Go Phase 2B verification."), 0644)
		}
		filePaths := []string{"test_data/sample.txt"} // Changed to use existing file
		if err := cogneeService.Add(ctx, filePaths, "test_dataset", "user1"); err != nil {
			log.Fatalf("❌ Add failed: %v", err)
		}

		// Verify Data Count
		var count int
		duckDBConn.QueryRow("SELECT COUNT(*) FROM data").Scan(&count)
		log.Printf("Data count after Add: %d", count)

		if count > 0 {
			log.Println("✅ Add functionality works")
		} else {
			log.Fatalf("❌ Add failed: No data found in DuckDB")
		}

	case "cognify":
		// Example: cognify
		log.Println("--- Phase 2C1 Verification: Cognify Pipeline ---")

		// Verify Data Count before
		var countBefore int
		duckDBConn.QueryRow("SELECT COUNT(*) FROM data").Scan(&countBefore)
		log.Printf("Data count before Cognify: %d", countBefore)

		if err := cogneeService.Cognify(ctx, "test_dataset", "user1"); err != nil {
			log.Fatalf("❌ Cognify failed: %v", err)
		}
		log.Println("✅ Cognify functionality works")

		// Verify Data in DBs
		log.Println("--- Verifying Data in DBs ---")
		// Check Chunks
		var chunkCount int
		duckDBConn.QueryRow("SELECT COUNT(*) FROM chunks").Scan(&chunkCount)
		log.Printf("DuckDB Chunks: %d", chunkCount)

		// Check Nodes/Edges
		res, err := cozodbInstance.Run("?[count(id)] := *nodes[id, type, props]", nil)
		if err != nil {
			log.Printf("Failed to query CozoDB nodes: %v", err)
		} else {
			log.Printf("CozoDB Nodes: %+v", res)
		}
		resEdges, err := cozodbInstance.Run("?[count(source_id)] := *edges[source_id, target_id, type, props]", nil)
		if err != nil {
			log.Printf("Failed to query CozoDB edges: %v", err)
		} else {
			log.Printf("CozoDB Edges: %+v", resEdges)
		}

	case "search":
		// Parse flags
		searchCmd := flag.NewFlagSet("search", flag.ExitOnError)
		queryPtr := searchCmd.String("q", "", "Search query")
		searchCmd.Parse(os.Args[2:])

		if *queryPtr == "" {
			log.Fatal("Query is required. Use -q 'query'")
		}

		// Verify Vectors Count
		var vectorCount int
		duckDBConn.QueryRow("SELECT COUNT(*) FROM vectors").Scan(&vectorCount)
		log.Printf("DuckDB Vectors: %d", vectorCount)

		log.Printf("Searching for: %s", *queryPtr)
		result, err := cogneeService.Search(ctx, *queryPtr, cognee.SearchTypeGraphCompletion, "user1") // Changed dataset/user to match add/cognify
		if err != nil {
			log.Fatalf("Search failed: %v", err)
		}
		log.Printf("Search Result:\n%s", result)

	case "benchmark":
		// Parse flags
		benchCmd := flag.NewFlagSet("benchmark", flag.ExitOnError)
		jsonFilePtr := benchCmd.String("j", "", "QA JSON file path")
		numPtr := benchCmd.Int("n", 0, "Number of questions to run")
		benchCmd.Parse(os.Args[2:])

		if *jsonFilePtr == "" {
			log.Fatal("QA JSON file is required. Use -j 'path/to/qa.json'")
		}

		log.Printf("Running benchmark with %s (n=%d)", *jsonFilePtr, *numPtr)
		if err := benchmark.RunBenchmark(ctx, *jsonFilePtr, *numPtr, cogneeService); err != nil {
			log.Fatalf("Benchmark failed: %v", err)
		}

	default:
		log.Printf("Unknown command: %s", command)
	}

	// Force Checkpoint
	if _, err := duckDBConn.Exec("CHECKPOINT"); err != nil {
		log.Printf("Failed to checkpoint DuckDB: %v", err)
	}

	duckDBConn.Close()
	log.Println("Database closed.")
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
