use prusti_contracts::*;

#[requires(values.len() >= 2 && stop == 1)]
fn visit(values: &mut [u32], stop: usize) {
    let len = values.len();
    let owner = ghost! { (&*values) as *const [u32] };
    let mut completed = ghost! { Seq::<u32>::new() };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *completed);
        body_invariant!(stop == old(stop));
        body_invariant!(position <= len);
        body_invariant!((*completed).len() == Int::from(position));
        body_invariant!(forall(|i: usize| i < position ==>
            (if i < stop { *(*completed)[i] == 7 } else { *(*completed)[i] == 8 })));
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        match it.next() {
            Some(value) => {
                let replacement = if position < stop { 7u32 } else { 8u32 };
                *value = replacement;
                completed = ghost! { (*completed).append(Seq::single(replacement)) };
                position += 1;
                if position <= stop { continue; }
            }
            None => break,
        }
    }
    drop(it);
    prusti_assert!(position == len);
    prusti_assert!(values[1] == 7); // expected-error: assertion might not hold
}

fn main() {}
