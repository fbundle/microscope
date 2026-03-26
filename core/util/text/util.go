package text

import (
	"microscope/util/persistent/seq"
)

func MakeTextFromLine(lines [][]rune) Text {
	s := seq.Empty[Line]()
	for _, line := range lines {
		s = s.Ins(s.Len(), MakeLineFromData(runesToBytes(line)))
	}
	return Text{
		reader: nil,
		lines:  s,
	}
}

func runesToBytes(rs []rune) (bs []byte) {
	return []byte(string(rs))
}

func bytesToRunes(bs []byte) (rs []rune) {
	return []rune(string(bs))
}
