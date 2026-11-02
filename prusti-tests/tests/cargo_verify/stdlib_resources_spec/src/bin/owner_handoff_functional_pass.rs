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

#[requires(*counter == 0)]
#[ensures(*counter == count && *frame == old(*frame))]
fn exercise<V: Project>(owner: &mut V, counter: &mut usize, frame: &mut usize, count: usize) {
    let mut index = 0usize;
    while index < count {
        body_invariant!(index < count);
        body_invariant!(count == old(count));
        body_invariant!(*counter == index);
        body_invariant!(*frame == old(*frame));
        *owner.project() = 11;
        *counter += 1;
        index += 1;
    }
}
#[trusted]
fn main() {
    let mut nested = 7;
    let mut owner = Holder {
        value: 3,
        nested: &mut nested,
    };
    let mut counter = 0;
    let mut frame = 31;
    exercise(&mut owner, &mut counter, &mut frame, 0);
    assert_eq!(counter, 0);
    assert_eq!(frame, 31);
    assert_eq!(owner.value, 3);
    exercise(&mut owner, &mut counter, &mut frame, 4);
    assert_eq!(counter, 4);
    assert_eq!(frame, 31);
    assert_eq!(owner.value, 11);
    assert_eq!(*owner.nested, 7);
}
