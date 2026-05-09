// Finger tree persistent sequence, ported from the Go implementation
// (which was itself ported from Data.FingerTree by Hinze & Paterson 2006).
// Specialized to size annotation — no Measured typeclass needed.

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Internal node
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct FtNode<T: Clone + Send + Sync + 'static> {
    size: usize,
    a: Elem<T>,
    b: Elem<T>,
    c: Option<Elem<T>>, // None = 2-node, Some = 3-node
}

impl<T: Clone + Send + Sync + 'static> FtNode<T> {
    fn node2(a: Elem<T>, b: Elem<T>) -> Arc<Self> {
        let size = elem_size(&a) + elem_size(&b);
        Arc::new(FtNode { size, a, b, c: None })
    }
    fn node3(a: Elem<T>, b: Elem<T>, c: Elem<T>) -> Arc<Self> {
        let size = elem_size(&a) + elem_size(&b) + elem_size(&c);
        Arc::new(FtNode { size, a, b, c: Some(c) })
    }
    fn is3(&self) -> bool {
        self.c.is_some()
    }
    fn to_digit(&self) -> Digit<T> {
        if self.is3() {
            dig3(self.a.clone(), self.b.clone(), self.c.clone().unwrap())
        } else {
            dig2(self.a.clone(), self.b.clone())
        }
    }
}

// ---------------------------------------------------------------------------
// Elem — leaf or internal node
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum Elem<T: Clone + Send + Sync + 'static> {
    Leaf(T),
    Node(Arc<FtNode<T>>),
}

fn elem_size<T: Clone + Send + Sync + 'static>(e: &Elem<T>) -> usize {
    match e {
        Elem::Leaf(_) => 1,
        Elem::Node(n) => n.size,
    }
}

// ---------------------------------------------------------------------------
// Digit — 1..4 elements (prefix / suffix)
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Digit<T: Clone + Send + Sync + 'static> {
    a: Elem<T>,
    b: Option<Elem<T>>,
    c: Option<Elem<T>>,
    d: Option<Elem<T>>,
    n: usize,
    size: usize,
}

fn dig1<T: Clone + Send + Sync + 'static>(a: Elem<T>) -> Digit<T> {
    let size = elem_size(&a);
    Digit { a, b: None, c: None, d: None, n: 1, size }
}
fn dig2<T: Clone + Send + Sync + 'static>(a: Elem<T>, b: Elem<T>) -> Digit<T> {
    let size = elem_size(&a) + elem_size(&b);
    Digit { a, b: Some(b), c: None, d: None, n: 2, size }
}
fn dig3<T: Clone + Send + Sync + 'static>(a: Elem<T>, b: Elem<T>, c: Elem<T>) -> Digit<T> {
    let size = elem_size(&a) + elem_size(&b) + elem_size(&c);
    Digit { a, b: Some(b), c: Some(c), d: None, n: 3, size }
}
fn dig4<T: Clone + Send + Sync + 'static>(a: Elem<T>, b: Elem<T>, c: Elem<T>, d: Elem<T>) -> Digit<T> {
    let size = elem_size(&a) + elem_size(&b) + elem_size(&c) + elem_size(&d);
    Digit { a, b: Some(b), c: Some(c), d: Some(d), n: 4, size }
}

impl<T: Clone + Send + Sync + 'static> Digit<T> {
    fn head(&self) -> Elem<T> { self.a.clone() }
    fn last(&self) -> Elem<T> {
        match self.n {
            1 => self.a.clone(),
            2 => self.b.clone().unwrap(),
            3 => self.c.clone().unwrap(),
            4 => self.d.clone().unwrap(),
            _ => panic!("digit last"),
        }
    }
    fn tail(&self) -> Digit<T> {
        match self.n {
            2 => dig1(self.b.clone().unwrap()),
            3 => dig2(self.b.clone().unwrap(), self.c.clone().unwrap()),
            4 => dig3(self.b.clone().unwrap(), self.c.clone().unwrap(), self.d.clone().unwrap()),
            _ => panic!("tail on One"),
        }
    }
    fn init(&self) -> Digit<T> {
        match self.n {
            2 => dig1(self.a.clone()),
            3 => dig2(self.a.clone(), self.b.clone().unwrap()),
            4 => dig3(self.a.clone(), self.b.clone().unwrap(), self.c.clone().unwrap()),
            _ => panic!("init on One"),
        }
    }
    fn cons(&self, a: Elem<T>) -> Digit<T> {
        match self.n {
            1 => dig2(a, self.a.clone()),
            2 => dig3(a, self.a.clone(), self.b.clone().unwrap()),
            3 => dig4(a, self.a.clone(), self.b.clone().unwrap(), self.c.clone().unwrap()),
            _ => panic!("cons on Four"),
        }
    }
    fn snoc(&self, a: Elem<T>) -> Digit<T> {
        match self.n {
            1 => dig2(self.a.clone(), a),
            2 => dig3(self.a.clone(), self.b.clone().unwrap(), a),
            3 => dig4(self.a.clone(), self.b.clone().unwrap(), self.c.clone().unwrap(), a),
            _ => panic!("snoc on Four"),
        }
    }
    fn to_slice(&self) -> Vec<Elem<T>> {
        let mut v = vec![self.a.clone()];
        if let Some(b) = &self.b { v.push(b.clone()); }
        if let Some(c) = &self.c { v.push(c.clone()); }
        if let Some(d) = &self.d { v.push(d.clone()); }
        v
    }
}

