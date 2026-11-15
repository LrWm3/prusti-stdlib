use prusti_contracts::*;
fn mutate(flag: bool) {
    let mut value = 4u32;
    let parent = &mut value;
    if flag {
        let child = &mut *parent;
        *child = 7;
    } else {
        *parent = 8;
    }
    prusti_assert!(value == if flag { 7 } else { 8 });
}
fn main() { mutate(true); mutate(false); }
