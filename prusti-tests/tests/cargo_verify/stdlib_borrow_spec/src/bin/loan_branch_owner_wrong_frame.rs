use prusti_contracts::*;
fn mutate(flag: bool) {
    let mut left = 4u32;
    let mut right = 11u32;
    let parent = if flag { &mut left } else { &mut right };
    let child = &mut *parent;
    *child = 7;
    prusti_assert!((if flag { right } else { left }) == 7); // expected-error: assertion might not hold
}
fn main() {}
