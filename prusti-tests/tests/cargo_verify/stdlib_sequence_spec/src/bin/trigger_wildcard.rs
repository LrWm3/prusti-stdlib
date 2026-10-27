use prusti_contracts::*;
#[pure]
fn f(x: i32) -> i32 {
    x
}
#[requires(forall(|x: i32, _: i32| true, triggers = [(f(x),)]))] // expected-parse: quantifier arguments with explicit triggers must be named identifiers
fn malformed() {}
fn main() {}
