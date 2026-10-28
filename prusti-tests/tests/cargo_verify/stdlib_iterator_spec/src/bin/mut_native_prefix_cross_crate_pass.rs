use prusti_contracts::*;

fn visit(values: &mut [u32], stop: usize) {
    let before = ghost! { Seq::from_slice(&*values) };
    let len = values.len();
    let count = iterator_specs::set_prefix_native(values, stop);
    prusti_assert!(count == if stop < len { stop } else { len });
    prusti_assert!(forall(|i: usize| i < count ==> values[i] == 7));
    prusti_assert!(Seq::from_slice(&*values)[count..] == (*before)[count..]);
}

#[trusted]
fn main() {
    let mut values = [1, 2, 3];
    visit(&mut values, 2);
    assert_eq!(values, [7, 7, 3]);
}
