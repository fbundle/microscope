package ui

import (
	"fmt"
	"io"
	"os"
	"strings"
	"microscope/config"
	"microscope/core/editor"
	"microscope/core/log_writer"
)

func RunLog(logFilename string) error {
	s, err := log_writer.GetSerializer(config.Load().INITIAL_SERIALIZER_VERSION)
	if err != nil {
		return err
	}

	err = log_writer.Read(logFilename, func(e editor.LogEntry) bool {
		var b []byte
		b, err = s.Marshal(e)
		if err != nil {
			return false
		}
		_, err = fmt.Fprintln(os.Stdout, strings.TrimSpace(string(b)))
		if err != nil {
			return false
		}
		return true
	})
	if err == io.EOF {
		err = nil
	}
	return err
}
