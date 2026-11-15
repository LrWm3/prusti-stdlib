use prusti_contracts::*;
#[pure]
fn marker(value: u32) -> u32 { value }
fn main() {
    let bound = 4u32;
    prusti_assert!(forall(move |i: u32| i <= bound ==> i <= 4, triggers = [(marker(i),)]));
    // Supply a checked ground instance for the explicit existential trigger.
    prusti_assert!(marker(bound) == 4);
    prusti_assert!(exists(move |i: u32| i == bound && i == 4, triggers = [(marker(i),)]));
}
