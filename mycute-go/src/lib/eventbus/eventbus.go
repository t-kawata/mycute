package eventbus

import (
	"errors"
	"fmt"
	"sync"
)

// Handler はイベントを処理する関数型（ジェネリクス版）
type Handler[T any] func(payload T) error

// EventBus はイベントの登録・発行を管理する構造体
type EventBus struct {
	mu       sync.RWMutex
	handlers map[string][]func(payload any) error
}

// New は新しいEventBusインスタンスを作成
func New() *EventBus {
	return &EventBus{
		handlers: make(map[string][]func(payload any) error),
	}
}

// Subscribe はイベント名に対して型安全なハンドラを登録
func Subscribe[T any](eb *EventBus, eventName string, handler Handler[T]) error {
	if eb == nil {
		return errors.New("EventBus: eventbus is nil.")
	}
	if eventName == "" {
		return errors.New("EventBus: eventName is empty.")
	}
	if handler == nil {
		return errors.New("EventBus: handler is nil.")
	}
	eb.mu.Lock()
	defer eb.mu.Unlock()
	wrapper := func(payload any) error {
		typedPayload, ok := payload.(T)
		if !ok {
			return fmt.Errorf("EventBus: Type assertion failed for event '%s': expected %T, got %T", eventName, *new(T), payload)
		}
		return handler(typedPayload)
	}
	eb.handlers[eventName] = append(eb.handlers[eventName], wrapper)
	return nil
}

// Unsubscribe は指定イベントの全ハンドラを削除
func Unsubscribe(eb *EventBus, eventName string) error {
	if eb == nil {
		return errors.New("EventBus: eventbus is nil.")
	}
	if eventName == "" {
		return errors.New("EventBus: eventName is empty.")
	}
	eb.mu.Lock()
	defer eb.mu.Unlock()
	delete(eb.handlers, eventName)
	return nil
}

// Emit はイベントを発行し、登録された全ハンドラを非同期実行
func Emit[T any](eb *EventBus, eventName string, payload T) error {
	if eb == nil {
		return errors.New("EventBus: eventbus is nil.")
	}
	if eventName == "" {
		return errors.New("EventBus: eventName is empty.")
	}
	eb.mu.RLock()
	handlers, ok := eb.handlers[eventName]
	eb.mu.RUnlock()
	if !ok {
		return fmt.Errorf("EventBus: No handlers found for event '%s'", eventName)
	}
	for _, handler := range handlers {
		go func(h func(any) error) {
			_ = h(payload)
		}(handler)
	}
	return nil
}

// EmitSync はイベントを発行し、登録された全ハンドラを同期実行
func EmitSync[T any](eb *EventBus, eventName string, payload T) error {
	if eb == nil {
		return errors.New("EventBus: eventbus is nil.")
	}
	if eventName == "" {
		return errors.New("EventBus: eventName is empty.")
	}
	eb.mu.RLock()
	handlers, ok := eb.handlers[eventName]
	eb.mu.RUnlock()
	if !ok {
		return fmt.Errorf("EventBus: No handlers found for event '%s'", eventName)
	}
	var errs []error
	for i, handler := range handlers {
		if err := handler(payload); err != nil {
			errs = append(errs, fmt.Errorf("EventBus: Handler[%d] for event '%s': %w", i, eventName, err))
		}
	}
	if len(errs) > 0 {
		return errors.Join(errs...)
	}
	return nil
}
