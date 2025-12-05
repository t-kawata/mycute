package main

import (
	"context"
	_ "embed"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"mycute/config"
	"mycute/pkg/cognee"
	"mycute/pkg/cognee/tools/benchmark"
	"mycute/pkg/cognee/tools/search"

	_ "github.com/duckdb/duckdb-go/v2"
	"github.com/joho/godotenv"
)

func main() {
	// CLI Command Handling
	args := os.Args[1:]
	if len(args) == 0 {
		log.Println("No command specified. Usage: mycute <command> [args]")
		return
	}
	command := args[0]
	if command == "-v" {
		fmt.Println(config.VERSION)
		return
	}

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

	// Initialize Cognee Service
	config := cognee.CogneeConfig{
		DuckDBPath: func() string {
			if p := os.Getenv("DUCKDB_DATA_DIR"); p != "" {
				return filepath.Join(p, "vectors.duckdb")
			} else {
				return filepath.Join(dataDir, "cognee_v2.db")
			}
		}(),
		CozoDBPath: filepath.Join(func() string {
			if p := os.Getenv("COZODB_DATA_DIR"); p != "" {
				return p
			} else {
				return dataDir
			}
		}(), "graph.cozodb"),
		OpenAIAPIKey: os.Getenv("OPENAI_API_KEY"),
	}

	cogneeService, err := cognee.NewCogneeService(config)
	if err != nil {
		log.Fatalf("Failed to initialize Cognee Service: %v", err)
	}
	defer cogneeService.Close()

	ctx := context.Background()

	switch command {
	case "add":
		// Parse flags
		addCmd := flag.NewFlagSet("add", flag.ExitOnError)
		datasetPtr := addCmd.String("d", "test_dataset", "Dataset name")
		userPtr := addCmd.String("u", "user1", "User ID")
		filesPtr := addCmd.String("f", "", "Comma separated file paths")
		addCmd.Parse(os.Args[2:])

		log.Println("--- Phase 5 Verification: Add Pipeline ---")
		log.Printf("User: %s, Dataset: %s", *userPtr, *datasetPtr)

		files := []string{}
		if *filesPtr == "" {
			// Create dummy if default
			if _, err := os.Stat("test_data/sample.txt"); os.IsNotExist(err) {
				os.MkdirAll("test_data", 0755)
				os.WriteFile("test_data/sample.txt", []byte("This is a sample text for Cognee Go Phase 5 verification."), 0644)
			}
			files = []string{"test_data/sample.txt"}
		} else {
			files = []string{*filesPtr} // Split by comma if needed, for now single file
		}

		if err := cogneeService.Add(ctx, files, *datasetPtr, *userPtr); err != nil {
			log.Fatalf("❌ Add failed: %v", err)
		}

		// Verify Data Count (using Service Abstraction)
		groupID := *userPtr + "-" + *datasetPtr
		dataList, err := cogneeService.VectorStorage.GetDataList(ctx, groupID)
		if err != nil {
			log.Printf("Verification warning: Failed to fetch data list: %v", err)
		} else {
			log.Printf("Data count for group %s: %d", groupID, len(dataList))
			if len(dataList) > 0 {
				log.Println("✅ Add functionality works")
			} else {
				log.Fatalf("❌ Add failed: No data found for group")
			}
		}

	case "cognify":
		cognifyCmd := flag.NewFlagSet("cognify", flag.ExitOnError)
		datasetPtr := cognifyCmd.String("d", "test_dataset", "Dataset name")
		userPtr := cognifyCmd.String("u", "user1", "User ID")
		cognifyCmd.Parse(os.Args[2:])

		log.Println("--- Phase 5 Verification: Cognify Pipeline ---")
		log.Printf("User: %s, Dataset: %s", *userPtr, *datasetPtr)

		if err := cogneeService.Cognify(ctx, *datasetPtr, *userPtr); err != nil {
			log.Fatalf("❌ Cognify failed: %v", err)
		}
		log.Println("✅ Cognify functionality works")

	case "search":
		// Parse flags
		searchCmd := flag.NewFlagSet("search", flag.ExitOnError)
		queryPtr := searchCmd.String("q", "", "Search query")
		typePtr := searchCmd.String("t", "GRAPH_COMPLETION", "Search type: SUMMARIES, GRAPH_SUMMARY_COMPLETION, GRAPH_COMPLETION")
		datasetPtr := searchCmd.String("d", "test_dataset", "Dataset name")
		userPtr := searchCmd.String("u", "user1", "User ID")
		searchCmd.Parse(os.Args[2:])

		if *queryPtr == "" {
			log.Fatal("Query is required. Use -q 'query'")
		}

		var searchType search.SearchType
		switch *typePtr {
		case "SUMMARIES":
			searchType = search.SearchTypeSummaries
		case "GRAPH_SUMMARY_COMPLETION":
			searchType = search.SearchTypeGraphSummaryCompletion
		default:
			searchType = search.SearchTypeGraphCompletion
		}

		log.Printf("Searching for: %s (Type: %s, User: %s, Dataset: %s)", *queryPtr, searchType, *userPtr, *datasetPtr)
		result, err := cogneeService.Search(ctx, *queryPtr, searchType, *datasetPtr, *userPtr)
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
}