fn slice_to_digit<T: Clone + Send + Sync + 'static>(s: Vec<Elem<T>>) -> Digit<T> {
    match s.len() {
        1 => dig1(s.into_iter().next().unwrap()),
        2 => { let mut it = s.into_iter(); dig2(it.next().unwrap(), it.next().unwrap()) }
        3 => { let mut it = s.into_iter(); dig3(it.next().unwrap(), it.next().unwrap(), it.next().unwrap()) }
        4 => { let mut it = s.into_iter(); dig4(it.next().unwrap(), it.next().unwrap(), it.next().unwrap(), it.next().unwrap()) }
        _ => panic!("slice_to_digit"),
    }
}

// ---------------------------------------------------------------------------
// FtTree
// ---------------------------------------------------------------------------

#[derive(Clone)]
enum FtTree<T: Clone + Send + Sync + 'static> {
    Empty,
    Single(Elem<T>),
    Deep {
        size: usize,
        prefix: Digit<T>,
        middle: Box<FtTree<T>>,
        suffix: Digit<T>,
    },
}

impl<T: Clone + Send + Sync + 'static> FtTree<T> {
    fn size(&self) -> usize {
        match self {
            FtTree::Empty => 0,
            FtTree::Single(e) => elem_size(e),
            FtTree::Deep { size, .. } => *size,
        }
    }
}

fn ft_deep<T: Clone + Send + Sync + 'static>(prefix: Digit<T>, middle: FtTree<T>, suffix: Digit<T>) -> FtTree<T> {
    let size = prefix.size + middle.size() + suffix.size;
    FtTree::Deep { size, prefix, middle: Box::new(middle), suffix }
}

fn digit_to_tree<T: Clone + Send + Sync + 'static>(dg: Digit<T>) -> FtTree<T> {
    match dg.n {
        1 => FtTree::Single(dg.a),
        2 => ft_deep(dig1(dg.a), FtTree::Empty, dig1(dg.b.unwrap())),
        3 => ft_deep(dig2(dg.a, dg.b.unwrap()), FtTree::Empty, dig1(dg.c.unwrap())),
        4 => ft_deep(dig2(dg.a, dg.b.unwrap()), FtTree::Empty, dig2(dg.c.unwrap(), dg.d.unwrap())),
        _ => panic!("digit_to_tree"),
    }
}

// ---------------------------------------------------------------------------
// Push front / back
// ---------------------------------------------------------------------------

fn push_front<T: Clone + Send + Sync + 'static>(a: Elem<T>, t: FtTree<T>) -> FtTree<T> {
    match t {
        FtTree::Empty => FtTree::Single(a),
        FtTree::Single(x) => ft_deep(dig1(a), FtTree::Empty, dig1(x)),
        FtTree::Deep { prefix, middle, suffix, .. } => {
            if prefix.n < 4 {
                ft_deep(prefix.cons(a), *middle, suffix)
            } else {
                // prefix full: push node3(b,c,d) into middle, new prefix Two(a, pr.a)
                let node = FtNode::node3(prefix.b.unwrap(), prefix.c.unwrap(), prefix.d.unwrap());
                let new_mid = push_front(Elem::Node(node), *middle);
                ft_deep(dig2(a, prefix.a), new_mid, suffix)
            }
        }
    }
}

