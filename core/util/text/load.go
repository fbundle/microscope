package text

import (
	"iter"
	"microscope/util/buffer"
)

func IndexFile(reader buffer.Reader) iter.Seq[int] {
	return func(yield func(offset int) bool) {
		offset := 0
		for i := 0; i < reader.Len(); i++ {
			b := reader.At(i)
			if b == delim {
				if !yield(offset) {
					return
				}
				offset = i + 1
			}
		}
		if offset < reader.Len() {
			yield(offset)
		}
	}
}
