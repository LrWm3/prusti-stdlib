use prusti_contracts::*;

#[ensures(mut_slice_contents(&values) == old(mut_slice_contents(&values)))]
fn visit(values: &mut [&mut u32]) {
    let initial = ghost! { mut_slice_contents(&values) };
    let len = values.len();
    let mut i = 0;
    while i < len {
        body_invariant!(i < len);
        body_invariant!(len == values.len());
        body_invariant!(mut_slice_contents(&values) == *initial);
        *values[i] = 7;
        let updated = *values[i];
        prusti_assert!(updated == 7);
        i += 1;
    }
}

#[trusted]
fn main() {
    visit(&mut []);
    let (mut first, mut second, mut third) = (1, 2, 3);
    visit(&mut [&mut first, &mut second, &mut third]);
    assert_eq!((first, second, third), (7, 7, 7));
}
