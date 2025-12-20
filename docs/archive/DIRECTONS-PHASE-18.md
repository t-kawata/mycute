# Phase-18 Log Refactoring

src/pkg/cuber 配下のロギングを、`fmt.Print*` や `log.Print*` から、`CuberService` が保持する `*zap.Logger` を使用した構造化ログへ統一するための指示書です。

## 1. 共通ログ関数の定義

まず、全てのログに `[CUBER]: ` プレフィックスを一貫して付与し、かつ `nil` 安全に呼び出せる共通関数を定義します。
`src/pkg/cuber/types` パッケージは全パッケージから参照可能であるため、ここに配置します。

**新規ファイル**: `src/pkg/cuber/types/logger.go`

```go
package types

import (
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

const LOG_PREFIX = "[CUBER]: "

// LogDebug outputs a debug log with [CUBER] prefix.
func LogDebug(l *zap.Logger, msg string, fields ...zap.Field) {
	if l != nil {
		l.Debug(LOG_PREFIX+msg, fields...)
	}
}

// LogInfo outputs an info log with [CUBER] prefix.
// Use this only for significant milestones.
func LogInfo(l *zap.Logger, msg string, fields ...zap.Field) {
	if l != nil {
		l.Info(LOG_PREFIX+msg, fields...)
	}
}

// LogWarn outputs a warning log with [CUBER] prefix.
// Use this for recoverable errors or issues.
func LogWarn(l *zap.Logger, msg string, fields ...zap.Field) {
	if l != nil {
		l.Warn(LOG_PREFIX+msg, fields...)
	}
}
```

---

## 2. ログ書き換え対象と変更内容

以下に、現在の「簡易ログ」使用箇所と、それをどのように「構造化冗長ログ」へ変更するかの詳細を示します。
各コンポーネント（Struct）には `Logger *zap.Logger` フィールドを追加し、初期化時に `CuberService` から伝播させる必要があります。

### 2.1. CuberService (`src/pkg/cuber/cuber.go`)

`CuberService` は既に `Logger` を持っています。

| 行番号 (目安) | 現在のコード | 変更後の実装イメージ (types.Log... を使用) |
| :--- | :--- | :--- |
| L364 | `log.Printf("[Cuber] Created new Cube at %s", dbFilePath)` | `types.LogInfo(s.Logger, "Created new Cube", zap.String("path", dbFilePath))` |
| L126, L181 | `fmt.Printf("Warning: Failed to cleanup S3 download cache: %v\n", err)` | `types.LogWarn(s.Logger, "Failed to cleanup S3 download cache", zap.Error(err))` |
| L208 | `fmt.Printf("[CuberService] Closing idle storage for Cube %s (Idle: %v)\n", uuid, idleTime)` | `types.LogDebug(s.Logger, "Closing idle storage", zap.String("uuid", uuid), zap.Duration("idle_time", idleTime))` |
| L218 | `fmt.Printf("[CuberService] Warning: Error closing vector storage for %s: %v\n", uuid, err)` | `types.LogWarn(s.Logger, "Error closing vector storage", zap.String("uuid", uuid), zap.Error(err))` |
| L222 | `fmt.Printf("[CuberService] Warning: Error closing graph storage for %s: %v\n", uuid, err)` | `types.LogWarn(s.Logger, "Error closing graph storage", zap.String("uuid", uuid), zap.Error(err))` |
| L666 | `log.Printf("Warning: Failed to delete file %s: %v", ...)` | `types.LogWarn(s.Logger, "Failed to delete file", zap.String("location", data.RawDataLocation), zap.Error(err))` |
| L669 | `log.Printf("Deleted file: %s", data.RawDataLocation)` | `types.LogDebug(s.Logger, "Deleted file", zap.String("location", data.RawDataLocation))` |
| L579 | `fmt.Println("No data to process for this group.")` | `types.LogDebug(s.Logger, "Absorb: No data to process", zap.String("group", memoryGroup))` |
| L727 | `fmt.Println("Memify: Phase A - Prioritizing Unknown Resolution")` | `types.LogDebug(s.Logger, "Memify: Starting Phase A (Unknown Resolution)", zap.String("group", config.MemoryGroup))` |
| L740 | `fmt.Printf("Warning: Failed to get unresolved unknowns: %v\n", err)` | `types.LogWarn(s.Logger, "Failed to get unresolved unknowns", zap.Error(err))` |
| L755 | `fmt.Printf("Resolved Unknown: %s\nInsight: %s\n", ...)` | `types.LogDebug(s.Logger, "Resolved Unknown", zap.String("unknown", unknown.Text), zap.String("insight", insight))` |
| L757 | `fmt.Printf("Failed to resolve unknown %s: %v\n", ...)` | `types.LogWarn(s.Logger, "Failed to resolve unknown", zap.String("id", unknown.ID), zap.Error(err))` |
| L759 | `fmt.Printf("Could not resolve Unknown: %s\n", unknown.Text)` | `types.LogDebug(s.Logger, "Could not resolve Unknown", zap.String("text", unknown.Text))` |
| L764 | `fmt.Println("Memify: Phase B - Graph Expansion")` | `types.LogDebug(s.Logger, "Memify: Starting Phase B (Graph Expansion)")` |
| L774 | `fmt.Printf("Memify: Level %d / %d\n", level, ...)` | `types.LogDebug(s.Logger, "Memify: Recursive Level", zap.Int("level", level), zap.Int("max_depth", config.RecursiveDepth))` |
| L862 | `fmt.Printf("Memify [BATCH]: Processing with batch size ~%d chars...")` | `types.LogDebug(s.Logger, "Memify: Batch Processing Start", zap.Int("batch_size_chars", batchCharSize), zap.Int("overlap_percent", overlapPercent))` |
| L870 | `fmt.Printf("Memify [BATCH]: Split into %d batches...")` | `types.LogDebug(s.Logger, "Memify: Batch Split", zap.Int("batch_count", len(batches)))` |
| L873 | `fmt.Printf("Memify [BATCH]: Processing batch %d/%d...")` | `types.LogDebug(s.Logger, "Memify: Processing Batch", zap.Int("index", i+1), zap.Int("total", len(batches)), zap.Int("chars", count))` |

