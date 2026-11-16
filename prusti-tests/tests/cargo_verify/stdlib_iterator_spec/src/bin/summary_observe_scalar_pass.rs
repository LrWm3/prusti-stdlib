use prusti_contracts::*;
fn observe(value: &mut u32) {
    *value = 7;
    let seen = ghost! { Seq::single_ref(&*value) };
    prusti_assert!((*seen).len() == Int::from(1));
    prusti_assert!(*(*seen)[0] == 7);
}
fn main() {}
