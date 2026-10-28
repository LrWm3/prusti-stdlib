use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let len = values.len();
    let owner = ghost! { Ghost::new_ref(&values) };
    let mut it = values.iter_mut();
    loop {
        body_invariant!(mut_slice_contents(&**owner).len() == Int::from(len));
        match it.next() {
            Some(value) => *value = 7,
            None => break,
        }
    }
    drop(it);
}

#[trusted]
fn main() { visit(&mut [1, 2]); }
