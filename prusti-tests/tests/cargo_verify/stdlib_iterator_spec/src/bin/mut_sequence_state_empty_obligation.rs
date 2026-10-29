use prusti_contracts::*;

#[assert_on_expiry(*value => false, true)]
fn relay<T>(value: &mut T) -> &mut T { value }

fn visit(value: &mut u32) {
    let returned = relay(value);
    let observed = *returned; // expected-unsupported: cannot encode method body `visit`: function-call wand has no identifiable source projections
    prusti_assert!(observed == observed);
    let _ = *value;
}

fn main() {}
