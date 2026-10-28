use prusti_contracts::*;

#[requires(!values.is_empty() && stop > 0)]
fn visit(values: &mut [u32], stop: usize) {
    let len = values.len();
    let initial = ghost! { Seq::from_slice(&*values) };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_invariant!(*initial == *initial);
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
    prusti_assert!(position == 0); // expected-error: assertion might not hold
    prusti_assert!(mut_slice_contents(&*mut_iter_tail(&it)) == (*initial)[position..]);
}

fn main() {}
