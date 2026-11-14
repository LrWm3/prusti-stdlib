use prusti_contracts::*;

#[pure]
#[trusted]
fn len(a: u32) -> usize {
    0
}

#[pure]
#[trusted]
#[requires(i < len(a))]
fn lookup(a: u32, i: usize) -> i32 {
    0
}

#[requires(len(a) > 3)]
#[requires(forall(|i: usize| i < len(a) ==> lookup(a, i) > 0,
                  triggers = [(lookup(a, i),)]))]
fn wrong(a: u32) {
    prusti_assert!(lookup(a, 2) <= 0); // expected-error: assertion might not hold
}

fn main() {}
