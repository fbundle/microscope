package ui

import (
	"bufio"
	"context"
	"os"
	"microscope/core/editor"
	"microscope/core/insert_editor"
	"microscope/core/log_writer"

	"microscope/util/side_channel"

	"microscope/util/buffer"

	"golang.org/x/exp/mmap"
)

type finalizer struct {
	flush      func() error
	closerList []func() error
}

func (c *finalizer) Close() {
	for i := len(c.closerList) - 1; i >= 0; i-- {
		_ = c.closerList[i]()
	}
}

func (c *finalizer) Flush() error {
	if c.flush == nil {
		return nil
	}
	return c.flush()
}

func makeInsertEditor(
	ctx context.Context,
	inputFilename string, logFilename string,
	width int, height int,
) (insertEditor *insert_editor.Editor, loadCtx context.Context, f *finalizer, err error) {
	f = &finalizer{}

	insertEditor, err = insert_editor.New(height, width)
	if err != nil {
		f.Close()
		return nil, nil, nil, err
	}
	var inputBuffer buffer.Reader = nil
	if len(inputFilename) > 0 {
		inputMmapReader, err := mmap.Open(inputFilename)
		if err != nil {
			f.Close()
			return nil, nil, nil, err
		}
		f.closerList = append(f.closerList, inputMmapReader.Close)
		inputBuffer = inputMmapReader
	}
	loadCtx, err = insertEditor.Load(ctx, inputBuffer)
	if err != nil {
		f.Close()
		return nil, nil, nil, err
	}
	if len(logFilename) > 0 {
		logFile, err := os.OpenFile(logFilename, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0600)
		if err != nil {
			f.Close()
			return nil, nil, nil, err
		}
		f.closerList = append(f.closerList, logFile.Close)
		writer := bufio.NewWriter(logFile)
		f.closerList = append(f.closerList, writer.Flush)
		logWriter, err := log_writer.New(writer)
		if err != nil {
			f.Close()
			return nil, nil, nil, err
		}
		f.flush = writer.Flush

		insertEditor.Subscribe(func(entry editor.LogEntry) {
			err := logWriter.Write(entry)
			if err != nil {
				side_channel.Panic(err)
			}
		})
	}
	return insertEditor, loadCtx, f, nil
}

func writeMessage(e editor.Editor, message string) {
	e.Status(func(status editor.Status) editor.Status {
		status.Message = message
		return status
	})
}
