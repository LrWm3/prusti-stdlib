use prusti_contracts::*;
#[ensures(*value == if flag { 7 } else { 9 })]
fn mutate(value: &mut u32, flag: bool) {
    let parent = &mut *value;
    if flag {
        let child = &mut *parent;
        *child = 7;
        return;
    }
    *parent = 9;
}
fn main() {
    let mut value = 4u32;
    mutate(&mut value, true);
    prusti_assert!(value == 7);
    mutate(&mut value, false);
    prusti_assert!(value == 9);
}
