use prusti_contracts::*;

fn replace_branch(choose: bool) {
    let mut first = 4usize;
    let mut second = 9usize;
    let mut current = &mut first;
    let previous = std::mem::replace(&mut current, &mut second);
    if choose { *previous = 7; } else { *previous = 8; }
    *current = 11;
    let previous_value = *previous;
    let current_value = *current;
    prusti_assert!(previous_value == if choose { 7 } else { 8 });
    prusti_assert!(current_value == 11);
    prusti_assert!(first == if choose { 7 } else { 8 });
    prusti_assert!(second == 11);
}
fn main() { replace_branch(true); replace_branch(false); }
