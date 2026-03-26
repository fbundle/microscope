package insert_editor

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"microscope/config"
	"microscope/core/editor"
	"microscope/core/util/hist"
	"microscope/core/util/text"
	"time"

	"microscope/util/subsciber_pool"

	"microscope/util/buffer"
)

type Editor struct {
	renderCh chan editor.View

	mu     sync.Mutex // the fields below are protected by mu
	text   *hist.Hist[text.Text]
	cursor editor.Cursor
	window editor.Window
	status editor.Status
	pool   *subsciber_pool.Pool[func(editor.LogEntry)]
}

func New(
	height int, width int,
) (*Editor, error) {
	e := &Editor{
		// buffered iterator is necessary  for preventing deadlock
		renderCh: make(chan editor.View, config.Load().VIEW_CHANNEL_SIZE),

		mu:   sync.Mutex{},
		text: nil, // awaiting Load
		cursor: editor.Cursor{
			Row: 0, Col: 0,
		},
		window: editor.Window{
			TlRow:  0,
			TlCol:  0,
			Width:  width,
			Height: height,
		},
		status: editor.Status{
			Message:    "",
			Background: "",
			Other:      nil,
		},
		pool: subsciber_pool.New[func(editor.LogEntry)](),
	}
	return e, nil
}

func (e *Editor) lock(f func()) {
	e.mu.Lock()
	defer e.mu.Unlock()

	f()
}

func (e *Editor) lockRender(f func()) {
	e.mu.Lock()
	defer e.mu.Unlock()
	defer e.renderWithoutLock()

	f()
}

func (e *Editor) setMessageWithoutLock(format string, a ...any) {
	e.status.Message = fmt.Sprintf(format, a...)
}

func (e *Editor) writeLogWithoutLock(entry editor.LogEntry) {
	go func() {
		for _, consume := range e.pool.Iter {
			consume(entry)
		}
	}()
}

func (e *Editor) Subscribe(consume func(editor.LogEntry)) uint64 {
	e.mu.Lock()
	defer e.mu.Unlock()
	return e.pool.Subscribe(consume)
}

func (e *Editor) Unsubscribe(key uint64) {
	e.mu.Lock()
	defer e.mu.Unlock()
	e.pool.Unsubscribe(key)
}

func (e *Editor) Resize(height int, width int) {
	e.lockRender(func() {
		if e.window.Height == height && e.window.Width == width {
			return
		}
		e.window.Height, e.window.Width = height, width
		e.moveRelativeAndFixWithoutLock(0, 0)
		e.setMessageWithoutLock("resize to %dx%d", height, width)
	})
}

func (e *Editor) Status(update func(status editor.Status) editor.Status) {
	e.lockRender(func() {
		e.status = update(e.status)
	})
}

func (e *Editor) load(ctx context.Context, reader buffer.Reader, loadDone func()) {
	t0 := time.Now()
	defer loadDone()
	defer e.lockRender(func() {
		totalTime := time.Since(t0)
		e.status.Background = ""
		select {
		case <-ctx.Done():
			e.status.Message = fmt.Sprintf(
				"loading was cancelled after %d seconds",
				int(totalTime.Seconds()),
			)
		default:
			e.status.Message = fmt.Sprintf(
				"loaded for %d seconds",
				int(totalTime.Seconds()),
			)
		}
	})
	if reader == nil {
		return // nothing to load
	}

	loader := newLoader(reader.Len())

	lastPoll := time.Now()
	for offset := range text.IndexFile(reader) {
		now := time.Now()
		if now.Sub(lastPoll) >= config.Load().LOAD_ESCAPE_INTERVAL {
			lastPoll = now
			if !pollCtx(ctx) {
				return
			}
		}
		e.lock(func() {
			e.text.Update(func(t text.Text) text.Text {
				return t.Append(text.MakeLineFromOffset(offset))
			})
			if loader.set(offset) {
				e.status.Background = fmt.Sprintf(
					"loading %d/%d (%d%%)",
					loader.loadedSize, loader.totalSize, loader.lastRenderPercentage,
				)
				e.renderWithoutLock()
			}
		})
	}

}

func (e *Editor) Load(ctx context.Context, reader buffer.Reader) (context.Context, error) {
	loadCtx, loadDone := context.WithCancel(context.Background())
	var err error = nil
	e.lockRender(func() {
		if e.text != nil {
			loadDone()
			err = errors.New("load twice")
			return
		}
		e.text = hist.New(text.New(reader))
		// load file asynchronously
		go e.load(ctx, reader, loadDone)
		e.status.Background = "loading started"
	})

	return loadCtx, err
}

func (e *Editor) Action(key string, vals ...any) {
	// nothing
}
