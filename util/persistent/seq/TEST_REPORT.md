# Test Report — util/persistent/seq (finger tree)

**Date:** 2026-05-09
**Implementation:** 2-3 finger tree, ported from `fingertree-0.1.6.3` (Hinze & Paterson, 2006)
**Ported by:** Claude Sonnet 4.6 (claude-sonnet-4-6), Anthropic

---

## Result

11/11 tests passed. Total run time: ~1.6 s.

---

## Tests

### Depth / structural integrity

These tests verify that spine depth stays within `2·log₂(n) + 4` under adversarial access patterns.

| Test | n | Pattern | Result |
|------|---|---------|--------|
| TestDepthPushFrontOnly | 100 000 | always push to front | PASS |
| TestDepthPushBackOnly | 100 000 | always push to back | PASS |
| TestDepthAlternatingEnds | 100 000 | alternating front/back | PASS |
| TestDepthRepeatedMerge | 100 000 | merge two 50 000-element sequences | PASS |
| TestDepthRepeatedSplit | 100 000 | split at midpoint 20 times | PASS |
| TestDepthRandomOps | 200 000 | pushFront / pushBack / popFront / split+merge | PASS |
| TestDepthMergeCascade | 100 000 | cascade-merge 1 000 × 100-element sequences | PASS |
| TestDepthInsertMiddle | 10 000 | always insert at exact midpoint | PASS |
| TestDepthDeleteMiddle | 10 000 | always delete exact midpoint | PASS |

### Correctness

| Test | Detail | Result |
|------|--------|--------|
| TestCorrectnessRandomInsertDelete | 5 000 random insert/delete ops compared against a reference slice | PASS |

### Depth growth rate

Depth measured at every power of 2 from n=1 to n=131 072 (pushBack only).

| n | depth | bound (2·log₂n + 4) |
|---|-------|---------------------|
| 1 | 1 | 2 |
| 2 | 1 | 6 |
| 4 | 1 | 8 |
| 8 | 2 | 10 |
| 16 | 2 | 12 |
| 32 | 3 | 14 |
| 64 | 3 | 16 |
| 128 | 4 | 18 |
| 256 | 5 | 20 |
| 512 | 5 | 22 |
| 1 024 | 6 | 24 |
| 2 048 | 7 | 26 |
| 4 096 | 7 | 28 |
| 8 192 | 8 | 30 |
| 16 384 | 8 | 32 |
| 32 768 | 9 | 34 |
| 65 536 | 10 | 36 |
| 131 072 | 10 | 38 |

Depth grows at roughly log₃(n) in practice (tighter than the log₂ bound), consistent with 2-3 node fanout in the spine.

---

## Files

| File | Description |
|------|-------------|
| `node.go` | Internal finger tree types and operations |
| `seq.go` | Public `Seq[T]` API |
| `seq_extra.go` | Convenience methods (Front, Back, PushFront, PushBack, Slice, …) |
| `seq_test.go` | This test suite |
| `FingerTree.hs` | Original Haskell source (reference) |
