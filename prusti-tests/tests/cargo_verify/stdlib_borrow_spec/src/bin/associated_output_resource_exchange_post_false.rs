use prusti_contracts::*;

#[ensures(result.0 === right)] // expected-error: postcondition might not hold
fn exchange<T>(left: T, right: T) -> (T, T) {
    (left, right)
}

fn main() {}
