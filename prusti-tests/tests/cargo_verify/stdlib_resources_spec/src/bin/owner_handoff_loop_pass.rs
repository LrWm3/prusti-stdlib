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

fn exercise<V: Project>(owner: &mut V, count: usize) {
    let mut index = 0usize;
    while index < count {
        body_invariant!(index < count);
        *owner.project() = 11;
        index += 1;
    }
}

#[trusted]
fn main() {
    let mut pointee = 7u32;
    {
        let mut owner = Holder {
            value: 3,
            nested: &mut pointee,
        };
        exercise(&mut owner, 0);
        assert_eq!(owner.value, 3);
        exercise(&mut owner, 4);
        assert_eq!(owner.value, 11);
        assert_eq!(*owner.nested, 7);
    }
    assert_eq!(pointee, 7);
}
