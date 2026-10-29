use prusti_contracts::*;

#[requires(limit <= values.len())]
#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
#[ensures(forall(|j: Int| (Int::from(limit) <= j && j < Int::from(mut_slice_len(&values))) ==> *mut_slice_referent_value(&values, j) == 7))] // expected-error: postcondition might not hold
fn visit(values: &mut [&mut u32], limit: usize) {
    let initial = ghost! { mut_slice_contents(&values) };
    let len = values.len();
    let mut i = 0;
    while i < limit {
        body_invariant!(i < limit);
        body_invariant!(limit <= len);
        body_invariant!(limit == old(limit));
        body_invariant!(len == values.len());
        body_invariant!(mut_slice_contents(&values) == *initial);
        body_invariant!(forall(|j: Int| (Int::from(0) <= j && j < Int::from(len)) ==> (
            if j < Int::from(i) { *mut_slice_referent_value(&values, j) == 7 }
            else { *mut_slice_referent_value(&values, j) == old(*mut_slice_referent_value(&values, j)) }
        )));
        *values[i] = 7;
        i += 1;
    }
}
#[trusted]
fn main() {
    visit(&mut [], 0);
    let mut single = 1;
    visit(&mut [&mut single], 0);
    assert_eq!(single, 1);
    visit(&mut [&mut single], 1);
    assert_eq!(single, 7);
    let (mut a, mut b, mut c) = (1, 2, 3);
    visit(&mut [&mut a, &mut b, &mut c], 2);
    assert_eq!((a, b, c), (7, 7, 3));
}
