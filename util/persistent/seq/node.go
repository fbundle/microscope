package seq

// implement persistent sequence using weight-balanced tree

type node[T any] struct {
	weight uint64
	height uint64
	entry  T
	left   *node[T]
	right  *node[T]
}

func height[T any](n *node[T]) uint64 {
	if n == nil {
		return 0
	}
	return n.height
}

func weight[T any](n *node[T]) uint64 {
	if n == nil {
		return 0
	}
	return n.weight
}

func makeNode[T any](entry T, left *node[T], right *node[T]) *node[T] {
	return &node[T]{
		weight: 1 + weight(left) + weight(right),
		height: 1 + max(height(left), height(right)),
		entry:  entry,
		left:   left,
		right:  right,
	}
}

func get[T any](n *node[T], i uint64) T {
	if n == nil {
		panic("index out of range")
	}
	if i < weight(n.left) {
		return get(n.left, i)
	}
	if i < weight(n.left)+1 {
		return n.entry
	}
	if i < weight(n.left)+1+weight(n.right) {
		return get(n.right, i-(weight(n.left)+1))
	}
	panic("index out of range")
}

func iter[T any](n *node[T], f func(e T) bool) bool {
	if n == nil {
		return true // continue
	}

	ok := iter(n.left, f)
	if !ok {
		return false
	}
	ok = f(n.entry)
	if !ok {
		return false
	}
	return iter(n.right, f)
}

const delta uint64 = 3

type ordering int

const (
	orderingEq ordering = 0
	orderingGt ordering = +1
	orderingLt ordering = -1
)

// cmp - returns orderingEq, orderingGt, or orderingLt
func cmp[T any](n *node[T]) ordering {
	if n == nil {
		return orderingEq
	} else {
		l, r := weight(n.left), weight(n.right)
		if l+r <= 1 {
			return orderingEq
		} else if l > delta*r {
			return orderingGt
		} else if delta*l < r {
			return orderingLt
		} else {
			return orderingEq
		}
	}
}

func rightRotate[T any](n *node[T]) *node[T] {
	// assume n is non-nil
	if n == nil {
		panic("not_here")
	}

	// right rotate
	//         n
	//   l           r
	// ll lr
	//
	//      becomes
	//
	//         l
	//   ll          n
	//             lr r

	l, r := n.left, n.right
	ll, lr := l.left, l.right

	n1 := makeNode(n.entry, lr, r)
	l1 := makeNode(l.entry, ll, n1)
	return l1
}

func leftRotate[T any](n *node[T]) *node[T] {
	// assume n is non-nil
	if n == nil {
		panic("not_here")
	}

	// left rotate
	//         n
	//   l           r
	//             rl rr
	//
	//      becomes
	//
	//         r
	//   n          rr
	//  l rl

	r, l := n.right, n.left
	rl, rr := r.left, r.right
	n1 := makeNode(n.entry, l, rl)
	r1 := makeNode(r.entry, n1, rr)
	return r1
}

func balance[T any](n *node[T]) *node[T] {
	// assuming δ ≥ 3
	// assuming the two subtrees n.left and n.right are balanced
	// do single rotation or double rotation to rebalance the tree
	// double rotation is necessary - see `why_double_rotation.jpeg`
	// will be be sufficient for merge and split? TODO

	switch cmp(n) {
	case orderingEq:
		return n
	case orderingGt:
		n1 := rightRotate(n)
		if cmp(n1) == orderingEq {
			return n1
		} else {
			l := n.left
			l1 := leftRotate(l)
			n2 := makeNode(n.entry, l1, n.right)
			n3 := rightRotate(n2)
			return n3
		}
	case orderingLt:
		n1 := leftRotate(n)
		if cmp(n1) == orderingEq {
			return n1
		} else {
			r := n.right
			r1 := rightRotate(r)
			n2 := makeNode(n.entry, n.left, r1)
			n3 := leftRotate(n2)
			return n3
		}
	default:
		panic("not_here")
	}
}
func set[T any](n *node[T], i uint64, entry T) *node[T] {
	if n == nil {
		panic("index out of range")
	}
	if i < weight(n.left) {
		l1 := set(n.left, i, entry)
		n1 := makeNode(n.entry, l1, n.right)
		return balance(n1)
	}
	if i < weight(n.left)+1 {
		n1 := makeNode(entry, n.left, n.right)
		return n1
	}
	if i < weight(n.left)+1+weight(n.right) {
		r1 := set(n.right, i-(weight(n.left)+1), entry)
		n1 := makeNode(n.entry, n.left, r1)
		return balance(n1)
	}
	panic("index out of range")
}

