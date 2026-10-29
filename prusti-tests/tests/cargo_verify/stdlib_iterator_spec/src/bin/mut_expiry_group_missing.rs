use prusti_contracts::*;
#[after_expiry(value[_] => true)]
fn relay(value: &mut u32) -> &mut u32 { // expected-unsupported: cannot encode method `relay`: expiry selector must identify a unique opaque input resource group
    value
}
fn main() {}
