package text

import "microscope/util/buffer"

const delim byte = '\n'

// Line - if offset >= 0, this is a file else this is a []byte buffer
type Line struct {
	offset int64   // 8 bytes
	data   *[]byte // 8 bytes on 64-bit system
}

func MakeLineFromData(data []byte) Line {
	return Line{
		offset: -1,
		data:   &data,
	}
}

func MakeLineFromOffset(offset int) Line {
	return Line{
		offset: int64(offset),
		data:   nil,
	}
}

func (l Line) Offset() int64 {
	return l.offset
}

func (l Line) Repr(reader buffer.Reader) []byte {
	if l.offset < 0 {
		// in-memory
		return *l.data
	} else {
		// from file
		buf := make([]byte, 0)
		for i := int(l.offset); i < reader.Len(); i++ {
			b := reader.At(i)
			if b == delim {
				break
			}
			buf = append(buf, b)
		}
		return buf
	}
}
