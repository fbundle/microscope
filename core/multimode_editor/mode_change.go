package multimode_editor

import (
	"fmt"
	"regexp"
	"strconv"
	"strings"
	"microscope/config"
	"microscope/core/util/text"
	"microscope/util/file_util"
	"time"

	"microscope/util/side_channel"
)

// all functions resulting in mode change

func (c *Editor) keyEscapeWithoutLock() {
	c.enterNormalModeWithoutLock()
	c.writeWithoutLock("")
}

func (c *Editor) Enter() {
	c.lock(func() {
		switch c.state.mode {
		case ModeNormal:
			// do nothing
		case ModeInsert:
			c.e.Enter()
		case ModeCommand:
			c.applyCommandWithoutLock()
		case ModeSelect:
			// do nothing
		default:
			side_channel.Panic("unknown mode: ", c.state)
		}
	})
}

func (c *Editor) maybeUpdateSelectorEndWithoutLock() {
	if c.state.mode == ModeSelect {
		row := c.e.Render().Cursor.Row
		c.state.selector.end = row
		c.writeWithoutLock("select more")
	}
}

func (c *Editor) Type(ch rune) {
	c.lock(func() {
		switch c.state.mode {
		case ModeNormal:
			switch ch {
			case 'i':
				c.enterInsertModeWithoutLock()
				c.writeWithoutLock("")
			case ':', '/':
				c.enterCommandModeWithoutLock(string(ch))
				c.writeWithoutLock("")
			case 'V': // start selecting
				row := c.e.Render().Cursor.Row
				c.enterSelectModeWithoutLock(row)
				c.writeWithoutLock("")

			case 'p': // paste
				if c.state.clipboard.Len() == 0 {
					c.writeWithoutLock("clipboard is empty")
					return
				}
				c.e.InsertLine(c.state.clipboard)
				c.writeWithoutLock("pasted")
			case 'u':
				c.e.Undo()
			case 'r':
				c.e.Redo()

			case 'b', 'g': // go to beg of file
				c.e.Goto(0, 0)

			case 'e', 'G': // go to end of file
				row := c.e.Render().Text.Len() - 1
				c.e.Goto(row, 0)
			default:
			}
		case ModeInsert:
			c.e.Type(ch)
		case ModeCommand:
			c.state.command += string(ch)
			c.writeWithoutLock("")
		case ModeSelect:
			switch ch {
			case 'd': // cut
				beg, end := c.state.selector.Interval()
				t := c.e.Render().Text
				c.state.clipboard = text.Slice(t, beg, end+1)
				// delete
				c.e.Goto(beg, 0)
				c.e.DeleteLine(c.state.clipboard.Len())

				c.enterNormalModeWithoutLock()
				c.writeWithoutLock("cut")

			case 'y': //copy
				beg, end := c.state.selector.Interval()
				t := c.e.Render().Text
				c.state.clipboard = text.Slice(t, beg, end+1)
				c.enterNormalModeWithoutLock()
				c.writeWithoutLock("copied")

			case 'b', 'g': // go to beg of file
				c.e.Goto(0, 0)
				c.maybeUpdateSelectorEndWithoutLock()
			case 'e', 'G': // go to end of file
				row := c.e.Render().Text.Len() - 1
				c.e.Goto(row, 0)
				c.maybeUpdateSelectorEndWithoutLock()
			}
		default:
			side_channel.Panic("unknown mode: ", c.state)
		}
	})
}

type command string

const (
	commandInsert    command = "i"
	commandQuit      command = "q"
	commandWriteQuit command = "wq"
	commandSearch    command = "s"
	commandRegex     command = "r"
	commandGoto      command = "g"
	commandWrite     command = "w"
	commandUnknown   command = "u"
)

func parseCommand(cmd string) (command, []string) {
	if cmd == ":i" || cmd == ":insert" {
		return commandInsert, nil
	}
	if cmd == ":q" || cmd == ":quit" || cmd == ":q!" {
		return commandQuit, nil
	}
	if cmd == ":w" || cmd == ":write" || cmd == ":wq" {
		return commandWriteQuit, nil
	}

	for _, prefix := range []string{"/", ":s ", ":search "} {
		if strings.HasPrefix(cmd, prefix) {
			cmd = strings.TrimPrefix(cmd, prefix)
			return commandSearch, strings.Fields(cmd)
		}
	}

	for _, prefix := range []string{":regex "} {
		if strings.HasPrefix(cmd, prefix) {
			cmd = strings.TrimPrefix(cmd, prefix)
			return commandRegex, strings.Fields(cmd)
		}
	}

	for _, prefix := range []string{":g ", ":goto "} {
		if strings.HasPrefix(cmd, prefix) {
			cmd = strings.TrimPrefix(cmd, prefix)
			return commandGoto, strings.Fields(cmd)
		}
	}
	for _, prefix := range []string{":w ", ":write "} {
		if strings.HasPrefix(cmd, prefix) {
			cmd = strings.TrimPrefix(cmd, prefix)
			return commandWrite, strings.Fields(cmd)
		}
	}

	// default to goto
	for _, prefix := range []string{":"} {
		if strings.HasPrefix(cmd, prefix) {
			cmd = strings.TrimPrefix(cmd, prefix)
			return commandGoto, strings.Fields(cmd)
		}
	}
	return commandUnknown, nil
}

