package ui

import (
	"context"
	"errors"
	"fmt"
	"os"
	"runtime/debug"
	"microscope/config"
	"microscope/core/editor"
	"microscope/core/multimode_editor"
	"time"

	"microscope/util/side_channel"

	"github.com/gdamore/tcell/v2"
)

var interruptError = errors.New("interrupted")

func getModeAndCommand(m map[string]any) (string, string) {
	if m == nil {
		return "", ""
	}
	mode := ""
	s, ok := m["mode"]
	if ok {
		mode = fmt.Sprintf("%v", s)
	}
	command := ""
	s, ok = m["command"]
	if ok {
		command = fmt.Sprintf("%v", s)
	}
	return mode, command
}

func getSelector(m map[string]any) *multimode_editor.Selector {
	if m == nil {
		return nil
	}
	s, ok := m["selector"]
	if !ok {
		return nil
	}
	selector, ok := s.(*multimode_editor.Selector)
	if !ok {
		return nil
	}
	if selector == nil {
		return nil
	}
	return selector
}

func getStatusStyle(mode string) tcell.Style {
	switch mode {
	case multimode_editor.ModeNormal:
		return tcell.StyleDefault.Background(tcell.ColorLightGray).Foreground(tcell.ColorBlack)
	case multimode_editor.ModeInsert:
		return tcell.StyleDefault.Background(tcell.ColorLightYellow).Foreground(tcell.ColorBlack)
	case multimode_editor.ModeSelect:
		return tcell.StyleDefault.Background(tcell.ColorLightGreen).Foreground(tcell.ColorBlack)
	case multimode_editor.ModeCommand:
		return tcell.StyleDefault.Background(tcell.ColorLightBlue).Foreground(tcell.ColorBlack)
	default:
		return tcell.StyleDefault.Background(tcell.ColorLightGray).Foreground(tcell.ColorBlack)
	}
}

func getTextStyle(textRow int, selector *multimode_editor.Selector) tcell.Style {
	if selector != nil {
		beg, end := selector.Interval()
		if beg <= textRow && textRow <= end {
			return tcell.StyleDefault.Background(tcell.ColorLightGray).Foreground(tcell.ColorBlack)
		}
	}
	return tcell.StyleDefault
}

func draw(s tcell.Screen, view editor.View) {
	s.Clear()
	screenWidth, screenHeight := s.Size()
	selector := getSelector(view.Status.Other)

	// Draw cursor from (0, 0)
	col := view.Cursor.Col - view.Window.TlCol
	row := view.Cursor.Row - view.Window.TlRow
	s.ShowCursor(col, row)

	// Draw content from (0, 0) -> (screenWidth-1, screenHeight-2)
	contentDrawContext := makeDrawContext(s, 0, 0, screenWidth, screenHeight-1)
	contentDrawContext(func(width int, height int, draw drawFunc) {
		t := view.Text
		for relRow := 0; relRow < height; relRow++ {
			row := view.Window.TlRow + relRow
			style := getTextStyle(row, selector)
			var line []rune = nil
			if row < t.Len() {
				line = t.Get(row)
			}

			for relCol := 0; relCol < width; relCol++ {
				col := view.Window.TlCol + relCol
				ch := ' '
				if col < len(line) {
					ch = line[col]
				}
				if row >= t.Len() && relCol == 0 {
					ch = '~' // special case
				}
				draw(relCol, relRow, ch, nil, style)
			}
		}
	})

	// Draw the status bar at the bottom (screenHeight-1)
	statusDrawContext := makeDrawContext(s, 0, screenHeight-1, screenWidth, 1)
	statusDrawContext(func(width int, height int, draw drawFunc) {
		sep := []rune(" > ")
		mode, command := getModeAndCommand(view.Status.Other)
		style := getStatusStyle(mode)

		// draw bottom bar
		for col := 0; col < width; col++ {
			draw(col, 0, ' ', nil, style)
		}
		// draw background
		var fromRight []rune = nil
		if len(view.Status.Background) > 0 {
			fromRight = append(fromRight, sep...)
			fromRight = append(fromRight, []rune(view.Status.Background)...)
			fromRight = append(fromRight, ' ')
		}
		for i, ch := range fromRight {
			col := i + width - len(fromRight)
			draw(col, 0, ch, nil, style)
		}
		// draw mode, cursor, command, messge
		var fromLeft []rune
		fromLeft = append(fromLeft, []rune(fmt.Sprintf(" %s (%d, %d)", mode, view.Cursor.Row+1, view.Cursor.Col+1))...)
		if len(command) > 0 {
			fromLeft = append(fromLeft, sep...)
			fromLeft = append(fromLeft, []rune(command)...)
		}
		if len(view.Status.Message) > 0 {
			fromLeft = append(fromLeft, sep...)
			fromLeft = append(fromLeft, []rune(view.Status.Message)...)
		}
		for col, ch := range fromLeft {
			draw(col, 0, ch, nil, style)
		}

	})

	s.Show()
}

type quitEvent struct {
	when time.Time
}

func (e quitEvent) When() time.Time {
	return e.when
}

