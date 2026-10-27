use prusti_contracts::*;
#[pure]
fn f(x: i32) -> i32 {
    x
}
#[requires(forall(|x: i32, y: i32| true, triggers = [(f(x),)]))] // expected-parse: a trigger set must mention all bound variables
fn malformed() {}
fn main() {}
