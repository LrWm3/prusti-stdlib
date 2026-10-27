// Every explicit trigger term must mention a bound variable. Captured-only
// terms such as slice.len() are covered by the malformed-trigger tests.
use prusti_contracts::*;

fn main() {}

#[pure]
#[requires(forall(|i: usize| (0 <= i && i < slice.len()) ==> (slice[i] > 10), triggers=[(slice[i],)]))]
fn foo(slice: &[i32]) {}

#[requires(forall(|i: usize| (0 <= i && i < array.len()) ==> (array[i] > 10), triggers=[(array[i],)]))]
fn bar(array: &[i32; 10]) {}
