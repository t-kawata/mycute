package main

import (
	"context"
	_ "embed"
	"flag"
	"fmt"
	"log"
	"os"
	"strconv"

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
		DBDirPath: func() string {
			if p := os.Getenv("DB_DIR_PATH"); p != "" {
				return p
			}
			return dataDir
		}(),
		DBName: func() string {
			if n := os.Getenv("DB_NAME"); n != "" {
				return n
			}
			return "cognee"
		}(),

		// Completion (LLM) Configuration
		CompletionAPIKey:  os.Getenv("COMPLETION_API_KEY"),
		CompletionBaseURL: os.Getenv("COMPLETION_BASE_URL"),
		CompletionModel:   os.Getenv("COMPLETION_MODEL"),

		// Embeddings Configuration
		EmbeddingsAPIKey:  os.Getenv("EMBEDDINGS_API_KEY"),
		EmbeddingsBaseURL: os.Getenv("EMBEDDINGS_BASE_URL"),
		EmbeddingsModel:   os.Getenv("EMBEDDINGS_MODEL"),

		// Memify Configuration
		MemifyMaxCharsForBulkProcess: func() int {
			if v := os.Getenv("COGNEE_MEMIFY_MAX_CHARS"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MemifyBatchOverlapPercent: func() int {
			if v := os.Getenv("COGNEE_MEMIFY_OVERLAP_PERCENT"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MemifyBatchMinChars: func() int {
			if v := os.Getenv("COGNEE_MEMIFY_BATCH_MIN_CHARS"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),

		// Metacognition Configuration
		MetaSimilarityThresholdUnknown: func() float64 {
			if v := os.Getenv("COGNEE_META_SIMILARITY_THRESHOLD_UNKNOWN"); v != "" {
				if f, err := strconv.ParseFloat(v, 64); err == nil {
					return f
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MetaSimilarityThresholdReflection: func() float64 {
			if v := os.Getenv("COGNEE_META_SIMILARITY_THRESHOLD_REFLECTION"); v != "" {
				if f, err := strconv.ParseFloat(v, 64); err == nil {
					return f
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MetaSimilarityThresholdCrystallization: func() float64 {
			if v := os.Getenv("COGNEE_META_SIMILARITY_THRESHOLD_CRYSTALLIZATION"); v != "" {
				if f, err := strconv.ParseFloat(v, 64); err == nil {
					return f
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MetaSearchLimitUnknown: func() int {
			if v := os.Getenv("COGNEE_META_SEARCH_LIMIT_UNKNOWN"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MetaSearchLimitReflectionChunk: func() int {
			if v := os.Getenv("COGNEE_META_SEARCH_LIMIT_REFLECTION_CHUNK"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MetaSearchLimitReflectionRule: func() int {
			if v := os.Getenv("COGNEE_META_SEARCH_LIMIT_REFLECTION_RULE"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		MetaCrystallizationMinCluster: func() int {
			if v := os.Getenv("COGNEE_META_CRYSTALLIZATION_MIN_CLUSTER"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),

		// Storage Config
		S3UseLocal: func() bool {
			if v := os.Getenv("COGNEE_S3_USE_LOCAL"); v == "false" {
				return false
			}
			return true // デフォルトはローカル
		}(),
		S3LocalDir: "data/files", // デフォルトの保存先

		// S3 Cleanup Configuration
		S3CleanupIntervalMinutes: func() int {
			if v := os.Getenv("COGNEE_S3_CLEANUP_INTERVAL_MINUTES"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),
		S3RetentionHours: func() int {
			if v := os.Getenv("COGNEE_S3_RETENTION_HOURS"); v != "" {
				if i, err := strconv.Atoi(v); err == nil {
					return i
				}
			}
			return 0 // default handled in NewCogneeService
		}(),

		S3AccessKey: os.Getenv("AWS_ACCESS_KEY_ID"),
		S3SecretKey: os.Getenv("AWS_SECRET_ACCESS_KEY"),
		S3Region:    os.Getenv("AWS_REGION"),
		S3Bucket:    os.Getenv("S3_BUCKET"),
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

	case "memify":
		memifyCmd := flag.NewFlagSet("memify", flag.ExitOnError)
		datasetPtr := memifyCmd.String("d", "test_dataset", "Dataset name")
		userPtr := memifyCmd.String("u", "user1", "User ID")
		rulesNodeSetPtr := memifyCmd.String("r", "coding_agent_rules", "Rules NodeSet Name")
		// Phase-07: Recursive Memify Flags
		recursivePtr := memifyCmd.Bool("recursive", false, "Enable recursive memify")
		depthPtr := memifyCmd.Int("depth", 1, "Recursive depth")
		prioritizeUnknownsPtr := memifyCmd.Bool("prioritize-unknowns", true, "Prioritize Unknown resolution")
		memifyCmd.Parse(os.Args[2:])

		log.Println("--- Phase 6/7: Memify (Hybrid/Recursive) ---")
		log.Printf("User: %s, Dataset: %s", *userPtr, *datasetPtr)

		config := &cognee.MemifyConfig{
			RulesNodeSetName:   *rulesNodeSetPtr,
			EnableRecursive:    *recursivePtr,
			RecursiveDepth:     *depthPtr,
			PrioritizeUnknowns: *prioritizeUnknownsPtr,
		}

		if *recursivePtr {
			log.Println("Running in RECURSIVE mode")
			if err := cogneeService.RecursiveMemify(ctx, *datasetPtr, *userPtr, config); err != nil {
				log.Fatalf("❌ Recursive Memify failed: %v", err)
			}
		} else {
			if err := cogneeService.Memify(ctx, *datasetPtr, *userPtr, config); err != nil {
				log.Fatalf("❌ Memify failed: %v", err)
			}
		}
		log.Println("✅ Memify functionality completed")

	default:
		log.Printf("Unknown command: %s", command)
	}
}
