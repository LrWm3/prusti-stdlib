use prusti_contracts::*;
#[after_expiry(value[_] => true)]
fn relay<T, U>(value: (T, U)) -> (T, U) { // expected-unsupported: cannot encode method `relay`: expiry selector must identify a unique opaque input resource group
    value
}
fn main() {}
