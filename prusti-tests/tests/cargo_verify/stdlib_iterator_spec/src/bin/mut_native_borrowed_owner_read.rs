use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let owner = ghost! { Ghost::new_ref(&values) };
    let mut it = values.iter_mut();
    prusti_assert!(mut_slice_contents(&**owner).len() == Int::from(0)); // expected-error: insufficient permission to read mutable slice contents
    let _ = it.next();
}

fn main() {}
