use prusti_contracts::*;

#[requires(*initial != *other)]
fn change<'a>(initial: &'a u8, other: &'a u8) {
    let mut current = initial;
    let mut running = true;
    loop {
        body_invariant!(*current == *initial); // expected-error: loop invariant might not be preserved
        if !running {
            break;
        }
        current = other;
        running = false;
    }
}

fn main() {}
