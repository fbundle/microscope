package editor

import (
	"context"
	"microscope/core/util/text"

	"microscope/util/buffer"
)

type Cursor struct {
	Row int
	Col int
}

type Status struct {
	Message    string
	Background string
	Other      map[string]any // arbitrary view
}

type Window struct {
	TlRow  int
	TlCol  int
	Width  int
	Height int
}

type View struct {
	Text   text.Text
	Cursor Cursor
	Window Window
	Status Status
}

// window is a part of view since moving the cursor changes window's position

type Move interface {
	MoveLeft()
	MoveRight()
	MoveUp()
	MoveDown()
	MoveHome()
	MoveEnd()
	MovePageUp()
	MovePageDown()
	Goto(row int, col int)
}

type Edit interface {
	Type(ch rune)
	Backspace()
	Delete()
	Enter()
	Undo()
	Redo()

	InsertLine(t2 text.Text)
	DeleteLine(count int)

	Apply(entry LogEntry)
}

type Render interface {
	Render() View
	Update() <-chan View
}

type Editor interface {
	Load(ctx context.Context, reader buffer.Reader) (context.Context, error)
	Resize(height int, width int)
	Status(update func(status Status) Status)
	Action(key string, vals ...any) // arbitrary action
	Subscribe(func(LogEntry)) uint64
	Unsubscribe(key uint64)
	Render
	Edit
	Move
}
