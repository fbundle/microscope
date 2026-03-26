package log_writer

import (
	"encoding/json"
	"errors"
	"microscope/config"
	"microscope/core/editor"

	"microscope/util/side_channel"
)

type Serializer interface {
	Marshal(editor.LogEntry) ([]byte, error)
	Unmarshal([]byte) (editor.LogEntry, error)
	Version() uint64
}

func GetSerializer(version uint64) (Serializer, error) {
	switch version {
	case config.HUMAN_READABLE_SERIALIZER:
		return humanReadableSerializer{}, nil
	case config.BINARY_SERIALIZER:
		side_channel.Panic("binary serializer is not fully implemented yet")
		return binarySerializer{}, nil
	default:
		return nil, errors.New("serializer not found")
	}
}

type humanReadableSerializer struct{}

func (humanReadableSerializer) Marshal(e editor.LogEntry) ([]byte, error) {
	b, err := json.Marshal(e)
	// padding for human readability
	b1 := []byte{' '}
	b1 = append(b1, b...)
	b1 = append(b1, '\n')

	return b1, err
}

func (humanReadableSerializer) Unmarshal(b []byte) (e editor.LogEntry, err error) {
	err = json.Unmarshal(b, &e)
	return e, err
}

func (humanReadableSerializer) Version() uint64 {
	return config.HUMAN_READABLE_SERIALIZER
}

type binarySerializer struct{}

var commandList = []editor.Command{
	editor.CommandSetVersion,
	editor.CommandType,
	editor.CommandEnter,
	editor.CommandBackspace,
	editor.CommandDelete,
	editor.CommandUndo,
	editor.CommandRedo,
}
var commandToByteMap map[editor.Command]byte = nil

func commandToByte(c editor.Command) byte {
	if commandToByteMap == nil {
		commandToByteMap = make(map[editor.Command]byte)
		for i, cmd := range commandList {
			commandToByteMap[cmd] = byte(i)
		}
	}
	return commandToByteMap[c]
}

func byteToCommand(b byte) editor.Command {
	return commandList[b]
}

func consume(buffer []byte, n int) ([]byte, []byte) {
	if len(buffer) < n {
		return buffer, make([]byte, n)
	}
	return buffer[n:], buffer[:n]
}

func (binarySerializer) Marshal(e editor.LogEntry) ([]byte, error) {
	var buffer []byte
	buffer = append(buffer, commandToByte(e.Command))
	switch e.Command {
	case editor.CommandSetVersion:
		buffer = append(buffer, uint64ToBytes(e.Version)...)
		return buffer, nil
	case editor.CommandType:
		buffer = append(buffer, uint64ToBytes(e.Row)...)
		buffer = append(buffer, uint64ToBytes(e.Col)...)
		buffer = append(buffer, runeToBytes(e.Rune)...)
		return buffer, nil
	case editor.CommandEnter, editor.CommandBackspace, editor.CommandDelete:
		buffer = append(buffer, uint64ToBytes(e.Row)...)
		buffer = append(buffer, uint64ToBytes(e.Col)...)
		return buffer, nil
	case editor.CommandUndo, editor.CommandRedo:
		return buffer, nil
	default:
		return nil, errors.New("command not found")
	}
}

func (binarySerializer) Unmarshal(buffer []byte) (e editor.LogEntry, err error) {
	buffer, b := consume(buffer, 1)
	e.Command = byteToCommand(b[0])
	switch e.Command {
	case editor.CommandSetVersion:
		buffer, b = consume(buffer, 8)
		e.Version = bytesToUint64(b)
		return e, nil
	case editor.CommandType:
		buffer, b = consume(buffer, 8)
		e.Row = bytesToUint64(b)
		buffer, b = consume(buffer, 8)
		e.Col = bytesToUint64(b)
		buffer, b = consume(buffer, 4)
		e.Rune = bytesToRune(b)
		return e, nil
	case editor.CommandEnter, editor.CommandBackspace, editor.CommandDelete:
		buffer, b = consume(buffer, 8)
		e.Row = bytesToUint64(b)
		buffer, b = consume(buffer, 8)
		e.Col = bytesToUint64(b)
		return e, nil
	case editor.CommandUndo, editor.CommandRedo:
		return e, nil
	}
	return e, errors.New("parse error")
}

func (binarySerializer) Version() uint64 {
	return config.BINARY_SERIALIZER
}