fn push_back<T: Clone + Send + Sync + 'static>(t: FtTree<T>, a: Elem<T>) -> FtTree<T> {
    match t {
        FtTree::Empty => FtTree::Single(a),
        FtTree::Single(x) => ft_deep(dig1(x), FtTree::Empty, dig1(a)),
        FtTree::Deep { prefix, middle, suffix, .. } => {
            if suffix.n < 4 {
                ft_deep(prefix, *middle, suffix.snoc(a))
            } else {
                let node = FtNode::node3(suffix.a, suffix.b.unwrap(), suffix.c.unwrap());
                let new_mid = push_back(*middle, Elem::Node(node));
                ft_deep(prefix, new_mid, dig2(suffix.d.unwrap(), a))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// View front / back
// ---------------------------------------------------------------------------

fn view_front<T: Clone + Send + Sync + 'static>(t: FtTree<T>) -> (Elem<T>, FtTree<T>) {
    match t {
        FtTree::Empty => panic!("view_front on empty"),
        FtTree::Single(x) => (x, FtTree::Empty),
        FtTree::Deep { prefix, middle, suffix, .. } => {
            let x = prefix.head();
            let rest = rot_l(prefix, *middle, suffix);
            (x, rest)
        }
    }
}

fn rot_l<T: Clone + Send + Sync + 'static>(pr: Digit<T>, m: FtTree<T>, sf: Digit<T>) -> FtTree<T> {
    if pr.n > 1 {
        return ft_deep(pr.tail(), m, sf);
    }
    if matches!(m, FtTree::Empty) {
        return digit_to_tree(sf);
    }
    let (front, m2) = view_front(m);
    match front {
        Elem::Node(n) => ft_deep(n.to_digit(), m2, sf),
        _ => panic!("rot_l: expected node"),
    }
}

fn view_back<T: Clone + Send + Sync + 'static>(t: FtTree<T>) -> (FtTree<T>, Elem<T>) {
    match t {
        FtTree::Empty => panic!("view_back on empty"),
        FtTree::Single(x) => (FtTree::Empty, x),
        FtTree::Deep { prefix, middle, suffix, .. } => {
            let x = suffix.last();
            let rest = rot_r(prefix, *middle, suffix);
            (rest, x)
        }
    }
}

fn rot_r<T: Clone + Send + Sync + 'static>(pr: Digit<T>, m: FtTree<T>, sf: Digit<T>) -> FtTree<T> {
    if sf.n > 1 {
        return ft_deep(pr, m, sf.init());
    }
    if matches!(m, FtTree::Empty) {
        return digit_to_tree(pr);
    }
    let (m2, back) = view_back(m);
    match back {
        Elem::Node(n) => ft_deep(pr, m2, n.to_digit()),
        _ => panic!("rot_r: expected node"),
    }
}

// ---------------------------------------------------------------------------
// Append / addDigits
// ---------------------------------------------------------------------------

fn append_tree<T: Clone + Send + Sync + 'static>(l: FtTree<T>, r: FtTree<T>) -> FtTree<T> {
    match (&l, &r) {
        (FtTree::Empty, _) => r,
        (_, FtTree::Empty) => l,
        (FtTree::Single(x), _) => push_front(x.clone(), r),
        (_, FtTree::Single(x)) => push_back(l, x.clone()),
        (FtTree::Deep { prefix: lp, middle: lm, suffix: ls, .. },
         FtTree::Deep { prefix: rp, middle: rm, suffix: rs, .. }) => {
            let lp = lp.clone(); let ls = ls.clone(); let lm = *lm.clone();
            let rp = rp.clone(); let rs = rs.clone(); let rm = *rm.clone();
            let new_mid = add_digits0(lm, ls, rp, rm);
            ft_deep(lp, new_mid, rs)
        }
    }
}

type N<T> = Arc<FtNode<T>>;

fn add_digits0<T: Clone + Send + Sync + 'static>(
    m1: FtTree<T>, sf1: Digit<T>, pr2: Digit<T>, m2: FtTree<T>,
) -> FtTree<T> {
    let s = sf1.to_slice();
    let p = pr2.to_slice();
    macro_rules! n2 { ($a:expr, $b:expr) => { FtNode::node2($a, $b) } }
    macro_rules! n3 { ($a:expr, $b:expr, $c:expr) => { FtNode::node3($a, $b, $c) } }
    let key = sf1.n * 10 + pr2.n;
    match key {
        11 => append_tree1(m1, n2!(s[0].clone(),p[0].clone()), m2),
        12 => append_tree1(m1, n3!(s[0].clone(),p[0].clone(),p[1].clone()), m2),
        13 => append_tree2(m1, n2!(s[0].clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        14 => append_tree2(m1, n3!(s[0].clone(),p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        21 => append_tree1(m1, n3!(s[0].clone(),s[1].clone(),p[0].clone()), m2),
        22 => append_tree2(m1, n2!(s[0].clone(),s[1].clone()), n2!(p[0].clone(),p[1].clone()), m2),
        23 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        24 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),p[0].clone()), n3!(p[1].clone(),p[2].clone(),p[3].clone()), m2),
        31 => append_tree2(m1, n2!(s[0].clone(),s[1].clone()), n2!(s[2].clone(),p[0].clone()), m2),
        32 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(p[0].clone(),p[1].clone()), m2),
        33 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(p[0].clone(),p[1].clone(),p[2].clone()), m2),
        34 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        41 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(s[3].clone(),p[0].clone()), m2),
        42 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),p[0].clone(),p[1].clone()), m2),
        43 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(s[3].clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        44 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        _ => panic!("add_digits0"),
    }
}

