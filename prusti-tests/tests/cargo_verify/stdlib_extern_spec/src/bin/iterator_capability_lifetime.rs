use prusti_contracts::*;

// This test-only capability is intentionally assumed.
trait StaticOnly {}
impl StaticOnly for &'static u8 {}

#[extern_spec(target_api)]
trait Observe {
    #[trusted]
    #[refine_spec(where Self: StaticOnly, [ensures(result == 7)])]
    fn observe(&self) -> u32;
}

fn arbitrary<'a>(value: &'a u8) {
    let result = <&u8 as target_api::Observe>::observe(&value);
    prusti_assert!(result == 7); // expected-error: assertion might not hold
}

fn main() {}
