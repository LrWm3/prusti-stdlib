use prusti_contracts::*;
fn mutate(flag: bool) {
    let mut left = 4u32;
    let mut right = 11u32;
    let parent = if flag { &mut left } else { &mut right };
    let child = &mut *parent;
    *child = 7;
    prusti_assert!(left == if flag { 7 } else { 4 });
    prusti_assert!(right == if flag { 11 } else { 7 });
}
fn main() { mutate(true); mutate(false); }
