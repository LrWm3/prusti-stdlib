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
#[ensures(false)] // expected-error: postcondition might not hold
fn exercise<V: Project>(left: &mut V, right: &mut V, choose: bool) {
    {
        let selected = if choose { &mut *left } else { &mut *right };
        *selected.project() = 11;
        *selected.project() = 13;
    }
    *left.project() = 17;
    *right.project() = 19;
}
#[trusted]
fn main() {
    let mut x = 7;
    let mut y = 9;
    let mut a = Holder {
        value: 0,
        nested: &mut x,
    };
    let mut b = Holder {
        value: 0,
        nested: &mut y,
    };
    exercise(&mut a, &mut b, true);
    assert_eq!(a.value, 17);
    assert_eq!(b.value, 19);
    exercise(&mut a, &mut b, false);
    assert_eq!(a.value, 17);
    assert_eq!(b.value, 19);
    assert_eq!(*a.nested, 7);
    assert_eq!(*b.nested, 9);
}
