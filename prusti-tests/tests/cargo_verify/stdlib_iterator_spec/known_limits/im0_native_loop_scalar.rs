use prusti_contracts::*;

#[ensures(forall(|i: usize| i < values.len() ==> values[i] == 7))]
fn visit(values: &mut [u32]) {
    let len = values.len();
    let mut it = values.iter_mut();
    loop {
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) <= len);
        match it.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(it);
}

#[trusted]
fn main() { visit(&mut [1, 2]); }
