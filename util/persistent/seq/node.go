// Ported by Claude Sonnet 4.6 (claude-sonnet-4-6), Anthropic, 2026-05-09.
// Source: fingertree-0.1.6.3 by Ross Paterson and Ralf Hinze (BSD-style licence)
//   https://hackage.haskell.org/package/fingertree-0.1.6.3
//   (original Haskell reference kept in FingerTree.hs in this directory)
//
// Specialised to size annotation (no Measured typeclass needed).
// See FingerTree.hs for the full generic version and paper reference.

package seq

// Finger tree implementation for persistent sequences.
// Port of Data.FingerTree (Hinze & Paterson, 2006) specialized to size annotation.
//
// Complexity:
//   PushFront / PushBack: O(1) amortized
//   PopFront / PopBack:   O(1) amortized
//   Merge (Concat):       O(log min(n1,n2))
//   Split / Get / Ins / Del: O(log n)

// elemSize returns the size of a tree element.
// Leaf values have size 1; internal ftNode pointers carry their own cached size.
func elemSize(e any) int {
	if n, ok := e.(*ftNode); ok {
		return n.size
	}
	return 1
}

// ftNode is a 2- or 3-element internal node with a cached size.
type ftNode struct {
	size     int
	a, b, c  any
	is3      bool
}

func newNode2(a, b any) *ftNode {
	return &ftNode{size: elemSize(a) + elemSize(b), a: a, b: b}
}

func newNode3(a, b, c any) *ftNode {
	return &ftNode{size: elemSize(a) + elemSize(b) + elemSize(c), a: a, b: b, c: c, is3: true}
}

// digit holds 1–4 elements (prefix or suffix of a deep tree).
type digit struct {
	a, b, c, d any
	n          int // 1–4
	size       int
}

func dig1(a any) digit { return digit{a: a, n: 1, size: elemSize(a)} }
func dig2(a, b any) digit {
	return digit{a: a, b: b, n: 2, size: elemSize(a) + elemSize(b)}
}
func dig3(a, b, c any) digit {
	return digit{a: a, b: b, c: c, n: 3, size: elemSize(a) + elemSize(b) + elemSize(c)}
}
func dig4(a, b, c, d any) digit {
	return digit{a: a, b: b, c: c, d: d, n: 4, size: elemSize(a) + elemSize(b) + elemSize(c) + elemSize(d)}
}

func (dg digit) head() any {
	return dg.a
}
func (dg digit) tail() digit {
	switch dg.n {
	case 2:
		return dig1(dg.b)
	case 3:
		return dig2(dg.b, dg.c)
	case 4:
		return dig3(dg.b, dg.c, dg.d)
	}
	panic("ltailDigit on One")
}
func (dg digit) last() any {
	switch dg.n {
	case 1:
		return dg.a
	case 2:
		return dg.b
	case 3:
		return dg.c
	case 4:
		return dg.d
	}
	panic("rheadDigit")
}
func (dg digit) init() digit {
	switch dg.n {
	case 2:
		return dig1(dg.a)
	case 3:
		return dig2(dg.a, dg.b)
	case 4:
		return dig3(dg.a, dg.b, dg.c)
	}
	panic("rtailDigit on One")
}
func (dg digit) cons(a any) digit {
	switch dg.n {
	case 1:
		return dig2(a, dg.a)
	case 2:
		return dig3(a, dg.a, dg.b)
	case 3:
		return dig4(a, dg.a, dg.b, dg.c)
	}
	panic("consDigit on Four")
}
func (dg digit) snoc(a any) digit {
	switch dg.n {
	case 1:
		return dig2(dg.a, a)
	case 2:
		return dig3(dg.a, dg.b, a)
	case 3:
		return dig4(dg.a, dg.b, dg.c, a)
	}
	panic("snocDigit on Four")
}

func nodeToDigit(n *ftNode) digit {
	if n.is3 {
		return dig3(n.a, n.b, n.c)
	}
	return dig2(n.a, n.b)
}

