use prusti_contracts::*;
trait Unchanged {
    #[ensures(*value == old(*value))]
    fn update(&self, value: &mut u32);
}
struct Bad;
#[refine_trait_spec]
impl Unchanged for Bad {
    #[ensures(true)] // expected-error: the implementation's postcondition may be weaker than the trait method's
    fn update(&self, value: &mut u32) {
        *value = 1;
    }
}
fn main() {}
