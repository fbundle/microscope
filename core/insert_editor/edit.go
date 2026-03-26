package insert_editor

import (
	"slices"
	"microscope/core/editor"
	"microscope/core/util/text"

	"microscope/util/side_channel"
)

func (e *Editor) Type(ch rune) {
	e.lockRender(func() {
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandType,
			Rune:    ch,
			Row:     uint64(e.cursor.Row),
			Col:     uint64(e.cursor.Col),
		})

		updateText := func(t text.Text) text.Text {
			row, col := e.cursor.Row, e.cursor.Col
			// NOTE - handle empty file
			if t.Len() == 0 {
				t = t.Ins(0, []rune{ch})
				return t
			}
			// t.Get(row) always well-defined
			line := slices.Clone(t.Get(row))
			line = insertToSlice(line, col, ch)
			t = t.Set(row, line)
			return t
		}

		e.text.Update(updateText)
		e.moveRelativeAndFixWithoutLock(0, 1) // move right
		e.setMessageWithoutLock("type '%c'", ch)
	})
}

func (e *Editor) Backspace() {
	e.lockRender(func() {
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandBackspace,
			Row:     uint64(e.cursor.Row),
			Col:     uint64(e.cursor.Col),
		})

		moveRow, moveCol := 0, 0
		updateText := func(t text.Text) text.Text {
			row, col := e.cursor.Row, e.cursor.Col
			// NOTE - handle empty file
			if t.Len() == 0 {
				return t
			}
			switch {
			case col == 0 && row == 0:
			// first line do nothing
			case col == 0 && row != 0:
				// merge 2 lines
				line1 := t.Get(row - 1)
				line2 := t.Get(row)

				t = t.Set(row-1, concatSlices(line1, line2)).Del(row)
				moveRow, moveCol = -1, len(line1) // move up and to the end of last line
			case col != 0:
				// t.Get(row) always well-defined
				line := slices.Clone(t.Get(row))
				line = deleteFromSlice(line, col-1)
				t = t.Set(row, line)
				moveRow, moveCol = 0, -1 // move left
			default:
				side_channel.Panic("unreachable")
			}
			return t
		}
		e.text.Update(updateText)
		e.moveRelativeAndFixWithoutLock(moveRow, moveCol)
		e.setMessageWithoutLock("backspace")
	})
}

func (e *Editor) Delete() {
	e.lockRender(func() {
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandDelete,
			Row:     uint64(e.cursor.Row),
			Col:     uint64(e.cursor.Col),
		})

		updateText := func(t text.Text) text.Text {
			row, col := e.cursor.Row, e.cursor.Col
			// NOTE - handle empty file
			if t.Len() == 0 {
				return t
			}
			// t.Get(row) always well-defined
			line1 := t.Get(row)
			switch {
			case col == len(line1) && row == t.Len()-1:
			// last line, do nothing
			case col == len(line1) && row < t.Len()-1:
				// merge 2 lines
				line2 := t.Get(row + 1)
				t = t.Set(row, concatSlices(line1, line2)).Del(row + 1)
			case col != len(line1):
				line := slices.Clone(line1)
				line = deleteFromSlice(line, col)
				t = t.Set(row, line)
			default:
				side_channel.Panic("unreachable")
			}
			return t
		}
		e.text.Update(updateText)
		e.setMessageWithoutLock("delete")
	})
}

func (e *Editor) Enter() {
	e.lockRender(func() {
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandEnter,
			Row:     uint64(e.cursor.Row),
			Col:     uint64(e.cursor.Col),
		})

		updateText := func(t text.Text) text.Text {
			row, col := e.cursor.Row, e.cursor.Col
			// NOTE - handle empty file
			if t.Len() == 0 {
				t = t.Ins(0, nil)
				return t
			}
			// t.Get(row) always well-defined
			line := t.Get(row)
			switch {
			case col == len(line):
				// add new line
				t = t.Ins(row+1, nil)
				return t
			case col < len(line):
				// split a line
				line1 := slices.Clone(line[:col])
				line2 := slices.Clone(line[col:])
				t = t.Set(row, line1)
				t = t.Ins(row+1, line2)
				return t
			default:
				side_channel.Panic("unreachable")
				return t
			}
		}
		e.text.Update(updateText)
		e.moveRelativeAndFixWithoutLock(1, 0)             // move down
		e.moveRelativeAndFixWithoutLock(0, -e.cursor.Col) // move home
		e.setMessageWithoutLock("enter")
	})
}

func (e *Editor) Undo() {
	e.lockRender(func() {
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandUndo,
		})
		e.text.Undo()
		e.moveRelativeAndFixWithoutLock(0, 0)
		e.setMessageWithoutLock("undo")
	})
}

func (e *Editor) Redo() {
	e.lockRender(func() {
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandRedo,
		})
		e.text.Redo()
		e.moveRelativeAndFixWithoutLock(0, 0)
		e.setMessageWithoutLock("redo")
	})
}

func (e *Editor) Apply(entry editor.LogEntry) {
	switch entry.Command {
	case editor.CommandEnter:
		e.Goto(int(entry.Row), int(entry.Col))
		e.Enter()
	case editor.CommandBackspace:
		e.Goto(int(entry.Row), int(entry.Col))
		e.Backspace()
	case editor.CommandDelete:
		e.Goto(int(entry.Row), int(entry.Col))
		e.Delete()
	case editor.CommandType:
		e.Goto(int(entry.Row), int(entry.Col))
		e.Type(entry.Rune)
	case editor.CommandUndo:
		e.Undo()
	case editor.CommandRedo:
		e.Redo()
	case editor.CommandInsertLine:
		e.Goto(int(entry.Row), 0)
		e.InsertLine(text.MakeTextFromLine(entry.Text))
	case editor.CommandDeleteLine:
		e.Goto(int(entry.Row), 0)
		e.DeleteLine(int(entry.Count))
	default:
		side_channel.Panic("command not found")
	}
}

func (e *Editor) InsertLine(t2 text.Text) {
	e.lockRender(func() {
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandInsertLine,
			Row:     uint64(e.cursor.Row),
			Text:    t2.Repr(),
		})
		row := e.cursor.Row
		update := func(t text.Text) text.Text {
			return text.Merge(
				text.Slice(t, 0, row),
				t2,
				text.Slice(t, row, t.Len()),
			)
		}
		e.text.Update(update)
		e.moveRelativeAndFixWithoutLock(t2.Len(), 0)
		e.setMessageWithoutLock("insert lines")
	})
}

func (e *Editor) DeleteLine(count int) {
	e.lockRender(func() {
		row := e.cursor.Row
		e.writeLogWithoutLock(editor.LogEntry{
			Command: editor.CommandDeleteLine,
			Row:     uint64(row),
			Count:   uint64(count),
		})
		update := func(t text.Text) text.Text {
			return text.Merge(
				text.Slice(t, 0, row),
				text.Slice(t, row+count, t.Len()),
			)
		}
		e.text.Update(update)
		e.setMessageWithoutLock("delete lines")
	})
}