func (c *Editor) applyCommandWithoutLock() {
	cmd, args := parseCommand(c.state.command)
	switch cmd {
	case commandInsert:
		c.enterInsertModeWithoutLock()
		c.writeWithoutLock("")
		return
	case commandQuit:
		c.stop()
		return
	case commandWriteQuit:
		filename := c.defaultOutputFile

		// write file
		err := file_util.SafeWriteFile(filename, c.e.Render().Text.Iter)
		if err != nil {
			c.enterNormalModeWithoutLock()
			c.writeWithoutLock("error write file " + err.Error())
			return
		}
		c.enterNormalModeWithoutLock()
		c.writeWithoutLock("file written into " + filename)

		// exit
		c.stop()
		return
	case commandSearch, commandRegex:
		if len(args) == 0 {
			c.enterNormalModeWithoutLock()
			c.writeWithoutLock("empty args")
			return
		}
		pattern := args[0]
		var match func(line string) bool
		if cmd == commandRegex {
			re, err := regexp.Compile(pattern)
			if err != nil {
				c.enterNormalModeWithoutLock()
				c.writeWithoutLock(fmt.Sprintf("regexp compile error %s", err.Error()))
				return
			}
			match = re.MatchString
		} else {
			match = func(line string) bool {
				return strings.Contains(line, pattern)
			}
		}

		view := c.e.Render()
		row, t := view.Cursor.Row, view.Text
		t = text.Slice(t, row+1, t.Len())

		t0 := time.Now()
		for i, line := range t.Iter {
			if match(string(line)) {
				c.e.Goto(row+1+i, 0)
				c.writeWithoutLock("found substring " + pattern)
				return
			}
			t1 := time.Now()
			if t1.Sub(t0) > config.Load().MAX_SEACH_TIME {
				c.e.Goto(row+1+i, 0)
				c.writeWithoutLock(fmt.Sprintf("search timeout after %d seconds and %d entries", config.Load().MAX_SEACH_TIME/time.Second, i+1))
				return
			}
		}
		c.e.Goto(t.Len()-1, 0)
		c.enterNormalModeWithoutLock()
		c.writeWithoutLock("end of file")
		return
	case commandGoto:
		if len(args) == 0 {
			c.enterNormalModeWithoutLock()
			c.writeWithoutLock("empty args")
			return
		}
		lineStr := args[0]
		lineNum, err := strconv.Atoi(lineStr)
		if err != nil {
			c.enterNormalModeWithoutLock()
			c.writeWithoutLock("invalid line number " + lineStr)
			return
		}
		c.e.Goto(lineNum-1, 0)
		c.enterNormalModeWithoutLock()
		c.writeWithoutLock("goto line " + lineStr)
		return
	case commandWrite:
		if len(args) == 0 {
			c.enterNormalModeWithoutLock()
			c.writeWithoutLock("empty args")
			return
		}
		filename := args[0]
		// write file
		err := file_util.SafeWriteFile(filename, c.e.Render().Text.Iter)
		if err != nil {
			c.enterNormalModeWithoutLock()
			c.writeWithoutLock("error write file " + err.Error())
			return
		}
		c.enterNormalModeWithoutLock()
		c.writeWithoutLock("file written into " + filename)

	default:
		c.enterNormalModeWithoutLock()
		c.writeWithoutLock("unknown command: " + c.state.command)
	}
}
func (c *Editor) Delete() {
	c.lock(func() {
		switch c.state.mode {
		case ModeNormal:
			c.enterInsertModeWithoutLock()
			c.writeWithoutLock("")
			c.e.Delete()
		case ModeInsert:
			c.e.Delete()
		case ModeCommand:
		// do nothing
		case ModeSelect:
			// do nothing
		default:
			side_channel.Panic("unknown mode: ", c.state)
		}
	})
}

func (c *Editor) Backspace() {
	c.lock(func() {
		switch c.state.mode {
		case ModeNormal:
			// do nothing
		case ModeInsert:
			c.e.Backspace()
		case ModeCommand:
			if len(c.state.command) > 0 {
				c.state.command = c.state.command[:len(c.state.command)-1]
				if len(c.state.command) == 0 {
					c.enterNormalModeWithoutLock()
					c.writeWithoutLock("")
				}
			}
			c.writeWithoutLock("")
		case ModeSelect:
			// do nothing
		default:
			side_channel.Panic("unknown mode: ", c.state)
		}
	})
}
