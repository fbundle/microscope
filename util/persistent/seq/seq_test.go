package seq

import (
	"math"
	"math/rand"
	"testing"
)

// depthBound returns the theoretical maximum depth for a finger tree of size n.
// A 2-3 finger tree of size n has spine depth ≤ log₂(n) + 2 (generous bound).
// We use 2*log2(n)+4 as a comfortable upper bound.
func depthBound(n int) int {
	if n <= 1 {
		return 2
	}
	return int(2*math.Log2(float64(n))) + 4
}

// checkDepth fails the test if the actual depth exceeds the bound.
func checkDepth(t *testing.T, s Seq[int], label string) {
	t.Helper()
	n := s.Len()
	d := s.Depth()
	bound := depthBound(n)
	if d > bound {
		t.Errorf("%s: n=%d depth=%d exceeds bound=%d", label, n, d, bound)
	}
}

// checkContents verifies the sequence holds exactly [lo, lo+1, ..., hi-1].
func checkContents(t *testing.T, s Seq[int], lo, hi int, label string) {
	t.Helper()
	want := hi - lo
	if s.Len() != want {
		t.Errorf("%s: len=%d want=%d", label, s.Len(), want)
		return
	}
	for i := lo; i < hi; i++ {
		got := s.Get(i - lo)
		if got != i {
			t.Errorf("%s: Get(%d)=%d want=%d", label, i-lo, got, i)
			return
		}
	}
}

// --- adversarial construction patterns ---

func TestDepthPushFrontOnly(t *testing.T) {
	s := Empty[int]()
	for i := 0; i < 100_000; i++ {
		s = Seq[int]{t: pushFront(i, s.t)}
		if i > 0 && i%10_000 == 0 {
			checkDepth(t, s, "pushFront")
		}
	}
	checkDepth(t, s, "pushFront final")
	// values are in reverse: s[0]=99999, s[1]=99998, ...
	for i := 0; i < 10; i++ {
		want := 100_000 - 1 - i
		if s.Get(i) != want {
			t.Errorf("pushFront: Get(%d)=%d want=%d", i, s.Get(i), want)
		}
	}
}

func TestDepthPushBackOnly(t *testing.T) {
	s := Empty[int]()
	for i := 0; i < 100_000; i++ {
		s = Seq[int]{t: pushBack(s.t, i)}
		if i > 0 && i%10_000 == 0 {
			checkDepth(t, s, "pushBack")
		}
	}
	checkDepth(t, s, "pushBack final")
	checkContents(t, s, 0, 100_000, "pushBack")
}

func TestDepthAlternatingEnds(t *testing.T) {
	s := Empty[int]()
	for i := 0; i < 100_000; i++ {
		if i%2 == 0 {
			s = Seq[int]{t: pushFront(i, s.t)}
		} else {
			s = Seq[int]{t: pushBack(s.t, i)}
		}
		if i > 0 && i%10_000 == 0 {
			checkDepth(t, s, "alternating")
		}
	}
	checkDepth(t, s, "alternating final")
}

func TestDepthRepeatedMerge(t *testing.T) {
	// Build two large sequences and merge repeatedly.
	n := 50_000
	a := Empty[int]()
	b := Empty[int]()
	for i := 0; i < n; i++ {
		a = Seq[int]{t: pushBack(a.t, i)}
		b = Seq[int]{t: pushBack(b.t, n+i)}
	}
	checkDepth(t, a, "pre-merge a")
	checkDepth(t, b, "pre-merge b")
	merged := a.Merge(b)
	checkDepth(t, merged, "merged")
	checkContents(t, merged, 0, 2*n, "merged")
}

func TestDepthRepeatedSplit(t *testing.T) {
	// Build a large sequence, split repeatedly, check depth at each step.
	n := 100_000
	s := Empty[int]()
	for i := 0; i < n; i++ {
		s = Seq[int]{t: pushBack(s.t, i)}
	}
	for i := 0; i < 20; i++ {
		mid := s.Len() / 2
		l, r := s.Split(mid)
		checkDepth(t, l, "split-l")
		checkDepth(t, r, "split-r")
		s = l // keep shrinking
	}
}

