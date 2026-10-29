use prusti_contracts::*;

#[requires(index < values.len())]
#[ensures(Seq::from_slice(&*result) == old(Seq::from_slice(&*values)))]
#[after_expiry(values => Seq::from_slice(&*values) == before_expiry(Seq::from_slice(&*result)))]
#[after_expiry(values[_] => {
    let source = old(Seq::from_slice(&*values));
    let returned = after_call(Seq::from_slice(&*result));
    sequence_resource_state(&source, Int::from(index))
        == before_expiry(sequence_resource_state(&returned, Int::from(index)))
})]
fn relay<T>(values: &mut [T], index: usize) -> &mut [T] { values }

#[requires(values.len() > 0)]
fn visit(values: &mut [&mut u32]) {
    let returned = relay(values, 0);
    *returned[0] = 7;
    let updated = *values[0];
    prusti_assert!(updated == 8); // expected-error: assertion might not hold
}

#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut [&mut value]);
    assert_eq!(value, 7);
}
