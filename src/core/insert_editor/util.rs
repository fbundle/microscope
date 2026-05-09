pub fn insert_to_slice<T: Clone>(l: &mut Vec<T>, i: usize, v: T) {
    if i > l.len() {
        panic!("insert_to_slice: invalid index {} len {}", i, l.len());
    }
    l.insert(i, v);
}

pub fn delete_from_slice<T: Clone>(l: &mut Vec<T>, i: usize) {
    if i >= l.len() {
        panic!("delete_from_slice: invalid index {} len {}", i, l.len());
    }
    l.remove(i);
}

pub fn concat_slices<T: Clone>(slices: &[&[T]]) -> Vec<T> {
    let mut out = Vec::new();
    for s in slices {
        out.extend_from_slice(s);
    }
    out
}
