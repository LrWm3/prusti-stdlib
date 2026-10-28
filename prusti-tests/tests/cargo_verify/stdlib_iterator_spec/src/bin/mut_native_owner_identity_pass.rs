use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let owner = ghost! { (&*values) as *const [u32] };
    let mut it = values.iter_mut();
    loop {
        body_invariant!(*owner == *owner);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) <= len);
        match it.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(it);
    prusti_assert!(*owner == ((&*values) as *const [u32]));
    if !values.is_empty() {
        values[0] = 11;
        prusti_assert!(values[0] == 11);
    }
}

fn main() {}
