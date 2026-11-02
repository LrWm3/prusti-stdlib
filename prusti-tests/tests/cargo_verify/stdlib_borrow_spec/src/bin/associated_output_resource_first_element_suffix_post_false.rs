use prusti_contracts::*;

#[requires(values.len() > 0)]
#[ensures(&*result === old(&values[0]))]
#[after_expiry(values[_] => {
    let source = old(Seq::from_slice(&*values));
    forall(move |j: Int| (Int::from(1) <= j && j < source.len()) ==>
        sequence_resource_state(&source, j) == old(sequence_resource_state(&Seq::from_slice(&*values), j)),
        triggers = [(source[j],)])
})]
#[ensures(false)] // expected-error: postcondition might not hold
fn first_element<T>(values: &mut [T]) -> &mut T {
    &mut values[0]
}

fn main() {
    let mut first = 4usize;
    let mut second = 9usize;
    let mut values = [&mut first, &mut second];
    let selected = first_element(&mut values);
    **selected = 11;
    prusti_assert!(first == 11);
    prusti_assert!(second == 9);
}
