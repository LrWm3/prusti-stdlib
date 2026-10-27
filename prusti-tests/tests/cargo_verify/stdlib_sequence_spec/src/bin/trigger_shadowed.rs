use prusti_contracts::*;
#[pure]
fn f(x: i32) -> i32 {
    x
}
#[requires(forall(|x: i32, y: i32| true, triggers = [(f({ let x = 0; x }), f(y))]))] // expected-parse: local bindings and macros are not supported in triggers
fn malformed() {}
fn main() {}
