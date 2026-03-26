package log_writer

import (
	"io"
	"os"
	"microscope/config"
	"microscope/core/editor"
)

func Read(filename string, apply func(e editor.LogEntry) bool) error {
	f, err := os.Open(filename)
	if err != nil {
		return err
	}
	defer f.Close()

	s, err := GetSerializer(config.Load().INITIAL_SERIALIZER_VERSION)
	if err != nil {
		return err
	}

	for {
		b, readErr := lengthPrefixRead(f)

		if readErr != nil {
			// if readErr is EOF - discard b
			return readErr
		}

		s1, ok, processErr := func(s Serializer, b []byte) (Serializer, bool, error) {
			e, err := s.Unmarshal(b)
			if err != nil {
				return nil, true, err
			}
			switch e.Command {
			case editor.CommandSetVersion:
				s1, err := GetSerializer(e.Version)
				return s1, true, err
			default:
				return s, apply(e), nil
			}
		}(s, b)

		if processErr != nil {
			return processErr
		}
		if !ok || readErr == io.EOF {
			return nil
		}

		s = s1
	}
}
