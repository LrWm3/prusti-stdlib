use prusti_contracts::*;

fn advance(values: &mut [&mut u32]) {
    let original = ghost! { Seq::from_slice(&*values) };
    let initial_states = ghost! { sequence_resource_states(&*original) };
    let mut it = values.iter_mut();
    match it.next() {
        Some(value) => {
            prusti_assert!({
                let tail = mut_slice_contents(&*mut_iter_tail(&it));
                forall(move |j: Int| (Int::from(0) <= j && j < tail.len()) ==>
                    sequence_resource_state(&tail, j) == *(*initial_states)[j + Int::from(1)],
                    triggers = [(tail[j],)])
            });
            **value = 7;
            prusti_assert!({ // expected-error: assertion might not hold
                let tail = mut_slice_contents(&*mut_iter_tail(&it));
                forall(move |j: Int| (Int::from(0) <= j && j < tail.len()) ==>
                    sequence_resource_state(&tail, j) == *(*initial_states)[j],
                    triggers = [(tail[j],)])
            });
        }
        None => {}
    }
    drop(it);
}

#[trusted]
fn main() {
    advance(&mut []);
    let (mut a, mut b, mut c) = (1, 2, 3);
    advance(&mut [&mut a]);
    assert_eq!(a, 7);
    a = 1;
    advance(&mut [&mut a, &mut b, &mut c]);
    assert_eq!((a, b, c), (7, 2, 3));
}
