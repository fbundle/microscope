package insert_editor

import (
	"microscope/core/editor"
)

func (e *Editor) renderWithoutLock() {
	e.renderCh <- e.makeView()
}

func (e *Editor) makeView() editor.View {
	render := func() editor.View {
		view := editor.View{
			Cursor: e.cursor,
			Window: e.window,
			Status: e.status,
		}

		view.Text = e.text.Get()
		return view

	}
	return render()
}

func (e *Editor) Render() (view editor.View) {
	e.lock(func() {
		view = e.makeView()
	})
	return view
}

func (e *Editor) Update() <-chan editor.View {
	return e.renderCh
}
