use prusti_contracts::*;

#[ensures(**slot == *next)]
fn redirect<'a>(slot: &mut &'a u32, next: &'a u32) {
    *slot = next;
}

#[requires(count > 0)]
#[requires(*second == 7)]
fn visit<'a>(first: &'a u32, second: &'a u32, count: u32) {
    let mut selected = first;
    let mut i = 0u32;
    while i < count {
        body_invariant!(i < count && (i == 0 || *selected == 7));
        redirect(&mut selected, second);
        i += 1;
    }
    prusti_assert!(*selected == 7);
}

fn main() {
    visit(&1, &7, 1);
    visit(&2, &7, 3);
}