### 2.2. KuzuDBStorage (`src/pkg/cuber/db/kuzudb/kuzudb_storage.go`)

**構造体変更**: `KuzuDBStorage` に `Logger *zap.Logger` を追加し、`NewKuzuDBStorage` の引数に追加してください。

| 行番号 (目安) | 現在のコード | 変更後の実装イメージ |
| :--- | :--- | :--- |
| L36 | `log.Println("[KuzuDB] Opening in-memory database...")` | `types.LogInfo(s.Logger, "KuzuDB: Opening in-memory database")` |
| L80 | `log.Println("[KuzuDB] EnsureSchema: Starting schema creation...")` | `types.LogDebug(s.Logger, "KuzuDB: Starting schema creation")` |
| L214 | `log.Println("[KuzuDB] EnsureSchema: Schema creation completed.")` | `types.LogDebug(s.Logger, "KuzuDB: Schema creation completed")` |
| L303 | `log.Printf("[WARN] Exists query failed: %v", err)` | `types.LogWarn(s.Logger, "KuzuDB: Exists query failed", zap.Error(err))` |

その他、`log.Printf("[WARN] ...")` となっている箇所は全て `types.LogWarn` に置換し、適切なフィールド (expected type, actual value 等) を付与してください。

### 2.3. タスク (Metacognition) (`src/pkg/cuber/tasks/metacognition/*.go`)

各タスク構造体に `Logger *zap.Logger` を追加し、コンストラクタ経由で受け取るように変更します。

#### IgnoranceManager (`ignorance_manager.go`)
| 現在のコード | 変更後の実装イメージ |
| :--- | :--- |
| `fmt.Printf("IgnoranceManager: Registered Unknown: %s...", ...)` | `types.LogDebug(m.Logger, "IgnoranceManager: Registered Unknown", zap.String("text", text), zap.String("requirement", requirement))` |
| `fmt.Printf("IgnoranceManager: Registered Capability: %s\n", text)` | `types.LogDebug(m.Logger, "IgnoranceManager: Registered Capability", zap.String("text", text))` |
| `fmt.Printf("IgnoranceManager: Warning - failed to register resolved capability: %v\n", err)` | `types.LogWarn(m.Logger, "IgnoranceManager: Failed to register resolved capability", zap.Error(err))` |