fn append_tree1<T: Clone + Send + Sync + 'static>(l: FtTree<T>, n1: N<T>, r: FtTree<T>) -> FtTree<T> {
    match (&l, &r) {
        (FtTree::Empty, _) => push_front(Elem::Node(n1), r),
        (_, FtTree::Empty) => push_back(l, Elem::Node(n1)),
        (FtTree::Single(x), _) => push_front(x.clone(), push_front(Elem::Node(n1), r)),
        (_, FtTree::Single(x)) => push_back(push_back(l, Elem::Node(n1)), x.clone()),
        (FtTree::Deep { prefix: lp, middle: lm, suffix: ls, .. },
         FtTree::Deep { prefix: rp, middle: rm, suffix: rs, .. }) => {
            let lp = lp.clone(); let ls = ls.clone(); let lm = *lm.clone();
            let rp = rp.clone(); let rs = rs.clone(); let rm = *rm.clone();
            let mid = add_digits1(lm, ls, Elem::Node(n1), rp, rm);
            ft_deep(lp, mid, rs)
        }
    }
}

fn add_digits1<T: Clone + Send + Sync + 'static>(
    m1: FtTree<T>, sf1: Digit<T>, x: Elem<T>, pr2: Digit<T>, m2: FtTree<T>,
) -> FtTree<T> {
    let s = sf1.to_slice();
    let p = pr2.to_slice();
    macro_rules! n2 { ($a:expr, $b:expr) => { FtNode::node2($a, $b) } }
    macro_rules! n3 { ($a:expr, $b:expr, $c:expr) => { FtNode::node3($a, $b, $c) } }
    let key = sf1.n * 10 + pr2.n;
    match key {
        11 => append_tree1(m1, n3!(s[0].clone(),x.clone(),p[0].clone()), m2),
        12 => append_tree2(m1, n2!(s[0].clone(),x.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        13 => append_tree2(m1, n3!(s[0].clone(),x.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        14 => append_tree2(m1, n3!(s[0].clone(),x.clone(),p[0].clone()), n3!(p[1].clone(),p[2].clone(),p[3].clone()), m2),
        21 => append_tree2(m1, n2!(s[0].clone(),s[1].clone()), n2!(x.clone(),p[0].clone()), m2),
        22 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        23 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(p[0].clone(),p[1].clone(),p[2].clone()), m2),
        24 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n2!(p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        31 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(x.clone(),p[0].clone()), m2),
        32 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),p[0].clone(),p[1].clone()), m2),
        33 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(x.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        34 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        41 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),p[0].clone()), m2),
        42 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(s[3].clone(),x.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        43 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        44 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),p[0].clone()), n3!(p[1].clone(),p[2].clone(),p[3].clone()), m2),
        _ => panic!("add_digits1"),
    }
}

fn append_tree2<T: Clone + Send + Sync + 'static>(l: FtTree<T>, n1: N<T>, n2: N<T>, r: FtTree<T>) -> FtTree<T> {
    match (&l, &r) {
        (FtTree::Empty, _) => push_front(Elem::Node(n1), push_front(Elem::Node(n2), r)),
        (_, FtTree::Empty) => push_back(push_back(l, Elem::Node(n1)), Elem::Node(n2)),
        (FtTree::Single(x), _) => push_front(x.clone(), push_front(Elem::Node(n1), push_front(Elem::Node(n2), r))),
        (_, FtTree::Single(x)) => push_back(push_back(push_back(l, Elem::Node(n1)), Elem::Node(n2)), x.clone()),
        (FtTree::Deep { prefix: lp, middle: lm, suffix: ls, .. },
         FtTree::Deep { prefix: rp, middle: rm, suffix: rs, .. }) => {
            let lp = lp.clone(); let ls = ls.clone(); let lm = *lm.clone();
            let rp = rp.clone(); let rs = rs.clone(); let rm = *rm.clone();
            let mid = add_digits2(lm, ls, Elem::Node(n1), Elem::Node(n2), rp, rm);
            ft_deep(lp, mid, rs)
        }
    }
}

