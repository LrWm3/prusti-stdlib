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

fn exercise(owner: &mut Holder<'_>) {
    *owner.project() = 11;
    *owner.project() = 13;
}
#[trusted]
fn main() {
    let mut x = 7u32;
    {
        let mut owner = Holder {
            value: 1,
            nested: &mut x,
        };
        exercise(&mut owner);
        assert_eq!(owner.value, 13);
        assert_eq!(*owner.nested, 7);
    }
    assert_eq!(x, 7);
}
