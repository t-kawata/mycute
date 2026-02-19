# Cognee Go Implementation: Phase-02 Development Directives

## 0. はじめに (Context & Prerequisites)

本ドキュメントは、Phase-01完了後の **Phase-02** における開発指示書のマスタードキュメントです。
Phase-02は実装規模が大きいため、以下の4つのサブフェーズに分割して進行します。

1.  **Phase-02A**: DB Infrastructure Layer (`docs/DIRECTONS-PHASE-02A.md`)
2.  **Phase-02B**: Pipeline & Task Infrastructure (`docs/DIRECTONS-PHASE-02B.md`)
3.  **Phase-02C1**: Cognify Pipeline (`docs/DIRECTONS-PHASE-02C1.md`)
4.  **Phase-02C2**: Search & Verification (`docs/DIRECTONS-PHASE-02C2.md`)

開発を開始する前に、以下のドキュメントを必ず確認し、Phase-01の実装内容と前提条件を理解してください。

1.  `docs/DIRECTONS-PHASE-01.md`: Phase-01の開発指示書（完了済み）。
2.  `docs/DIFF-BETWEEN-PHASE-01-PYTHON-AND-GO.md`: Python版とGo版の差異分析およびPhase-02の方針。

### ⚠️ 重要: Phase-02開始時点のステータス
Phase-01完了後、Phase-02の実装準備として以下の**インフラストラクチャ構築が既に完了しています**。

*   **DuckDB (Vector/Relational)**: CGOによる埋め込み、インスタンス初期化 (`src/main.go`)。
*   **CozoDB (Graph)**: CGOによる埋め込み、インスタンス初期化 (`src/main.go`)。
*   **ビルド環境**: `Makefile` および `sh/build` によるクロスコンパイル環境。

**Phase-02の責務は、これらの構築済みDB上で動作する「ロジック」と「アーキテクチャ」を実装することです。**

---

## 1. 重要な方針変更 (Critical Policy Change)

> [!IMPORTANT]
> **SurrealDBの使用計画は完全に破棄されました。**
>
> *   **変更前**: ベクトル・グラフ兼用DBとして SurrealDB を使用予定。
> *   **変更後**:
>     *   **ベクトル・メタデータ**: **DuckDB** (`go-duckdb` via CGO)
>     *   **グラフデータ**: **CozoDB** (`cozo-lib-go` via CGO)
>
> 今後、SurrealDBに関する言及や設計は全て無効とし、DuckDBとCozoDBのハイブリッド構成を正とします。

---

## 2. 開発の目的 (Objectives)

Phase-02の目的は、Phase-01で作成したプロトタイプ（インメモリ・モノリシック）を、**Python版Cogneeのアーキテクチャ（パイプライン・タスク構造）に近づけ、かつ永続化層をDuckDB/CozoDBに置き換えること**です。

「完全なクローン」へのステップとして、以下の4点を達成します。

1.  **インフラの刷新**: インメモリ保存を廃止し、DuckDB/CozoDBへのスキーマ定義とデータ操作を実装する。
2.  **アーキテクチャの刷新**: `Add`, `Cognify` をモノリシックな関数から、`Pipeline` と `Task` の集合体へリファクタリングする。
3.  **検索機能の高度化**: ベクトル検索 (DuckDB) とグラフ探索 (CozoDB) を組み合わせた `GetContext` を実装する。
4.  **抽象化と拡張性**: `SearchTool` パターンや `Chunker`/`GraphModel` のコンポーネント化を行い、拡張性を確保する。

---

## 3. 開発ロードマップ (Roadmap)

詳細な実装指示は各サブフェーズのドキュメントを参照してください。

### [Phase-02A: DB Infrastructure Layer](docs/DIRECTONS-PHASE-02A.md)
**ゴール**: DuckDB/CozoDBのスキーマ適用と基本CRUDの実装。
*   DuckDB Schema (`data`, `documents`, `chunks`, `vectors`)
*   CozoDB Schema (`nodes`, `edges`)
*   Storage Interfaces (`VectorStorage`, `GraphStorage`)

### [Phase-02B: Pipeline & Task Infrastructure](docs/DIRECTONS-PHASE-02B.md)
**ゴール**: Pipeline/Taskアーキテクチャの確立とIngestion機能の実装。
*   Core Pipeline (`Task` interface, `Pipeline` runner)
*   Ingestion Task (`Add` functionality, Deduplication)
*   Orchestration (`CogneeService`)