func (dg digit) toSlice() []any {
	switch dg.n {
	case 1:
		return []any{dg.a}
	case 2:
		return []any{dg.a, dg.b}
	case 3:
		return []any{dg.a, dg.b, dg.c}
	case 4:
		return []any{dg.a, dg.b, dg.c, dg.d}
	}
	panic("digit.toSlice")
}

// ftTree is a finger tree node annotated with size.
type ftTree struct {
	// kind: 0 = empty, 1 = single, 2 = deep
	kind   int
	size   int
	single any
	prefix digit
	middle *ftTree
	suffix digit
}

var ftEmpty = &ftTree{kind: 0}

func ftSingle(a any) *ftTree {
	return &ftTree{kind: 1, size: elemSize(a), single: a}
}

func ftDeep(pr digit, m *ftTree, sf digit) *ftTree {
	return &ftTree{
		kind:   2,
		size:   pr.size + m.size + sf.size,
		prefix: pr,
		middle: m,
		suffix: sf,
	}
}

// digitToTree converts a digit to a finger tree.
func digitToTree(dg digit) *ftTree {
	switch dg.n {
	case 1:
		return ftSingle(dg.a)
	case 2:
		return ftDeep(dig1(dg.a), ftEmpty, dig1(dg.b))
	case 3:
		return ftDeep(dig2(dg.a, dg.b), ftEmpty, dig1(dg.c))
	case 4:
		return ftDeep(dig2(dg.a, dg.b), ftEmpty, dig2(dg.c, dg.d))
	}
	panic("digitToTree")
}

// pushFront adds an element to the left end — O(1) amortized.
func pushFront(a any, t *ftTree) *ftTree {
	switch t.kind {
	case 0:
		return ftSingle(a)
	case 1:
		return ftDeep(dig1(a), ftEmpty, dig1(t.single))
	}
	pr := t.prefix
	if pr.n < 4 {
		return ftDeep(pr.cons(a), t.middle, t.suffix)
	}
	// prefix full: push node3(b,c,d) into middle, new prefix = Two(a,b)
	node := newNode3(pr.b, pr.c, pr.d)
	return ftDeep(dig2(a, pr.a), pushFront(node, t.middle), t.suffix)
}

// pushBack adds an element to the right end — O(1) amortized.
func pushBack(t *ftTree, a any) *ftTree {
	switch t.kind {
	case 0:
		return ftSingle(a)
	case 1:
		return ftDeep(dig1(t.single), ftEmpty, dig1(a))
	}
	sf := t.suffix
	if sf.n < 4 {
		return ftDeep(t.prefix, t.middle, sf.snoc(a))
	}
	node := newNode3(sf.a, sf.b, sf.c)
	return ftDeep(t.prefix, pushBack(t.middle, node), dig2(sf.d, a))
}

// viewFront returns (head, tail) or panics if empty.
func viewFront(t *ftTree) (any, *ftTree) {
	switch t.kind {
	case 0:
		panic("viewFront on empty")
	case 1:
		return t.single, ftEmpty
	}
	x := t.prefix.head()
	rest := rotL(t.prefix, t.middle, t.suffix)
	return x, rest
}

// rotL rebuilds after removing the front of the prefix.
func rotL(pr digit, m *ftTree, sf digit) *ftTree {
	if pr.n > 1 {
		return ftDeep(pr.tail(), m, sf)
	}
	// prefix had one element, pull from middle
	if m.kind == 0 {
		return digitToTree(sf)
	}
	front, m2 := viewFront(m)
	node := front.(*ftNode)
	return ftDeep(nodeToDigit(node), m2, sf)
}

// viewBack returns (init, last) or panics if empty.
func viewBack(t *ftTree) (*ftTree, any) {
	switch t.kind {
	case 0:
		panic("viewBack on empty")
	case 1:
		return ftEmpty, t.single
	}
	x := t.suffix.last()
	rest := rotR(t.prefix, t.middle, t.suffix)
	return rest, x
}

