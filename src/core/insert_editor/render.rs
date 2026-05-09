use crate::core::editor::View;
use crate::core::insert_editor::InsertEditor;

impl InsertEditor {
    pub fn get_view(&self) -> View {
        self.mu.lock().unwrap().make_view()
    }
}
