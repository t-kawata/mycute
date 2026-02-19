// Package pipeline は、Cuberのデータ処理パイプラインを実装します。
// パイプラインは、複数のタスクを順番に実行し、各タスクの出力を
// 次のタスクの入力として渡します。
package pipeline

import (
	"context"
	"fmt"

	"github.com/t-kawata/mycute/pkg/cuber/types"
)

// Task は、パイプライン内で実行される単一のタスクを表すインターフェースです。
// 各タスクは、入力を受け取り、処理を行い、出力を返します。
type Task interface {
	// Run は、タスクを実行します。
	// 引数:
	//   - ctx: コンテキスト（キャンセル処理等に使用）
	//   - input: タスクへの入力（前のタスクの出力、または初期入力）
	// 返り値:
	//   - any: タスクの出力（次のタスクへの入力となる）
	//   - types.TokenUsage: トークン使用量
	//   - error: エラーが発生した場合
	Run(ctx context.Context, input any) (any, types.TokenUsage, error)
}

// Pipeline は、複数のタスクを順番に実行するパイプラインを表します。
// パイプラインは、データ処理の流れを定義します。
type Pipeline struct {
	Tasks []Task // 実行するタスクのリスト（順番に実行される）
}

// NewPipeline は、新しいパイプラインを作成します。
// 引数:
//   - tasks: パイプラインで実行するタスクのリスト
//
// 返り値:
//   - *Pipeline: 新しいパイプラインインスタンス
func NewPipeline(tasks []Task) *Pipeline {
	return &Pipeline{Tasks: tasks}
}

// Run は、パイプラインを実行します。
// この関数は以下の処理を行います：
//  1. 初期入力を最初のタスクに渡す
//  2. 各タスクを順番に実行
//  3. 各タスクの出力を次のタスクの入力として渡す
//  4. 最後のタスクの出力を返す
//
// いずれかのタスクがエラーを返した場合、パイプラインの実行は中断され、
// エラーが返されます。
//
// 引数:
//   - ctx: コンテキスト
//   - initialInput: パイプラインへの初期入力（最初のタスクへの入力）
//
// 返り値:
//   - any: 最後のタスクの出力
//   - types.TokenUsage: 全タスクの合計トークン使用量
//   - error: いずれかのタスクがエラーを返した場合
func (p *Pipeline) Run(ctx context.Context, initialInput any) (any, types.TokenUsage, error) {
	// 現在の入力を初期入力で初期化
	currentInput := initialInput
	var totalUsage types.TokenUsage

	// 各タスクを順番に実行
	for _, task := range p.Tasks {
		var err error
		var taskUsage types.TokenUsage
		// タスクを実行し、出力を次の入力として使用
		currentInput, taskUsage, err = task.Run(ctx, currentInput)
		totalUsage.Add(taskUsage)
		if err != nil {
			// タスクがエラーを返した場合、パイプラインを中断
			return nil, totalUsage, fmt.Errorf("Task failed: %w", err)
		}
	}

	// 最後のタスクの出力を返す
	return currentInput, totalUsage, nil
}
