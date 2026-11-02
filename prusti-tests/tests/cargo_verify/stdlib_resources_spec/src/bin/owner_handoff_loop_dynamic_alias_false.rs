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

#[requires(count > 0)]
#[ensures(false)] // expected-error: postcondition might not hold
fn exercise<V: Project>(left: &mut V, right: &mut V, count: usize) {
    let mut index = 0usize;
    while index < count {
        body_invariant!(index < count);
        let selected = if index % 2 == 0 {
            &mut *left
        } else {
            &mut *right
        };
        *selected.project() = 11;
        index += 1;
    }
}
#[trusted]
fn main() {
    let (mut x, mut y) = (7u32, 9u32);
    let mut left = Holder {
        value: 1,
        nested: &mut x,
    };
    let mut right = Holder {
        value: 2,
        nested: &mut y,
    };
    exercise(&mut left, &mut right, 4);
    assert_eq!((left.value, right.value), (11, 11));
    assert_eq!((*left.nested, *right.nested), (7, 9));
}
