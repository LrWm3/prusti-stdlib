use prusti_contracts::*;
#[pure]
fn f(x: i32) -> i32 {
    x
}
#[requires(forall(|x: i32, y: i32| true, triggers = [(f(x), f(0))]))] // expected-parse: a trigger must mention at least one bound variable
fn malformed() {}
fn main() {}