func sendQuitEvent(s tcell.Screen) {
	now := time.Now()
	for i := 0; i < 5; i++ {
		if err := s.PostEvent(quitEvent{when: now}); err != nil {
			if errors.Is(err, tcell.ErrEventQFull) {
				time.Sleep(100 * time.Millisecond) // retry stopping
				continue
			}
			side_channel.Panic(err)
			return
		} else {
			return
		}
	}
}

func RunEditor(inputFilename string, logFilename string, multiMode bool) error {
	defer func() {
		if r := recover(); r != nil {
			side_channel.WriteLn(string(debug.Stack()))
			side_channel.Panic(r)
		}
	}()

	s, err := tcell.NewScreen()
	if err != nil {
		return err
	}
	if err = s.Init(); err != nil {
		return err
	}
	defer s.Fini()

	s.EnableMouse()

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	width, height := s.Size()

	var e editor.Editor
	// make editor
	insertEditor, loadCtx, finalizer, err := makeInsertEditor(ctx, inputFilename, logFilename, width, height-1)
	if err != nil {
		return err
	}
	defer finalizer.Close()
	flush := func() {
		if err := finalizer.Flush(); err != nil {
			writeMessage(e, fmt.Sprintf("flush error: %v", err))
		} else {
			writeMessage(e, "log_writer flushed")
		}
	}

	if multiMode {
		stop := func() {
			cancel()
			sendQuitEvent(s)
		}
		e = multimode_editor.New(insertEditor, stop, inputFilename)
	} else {
		e = insertEditor
	}

	// draw loop
	go func() {
		for {
			select {
			case <-ctx.Done():
				return
			case view := <-e.Update():
				draw(s, view)
			}
		}
	}()
	// manual flush loop in the event of crash
	go func() {
		ticker := time.NewTicker(config.Load().LOG_AUTOFLUSH_INTERVAL)
		defer ticker.Stop()
		for {
			select {
			case <-ctx.Done():
				return
			case <-ticker.C:
				flush()
			}
		}
	}()

	// event loop
	for {
		event := s.PollEvent()
		switch event := event.(type) {
		case quitEvent, *quitEvent:
			// quit from insert_editor - delete log
			writeMessage(e, "exiting... ")
			_ = os.Remove(logFilename)
			<-loadCtx.Done()
			return nil
		case *tcell.EventMouse:
			handleEditorMouse(e, event)
		case *tcell.EventKey:
			if event.Key() == tcell.KeyCtrlC {
				return interruptError
			} else if event.Key() == tcell.KeyCtrlS {
				flush()
			} else {
				handleEditorKey(e, event)
			}

		case *tcell.EventResize:
			s.Sync()
			width, height = s.Size()
			e.Resize(height-1, width)
		default:
			// nothing
		}
	}
}

func handleEditorMouse(e editor.Editor, ev *tcell.EventMouse) {
	col, row := ev.Position()
	button := ev.Buttons()
	switch {
	case button&tcell.Button1 != 0:
		e.Action("mouse_click_left", editor.Cursor{
			Row: row,
			Col: col,
		})
	case button&tcell.Button2 != 0:
		e.Action("mouse_click_right", editor.Cursor{
			Row: row,
			Col: col,
		})
	case button&tcell.Button3 != 0:
		e.Action("mouse_click_right", editor.Cursor{
			Row: row,
			Col: col,
		})
	case button&tcell.Button4 != 0:
		e.Action("mouse_scroll_left")

	case button&tcell.Button5 != 0:
		e.Action("mouse_scroll_right")
	case button&tcell.WheelUp != 0:
		e.Action("mouse_scroll_up")

	case button&tcell.WheelDown != 0:
		e.Action("mouse_scroll_down")
	case button == tcell.ButtonNone:
		//e.Action("mouse_none", editor.Position{
		//		Row: row,
		//		Col: col,
		//	})
	}
}

func handleEditorKey(e editor.Editor, ev *tcell.EventKey) {
	switch ev.Key() {
	case tcell.KeyRune:
		e.Type(ev.Rune())
	case tcell.KeyEnter:
		e.Enter()
	case tcell.KeyBackspace, tcell.KeyBackspace2:
		e.Backspace()
	case tcell.KeyDelete:
		e.Delete()

	case tcell.KeyEsc:
		e.Action("key_escape")
	case tcell.KeyTab:
		e.Action("key_tabular")

	case tcell.KeyRight:
		e.MoveRight()
	case tcell.KeyLeft:
		e.MoveLeft()
	case tcell.KeyUp:
		e.MoveUp()
	case tcell.KeyDown:
		e.MoveDown()
	case tcell.KeyHome:
		e.MoveHome()
	case tcell.KeyEnd:
		e.MoveEnd()
	case tcell.KeyPgUp:
		e.MovePageUp()
	case tcell.KeyPgDn:
		e.MovePageDown()
	case tcell.KeyCtrlU:
		e.Undo()
	case tcell.KeyCtrlR:
		e.Redo()
	default:
		writeMessage(e, fmt.Sprintf("unknown key %v", ev.Name()))
	}
}