fn add_digits2<T: Clone + Send + Sync + 'static>(
    m1: FtTree<T>, sf1: Digit<T>, x: Elem<T>, y: Elem<T>, pr2: Digit<T>, m2: FtTree<T>,
) -> FtTree<T> {
    let s = sf1.to_slice();
    let p = pr2.to_slice();
    macro_rules! n2 { ($a:expr, $b:expr) => { FtNode::node2($a, $b) } }
    macro_rules! n3 { ($a:expr, $b:expr, $c:expr) => { FtNode::node3($a, $b, $c) } }
    let key = sf1.n * 10 + pr2.n;
    match key {
        11 => append_tree2(m1, n2!(s[0].clone(),x.clone()), n2!(y.clone(),p[0].clone()), m2),
        12 => append_tree2(m1, n3!(s[0].clone(),x.clone(),y.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        13 => append_tree2(m1, n3!(s[0].clone(),x.clone(),y.clone()), n3!(p[0].clone(),p[1].clone(),p[2].clone()), m2),
        14 => append_tree3(m1, n3!(s[0].clone(),x.clone(),y.clone()), n2!(p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        21 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n2!(y.clone(),p[0].clone()), m2),
        22 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),p[0].clone(),p[1].clone()), m2),
        23 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n2!(y.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        24 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        31 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),p[0].clone()), m2),
        32 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(x.clone(),y.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        33 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        34 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),p[0].clone()), n3!(p[1].clone(),p[2].clone(),p[3].clone()), m2),
        41 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(s[3].clone(),x.clone()), n2!(y.clone(),p[0].clone()), m2),
        42 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        43 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n3!(p[0].clone(),p[1].clone(),p[2].clone()), m2),
        44 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n2!(p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        _ => panic!("add_digits2"),
    }
}

fn append_tree3<T: Clone + Send + Sync + 'static>(l: FtTree<T>, n1: N<T>, n2: N<T>, n3: N<T>, r: FtTree<T>) -> FtTree<T> {
    match (&l, &r) {
        (FtTree::Empty, _) => push_front(Elem::Node(n1), push_front(Elem::Node(n2), push_front(Elem::Node(n3), r))),
        (_, FtTree::Empty) => push_back(push_back(push_back(l, Elem::Node(n1)), Elem::Node(n2)), Elem::Node(n3)),
        (FtTree::Single(x), _) => push_front(x.clone(), push_front(Elem::Node(n1), push_front(Elem::Node(n2), push_front(Elem::Node(n3), r)))),
        (_, FtTree::Single(x)) => push_back(push_back(push_back(push_back(l, Elem::Node(n1)), Elem::Node(n2)), Elem::Node(n3)), x.clone()),
        (FtTree::Deep { prefix: lp, middle: lm, suffix: ls, .. },
         FtTree::Deep { prefix: rp, middle: rm, suffix: rs, .. }) => {
            let lp = lp.clone(); let ls = ls.clone(); let lm = *lm.clone();
            let rp = rp.clone(); let rs = rs.clone(); let rm = *rm.clone();
            let mid = add_digits3(lm, ls, Elem::Node(n1), Elem::Node(n2), Elem::Node(n3), rp, rm);
            ft_deep(lp, mid, rs)
        }
    }
}

fn add_digits3<T: Clone + Send + Sync + 'static>(
    m1: FtTree<T>, sf1: Digit<T>, x: Elem<T>, y: Elem<T>, z: Elem<T>, pr2: Digit<T>, m2: FtTree<T>,
) -> FtTree<T> {
    let s = sf1.to_slice();
    let p = pr2.to_slice();
    macro_rules! n2 { ($a:expr, $b:expr) => { FtNode::node2($a, $b) } }
    macro_rules! n3 { ($a:expr, $b:expr, $c:expr) => { FtNode::node3($a, $b, $c) } }
    let key = sf1.n * 10 + pr2.n;
    match key {
        11 => append_tree2(m1, n3!(s[0].clone(),x.clone(),y.clone()), n2!(z.clone(),p[0].clone()), m2),
        12 => append_tree2(m1, n3!(s[0].clone(),x.clone(),y.clone()), n3!(z.clone(),p[0].clone(),p[1].clone()), m2),
        13 => append_tree3(m1, n3!(s[0].clone(),x.clone(),y.clone()), n2!(z.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        14 => append_tree3(m1, n3!(s[0].clone(),x.clone(),y.clone()), n3!(z.clone(),p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        21 => append_tree2(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),z.clone(),p[0].clone()), m2),
        22 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n2!(y.clone(),z.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        23 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),z.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        24 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),z.clone(),p[0].clone()), n3!(p[1].clone(),p[2].clone(),p[3].clone()), m2),
        31 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n2!(x.clone(),y.clone()), n2!(z.clone(),p[0].clone()), m2),
        32 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),z.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        33 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),z.clone()), n3!(p[0].clone(),p[1].clone(),p[2].clone()), m2),
        34 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),z.clone()), n2!(p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        41 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n2!(z.clone(),p[0].clone()), m2),
        42 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n3!(z.clone(),p[0].clone(),p[1].clone()), m2),
        43 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n2!(z.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        44 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n3!(z.clone(),p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        _ => panic!("add_digits3"),
    }
}

