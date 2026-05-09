use crate::config;

pub struct Hist<T: Clone> {
    latest: usize,
    stack: Vec<T>,
}

impl<T: Clone> Hist<T> {
    pub fn new(t: T) -> Self {
        Hist { latest: 0, stack: vec![t] }
    }

    pub fn update(&mut self, modifier: impl FnOnce(T) -> T) {
        let next = modifier(self.stack[self.latest].clone());
        self.stack.truncate(self.latest + 1);
        self.stack.push(next);
        self.latest += 1;
        let max = config::load().maxsize_history_stack;
        if self.stack.len() > max {
            self.stack.remove(0);
            self.latest -= 1;
        }
    }

    pub fn get(&self) -> T {
        self.stack[self.latest].clone()
    }

    pub fn undo(&mut self) {
        if self.latest > 0 {
            self.latest -= 1;
        }
    }

    pub fn redo(&mut self) {
        if self.latest < self.stack.len() - 1 {
            self.latest += 1;
        }
    }
}
