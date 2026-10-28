use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let mut it = values.iter_mut();
    loop {
        body_invariant!(before_expiry(mut_slice_contents(&*mut_iter_tail(&it))).len() <= Int::from(len));
        match it.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(it);
}

#[trusted]
fn main() { visit(&mut [1, 2]); }
