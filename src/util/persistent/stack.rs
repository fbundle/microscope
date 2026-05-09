use std::sync::Arc;

struct Node<T> {
    value: T,
    next: Option<Arc<Node<T>>>,
}

/// Immutable persistent stack.
#[derive(Clone)]
pub struct Stack<T: Clone> {
    node: Option<Arc<Node<T>>>,
}

impl<T: Clone> Stack<T> {
    pub fn empty() -> Self {
        Stack { node: None }
    }

    pub fn push(&self, v: T) -> Self {
        Stack {
            node: Some(Arc::new(Node { value: v, next: self.node.clone() })),
        }
    }

    pub fn peek(&self) -> Option<&T> {
        self.node.as_ref().map(|n| &n.value)
    }

    pub fn pop(&self) -> Self {
        match &self.node {
            None => Stack::empty(),
            Some(n) => Stack { node: n.next.clone() },
        }
    }

    pub fn depth(&self) -> usize {
        let mut count = 0;
        let mut cur = &self.node;
        while let Some(n) = cur {
            count += 1;
            cur = &n.next;
        }
        count
    }

    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        let mut out = Vec::new();
        let mut cur = &self.node;
        while let Some(n) = cur {
            out.push(n.value.clone());
            cur = &n.next;
        }
        out.into_iter()
    }
}