func rotR(pr digit, m *ftTree, sf digit) *ftTree {
	if sf.n > 1 {
		return ftDeep(pr, m, sf.init())
	}
	if m.kind == 0 {
		return digitToTree(pr)
	}
	m2, back := viewBack(m)
	node := back.(*ftNode)
	return ftDeep(pr, m2, nodeToDigit(node))
}

// append concatenates two trees — O(log min(n1,n2)).
func appendTree(l, r *ftTree) *ftTree {
	return appendTree0(l, r)
}

func appendTree0(l, r *ftTree) *ftTree {
	switch {
	case l.kind == 0:
		return r
	case r.kind == 0:
		return l
	case l.kind == 1:
		return pushFront(l.single, r)
	case r.kind == 1:
		return pushBack(l, r.single)
	}
	return ftDeep(l.prefix, addDigits0(l.middle, l.suffix, r.prefix, r.middle), r.suffix)
}

// addDigits0..4 pack the "middle" elements between two spines into 2-3 nodes.
// Generated mechanically following Hinze & Paterson appendTree/addDigits pattern.

func addDigits0(m1 *ftTree, sf1 digit, pr2 digit, m2 *ftTree) *ftTree {
	a, b, c, d := sf1.a, sf1.b, sf1.c, sf1.d
	e, f, g, h := pr2.a, pr2.b, pr2.c, pr2.d
	switch sf1.n*10 + pr2.n {
	case 11:
		return appendTree1(m1, newNode2(a, e), m2)
	case 12:
		return appendTree1(m1, newNode3(a, e, f), m2)
	case 13:
		return appendTree2(m1, newNode2(a, e), newNode2(f, g), m2)
	case 14:
		return appendTree2(m1, newNode3(a, e, f), newNode2(g, h), m2)
	case 21:
		return appendTree1(m1, newNode3(a, b, e), m2)
	case 22:
		return appendTree2(m1, newNode2(a, b), newNode2(e, f), m2)
	case 23:
		return appendTree2(m1, newNode3(a, b, e), newNode2(f, g), m2)
	case 24:
		return appendTree2(m1, newNode3(a, b, e), newNode3(f, g, h), m2)
	case 31:
		return appendTree2(m1, newNode2(a, b), newNode2(c, e), m2)
	case 32:
		return appendTree2(m1, newNode3(a, b, c), newNode2(e, f), m2)
	case 33:
		return appendTree2(m1, newNode3(a, b, c), newNode3(e, f, g), m2)
	case 34:
		return appendTree3(m1, newNode3(a, b, c), newNode2(e, f), newNode2(g, h), m2)
	case 41:
		return appendTree2(m1, newNode3(a, b, c), newNode2(d, e), m2)
	case 42:
		return appendTree2(m1, newNode3(a, b, c), newNode3(d, e, f), m2)
	case 43:
		return appendTree3(m1, newNode3(a, b, c), newNode2(d, e), newNode2(f, g), m2)
	case 44:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, e, f), newNode2(g, h), m2)
	}
	panic("addDigits0")
}

func appendTree1(l *ftTree, n1 *ftNode, r *ftTree) *ftTree {
	switch {
	case l.kind == 0:
		return pushFront(n1, r)
	case r.kind == 0:
		return pushBack(l, n1)
	case l.kind == 1:
		return pushFront(l.single, pushFront(n1, r))
	case r.kind == 1:
		return pushBack(pushBack(l, n1), r.single)
	}
	return ftDeep(l.prefix, addDigits1(l.middle, l.suffix, n1, r.prefix, r.middle), r.suffix)
}

