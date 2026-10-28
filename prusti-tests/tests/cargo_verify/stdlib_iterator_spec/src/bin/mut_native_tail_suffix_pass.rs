use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let initial = ghost! { Seq::from_slice(&*values) };
    let mut position = 0usize;
    let mut it = values.iter_mut();
    loop {
        body_invariant!(*initial == *initial);
        body_invariant!(position <= len);
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) == len - position);
        body_invariant!(mut_slice_contents(&*mut_iter_tail(&it)) == (*initial)[position..]);
        match it.next() {
            Some(value) => { *value = 7; position += 1; }
            None => break,
        }
    }
    prusti_assert!(position == len);
}

fn main() {}
