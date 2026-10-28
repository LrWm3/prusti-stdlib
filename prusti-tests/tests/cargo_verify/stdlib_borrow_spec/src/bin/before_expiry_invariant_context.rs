use prusti_contracts::*;

fn wrong(value: &mut u32) {
    loop {
        body_invariant!(before_expiry(*value) == 0); // expected-unsupported: cannot encode method body `wrong`: before_expiry is only supported in expiry pledges
        if *value == 0 { break; }
        *value -= 1;
    }
}

fn main() {}
