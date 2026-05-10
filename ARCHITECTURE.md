# Architecture

Microscope is a TUI text editor designed for instant editing of arbitrarily large files, backed by memory-mapped I/O, a persistent immutable data structure, and an append-only event log. See the README for motivation and design goals.

---

## High-level structure

```
cmd/microscope/
  main.go               entry point, argument parsing, mode dispatch

ui/
  editor.go             event loop, draw loop, input routing
  draw.go               tcell rendering
  replay.go             headless replay to stdout
  log.go                log inspection mode

core/
  editor/               shared types and the Editor interface
  multimode_editor/     modal editing (normal/insert/select/command)
  insert_editor/        core text editing engine
  log_writer/           event log serialization (reader + writer)
  util/
    text/               immutable line-based text data structure
    hist/               generic undo/redo history stack

util/
  buffer/               byte-level file reader abstraction
  subsciber_pool/       generic pub/sub registry
  side_channel/         emergency panic logging
  sync_util/            sync.Map wrapper
  file_util/            safe file write
```

---

## Layering

```
UI  ──────────────────────────────────────────────────────────────────
     tcell screen + event loop (ui/editor.go)
     │  communicates via: Editor interface + View channel
     ▼
Core ─────────────────────────────────────────────────────────────────
     multimode_editor.Editor   (modal state machine, wraps insert_editor)
     │  delegates to:
     ▼
     insert_editor.Editor      (text mutations, cursor, undo/redo, event emission)
     │  contains:
     ├─ hist.Hist[text.Text]   (undo/redo stack over immutable text snapshots)
     ├─ text.Text              (immutable, persistent line sequence)
     └─ subsciber_pool.Pool    (event subscribers, e.g. log writer)
     │  emits events to:
     ▼
     log_writer.Writer         (serializes LogEntry to disk)
```

The `Editor` interface (`core/editor/editor.go`) is the boundary between UI and core. The UI only depends on this interface; it never imports `insert_editor` or `multimode_editor` directly (except to construct them).

---

## Data flow

**Editing a character:**

1. `tcell.PollEvent` returns a key event in the UI event loop.
2. `handleEditorKey` calls `e.Type(ch)` on the `Editor` interface.
3. `multimode_editor` checks current mode; if in `ModeInsert`, delegates to `insert_editor.Type(ch)`.
4. `insert_editor` acquires its mutex, updates `hist.Hist[text.Text]` (producing a new immutable snapshot), moves the cursor, and emits a `LogEntry` to all subscribers in a goroutine.
5. `insert_editor` sends the new `View` on `renderCh` (buffered channel).
6. The draw goroutine receives the `View` and calls `draw(screen, view)`.
7. The log writer subscriber receives the `LogEntry` and serializes it to disk.

**Replaying a log:**

1. `ui.RunReplay` creates an `insert_editor` without a log writer subscriber.
2. Waits for the background file load to complete via `<-loadCtx.Done()`.
3. Calls `log_writer.Read`, which streams `LogEntry` values from the file.
4. Each entry is applied via `insert_editor.Apply(entry)`, which repositions the cursor and re-runs the original operation.
5. Final text state is printed to stdout line by line.

---

## Concurrency model

Two independent mutexes, never nested:

| Component | Mutex | Protects |
|---|---|---|
| `insert_editor` | `mu` | text, cursor, window, status, subscriber pool |
| `multimode_editor` | `mu` | mode, command string, selector, clipboard |

`multimode_editor` acquires its own lock, then calls methods on `insert_editor`, which acquire `insert_editor`'s lock. This is safe because `multimode_editor` never calls back into itself from within `insert_editor` — the call graph is strictly one-directional.

Goroutines spawned at runtime:

| Goroutine | Lifetime | Purpose |
|---|---|---|
| Draw loop | Program lifetime | Consumes `renderCh`, calls `draw` |
| Auto-flush loop | Program lifetime | Periodically flushes the log writer |
| File loader | Until load completes | Appends lines to `text.Text` in chunks |
| Subscriber notification | Per-mutation, short-lived | Notifies all log subscribers without blocking the editor |

The render channel is buffered (`VIEW_CHANNEL_SIZE`). This decouples edit throughput from rendering speed; if the draw loop falls behind, the editor continues and older views are skipped.

---

## Text data structure

`text.Text` is an immutable value backed by a persistent finger tree (`util/persistent/seq`). Mutations (`Set`, `Ins`, `Del`) return a new `Text` sharing structure with the original — no full copies.

Lines have two storage modes:
- **File-backed**: stores a byte offset into the original `buffer.Reader`; content is read on demand.
- **In-memory**: stores a `[]byte` directly, used for lines created or modified during editing.

This means opening a large file is cheap: the initial `Text` has no in-memory data, only file offsets. Lines are materialized lazily when accessed.

---

## Undo/redo

`hist.Hist[text.Text]` is a fixed-size stack of `Text` snapshots. Because `Text` is a persistent data structure, snapshots share most of their memory. `Undo` and `Redo` move a pointer within the stack; no copying occurs.

Undo history is local to `insert_editor`. The event log on disk is append-only and records `Undo`/`Redo` as explicit events, so replay produces the same final state including all undone and redone edits.

---

## Event log

`LogEntry` is the event type. Every edit operation in `insert_editor` emits one before mutating state. The entry captures the command and the cursor position at the time of the operation, making each entry self-contained for replay.

The log file uses length-prefixed binary framing. The first entry is always a `SetVersion` record that tells the reader which serializer to use for subsequent entries. This allows the wire format to evolve: the reader starts with the initial deserializer, and switches on seeing `SetVersion`.

---

## Modes (multimode_editor)

Four modes:

| Mode | Entry | Behavior |
|---|---|---|
| `NORMAL` | Escape, or on startup | Navigation and commands; `i` enters insert, `V` enters select, `:` or `/` enters command |
| `INSERT` | `i` from normal | All typing and movement delegated directly to `insert_editor` |
| `SELECT` | `V` from normal | Line-range selection; movement extends the selection; `y`/`d` copy/cut |
| `COMMAND` | `:` or `/` from normal | Accumulates a command string; Enter executes it |

Mode state is entirely in `multimode_editor`. The `insert_editor` has no concept of modes.

Commands supported: `:w` / `:wq` (write/quit), `:q` (quit), `:g N` / `:N` (goto line), `/pattern` / `:s pattern` (search), `:regex pattern` (regex search), `:w filename` (write to path).

---

## UI and rendering

The UI (`ui/editor.go`, `ui/draw.go`) contains:
- The tcell event loop
- Input routing (`handleEditorKey`, `handleEditorMouse`)
- The draw loop goroutine
- Editor construction (`makeInsertEditor`)

`draw(screen, view)` is a pure function of the `View` value. It renders:
- Text content occupying rows 0 to `height-2`
- A status bar at row `height-1` showing mode, cursor position, command buffer, and message
- Selection highlighting for rows in the active selector range

The status bar color varies by mode (gray/normal, yellow/insert, green/select, blue/command).

Mouse clicks translate screen coordinates to text coordinates using the viewport offset stored in `View.Window`.
