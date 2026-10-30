use prusti_contracts::*;
fn visit(values: &mut [&mut u32]) {
    let len = values.len();
    let elements = ghost! { Seq::from_slice(&*values) };
    let states = ghost! { sequence_resource_states(&*elements) };
    prusti_assert!((*states).len() == Int::from(len));
    prusti_assert!(forall(|j: Int| (Int::from(0) <= j && j < Int::from(len)) ==>
        *(*states)[j] == sequence_resource_state(&*elements, j)));
}
#[trusted]
fn main() {
    visit(&mut []);
    let (mut a, mut b) = (1, 2);
    visit(&mut [&mut a, &mut b]);
    assert_eq!((a, b), (1, 2));
}
