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
    #[ensures(false)] // expected-error: postcondition might not hold
    fn project(&mut self) -> &mut usize {
        &mut self.value
    }
}

#[trusted]
fn main() {}