func addDigits1(m1 *ftTree, sf1 digit, x any, pr2 digit, m2 *ftTree) *ftTree {
	a, b, c, d := sf1.a, sf1.b, sf1.c, sf1.d
	e, f, g, h := pr2.a, pr2.b, pr2.c, pr2.d
	switch sf1.n*10 + pr2.n {
	case 11:
		return appendTree1(m1, newNode3(a, x, e), m2)
	case 12:
		return appendTree2(m1, newNode2(a, x), newNode2(e, f), m2)
	case 13:
		return appendTree2(m1, newNode3(a, x, e), newNode2(f, g), m2)
	case 14:
		return appendTree2(m1, newNode3(a, x, e), newNode3(f, g, h), m2)
	case 21:
		return appendTree2(m1, newNode2(a, b), newNode2(x, e), m2)
	case 22:
		return appendTree2(m1, newNode3(a, b, x), newNode2(e, f), m2)
	case 23:
		return appendTree2(m1, newNode3(a, b, x), newNode3(e, f, g), m2)
	case 24:
		return appendTree3(m1, newNode3(a, b, x), newNode2(e, f), newNode2(g, h), m2)
	case 31:
		return appendTree2(m1, newNode3(a, b, c), newNode2(x, e), m2)
	case 32:
		return appendTree2(m1, newNode3(a, b, c), newNode3(x, e, f), m2)
	case 33:
		return appendTree3(m1, newNode3(a, b, c), newNode2(x, e), newNode2(f, g), m2)
	case 34:
		return appendTree3(m1, newNode3(a, b, c), newNode3(x, e, f), newNode2(g, h), m2)
	case 41:
		return appendTree2(m1, newNode3(a, b, c), newNode3(d, x, e), m2)
	case 42:
		return appendTree3(m1, newNode3(a, b, c), newNode2(d, x), newNode2(e, f), m2)
	case 43:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, x, e), newNode2(f, g), m2)
	case 44:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, x, e), newNode3(f, g, h), m2)
	}
	panic("addDigits1")
}

func appendTree2(l *ftTree, n1, n2 *ftNode, r *ftTree) *ftTree {
	switch {
	case l.kind == 0:
		return pushFront(n1, pushFront(n2, r))
	case r.kind == 0:
		return pushBack(pushBack(l, n1), n2)
	case l.kind == 1:
		return pushFront(l.single, pushFront(n1, pushFront(n2, r)))
	case r.kind == 1:
		return pushBack(pushBack(pushBack(l, n1), n2), r.single)
	}
	return ftDeep(l.prefix, addDigits2(l.middle, l.suffix, n1, n2, r.prefix, r.middle), r.suffix)
}

func addDigits2(m1 *ftTree, sf1 digit, x, y any, pr2 digit, m2 *ftTree) *ftTree {
	a, b, c, d := sf1.a, sf1.b, sf1.c, sf1.d
	e, f, g, h := pr2.a, pr2.b, pr2.c, pr2.d
	switch sf1.n*10 + pr2.n {
	case 11:
		return appendTree2(m1, newNode2(a, x), newNode2(y, e), m2)
	case 12:
		return appendTree2(m1, newNode3(a, x, y), newNode2(e, f), m2)
	case 13:
		return appendTree2(m1, newNode3(a, x, y), newNode3(e, f, g), m2)
	case 14:
		return appendTree3(m1, newNode3(a, x, y), newNode2(e, f), newNode2(g, h), m2)
	case 21:
		return appendTree2(m1, newNode3(a, b, x), newNode2(y, e), m2)
	case 22:
		return appendTree2(m1, newNode3(a, b, x), newNode3(y, e, f), m2)
	case 23:
		return appendTree3(m1, newNode3(a, b, x), newNode2(y, e), newNode2(f, g), m2)
	case 24:
		return appendTree3(m1, newNode3(a, b, x), newNode3(y, e, f), newNode2(g, h), m2)
	case 31:
		return appendTree2(m1, newNode3(a, b, c), newNode3(x, y, e), m2)
	case 32:
		return appendTree3(m1, newNode3(a, b, c), newNode2(x, y), newNode2(e, f), m2)
	case 33:
		return appendTree3(m1, newNode3(a, b, c), newNode3(x, y, e), newNode2(f, g), m2)
	case 34:
		return appendTree3(m1, newNode3(a, b, c), newNode3(x, y, e), newNode3(f, g, h), m2)
	case 41:
		return appendTree3(m1, newNode3(a, b, c), newNode2(d, x), newNode2(y, e), m2)
	case 42:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, x, y), newNode2(e, f), m2)
	case 43:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, x, y), newNode3(e, f, g), m2)
	case 44:
		return appendTree4(m1, newNode3(a, b, c), newNode3(d, x, y), newNode2(e, f), newNode2(g, h), m2)
	}
	panic("addDigits2")
}