func ins[T any](n *node[T], i uint64, entry T) *node[T] {
	if n == nil && i > 0 {
		panic("index out of range")
	}
	if n == nil && i == 0 {
		return makeNode(entry, nil, nil)
	}
	if i < weight(n.left) {
		l1 := ins(n.left, i, entry)
		n1 := makeNode(n.entry, l1, n.right)
		return balance(n1)
	}
	if i < weight(n.left)+1 {
		r1 := ins(n.right, 0, n.entry)
		n1 := makeNode(entry, n.left, r1)
		return balance(n1)
	}
	if i <= weight(n.left)+1+weight(n.right) { // we handle both < and =
		r1 := ins(n.right, i-(weight(n.left)+1), entry)
		n1 := makeNode(n.entry, n.left, r1)
		return balance(n1)
	}
	panic("index out of range")
}

func del[T any](n *node[T], i uint64) *node[T] {
	if n == nil {
		panic("index out of range")
	}
	if i < weight(n.left) {
		l1 := del(n.left, i)
		n1 := makeNode(n.entry, l1, n.right)
		return balance(n1)
	}
	if i < weight(n.left)+1 {
		if weight(n.right) == 0 {
			return n.left
		}
		entry := get(n.right, 0)
		r1 := del(n.right, 0)
		n1 := makeNode(entry, n.left, r1)
		return balance(n1)
	}
	if i < weight(n.left)+1+weight(n.right) {
		r1 := del(n.right, i-(weight(n.left)+1))
		n1 := makeNode(n.entry, n.left, r1)
		return balance(n1)
	}
	panic("index out of range")
}

func merge[T any](l *node[T], r *node[T]) *node[T] {
	if l == nil {
		return r
	}
	if r == nil {
		return l
	}
	// small optimization, we del from the larger subtree
	wl, wr := weight(l), weight(r)
	if wl > wr {
		entry := get(l, wl-1)
		l1 := del(l, wl-1)
		n1 := makeNode(entry, l1, r)
		return balance(n1)
	} else {
		entry := get(r, 0)
		r1 := del(r, 0)
		n1 := makeNode(entry, l, r1)
		return balance(n1)
	}
}

// split - ([0, 1, 2, 3, 4], 2) -> [0, 1] , [2, 3, 4]
func split[T any](n *node[T], i uint64) (*node[T], *node[T]) {
	if n == nil {
		return nil, nil
	}
	if i <= 0 {
		return nil, n
	}
	if i < weight(n.left) {
		ll1, lr1 := split(n.left, i)
		n1 := makeNode(n.entry, lr1, n.right)
		n2 := balance(n1)
		return ll1, n2
	}
	if i < weight(n.left)+1 {
		r1 := ins(n.right, 0, n.entry)
		return n.left, r1
	}
	if i < weight(n.left)+1+weight(n.right) {
		rl1, rr1 := split(n.right, i-(weight(n.left)+1))
		n1 := makeNode(n.entry, n.left, rl1)
		n2 := balance(n1)
		return n2, rr1
	}
	return n, nil
}

// helper functions for monad

func _pure[T any](entry T) *node[T] {
	return makeNode(entry, nil, nil)
}

func _map[T any, T1 any](n *node[T], f func(T) T1) *node[T1] {
	if n == nil {
		return nil
	}
	return makeNode(f(n.entry), _map(n.left, f), _map(n.right, f))
}

func _seq[T any, T1 any](fs *node[func(T) T1], xs *node[T]) *node[T1] {
	var ys *node[T1] = nil
	iter(fs, func(f func(T) T1) bool {
		ys = merge(ys, _map(xs, f))
		return true
	})
	return ys
}

func _bind[T any, T1 any](xs *node[T], f func(T) *node[T1]) *node[T1] {
	var ys *node[T1] = nil
	iter(xs, func(x T) bool {
		ys = merge(ys, f(x))
		return true
	})
	return ys
}
