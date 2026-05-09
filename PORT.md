# Port Report — Go → Rust

**Date:** 2026-05-09
**Branch:** `claude_rust`
**Ported by:** Claude Sonnet 4.6 (claude-sonnet-4-6), Anthropic
**Go source:** 4 207 lines across 34 files
**Rust source:** 3 893 lines across 35 files (≈ 1:1 line ratio; Rust is more explicit but more concise in some areas)

---

## Motivation

The Go version cannot represent `Line` in fewer than 16 bytes because the GC requires all pointers to be stored as genuine pointer-typed values; packing a pointer into a `uintptr` integer would make it invisible to the garbage collector, allowing premature collection. For a file with billions of short lines the line index alone would consume 16 GB of RAM, defeating the purpose of lazy loading.

Rust's memory model allows safe pointer tagging through explicit `unsafe` blocks with well-defined invariants. This port reduces `Line` to **8 bytes**, halving the line-index footprint.

---

## Key design change: 8-byte `Line`

### Go (16 bytes)
```go
type Line struct {
    offset int64   // 8 bytes — file byte offset, or -1 if in-memory
    data   *[]byte // 8 bytes — non-nil only for in-memory lines
}
```

### Rust (8 bytes)
```rust
#[repr(transparent)]
pub struct Line(u64);
// bit 0 = 0  →  file offset.  Actual offset = value >> 1.  Max ≈ 4.6 EB.
// bit 0 = 1  →  Arc<Vec<u8>> pointer with bit 0 cleared.
```

**Safety invariants:**
- `Arc<Vec<u8>>` heap allocation is at least 8-byte aligned → bit 0 of the pointer is always 0 before tagging.
- x86-64 user-space virtual addresses use only bits 0–47; bits 48–63 are 0 → no collision with the tag bit.
- `Clone` manually increments the Arc ref count via `Arc::from_raw` / `Arc::into_raw`; `Drop` decrements it. The GC-equivalent lifetime is preserved without a garbage collector.

**Impact:** at one billion lines (a ~10 GB file of short lines) the line index shrinks from ~16 GB to ~8 GB.

---

## Finger tree: typed enum instead of `any`

### Go
```go
// Elements at inner levels stored as `any` (type-erased interface).
// Runtime cast: e.(*ftNode)
type ftNode struct { size int; a, b, c any; is3 bool }
```

### Rust
```rust
#[derive(Clone)]
pub enum Elem<T: Clone + Send + Sync + 'static> {
    Leaf(T),
    Node(Arc<InnerNode<T>>),
}
```

The enum variant carries the level information that Go expresses through runtime type assertions. `iter_elem` matches on `Leaf` vs `Node` without any `unsafe` cast. The compiler verifies that no non-leaf value can escape as a `T`.

---

## Structural mapping

| Go package | Rust module | Notes |
|---|---|---|
| `util/persistent/seq` | `src/util/persistent/seq.rs` | Finger tree; typed `Elem<T>` replaces `any` |
| `util/persistent/stack` | `src/util/persistent/stack.rs` | Direct port |
| `util/buffer` | `src/util/buffer.rs` | `Reader` trait; `MmapReader` wraps `memmap2::Mmap` |
| `util/side_channel` | `src/util/side_channel.rs` | `side_log!` / `side_panic!` macros |
| `util/subsciber_pool` | `src/util/subscriber_pool.rs` | `Arc<Mutex<HashMap>>` replaces `sync.Map` |
| `util/file_util` | `src/util/file_util.rs` | `safe_write_file`: write to tmp, then rename |
| `util/sync_util` | — | Replaced by `std::collections::HashMap` under a `Mutex` |
| `core/editor` | `src/core/editor.rs` | `Cursor`, `Window`, `Status`, `View`, `LogEntry`, `Command` |
| `core/util/hist` | `src/core/hist.rs` | `Hist<T: Clone>` with size cap |
| `core/util/text` | `src/core/text/` | `Line` (8 bytes), `index_file`, `Text` |
| `core/insert_editor` | `src/core/insert_editor/` | All edit / move / render / load logic |
| `core/multimode_editor` | `src/core/multimode_editor/` | NORMAL / INSERT / COMMAND / SELECT dispatch |
| `core/log_writer` | `src/core/log_writer/` | Length-prefix + JSON; log files are cross-compatible with Go |
| `ui` | `src/ui/` | crossterm event loop, draw, replay, log print |
| `config` | `src/config.rs` | `OnceLock` singleton replaces `sync.Mutex` + pointer |
| `cmd/microscope` | `src/main.rs` | Arg parsing, dispatch |

---

## Dependency mapping

| Go dependency | Rust crate |
|---|---|
| `github.com/gdamore/tcell/v2` | `crossterm 0.28` |
| `golang.org/x/exp/mmap` | `memmap2 0.9` |
| `encoding/json` | `serde 1` + `serde_json 1` |
| `regexp` | `regex 1` |
| `sync.Mutex` / `sync.Map` | `std::sync::Mutex` / `std::collections::HashMap` |
| `context.Context` | `Arc<AtomicBool>` stop flag + `std::sync::mpsc` |

---

## Threading model changes

| Concern | Go | Rust |
|---|---|---|
| Shared editor state | `sync.Mutex` on `Editor` struct | `Arc<Mutex<InsertEditorInner>>` |
| Render channel | `chan editor.View` (64-buffered) | `std::sync::mpsc::sync_channel(64)` |
| Log flush timer | `time.Ticker` goroutine | Background `std::thread` with `sleep` |
| File loading | `go e.load(ctx, reader, done)` | `std::thread::spawn` |
| Stop signal | `context.CancelFunc` | `Arc<AtomicBool>` + synthetic terminal event |
| Log subscriber callbacks | `go func() { for consume := range pool.Iter }` | Inline call under mutex (simpler; log writes are fast) |

---

## What is not yet complete

The Rust port compiles cleanly (`cargo build`: 0 errors, 46 style warnings) and the binary responds correctly to `--version`, `-h`, `-r` (replay), and `-l` (log print). The following items from the Go version are stubs or partially wired:

1. **Interactive TUI** (`RunEditor`): the crossterm event loop and draw function are written and structurally correct, but the render-channel consumer thread and the draw call are not fully connected to the live editor state. A session can be opened but keystrokes do not yet update the screen.

2. **Async file-loading progress**: the background loading thread indexes lines and appends them to the finger tree, but progress messages are not relayed to the render channel.

3. **Binary serializer**: marked TODO in both Go and Rust versions; not implemented in either.

4. **Experimental parallel indexing**: the `PARALLEL_INDEXING` Go experiment has no Rust equivalent yet.

These items require wiring the existing correct components together rather than new algorithmic work.

---

## Build

```
cargo build
# 0 errors, 46 warnings (unused imports / dead code from partial wiring)
# Finished dev profile

cargo run -- --version
# microscope version 0.1.9

cargo run -- --help
# (prints help text)
```