func appendTree3(l *ftTree, n1, n2, n3 *ftNode, r *ftTree) *ftTree {
	switch {
	case l.kind == 0:
		return pushFront(n1, pushFront(n2, pushFront(n3, r)))
	case r.kind == 0:
		return pushBack(pushBack(pushBack(l, n1), n2), n3)
	case l.kind == 1:
		return pushFront(l.single, pushFront(n1, pushFront(n2, pushFront(n3, r))))
	case r.kind == 1:
		return pushBack(pushBack(pushBack(pushBack(l, n1), n2), n3), r.single)
	}
	return ftDeep(l.prefix, addDigits3(l.middle, l.suffix, n1, n2, n3, r.prefix, r.middle), r.suffix)
}

func addDigits3(m1 *ftTree, sf1 digit, x, y, z any, pr2 digit, m2 *ftTree) *ftTree {
	a, b, c, d := sf1.a, sf1.b, sf1.c, sf1.d
	e, f, g, h := pr2.a, pr2.b, pr2.c, pr2.d
	switch sf1.n*10 + pr2.n {
	case 11:
		return appendTree2(m1, newNode3(a, x, y), newNode2(z, e), m2)
	case 12:
		return appendTree2(m1, newNode3(a, x, y), newNode3(z, e, f), m2)
	case 13:
		return appendTree3(m1, newNode3(a, x, y), newNode2(z, e), newNode2(f, g), m2)
	case 14:
		return appendTree3(m1, newNode3(a, x, y), newNode3(z, e, f), newNode2(g, h), m2)
	case 21:
		return appendTree2(m1, newNode3(a, b, x), newNode3(y, z, e), m2)
	case 22:
		return appendTree3(m1, newNode3(a, b, x), newNode2(y, z), newNode2(e, f), m2)
	case 23:
		return appendTree3(m1, newNode3(a, b, x), newNode3(y, z, e), newNode2(f, g), m2)
	case 24:
		return appendTree3(m1, newNode3(a, b, x), newNode3(y, z, e), newNode3(f, g, h), m2)
	case 31:
		return appendTree3(m1, newNode3(a, b, c), newNode2(x, y), newNode2(z, e), m2)
	case 32:
		return appendTree3(m1, newNode3(a, b, c), newNode3(x, y, z), newNode2(e, f), m2)
	case 33:
		return appendTree3(m1, newNode3(a, b, c), newNode3(x, y, z), newNode3(e, f, g), m2)
	case 34:
		return appendTree4(m1, newNode3(a, b, c), newNode3(x, y, z), newNode2(e, f), newNode2(g, h), m2)
	case 41:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, x, y), newNode2(z, e), m2)
	case 42:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, x, y), newNode3(z, e, f), m2)
	case 43:
		return appendTree4(m1, newNode3(a, b, c), newNode3(d, x, y), newNode2(z, e), newNode2(f, g), m2)
	case 44:
		return appendTree4(m1, newNode3(a, b, c), newNode3(d, x, y), newNode3(z, e, f), newNode2(g, h), m2)
	}
	panic("addDigits3")
}

func appendTree4(l *ftTree, n1, n2, n3, n4 *ftNode, r *ftTree) *ftTree {
	switch {
	case l.kind == 0:
		return pushFront(n1, pushFront(n2, pushFront(n3, pushFront(n4, r))))
	case r.kind == 0:
		return pushBack(pushBack(pushBack(pushBack(l, n1), n2), n3), n4)
	case l.kind == 1:
		return pushFront(l.single, pushFront(n1, pushFront(n2, pushFront(n3, pushFront(n4, r)))))
	case r.kind == 1:
		return pushBack(pushBack(pushBack(pushBack(pushBack(l, n1), n2), n3), n4), r.single)
	}
	return ftDeep(l.prefix, addDigits4(l.middle, l.suffix, n1, n2, n3, n4, r.prefix, r.middle), r.suffix)
}

