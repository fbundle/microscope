package text

import (
	"microscope/util/side_channel"

	"microscope/util/buffer"
	"microscope/util/persistent/seq"
)

func New(reader buffer.Reader) Text {
	return Text{
		reader: reader,
		lines:  seq.Empty[Line](),
	}
}

type Text struct {
	reader buffer.Reader
	lines  seq.Seq[Line]
}

func (t Text) Get(i int) []rune {
	return bytesToRunes(t.lines.Get(i).Repr(t.reader))
}

func (t Text) Set(i int, val []rune) Text {
	return Text{
		reader: t.reader,
		lines:  t.lines.Set(i, MakeLineFromData(runesToBytes(val))),
	}
}

func (t Text) Ins(i int, val []rune) Text {
	return Text{
		reader: t.reader,
		lines:  t.lines.Ins(i, MakeLineFromData(runesToBytes(val))),
	}
}

func (t Text) Append(line Line) Text {
	return Text{
		reader: t.reader,
		lines:  t.lines.Ins(t.lines.Len(), line),
	}
}

func (t Text) Del(i int) Text {
	return Text{
		reader: t.reader,
		lines:  t.lines.Del(i),
	}
}

func (t Text) Iter(f func(i int, val []rune) bool) {
	t.lines.Iter(func(i int, l Line) bool {
		return f(i, bytesToRunes(l.Repr(t.reader)))
	})
}

func (t Text) Len() int {
	return t.lines.Len()
}

func (t Text) Repr() [][]rune {
	text := make([][]rune, 0, t.lines.Len())
	for _, l := range t.lines.Iter {
		text = append(text, bytesToRunes(l.Repr(t.reader)))
	}
	return text
}

func Slice(t Text, beg int, end int) Text {
	return Text{
		reader: t.reader,
		lines:  t.lines.Slice(beg, end),
	}
}

func Merge(ts ...Text) Text {
	if len(ts) == 0 {
		side_channel.Panic("cannot merge empty text")
		return Text{}
	}
	t := ts[0]
	for i := 1; i < len(ts); i++ {
		t1 := ts[i]
		reader := t.reader
		if reader == nil {
			reader = t1.reader
		} else {
			if t1.reader != nil && t1.reader != reader {
				side_channel.Panic("cannot merge text with different reader")
				return Text{}
			}
		}
		t = Text{
			reader: reader,
			lines:  seq.Merge(t.lines, t1.lines),
		}
	}
	return t
}
