package ui

import (
	"context"
	"fmt"
	"io"
	"os"
	"microscope/core/editor"
	"microscope/core/log_writer"
)

func RunReplay(inputFilename string, logFilename string) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	_, _ = fmt.Fprintf(os.Stderr, "loading input file %s\n", inputFilename)

	// make insert_editor without log_writer
	insertEditor, loadCtx, finalizer, err := makeInsertEditor(ctx, inputFilename, "", 20, 20)
	if err != nil {
		return err
	}
	defer finalizer.Close()

	go func() {
		for {
			select {
			case <-ctx.Done():
				return
			case view := <-insertEditor.Update(): // consume view
				if len(view.Status.Background) > 0 {
					_, _ = fmt.Fprintln(os.Stderr, view.Status.Background)
				}
			}
		}
	}()

	// wait for loading
	<-loadCtx.Done()

	_, _ = fmt.Fprintf(os.Stderr, "loading log_writer file %s\n", logFilename)

	err = log_writer.Read(logFilename, func(entry editor.LogEntry) bool {
		insertEditor.Apply(entry)
		return true
	})
	if err != nil && err != io.EOF {
		return err
	}
	_, _ = fmt.Fprintf(os.Stderr, "replaying file\n")
	t := insertEditor.Render().Text
	for _, line := range t.Iter {
		_, _ = fmt.Fprintln(os.Stdout, string(line))
	}
	return nil
}
