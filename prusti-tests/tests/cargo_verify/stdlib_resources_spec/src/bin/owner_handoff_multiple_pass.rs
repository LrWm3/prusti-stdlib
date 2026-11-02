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

fn exercise<A: Project, B: Project>(left: &mut A, right: &mut B) {
    *left.project() = 11;
    *right.project() = 17;
    *left.project() = 13;
}
#[trusted]
fn main() {
    let mut x = 7u32;
    let mut y = 9u32;
    {
        let mut left = Holder {
            value: 1,
            nested: &mut x,
        };
        let mut right = Holder {
            value: 2,
            nested: &mut y,
        };
        exercise(&mut left, &mut right);
        assert_eq!(left.value, 13);
        assert_eq!(right.value, 17);
        assert_eq!(*left.nested, 7);
        assert_eq!(*right.nested, 9);
    }
    assert_eq!(x, 7);
    assert_eq!(y, 9);
}
