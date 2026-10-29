use prusti_contracts::*;

#[ensures(Seq::from_slice(&*result) == old(Seq::from_slice(&*values)))]
#[after_expiry(values => Seq::from_slice(&*values) == before_expiry(Seq::from_slice(&*result)))]
#[after_expiry(values[_] => {
    let source = old(Seq::from_slice(&*values));
    let returned = after_call(Seq::from_slice(&*result));
    forall(move |j: Int|
        (Int::from(0) <= j && j < source.len()) ==>
        sequence_resource_state(&source, j)
            == before_expiry(sequence_resource_state(&returned, j)),
        triggers = [(source[j],)]
    )
})]
fn relay<T>(values: &mut [T]) -> &mut [T] { values }

#[requires(index < values.len())]
fn visit(values: &mut [&mut u32], index: usize) {
    let returned = relay(values);
    *returned[index] = 7;
    let updated = *values[index];
    prusti_assert!(updated == 7);
}

#[trusted]
fn main() {
    let mut value = 1;
    visit(&mut [&mut value], 0);
    assert_eq!(value, 7);
}
