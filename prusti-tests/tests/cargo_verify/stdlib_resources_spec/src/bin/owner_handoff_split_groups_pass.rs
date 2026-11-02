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
fn exercise(owner: &mut Split<'_, '_>) {
    *owner.project() = 11;
    *owner.project() = 13;
}
#[trusted]
fn main() {
    let mut x = 7;
    let mut y = 9;
    let mut owner = Split {
        value: 0,
        left: &mut x,
        right: &mut y,
    };
    exercise(&mut owner);
    assert_eq!(owner.value, 13);
    assert_eq!(*owner.left, 7);
    assert_eq!(*owner.right, 9);
}
