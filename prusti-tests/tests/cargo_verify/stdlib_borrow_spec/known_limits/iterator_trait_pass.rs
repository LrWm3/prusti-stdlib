use prusti_contracts::*;

// This intentionally strong test-only capability assumes the result is Some.
// It is a probe for selecting the extern trait contract, not an iterator model.
trait Nonempty: core::iter::Iterator {}

#[extern_spec(core::iter)]
trait Iterator {
    #[trusted]
    #[refine_spec(where Self: Nonempty, [ensures(matches!(result, Some(_)))])]
    fn next(&mut self) -> Option<Self::Item>;
}

fn generic<I: core::iter::Iterator + Nonempty>(iterator: &mut I) {
    let result = iterator.next();
    prusti_assert!(matches!(result, Some(_)));
}

fn main() {}