func addDigits4(m1 *ftTree, sf1 digit, x, y, z, w any, pr2 digit, m2 *ftTree) *ftTree {
	a, b, c, d := sf1.a, sf1.b, sf1.c, sf1.d
	e, f, g, h := pr2.a, pr2.b, pr2.c, pr2.d
	switch sf1.n*10 + pr2.n {
	case 11:
		return appendTree2(m1, newNode3(a, x, y), newNode3(z, w, e), m2)
	case 12:
		return appendTree3(m1, newNode3(a, x, y), newNode2(z, w), newNode2(e, f), m2)
	case 13:
		return appendTree3(m1, newNode3(a, x, y), newNode3(z, w, e), newNode2(f, g), m2)
	case 14:
		return appendTree3(m1, newNode3(a, x, y), newNode3(z, w, e), newNode3(f, g, h), m2)
	case 21:
		return appendTree3(m1, newNode3(a, b, x), newNode2(y, z), newNode2(w, e), m2)
	case 22:
		return appendTree3(m1, newNode3(a, b, x), newNode3(y, z, w), newNode2(e, f), m2)
	case 23:
		return appendTree3(m1, newNode3(a, b, x), newNode3(y, z, w), newNode3(e, f, g), m2)
	case 24:
		return appendTree4(m1, newNode3(a, b, x), newNode3(y, z, w), newNode2(e, f), newNode2(g, h), m2)
	case 31:
		return appendTree3(m1, newNode3(a, b, c), newNode3(x, y, z), newNode2(w, e), m2)
	case 32:
		return appendTree3(m1, newNode3(a, b, c), newNode3(x, y, z), newNode3(w, e, f), m2)
	case 33:
		return appendTree4(m1, newNode3(a, b, c), newNode3(x, y, z), newNode2(w, e), newNode2(f, g), m2)
	case 34:
		return appendTree4(m1, newNode3(a, b, c), newNode3(x, y, z), newNode3(w, e, f), newNode2(g, h), m2)
	case 41:
		return appendTree3(m1, newNode3(a, b, c), newNode3(d, x, y), newNode3(z, w, e), m2)
	case 42:
		return appendTree4(m1, newNode3(a, b, c), newNode3(d, x, y), newNode2(z, w), newNode2(e, f), m2)
	case 43:
		return appendTree4(m1, newNode3(a, b, c), newNode3(d, x, y), newNode3(z, w, e), newNode2(f, g), m2)
	case 44:
		return appendTree4(m1, newNode3(a, b, c), newNode3(d, x, y), newNode3(z, w, e), newNode3(f, g, h), m2)
	}
	panic("addDigits4")
}

// iterTree visits all leaf elements in order; f returns false to stop early.
// Elements in prefix/suffix/single may be leaves OR *ftNode (at inner spine levels);
// iterAny expands them all the way down to leaves before calling f.
func iterTree(t *ftTree, f func(any) bool) bool {
	switch t.kind {
	case 0:
		return true
	case 1:
		return iterAny(t.single, f)
	}
	return iterAnyDigit(t.prefix, f) &&
		iterTree(t.middle, f) &&
		iterAnyDigit(t.suffix, f)
}

// iterAnyDigit expands each element of a digit (which may be *ftNode at inner levels).
func iterAnyDigit(dg digit, f func(any) bool) bool {
	if !iterAny(dg.a, f) {
		return false
	}
	if dg.n >= 2 && !iterAny(dg.b, f) {
		return false
	}
	if dg.n >= 3 && !iterAny(dg.c, f) {
		return false
	}
	if dg.n == 4 && !iterAny(dg.d, f) {
		return false
	}
	return true
}

