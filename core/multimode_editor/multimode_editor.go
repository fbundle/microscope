package multimode_editor

import (
	"context"
	"fmt"
	"sync"
	"microscope/config"
	"microscope/core/editor"
	"microscope/core/insert_editor"
	"microscope/core/util/text"

	"microscope/util/buffer"
)

type Mode = string

const (
	ModeNormal  Mode = "NORMAL"
	ModeCommand Mode = "COMMAND"
	ModeInsert  Mode = "INSERT"
	ModeSelect  Mode = "SELECT"
)

type Selector struct {
	beg int
	end int
}

func (s *Selector) Interval() (beg int, end int) {
	beg, end = s.beg, s.end
	if beg > end {
		beg, end = end, beg
	}
	return beg, end
}

type clipboard = text.Text

type state struct {
	mode      Mode
	command   string
	selector  *Selector
	clipboard clipboard
}

type Editor struct {
	stop              func()
	mu                sync.Mutex
	e                 *insert_editor.Editor
	defaultOutputFile string
	state             state
}

func (c *Editor) enterNormalModeWithoutLock() {
	c.state.mode = ModeNormal
	c.state.command = ""
	c.state.selector = nil
}
func (c *Editor) enterInsertModeWithoutLock() {
	c.state.mode = ModeInsert
	c.state.command = ""
	c.state.selector = nil
}
func (c *Editor) enterCommandModeWithoutLock(command string) {
	c.state.mode = ModeCommand
	c.state.command = command
	c.state.selector = nil
}
func (c *Editor) enterSelectModeWithoutLock(beg int) {
	c.state.mode = ModeSelect
	c.state.command = ""
	c.state.selector = &Selector{
		beg: beg,
		end: beg,
	}
}

func (c *Editor) Update() <-chan editor.View {
	return c.e.Update()
}

func (c *Editor) Load(ctx context.Context, reader buffer.Reader) (loadCtx context.Context, err error) {
	c.lock(func() {
		loadCtx, err = c.e.Load(ctx, reader)
	})
	return loadCtx, err
}

func (c *Editor) Resize(height int, width int) {
	c.lock(func() {
		c.e.Resize(height, width)
	})
}

func (c *Editor) Goto(row int, col int) {
	c.lock(func() {
		c.e.Goto(row, col)
	})
}

func (c *Editor) MoveLeft() {
	c.lock(func() {
		c.e.MoveLeft()
	})
}

func (c *Editor) MoveRight() {
	c.lock(func() {
		c.e.MoveRight()
	})
}

func (c *Editor) MoveUp() {
	c.lock(func() {
		c.e.MoveUp()
		if c.state.mode == ModeSelect {
			row := c.e.Render().Cursor.Row
			c.state.selector.end = row
			c.writeWithoutLock("select more")
		}
	})
}

func (c *Editor) MoveDown() {
	c.lock(func() {
		c.e.MoveDown()
		if c.state.mode == ModeSelect {
			row := c.e.Render().Cursor.Row
			c.state.selector.end = row
			c.writeWithoutLock("select more")
		}
	})
}

func (c *Editor) MoveHome() {
	c.lock(func() {
		c.e.MoveHome()
	})
}

func (c *Editor) MoveEnd() {
	c.lock(func() {
		c.e.MoveEnd()
	})
}

func (c *Editor) MovePageUp() {
	c.lock(func() {
		c.e.MovePageUp()
		c.maybeUpdateSelectorEndWithoutLock()
	})
}

func (c *Editor) MovePageDown() {
	c.lock(func() {
		c.e.MovePageDown()
		c.maybeUpdateSelectorEndWithoutLock()
	})
}

func (c *Editor) Undo() {
	c.lock(func() {
		c.e.Undo()
	})
}

func (c *Editor) Redo() {
	c.lock(func() {
		c.e.Redo()
	})
}

func (c *Editor) Apply(entry editor.LogEntry) {
	c.lock(func() {
		c.e.Apply(entry)
	})
}

func (c *Editor) Render() editor.View {
	return c.e.Render()
}

func (c *Editor) Status(update func(status editor.Status) editor.Status) {
	c.lock(func() {
		c.e.Status(update)
	})
}
func (c *Editor) InsertLine(t2 text.Text) {
	c.lock(func() {
		c.e.InsertLine(t2)
	})
}

func (c *Editor) DeleteLine(count int) {
	c.lock(func() {
		c.e.DeleteLine(count)
	})
}

func New(e *insert_editor.Editor, stop func(), defaultOutputFile string) *Editor {
	c := &Editor{
		stop:              stop,
		mu:                sync.Mutex{},
		e:                 e,
		defaultOutputFile: defaultOutputFile,
		state: state{
			mode:      ModeNormal,
			command:   "",
			selector:  nil,
			clipboard: text.Text{},
		},
	}
	c.writeWithoutLock("")
	return c
}

func (c *Editor) lock(f func()) {
	c.mu.Lock()
	defer c.mu.Unlock()

	f()
}

func (c *Editor) writeWithoutLock(message string) {
	c.e.Status(func(status editor.Status) editor.Status {
		if status.Other == nil {
			status.Other = make(map[string]any)
		}
		status.Other["command"] = c.state.command
		status.Other["mode"] = c.state.mode
		status.Other["selector"] = c.state.selector
		status.Message = message
		return status
	})
}

func (c *Editor) Action(key string, vals ...any) {
	c.lock(func() {
		// TODO - consider if we should move these mouse action into the API
		switch key {
		case "mouse_click_left":
			if c.state.mode == ModeInsert { // click only works for insert mode
				p := vals[0].(editor.Cursor)
				relRow, relCol := p.Row, p.Col
				view := c.e.Render()
				tlRow, tlCol := view.Window.TlRow, view.Window.TlCol
				row, col := tlRow+relRow, tlCol+relCol
				c.e.Goto(row, col)
			}
		case "mouse_scroll_up":
			for i := 0; i < config.Load().SCROLL_SPEED; i++ {
				c.e.MoveUp()
			}
		case "mouse_scroll_down":
			for i := 0; i < config.Load().SCROLL_SPEED; i++ {
				c.e.MoveDown()
			}
		case "mouse_scroll_left":
			for i := 0; i < config.Load().SCROLL_SPEED; i++ {
				c.e.MoveLeft()
			}
		case "mouse_scroll_right":
			for i := 0; i < config.Load().SCROLL_SPEED; i++ {
				c.e.MoveRight()
			}
		case "key_escape":
			c.keyEscapeWithoutLock()
		case "key_tabular":
			if c.state.mode == ModeInsert {
				for i := 0; i < config.Load().TAB_SIZE; i++ {
					c.e.Type(' ')
				}
			}
		default:
			c.writeWithoutLock(fmt.Sprintf("action not supported: %s", key))
		}
	})
}

func (c *Editor) Subscribe(consume func(editor.LogEntry)) uint64 {
	var key uint64
	c.lock(func() {
		key = c.e.Subscribe(consume)
	})
	return key
}
func (c *Editor) Unsubscribe(key uint64) {
	c.lock(func() {
		c.e.Unsubscribe(key)
	})
}
