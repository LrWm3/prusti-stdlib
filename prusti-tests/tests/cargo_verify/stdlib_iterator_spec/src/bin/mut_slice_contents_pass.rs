use prusti_contracts::*;

#[requires(values.len() == 2)]
fn observe<T>(values: &mut [T]) {
    let contents = ghost! { mut_slice_contents(&values) };
    prusti_assert!(*contents == Seq::from_slice(&*values));
}

fn main() {
    let mut values = [1u32, 2];
    observe(&mut values);
    let mut zero_sized = [(), ()];
    observe(&mut zero_sized);
}