fn append_tree4<T: Clone + Send + Sync + 'static>(l: FtTree<T>, n1: N<T>, n2: N<T>, n3: N<T>, n4: N<T>, r: FtTree<T>) -> FtTree<T> {
    match (&l, &r) {
        (FtTree::Empty, _) => push_front(Elem::Node(n1), push_front(Elem::Node(n2), push_front(Elem::Node(n3), push_front(Elem::Node(n4), r)))),
        (_, FtTree::Empty) => push_back(push_back(push_back(push_back(l, Elem::Node(n1)), Elem::Node(n2)), Elem::Node(n3)), Elem::Node(n4)),
        (FtTree::Single(x), _) => push_front(x.clone(), push_front(Elem::Node(n1), push_front(Elem::Node(n2), push_front(Elem::Node(n3), push_front(Elem::Node(n4), r))))),
        (_, FtTree::Single(x)) => push_back(push_back(push_back(push_back(push_back(l, Elem::Node(n1)), Elem::Node(n2)), Elem::Node(n3)), Elem::Node(n4)), x.clone()),
        (FtTree::Deep { prefix: lp, middle: lm, suffix: ls, .. },
         FtTree::Deep { prefix: rp, middle: rm, suffix: rs, .. }) => {
            let lp = lp.clone(); let ls = ls.clone(); let lm = *lm.clone();
            let rp = rp.clone(); let rs = rs.clone(); let rm = *rm.clone();
            let mid = add_digits4(lm, ls, Elem::Node(n1), Elem::Node(n2), Elem::Node(n3), Elem::Node(n4), rp, rm);
            ft_deep(lp, mid, rs)
        }
    }
}

fn add_digits4<T: Clone + Send + Sync + 'static>(
    m1: FtTree<T>, sf1: Digit<T>, x: Elem<T>, y: Elem<T>, z: Elem<T>, w: Elem<T>, pr2: Digit<T>, m2: FtTree<T>,
) -> FtTree<T> {
    let s = sf1.to_slice();
    let p = pr2.to_slice();
    macro_rules! n2 { ($a:expr, $b:expr) => { FtNode::node2($a, $b) } }
    macro_rules! n3 { ($a:expr, $b:expr, $c:expr) => { FtNode::node3($a, $b, $c) } }
    let key = sf1.n * 10 + pr2.n;
    match key {
        11 => append_tree2(m1, n3!(s[0].clone(),x.clone(),y.clone()), n3!(z.clone(),w.clone(),p[0].clone()), m2),
        12 => append_tree3(m1, n3!(s[0].clone(),x.clone(),y.clone()), n2!(z.clone(),w.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        13 => append_tree3(m1, n3!(s[0].clone(),x.clone(),y.clone()), n3!(z.clone(),w.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        14 => append_tree3(m1, n3!(s[0].clone(),x.clone(),y.clone()), n3!(z.clone(),w.clone(),p[0].clone()), n3!(p[1].clone(),p[2].clone(),p[3].clone()), m2),
        21 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n2!(y.clone(),z.clone()), n2!(w.clone(),p[0].clone()), m2),
        22 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),z.clone(),w.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        23 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),z.clone(),w.clone()), n3!(p[0].clone(),p[1].clone(),p[2].clone()), m2),
        24 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),x.clone()), n3!(y.clone(),z.clone(),w.clone()), n2!(p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        31 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),z.clone()), n2!(w.clone(),p[0].clone()), m2),
        32 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),z.clone()), n3!(w.clone(),p[0].clone(),p[1].clone()), m2),
        33 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),z.clone()), n2!(w.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        34 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(x.clone(),y.clone(),z.clone()), n3!(w.clone(),p[0].clone(),p[1].clone()), n2!(p[2].clone(),p[3].clone()), m2),
        41 => append_tree3(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n3!(z.clone(),w.clone(),p[0].clone()), m2),
        42 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n2!(z.clone(),w.clone()), n2!(p[0].clone(),p[1].clone()), m2),
        43 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n3!(z.clone(),w.clone(),p[0].clone()), n2!(p[1].clone(),p[2].clone()), m2),
        44 => append_tree4(m1, n3!(s[0].clone(),s[1].clone(),s[2].clone()), n3!(s[3].clone(),x.clone(),y.clone()), n3!(z.clone(),w.clone(),p[0].clone()), n3!(p[1].clone(),p[2].clone(),p[3].clone()), m2),
        _ => panic!("add_digits4"),
    }
}

