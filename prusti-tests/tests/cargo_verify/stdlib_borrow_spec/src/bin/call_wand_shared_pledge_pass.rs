use prusti_contracts::*;

// Shared references have no exclusive indirect predicate. The expiry pledge
// still creates an obligation, and must survive edge-to-wand selection.
#[ensures(*result == *value)]
#[after_expiry(*value == before_expiry(*result))]
fn borrow(value: &u32) -> &u32 { value }

fn visit(value: &u32, choice: bool) {
    let borrowed = if choice { borrow(value) } else { borrow(value) };
    prusti_assert!(*borrowed == *value);
    prusti_assert!(*value <= u32::MAX);
}

fn main() {
    visit(&1, true);
    visit(&2, false);
}
