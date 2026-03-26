package hist

import (
	"microscope/config"
)

func New[T any](t T) *Hist[T] {
	return &Hist[T]{
		latest: 0,
		stack:  []T{t},
	}
}

type Hist[T any] struct {
	latest uint64 // latest version
	stack  []T    // all versions
}

func (h *Hist[T]) Update(modifier func(T) T) {
	next := modifier(h.stack[h.latest])
	h.stack = append(h.stack[:h.latest+1], next)
	h.latest++
	if len(h.stack) > config.Load().MAXSIZE_HISTORY_STACK {
		h.stack = h.stack[1:]
		h.latest--
	}
}

func (h *Hist[T]) Get() T {
	return h.stack[h.latest]
}
func (h *Hist[T]) Undo() {
	if h.latest > 0 {
		h.latest--
	}
}

func (h *Hist[T]) Redo() {
	if int(h.latest) < len(h.stack)-1 {
		h.latest++
	}
}