### [Phase-02C1: Cognify Pipeline](docs/DIRECTONS-PHASE-02C1.md)
**ゴール**: Cognify機能の完成。
*   Chunking Task (Hierarchical)
*   Graph Extraction Task (LLM, Prompts)
*   Storage Task

### [Phase-02C2: Search & Verification](docs/DIRECTONS-PHASE-02C2.md)
**ゴール**: Search機能の完成と科学的検証。
*   Search Functionality (`GraphCompletionTool`)
*   Scientific Verification (`benchmark` command, Accuracy > 80%)

---

## 4. 開発環境の要件 (Environment Requirements)

*   **OS**: macOS (Apple Silicon) / Linux (AMD64)
*   **Language**: Go 1.25+
*   **Build Tool**: **Make** (必須)
    *   `make build`: macOS用ビルド
    *   `make build-linux-amd64`: Linux用クロスコンパイル
*   **Dependencies**:
    *   DuckDB: `github.com/duckdb/duckdb-go/v2` (CGO)
    *   CozoDB: `github.com/cozodb/cozo-lib-go` (CGO)
    *   LLM: `github.com/tmc/langchaingo`

---

## 5. Go言語での実装ガイドライン

1.  **CGOの扱い**: DB操作は必ず `src/pkg/cognee/db` 以下のパッケージにカプセル化し、ビジネスロジック層 (`src/pkg/cognee`) から直接CGO依存コードを呼ばないようにする（インターフェース経由で利用）。
2.  **エラーハンドリング**: `fmt.Errorf("%w", err)` でラップし、スタックトレースやコンテキストを失わないようにする。
3.  **並行処理**: `errgroup` を活用し、リソースリーク（goroutineリーク）を防ぐ。
4.  **テスト**: `src/main.go` の動作確認コードを更新し、新しいパイプラインとDBで `Add` -> `Cognify` -> `Search` が通ることを確認する。

---

## 6. テストデータについて (Test Data)

開発および検証には `test_data/sample.txt` を使用します。
このファイルは、以前の単純なテキストではなく、**プログラミングに関する長大で難解な文章（例: C言語の歴史や仕様に関する技術文書）** に差し替えられています。

**目的**:
*   単純なキーワードマッチングではなく、LLMが文脈を理解してグラフを構築できるかを検証するため。
*   チャンキングロジックが、複雑な構造のテキストを適切に分割できるかを確認するため。
*   "Unanswerable"（回答不能）な質問に対して、LLMが幻覚（Hallucination）を起こさずに正しく回答拒否できるかをテストするため。

---

## 7. エラーハンドリング戦略 (Error Handling Strategy)

全てのサブフェーズで、以下のエラーハンドリングパターンを採用してください。

### 6.1. LLM API呼び出し
- **Retry with Exponential Backoff**: 429エラーやタイムアウト時は最大3回まで再試行。
- **実装例**:
  ```go
  func callLLMWithRetry(ctx context.Context, llm llms.Model, prompt string, maxRetries int) (string, error) {
      for i := 0; i < maxRetries; i++ {
          resp, err := llm.GenerateContent(ctx, []llms.MessageContent{llms.TextParts(llms.ChatMessageTypeHuman, prompt)})
          if err == nil {
              return resp.Choices[0].Content, nil
          }
          if i < maxRetries-1 {
              time.Sleep(time.Second * time.Duration(math.Pow(2, float64(i))))
          }
      }
      return "", fmt.Errorf("LLM call failed after %d retries", maxRetries)
  }
  ```

### 6.2. DB操作
- **Graceful Degradation**: DB書き込みエラー時は、エラーをログに記録しつつ処理を継続（読み込みエラー時は即座に中断）。
- **Transaction**: 複数のテーブルに跨る書き込み（例: `data` → `documents`）は、可能であればトランザクション内で実行。

### 6.3. 並列処理 (errgroup)
- **Fail-Fast**: 1つのgoroutineがエラーを返したら、`errgroup.WithContext`のキャンセル機能で他のgoroutineを停止。
- **Partial Results**: 必要に応じて、成功した結果のみを収集する「Partial Success」パターンも検討。