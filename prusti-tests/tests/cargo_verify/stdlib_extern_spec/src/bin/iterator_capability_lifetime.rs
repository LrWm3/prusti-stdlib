use prusti_contracts::*;

// This test-only capability is intentionally assumed, not an iterator model.
trait Nonempty: core::iter::Iterator {}
impl<I: core::iter::Iterator + ?Sized> Nonempty for &'static mut I {}

#[extern_spec(core::iter)]
trait Iterator {
    #[trusted]
    #[refine_spec(where Self: Nonempty, [ensures(matches!(result, Some(_)))])]
    fn next(&mut self) -> Option<Self::Item>;
}

fn arbitrary<'a, I: core::iter::Iterator>(iterator: &'a mut I) {
    let mut reference = iterator;
    let result = <&mut I as core::iter::Iterator>::next(&mut reference);
    prusti_assert!(matches!(result, Some(_))); // expected-error: assertion might not hold
}

fn main() {}
