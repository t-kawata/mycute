package main

import (
	"context"
	"log"
	"os"

	"github.com/joho/godotenv"
	"github.com/t-kawata/mycute/pkg/cognee"
)

func main() {
	// 環境変数の読み込み（.envがあれば）
	godotenv.Load()

	// DEBUGモードの設定
	if os.Getenv("COGNEE_DEBUG") == "true" {
		log.SetFlags(log.LstdFlags | log.Lshortfile)
	}

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

	err := cognee.Add(ctx, []string{"test_data/sample.txt"}, "test_dataset", "user1")
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
