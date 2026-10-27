use prusti_contracts::*;

#[requires(i < value.len())]
fn observe_slice<T>(value: &[T], i: usize) {
    prusti_assert!(Seq::from_slice(value).len() == Int::from(value.len()));
    prusti_assert!(*Seq::from_slice(value)[i] === value[i]);
    prusti_assert!(sequence_specs::slice_contents(value) == Seq::from_slice(value));
    prusti_assert!(forall(|j: usize| j < value.len() ==> *Seq::from_slice(value)[j] === value[j],
                         triggers = [(Seq::from_slice(value)[j],)]));
}

#[requires(i < value.len())]
fn observe_vec<T>(value: &Vec<T>, i: usize) {
    prusti_assert!(
        prusti_contracts::alloc_spec::vec::vec_contents(value).len() == Int::from(value.len())
    );
    prusti_assert!(sequence_specs::vec_contents(value) == Seq::from_slice(value.as_slice()));
    prusti_assert!(*sequence_specs::vec_contents(value)[i] === value.as_slice()[i]);
}

struct Owned {
    value: u32,
}
fn owned_elements() {
    let values = [Owned { value: 17 }, Owned { value: 23 }];
    observe_slice(&values, 1);
    prusti_assert!((*Seq::from_slice(&values)[1]).value == 23);
    // Observation leaves the non-Copy value available to move.
    let moved = values;
    assert!(moved[0].value == 17);
    assert!(moved[1].value == 23);
}

fn main() {
    owned_elements();
}
