use prusti_contracts::*;
#[pure]
fn f(x: i32) -> i32 {
    x
}
#[requires(forall(|x: i32, y: i32| true, triggers = [(x == y,)]))] // expected-parse: triggers must be function calls, method calls, or indexing expressions
fn malformed() {}
fn main() {}
