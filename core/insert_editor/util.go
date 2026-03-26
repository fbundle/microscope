package insert_editor

import (
	"context"
	"iter"
	"microscope/config"
	"time"

	"microscope/util/side_channel"
)

func insertToSlice[T any](l []T, i int, v T) []T {
	if i < 0 || i > len(l) {
		side_channel.Panic("invalid index", i, l)
		return nil
	}
	if i == len(l) {
		return append(l, v)
	}
	l = append(l, v)
	copy(l[i+1:], l[i:])
	l[i] = v
	return l
}

func deleteFromSlice[T any](l []T, i int) []T {
	if i < 0 || i >= len(l) {
		side_channel.Panic("invalid index", i, l)
		return nil
	}
	copy(l[i:], l[i+1:])
	return l[:len(l)-1]
}
func concatSlices[T any](ls ...[]T) []T {
	c := make([]T, 0)
	for _, l := range ls {
		c = append(c, l...)
	}
	return c
}

func toIndexedIterator[T any](i iter.Seq[T]) iter.Seq2[int, T] {
	return func(yield func(int, T) bool) {
		index := 0
		for v := range i {
			if !yield(index, v) {
				return
			}
			index++
		}
	}
}

func pollCtx(ctx context.Context) bool {
	select {
	case <-ctx.Done():
		return false
	default:
		return true
	}
}

func newLoader(totalSize int) *loader {
	return &loader{
		totalSize:            totalSize,
		loadedSize:           0,
		lastRenderPercentage: -1,
		lastRenderTime:       time.Time{},
	}
}

type loader struct {
	totalSize            int
	loadedSize           int
	lastRenderPercentage int
	lastRenderTime       time.Time
}

func (l *loader) set(loadedSize int) bool {
	l.loadedSize = max(loadedSize, l.loadedSize)
	percentage := int(100 * float64(l.loadedSize) / float64(l.totalSize))
	t := time.Now()
	if percentage > l.lastRenderPercentage || t.Sub(l.lastRenderTime) >= config.Load().LOADING_PROGRESS_INTERVAL {
		l.lastRenderPercentage = percentage
		l.lastRenderTime = t
		return true
	}
	return false
}
