package seq

func Empty[T any]() Seq[T] {
	return Seq[T]{t: ftEmpty}
}

type Seq[T any] struct {
	t *ftTree
}

func (s Seq[T]) Len() int {
	return s.t.size
}

func (s Seq[T]) Get(i int) T {
	sp := splitTreeAt(s.t, i)
	return sp.pivot.(T)
}

func (s Seq[T]) Set(i int, val T) Seq[T] {
	sp := splitTreeAt(s.t, i)
	t := appendTree(sp.left, pushFront(val, sp.right))
	return Seq[T]{t: t}
}

func (s Seq[T]) Ins(i int, val T) Seq[T] {
	if i == 0 {
		return Seq[T]{t: pushFront(val, s.t)}
	}
	if i == s.t.size {
		return Seq[T]{t: pushBack(s.t, val)}
	}
	sp := splitTreeAt(s.t, i)
	t := appendTree(sp.left, pushFront(val, pushFront(sp.pivot, sp.right)))
	return Seq[T]{t: t}
}

func (s Seq[T]) Del(i int) Seq[T] {
	sp := splitTreeAt(s.t, i)
	return Seq[T]{t: appendTree(sp.left, sp.right)}
}

func (s Seq[T]) Split(i int) (Seq[T], Seq[T]) {
	if i <= 0 {
		return Empty[T](), s
	}
	if i >= s.t.size {
		return s, Empty[T]()
	}
	sp := splitTreeAt(s.t, i)
	right := pushFront(sp.pivot, sp.right)
	return Seq[T]{t: sp.left}, Seq[T]{t: right}
}

func (s Seq[T]) Merge(other Seq[T]) Seq[T] {
	return Seq[T]{t: appendTree(s.t, other.t)}
}

func (s Seq[T]) Iter(f func(i int, val T) bool) {
	i := 0
	iterTree(s.t, func(e any) bool {
		ok := f(i, e.(T))
		i++
		return ok
	})
}

func (s Seq[T]) Depth() int {
	return depth(s.t)
}

func depth(t *ftTree) int {
	if t == nil || t.kind == 0 {
		return 0
	}
	if t.kind == 1 {
		return 1
	}
	return 1 + depth(t.middle)
}

func (s Seq[T]) Repr() []T {
	out := make([]T, 0, s.t.size)
	iterTree(s.t, func(e any) bool {
		out = append(out, e.(T))
		return true
	})
	return out
}

func Merge[T any](ss ...Seq[T]) Seq[T] {
	if len(ss) == 0 {
		return Empty[T]()
	}
	s := ss[0]
	for i := 1; i < len(ss); i++ {
		s = s.Merge(ss[i])
	}
	return s
}

func Fmap[T any, T1 any](xs Seq[T], f func(T) T1) Seq[T1] {
	out := Empty[T1]()
	xs.Iter(func(_ int, v T) bool {
		out = Seq[T1]{t: pushBack(out.t, f(v))}
		return true
	})
	return out
}

func Ap[T any, T1 any](fs Seq[func(T) T1], xs Seq[T]) Seq[T1] {
	out := Empty[T1]()
	fs.Iter(func(_ int, f func(T) T1) bool {
		out = out.Merge(Fmap(xs, f))
		return true
	})
	return out
}

func Bind[T any, T1 any](s Seq[T], f func(T) Seq[T1]) Seq[T1] {
	out := Empty[T1]()
	s.Iter(func(_ int, v T) bool {
		out = out.Merge(f(v))
		return true
	})
	return out
}
