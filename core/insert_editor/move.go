package insert_editor

import (
	"microscope/core/editor"
	"microscope/core/util/text"
)

func (e *Editor) gotoAndFixWithoutLock(row int, col int) {
	finalizeCursorAndWindow := func(curRow int, curCol int, tlRow int, tlCol int, width int, height int, t text.Text) (newCurRow int, newCurCol int, newTlRow int, newTlCol int) {
		// fix cursor according text
		if t.Len() == 0 {
			curRow, curCol = 0, 0
		} else {
			if curRow < 0 {
				curRow = 0
			}
			if curRow >= t.Len() {
				curRow = t.Len() - 1
			}
			if curCol < 0 {
				curCol = 0
			}
			line := t.Get(curRow)
			if curCol > len(line) {
				curCol = len(line) // col can be 1 character outside of text
			}
		}
		// fix window according to cursor
		if curRow < tlRow {
			tlRow = curRow
		}
		if curRow >= tlRow+height {
			tlRow = curRow - height + 1
		}
		if curCol < tlCol {
			tlCol = curCol
		}
		if curCol >= tlCol+width {
			tlCol = curCol - width + 1
		}

		return curRow, curCol, tlRow, tlCol
	}

	t := e.text.Get()
	tlRow, tlCol := e.window.TlRow, e.window.TlCol
	width, height := e.window.Width, e.window.Height

	row, col, tlRow, tlCol = finalizeCursorAndWindow(row, col, tlRow, tlCol, width, height, t)

	// set
	e.cursor = editor.Cursor{
		Row: row,
		Col: col,
	}
	e.window = editor.Window{
		TlRow:  tlRow,
		TlCol:  tlCol,
		Width:  width,
		Height: height,
	}
}

func (e *Editor) moveRelativeAndFixWithoutLock(moveRow int, moveCol int) {
	e.gotoAndFixWithoutLock(e.cursor.Row+moveRow, e.cursor.Col+moveCol)
}

func (e *Editor) MoveLeft() {
	e.lockRender(func() {
		e.moveRelativeAndFixWithoutLock(0, -1)
		e.setMessageWithoutLock("move left")
	})
}
func (e *Editor) MoveRight() {
	e.lockRender(func() {
		e.moveRelativeAndFixWithoutLock(0, 1)
		e.setMessageWithoutLock("move right")
	})
}
func (e *Editor) MoveUp() {
	e.lockRender(func() {
		e.moveRelativeAndFixWithoutLock(-1, 0)
		e.setMessageWithoutLock("move up")
	})
}
func (e *Editor) MoveDown() {
	e.lockRender(func() {
		e.moveRelativeAndFixWithoutLock(1, 0)
		e.setMessageWithoutLock("move down")
	})
}
func (e *Editor) MoveHome() {
	e.lockRender(func() {
		e.moveRelativeAndFixWithoutLock(0, -e.cursor.Col)
		e.setMessageWithoutLock("move home")
	})
}
func (e *Editor) MoveEnd() {
	e.lockRender(func() {
		t := e.text.Get()
		if e.cursor.Row < t.Len() {
			line := t.Get(e.cursor.Row)
			e.moveRelativeAndFixWithoutLock(0, len(line)-e.cursor.Col)
		}
		e.setMessageWithoutLock("move end")
	})
}
func (e *Editor) MovePageUp() {
	e.lockRender(func() {
		e.moveRelativeAndFixWithoutLock(-e.window.Height, 0)
		e.setMessageWithoutLock("move page up")
	})
}
func (e *Editor) MovePageDown() {
	e.lockRender(func() {
		e.moveRelativeAndFixWithoutLock(e.window.Height, 0)
		e.setMessageWithoutLock("move page down")
	})
}

func (e *Editor) Goto(row int, col int) {
	e.lockRender(func() {
		e.gotoAndFixWithoutLock(row, col)
		e.setMessageWithoutLock("goto (%d, %d)", row+1, col+1)
	})
}