// ---------------------------------------------------------------------------
// Iteration
// ---------------------------------------------------------------------------

fn iter_elem<T: Clone + Send + Sync + 'static>(e: &Elem<T>, f: &mut impl FnMut(T)) {
    match e {
        Elem::Leaf(v) => f(v.clone()),
        Elem::Node(n) => {
            iter_elem(&n.a, f);
            iter_elem(&n.b, f);
            if let Some(c) = &n.c { iter_elem(c, f); }
        }
    }
}

fn iter_digit<T: Clone + Send + Sync + 'static>(dg: &Digit<T>, f: &mut impl FnMut(T)) {
    iter_elem(&dg.a, f);
    if let Some(b) = &dg.b { iter_elem(b, f); }
    if let Some(c) = &dg.c { iter_elem(c, f); }
    if let Some(d) = &dg.d { iter_elem(d, f); }
}

fn iter_tree<T: Clone + Send + Sync + 'static>(t: &FtTree<T>, f: &mut impl FnMut(T)) {
    match t {
        FtTree::Empty => {}
        FtTree::Single(e) => iter_elem(e, f),
        FtTree::Deep { prefix, middle, suffix, .. } => {
            iter_digit(prefix, f);
            iter_tree(middle, f);
            iter_digit(suffix, f);
        }
    }
}

// ---------------------------------------------------------------------------
// Split
// ---------------------------------------------------------------------------

fn split_digit_at<T: Clone + Send + Sync + 'static>(
    dg: Digit<T>, mut i: usize,
) -> (Option<Digit<T>>, Elem<T>, Option<Digit<T>>) {
    let elems = dg.to_slice();
    let mut acc = 0usize;
    for (idx, e) in elems.iter().enumerate() {
        acc += elem_size(e);
        if i < acc {
            let left = if idx > 0 {
                Some(slice_to_digit(elems[..idx].to_vec()))
            } else {
                None
            };
            let right = if idx < elems.len() - 1 {
                Some(slice_to_digit(elems[idx + 1..].to_vec()))
            } else {
                None
            };
            return (left, e.clone(), right);
        }
        let _ = i; // suppress warning
    }
    panic!("split_digit_at out of range");
}

fn split_node_at<T: Clone + Send + Sync + 'static>(
    n: &FtNode<T>, i: usize,
) -> (Option<Digit<T>>, Elem<T>, Option<Digit<T>>) {
    let mut elems = vec![n.a.clone(), n.b.clone()];
    if let Some(c) = &n.c { elems.push(c.clone()); }
    split_digit_at(slice_to_digit(elems), i)
}

fn deep_l_opt<T: Clone + Send + Sync + 'static>(
    pr: Option<Digit<T>>, m: FtTree<T>, sf: Digit<T>,
) -> FtTree<T> {
    match pr {
        None => {
            if matches!(m, FtTree::Empty) {
                return digit_to_tree(sf);
            }
            let (front, m2) = view_front(m);
            match front {
                Elem::Node(n) => ft_deep(n.to_digit(), m2, sf),
                _ => panic!("deep_l_opt: expected node"),
            }
        }
        Some(pr) => ft_deep(pr, m, sf),
    }
}

fn deep_r_opt<T: Clone + Send + Sync + 'static>(
    pr: Digit<T>, m: FtTree<T>, sf: Option<Digit<T>>,
) -> FtTree<T> {
    match sf {
        None => {
            if matches!(m, FtTree::Empty) {
                return digit_to_tree(pr);
            }
            let (m2, back) = view_back(m);
            match back {
                Elem::Node(n) => ft_deep(pr, m2, n.to_digit()),
                _ => panic!("deep_r_opt: expected node"),
            }
        }
        Some(sf) => ft_deep(pr, m, sf),
    }
}

struct FtSplit<T: Clone + Send + Sync + 'static> {
    left: FtTree<T>,
    pivot: Elem<T>,
    right: FtTree<T>,
}

