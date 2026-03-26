package subsciber_pool

import (
	"sync/atomic"

	"microscope/util/sync_util"
)

type Pool[T any] struct {
	handlerMap *sync_util.Map[uint64, T]
	lastKey    uint64
}

func New[T any]() *Pool[T] {
	return &Pool[T]{
		handlerMap: &sync_util.Map[uint64, T]{},
		lastKey:    0,
	}
}

func (p *Pool[T]) Subscribe(a T) uint64 {
	key := atomic.AddUint64(&p.lastKey, 1)
	p.handlerMap.Store(key, a)
	return key
}

func (p *Pool[T]) Unsubscribe(key uint64) {
	p.handlerMap.Delete(key)
}

func (p *Pool[T]) Iter(f func(k uint64, v T) bool) {
	p.handlerMap.Range(f)
}
