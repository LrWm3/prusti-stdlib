use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let owner = ghost! { Ghost::new_ref(&values) };
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
    if !values.is_empty() {
        values[0] = 11;
        prusti_assert!(values[0] == 11);
    }
}

#[trusted]
fn main() { visit(&mut [1, 2]); }
