package log_writer

import (
	"io"
	"sync"
	"microscope/config"
	"microscope/core/editor"
)

func New(writer io.Writer) (*Writer, error) {
	// use initial serializer
	version := config.Load().INITIAL_SERIALIZER_VERSION
	s, err := GetSerializer(version)
	if err != nil {
		return nil, err
	}
	w := &Writer{
		mu:      sync.Mutex{},
		writer:  writer,
		marshal: s.Marshal,
	}

	// write set_version using INITIAL_SERIALIZER_VERSION
	// tell reader to use SERIALIZER_VERSION
	err = w.Write(editor.LogEntry{
		Command: editor.CommandSetVersion,
		Version: config.Load().SERIALIZER_VERSION,
	})
	if err != nil {
		return nil, err
	}

	s1, err := GetSerializer(config.Load().SERIALIZER_VERSION)
	if err != nil {
		return nil, err
	}
	w.marshal = s1.Marshal
	return w, nil
}

type Writer struct {
	mu      sync.Mutex
	writer  io.Writer
	marshal func(editor.LogEntry) ([]byte, error)
}

func (w *Writer) Write(e editor.LogEntry) error {
	w.mu.Lock()
	defer w.mu.Unlock()

	b, err := w.marshal(e)
	if err != nil {
		return err
	}

	return lengthPrefixWrite(w.writer, b)
}