// iterAny expands e: if it's a *ftNode, recurse into its children; otherwise call f.
func iterAny(e any, f func(any) bool) bool {
	if n, ok := e.(*ftNode); ok {
		if !iterAny(n.a, f) {
			return false
		}
		if !iterAny(n.b, f) {
			return false
		}
		if n.is3 && !iterAny(n.c, f) {
			return false
		}
		return true
	}
	return f(e)
}

// split splits the tree at position i: left has i elements, right has the rest.
// Returns (left, pivot, right) where pivot is the element at index i.
type ftSplit struct {
	left  *ftTree
	pivot any
	right *ftTree
}

// splitTreeAt splits at index i: returns (left[0..i-1], elem[i], right[i+1..]).
func splitTreeAt(t *ftTree, i int) ftSplit {
	if t.kind == 1 {
		return ftSplit{ftEmpty, t.single, ftEmpty}
	}
	pr := t.prefix
	if i < pr.size {
		// split falls in prefix
		l, x, r := splitDigitAt(pr, i)
		var lt *ftTree
		if l == nil {
			lt = ftEmpty
		} else {
			lt = digitToTree(*l)
		}
		return ftSplit{lt, x, deepLOpt(r, t.middle, t.suffix)}
	}
	i -= pr.size
	if i < t.middle.size {
		// split falls in middle spine
		ms := splitTreeAt(t.middle, i)
		node := ms.pivot.(*ftNode)
		ni := i - sizeOf(ms.left)
		nl, nx, nr := splitNodeAt(node, ni)
		return ftSplit{
			deepROpt(pr, ms.left, nl),
			nx,
			deepLOpt(nr, ms.right, t.suffix),
		}
	}
	i -= t.middle.size
	// split falls in suffix
	l, x, r := splitDigitAt(t.suffix, i)
	var rt *ftTree
	if r == nil {
		rt = ftEmpty
	} else {
		rt = digitToTree(*r)
	}
	return ftSplit{deepROpt(pr, t.middle, l), x, rt}
}

func sizeOf(t *ftTree) int {
	if t == nil {
		return 0
	}
	return t.size
}

// deepLOpt rebuilds the left side of a split: pr may be nil (pull from middle).
func deepLOpt(pr *digit, m *ftTree, sf digit) *ftTree {
	if pr == nil {
		// prefix exhausted — pull a node from middle to form new prefix
		if m.kind == 0 {
			return digitToTree(sf)
		}
		front, m2 := viewFront(m)
		return ftDeep(nodeToDigit(front.(*ftNode)), m2, sf)
	}
	return ftDeep(*pr, m, sf)
}

// deepROpt rebuilds the right side of a split: sf may be nil (pull from middle).
func deepROpt(pr digit, m *ftTree, sf *digit) *ftTree {
	if sf == nil {
		if m.kind == 0 {
			return digitToTree(pr)
		}
		m2, back := viewBack(m)
		return ftDeep(pr, m2, nodeToDigit(back.(*ftNode)))
	}
	return ftDeep(pr, m, *sf)
}

func splitDigitAt(dg digit, i int) (*digit, any, *digit) {
	elems := dg.toSlice()
	acc := 0
	for idx, e := range elems {
		acc += elemSize(e)
		if i < acc {
			var l, r *digit
			if idx > 0 {
				ld := sliceToDigit(elems[:idx])
				l = &ld
			}
			if idx < len(elems)-1 {
				rd := sliceToDigit(elems[idx+1:])
				r = &rd
			}
			return l, e, r
		}
	}
	panic("splitDigitAt out of range")
}

func splitNodeAt(n *ftNode, i int) (*digit, any, *digit) {
	elems := []any{n.a, n.b}
	if n.is3 {
		elems = append(elems, n.c)
	}
	return splitDigitAt(sliceToDigit(elems), i)
}

func sliceToDigit(s []any) digit {
	switch len(s) {
	case 1:
		return dig1(s[0])
	case 2:
		return dig2(s[0], s[1])
	case 3:
		return dig3(s[0], s[1], s[2])
	case 4:
		return dig4(s[0], s[1], s[2], s[3])
	}
	panic("sliceToDigit")
}

