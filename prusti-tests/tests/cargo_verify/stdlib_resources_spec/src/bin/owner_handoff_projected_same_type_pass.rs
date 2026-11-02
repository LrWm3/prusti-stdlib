use prusti_contracts::*;
trait Project {
    fn project(&mut self) -> &mut usize;
}
struct Holder<'a> {
    value: usize,
    nested: &'a mut u32,
}
#[refine_trait_spec]
impl Project for Holder<'_> {
    fn project(&mut self) -> &mut usize {
        &mut self.value
    }
}

struct Split<'a, 'b> {
    value: usize,
    left: &'a mut u32,
    right: &'b mut u64,
}
#[refine_trait_spec]
impl Project for Split<'_, '_> {
    fn project(&mut self) -> &mut usize {
        &mut self.value
    }
}
struct Pair<A, B> {
    left: A,
    right: B,
}
fn exercise<A: Project>(owner: &mut Pair<A, A>) {
    *owner.left.project() = 11;
    *owner.right.project() = 17;
    *owner.left.project() = 13;
}
#[trusted]
fn main() {
    let mut x = 7;
    let mut y = 9;
    let mut pair = Pair {
        left: Holder {
            value: 0,
            nested: &mut x,
        },
        right: Holder {
            value: 0,
            nested: &mut y,
        },
    };
    exercise(&mut pair);
    assert_eq!(pair.left.value, 13);
    assert_eq!(pair.right.value, 17);
    assert_eq!(*pair.left.nested, 7);
    assert_eq!(*pair.right.nested, 9);
}