func TestDepthRandomOps(t *testing.T) {
	rng := rand.New(rand.NewSource(42))
	s := Empty[int]()
	for i := 0; i < 200_000; i++ {
		op := rng.Intn(4)
		switch op {
		case 0:
			s = Seq[int]{t: pushFront(i, s.t)}
		case 1:
			s = Seq[int]{t: pushBack(s.t, i)}
		case 2:
			if s.Len() > 0 {
				_, rest := viewFront(s.t)
				s = Seq[int]{t: rest}
			}
		case 3:
			if s.Len() > 1 {
				mid := s.Len() / 2
				l, r := s.Split(mid)
				s = l.Merge(r)
			}
		}
		if i > 0 && i%20_000 == 0 {
			checkDepth(t, s, "random")
		}
	}
	checkDepth(t, s, "random final")
}

func TestDepthMergeCascade(t *testing.T) {
	// Adversarial: merge many small sequences into one large one (cascade).
	k := 1000
	seqs := make([]Seq[int], k)
	for i := range seqs {
		seqs[i] = Empty[int]()
		for j := 0; j < 100; j++ {
			seqs[i] = Seq[int]{t: pushBack(seqs[i].t, i*100+j)}
		}
	}
	s := seqs[0]
	for i := 1; i < k; i++ {
		s = s.Merge(seqs[i])
	}
	checkDepth(t, s, "cascade merge")
	checkContents(t, s, 0, k*100, "cascade merge")
}

func TestDepthInsertMiddle(t *testing.T) {
	// Always insert at the exact middle — adversarial for some tree structures.
	s := Empty[int]()
	for i := 0; i < 10_000; i++ {
		mid := s.Len() / 2
		s = s.Ins(mid, i)
		if i > 0 && i%1000 == 0 {
			checkDepth(t, s, "insert-middle")
		}
	}
	checkDepth(t, s, "insert-middle final")
}

func TestDepthDeleteMiddle(t *testing.T) {
	n := 10_000
	s := Empty[int]()
	for i := 0; i < n; i++ {
		s = Seq[int]{t: pushBack(s.t, i)}
	}
	for s.Len() > 1 {
		s = s.Del(s.Len() / 2)
	}
	checkDepth(t, s, "delete-middle final")
}

// --- correctness under stress ---

func TestCorrectnessRandomInsertDelete(t *testing.T) {
	rng := rand.New(rand.NewSource(7))
	ref := make([]int, 0, 1000)
	s := Empty[int]()

	for iter := 0; iter < 5000; iter++ {
		if len(ref) > 0 && rng.Intn(3) == 0 {
			// delete random position
			i := rng.Intn(len(ref))
			ref = append(ref[:i], ref[i+1:]...)
			s = s.Del(i)
		} else {
			// insert at random position
			v := rng.Intn(10000)
			i := rng.Intn(len(ref) + 1)
			ref = append(ref, 0)
			copy(ref[i+1:], ref[i:])
			ref[i] = v
			s = s.Ins(i, v)
		}
		if iter%500 == 0 {
			// full contents check
			if s.Len() != len(ref) {
				t.Fatalf("iter %d: len mismatch got=%d want=%d", iter, s.Len(), len(ref))
			}
			for i, want := range ref {
				if got := s.Get(i); got != want {
					t.Fatalf("iter %d: Get(%d)=%d want=%d", iter, i, got, want)
				}
			}
			checkDepth(t, s, "random-ins-del")
		}
	}
}

func TestDepthGrowthRate(t *testing.T) {
	// Verify depth is O(log n): at each power of 2, depth must be ≤ 2*log2(n)+4.
	s := Empty[int]()
	next := 1
	for i := 0; i < 1<<17; i++ {
		s = Seq[int]{t: pushBack(s.t, i)}
		if s.Len() == next {
			n := s.Len()
			d := s.Depth()
			bound := depthBound(n)
			if d > bound {
				t.Errorf("n=%d depth=%d exceeds log bound=%d", n, d, bound)
			}
			t.Logf("n=%-8d depth=%d (bound=%d)", n, d, bound)
			next *= 2
		}
	}
}