#### CrystallizationTask (`crystallization_task.go`)
| 現在のコード | 変更後の実装イメージ |
| :--- | :--- |
| `fmt.Printf("CrystallizationTask: Warning - failed to delete old node %s: %v\n", ...)` | `types.LogWarn(t.Logger, "CrystallizationTask: Failed to delete old node", zap.String("node_id", oldNodeID), zap.Error(err))` |
| `fmt.Printf("CrystallizationTask: Crystallized %d rules into %s\n", ...)` | `types.LogDebug(t.Logger, "CrystallizationTask: Crystallized rules", zap.Int("rule_count", len(cluster)), zap.String("new_id", crystallizedID))` |
| `fmt.Printf("CrystallizationTask: Embedding cache hit: %d/%d (%.1f%%)\n", ...)` | `types.LogDebug(t.Logger, "CrystallizationTask: Embedding cache stats", zap.Int("hits", hitCount), zap.Int("total", inputs), zap.Float64("rate", rate))` |

その他、`Warning` を含む `fmt.Printf` は `types.LogWarn` に、それ以外のステータス表示は `types.LogDebug` に置換します。

#### SelfReflectionTask (`self_reflection_task.go`)
| 現在のコード | 変更後の実装イメージ |
| :--- | :--- |
| `fmt.Printf("SelfReflectionTask: Generated %d questions\n", len(questions))` | `types.LogDebug(t.Logger, "SelfReflectionTask: Generated questions", zap.Int("count", len(questions)))` |
| `fmt.Printf("SelfReflectionTask: Warning - TryAnswer failed: %v\n", err)` | `types.LogWarn(t.Logger, "SelfReflectionTask: TryAnswer failed", zap.Error(err))` |

#### GraphRefinementTask (`graph_refinement_task.go`)
| 現在のコード | 変更後の実装イメージ |
| :--- | :--- |
| `fmt.Println("GraphRefinementTask: No target nodes specified, skipping")` | `types.LogDebug(t.Logger, "GraphRefinementTask: No target nodes, skipping")` |
| `fmt.Printf("GraphRefinementTask: Strengthening edge %s -> %s...", ...)` | `types.LogDebug(t.Logger, "GraphRefinementTask: Strengthening edge", zap.String("from", edge.SourceID), zap.String("to", edge.TargetID), zap.Float64("old_conf", edge.Confidence), zap.Float64("new_conf", newConf))` |
| `fmt.Printf("GraphRefinementTask: Pruned edge %s -> %s...", ...)` | `types.LogDebug(t.Logger, "GraphRefinementTask: Pruned edge", zap.String("from", edge.SourceID), zap.String("to", edge.TargetID))` |

#### PruningTask (`pruning_task.go`)
| 現在のコード | 変更後の実装イメージ |
| :--- | :--- |
| `fmt.Printf("PruningTask: Starting pruning for group %s...", ...)` | `types.LogDebug(t.Logger, "PruningTask: Starting", zap.String("group", t.MemoryGroup), zap.Duration("grace_period", t.GracePeriod))` |
| `fmt.Printf("PruningTask: Found %d orphan nodes to delete\n", len(orphans))` | `types.LogDebug(t.Logger, "PruningTask: Found orphans", zap.Int("count", len(orphans)))` |
| `fmt.Printf("PruningTask: Completed. Deleted %d nodes, failed %d nodes\n", ...)` | `types.LogDebug(t.Logger, "PruningTask: Completed", zap.Int("deleted", deletedCount), zap.Int("failed", failedCount))` |

---

## 3. 追加の実装要件

### 3.1. ログレベル運用ルール
*   **Debug**: 通常の処理フロー、ステータス報告、詳細情報（大部分はこれになります）。
*   **Info**: プロセスの開始/終了、重要なリソース（Cubeなど）の作成/削除など、運用上重要なイベントのみ。
*   **Warn**: 予期しないが継続可能なエラー、リトライ発生、無視したエラーなど。
*   **Error**: 使用禁止（ユーザー指示により）。問題箇所は Warn に留める。

### 3.2. CuberService からの伝播
`CuberService` 内で各タスクやストレージを初期化する際、必ず `s.Logger` を渡してください。
例:
```go
// KuzuDBへ渡す
kuzudb.NewKuzuDBStorage(path, s.Logger)

// Taskへ渡す
task := metacognition.NewCrystallizationTask(..., s.Logger)
```

この変更に伴い、各 `New...` 関数のシグネチャ変更が必要になります。
