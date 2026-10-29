use prusti_contracts::*;

#[ensures(Seq::from_slice(&*result) == old(Seq::from_slice(&*values)))]
#[after_expiry(values => Seq::from_slice(&*values) == before_expiry(Seq::from_slice(&*result)))]
#[after_expiry(values[_] => { // expected-error: pledge postcondition might not hold
    let source = old(Seq::from_slice(&*values));
    let returned = after_call(Seq::from_slice(&*result));
    false && forall(move |j: Int|
        (Int::from(0) <= j && j < source.len()) ==>
        sequence_resource_state(&source, j)
            == before_expiry(sequence_resource_state(&returned, j)),
        triggers = [(sequence_resource_state(&source, j),)]
    )
})]
fn relay<T>(values: &mut [T]) -> &mut [T] { values }

#[trusted]
fn main() {
    let mut first = 1;
    let mut second = 2;
    let mut values = [&mut first, &mut second];
    let returned = relay(&mut values);
    *returned[0] = 7;
    *returned[1] = 8;
    assert_eq!(first, 7);
    assert_eq!(second, 8);
}
