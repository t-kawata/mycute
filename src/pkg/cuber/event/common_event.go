package event

import (
	"time"

	"github.com/t-kawata/mycute/lib/eventbus"
)

// EventName definition
type EventName string

// EventSet definition
type EventSet[T any] struct {
	Name    EventName
	Handler eventbus.Handler[T]
}

// BasePayload contains common fields for all events
type BasePayload struct {
	MemoryGroup string
	Timestamp   int64 // Unix Milliseconds
}

func NewBasePayload(memoryGroup string) BasePayload {
	return BasePayload{
		MemoryGroup: memoryGroup,
		Timestamp:   time.Now().UnixMilli(),
	}
}
