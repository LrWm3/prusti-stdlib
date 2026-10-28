use prusti_contracts::*;

#[requires(values.len() >= 2 && values[1] == 3)]
fn visit(values: &mut [u32]) {
    let stop = 1usize;
    let len = values.len();
    let initial = ghost! { Seq::from_slice(&*values) };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_invariant!(*initial == *initial);
        body_invariant!(len >= 2);
        body_invariant!(stop == 1);
        body_invariant!(*(*initial)[1] == 3);
        body_invariant!(position <= len);
        body_invariant!(position <= stop);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*initial)[position..]);
        if position == stop { break; }
        match it.next() {
            Some(value) => { *value = 7; position += 1; }
            None => break,
        }
    }
    prusti_assert!(position == len || position == stop);
    prusti_assert!(mut_slice_contents(&*mut_iter_tail(&it)) == (*initial)[position..]);
    prusti_assert!(position == 1);
    prusti_assert!(*(*initial)[1] == 3);
    prusti_assert!(*mut_slice_contents(&*mut_iter_tail(&it))[0] == 7); // expected-error: assertion might not hold
}

fn main() {}
