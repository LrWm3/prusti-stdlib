use prusti_contracts::*;
trait Read<'a> { fn get(&self) -> &'a u8; }
struct Borrowed<'a> { value: &'a u8 }
#[refine_trait_spec]
impl<'a> Read<'a> for Borrowed<'a> {
    #[requires(false)] // expected-error: the implementation's precondition may be stronger than the trait method's
    fn get(&self) -> &'a u8 { self.value }
}
fn main() {}
