use prusti_contracts::*;
use vec_specs::view;

// Intentionally has no Copy, Clone, PartialEq, or Eq implementation.
struct Owned {
    value: usize,
}

#[ensures(result.len() == n)]
#[ensures(forall(|j: usize| j < n ==> *view(&result)[j] === Owned { value: j }))]
fn build(n: usize) -> Vec<Owned> {
    let mut values = Vec::new();
    let mut i = 0;
    loop {
        body_invariant!(n == old(n));
        body_invariant!(i <= n);
        body_invariant!(values.len() == i);
        body_invariant!(forall(|j: usize| j < i ==> *view(&values)[j] === Owned { value: j }));
        if i == n {
            break;
        }
        values.push(Owned { value: i });
        i += 1;
    }
    values
}

fn main() {
    let empty = build(0);
    assert!(empty.is_empty());
    let mut values = build(8);
    assert!(values.len() == 8);
    assert!(values[5].value == 5);
    let last = values.pop().unwrap();
    assert!(last.value == 7);
    assert!(values.len() == 7);
    assert!(values[5].value == 5);
    let returned = vec_specs::round_trip(&mut values, Owned { value: 19 });
    assert!(returned.value == 19);
    assert!(values.len() == 7);
    assert!(values[5].value == 5);
}
