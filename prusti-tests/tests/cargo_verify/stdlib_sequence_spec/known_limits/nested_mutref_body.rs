// Exploratory failure outside P1-C's owned-value domain. Run with:
// ./x.py exec target/debug/prusti-rustc <this-file> --edition=2021 --crate-type=lib
// Reports unsupported pure dereference plus a permission backtranslation error.
use prusti_contracts::*;
#[pure]
fn read(value: &&mut &mut i32) -> i32 {
    ***value
}
#[requires(read(value) == 0)]
fn client(value: &&mut &mut i32) {}
