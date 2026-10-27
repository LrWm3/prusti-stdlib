use prusti_contracts::*;

#[requires(i < dest.len())]
fn preserve<T>(dest: &mut Vec<T>, src: Vec<T>, i: usize) {
    let before = ghost! { sequence_specs::vec_contents(&*dest) };
    let previous = sequence_specs::replace_vec(dest, src);
    prusti_assert!(sequence_specs::vec_contents(&previous) == *before);
    prusti_assert!(sequence_specs::vec_contents(&previous)[i] === before[i]);
}

#[requires(i < left.len())]
#[requires(Seq::from_slice(left) == Seq::from_slice(right))]
fn arbitrary_index<T>(left: &[T], right: &[T], i: usize) {
    prusti_assert!(left.len() == right.len());
    prusti_assert!(left[i] === right[i]);
}

fn main() {}