fn split_tree_at<T: Clone + Send + Sync + 'static>(t: FtTree<T>, i: usize) -> FtSplit<T> {
    match t {
        FtTree::Empty => panic!("split_tree_at on empty"),
        FtTree::Single(x) => FtSplit { left: FtTree::Empty, pivot: x, right: FtTree::Empty },
        FtTree::Deep { prefix, middle, suffix, .. } => {
            if i < prefix.size {
                let (l, x, r) = split_digit_at(prefix, i);
                let lt = match l {
                    None => FtTree::Empty,
                    Some(d) => digit_to_tree(d),
                };
                FtSplit { left: lt, pivot: x, right: deep_l_opt(r, *middle, suffix) }
            } else {
                let i = i - prefix.size;
                let mid_size = middle.size();
                if i < mid_size {
                    let ms = split_tree_at(*middle, i);
                    let node = match &ms.pivot {
                        Elem::Node(n) => n.clone(),
                        _ => panic!("split_tree_at: expected node in middle"),
                    };
                    let ni = i - ms.left.size();
                    let (nl, nx, nr) = split_node_at(&node, ni);
                    FtSplit {
                        left: deep_r_opt(prefix, ms.left, nl),
                        pivot: nx,
                        right: deep_l_opt(nr, ms.right, suffix),
                    }
                } else {
                    let i = i - mid_size;
                    let (l, x, r) = split_digit_at(suffix, i);
                    let rt = match r {
                        None => FtTree::Empty,
                        Some(d) => digit_to_tree(d),
                    };
                    FtSplit { left: deep_r_opt(prefix, *middle, l), pivot: x, right: rt }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tree depth
// ---------------------------------------------------------------------------

fn tree_depth<T: Clone + Send + Sync + 'static>(t: &FtTree<T>) -> usize {
    match t {
        FtTree::Empty => 0,
        FtTree::Single(_) => 1,
        FtTree::Deep { middle, .. } => 1 + tree_depth(middle),
    }
}

// ---------------------------------------------------------------------------
// Public Seq<T>
// ---------------------------------------------------------------------------

/// Persistent finger-tree sequence.
#[derive(Clone)]
pub struct Seq<T: Clone + Send + Sync + 'static> {
    tree: FtTree<T>,
}

impl<T: Clone + Send + Sync + 'static> Seq<T> {
    pub fn empty() -> Self {
        Seq { tree: FtTree::Empty }
    }

    pub fn len(&self) -> usize {
        self.tree.size()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, i: usize) -> T {
        let sp = split_tree_at(self.tree.clone(), i);
        match sp.pivot {
            Elem::Leaf(v) => v,
            _ => panic!("Seq::get: expected leaf"),
        }
    }

    pub fn set(&self, i: usize, val: T) -> Seq<T> {
        let sp = split_tree_at(self.tree.clone(), i);
        let tree = append_tree(sp.left, push_front(Elem::Leaf(val), sp.right));
        Seq { tree }
    }

    pub fn ins(&self, i: usize, val: T) -> Seq<T> {
        if i == 0 {
            return Seq { tree: push_front(Elem::Leaf(val), self.tree.clone()) };
        }
        if i == self.tree.size() {
            return Seq { tree: push_back(self.tree.clone(), Elem::Leaf(val)) };
        }
        let sp = split_tree_at(self.tree.clone(), i);
        let tree = append_tree(sp.left, push_front(Elem::Leaf(val), push_front(sp.pivot, sp.right)));
        Seq { tree }
    }

    pub fn del(&self, i: usize) -> Seq<T> {
        let sp = split_tree_at(self.tree.clone(), i);
        Seq { tree: append_tree(sp.left, sp.right) }
    }

    pub fn split(&self, i: usize) -> (Seq<T>, Seq<T>) {
        if i == 0 {
            return (Seq::empty(), self.clone());
        }
        if i >= self.tree.size() {
            return (self.clone(), Seq::empty());
        }
        let sp = split_tree_at(self.tree.clone(), i);
        let right = push_front(sp.pivot, sp.right);
        (Seq { tree: sp.left }, Seq { tree: right })
    }

    pub fn merge(&self, other: &Seq<T>) -> Seq<T> {
        Seq { tree: append_tree(self.tree.clone(), other.tree.clone()) }
    }

    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        let mut out = Vec::with_capacity(self.tree.size());
        iter_tree(&self.tree, &mut |v| out.push(v));
        out.into_iter()
    }

    pub fn depth(&self) -> usize {
        tree_depth(&self.tree)
    }

    pub fn repr(&self) -> Vec<T> {
        let mut out = Vec::with_capacity(self.tree.size());
        iter_tree(&self.tree, &mut |v| out.push(v));
        out
    }
}

pub fn merge_seqs<T: Clone + Send + Sync + 'static>(seqs: &[Seq<T>]) -> Seq<T> {
    if seqs.is_empty() {
        return Seq::empty();
    }
    let mut acc = seqs[0].clone();
    for s in &seqs[1..] {
        acc = acc.merge(s);
    }
    acc
}
