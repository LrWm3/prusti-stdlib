use prusti_contracts::*;

struct Token { value: u32 }
struct Pair<'a, 'b> { left: &'a mut Token, right: &'b mut Token }

#[requires(index < vector.len())]
fn update(vector: &mut Vec<Pair<'_, '_>>, index: usize) {
    let before = vector[index].right.value;
    let values = vec_specs::mutable_view(vector);
    values[index].left.value = 7;
    let left = vector[index].left.value;
    let right = vector[index].right.value;
    prusti_assert!(left == 7);
    prusti_assert!(right == before);
}

#[trusted]
fn main() {
    let (mut left, mut right) = (Token { value: 1 }, Token { value: 2 });
    update(&mut vec![Pair { left: &mut left, right: &mut right }], 0);
    assert_eq!((left.value, right.value), (7, 2));
}
