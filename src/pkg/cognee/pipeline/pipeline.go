package pipeline

import (
	"context"
	"fmt"
)

type Task interface {
	Run(ctx context.Context, input any) (any, error)
}

type Pipeline struct {
	Tasks []Task
}

func NewPipeline(tasks []Task) *Pipeline {
	return &Pipeline{Tasks: tasks}
}

func (p *Pipeline) Run(ctx context.Context, initialInput any) (any, error) {
	currentInput := initialInput
	for _, task := range p.Tasks {
		var err error
		currentInput, err = task.Run(ctx, currentInput)
		if err != nil {
			return nil, fmt.Errorf("task failed: %w", err)
		}
	}
	return currentInput, nil
}
