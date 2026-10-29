use prusti_contracts::*;

struct Owned { value: u32 }

#[assert_on_expiry(*value => false, true)]
fn relay<T>(value: &mut T) -> &mut T { value }

fn visit(value: &mut Owned) {
    let returned = relay(value);
    let observed = returned.value; // expected-unsupported: cannot encode method body `visit`: function-call wand has no identifiable source projections
    prusti_assert!(observed == observed);
    let _ = value.value;
}

fn main() {}
