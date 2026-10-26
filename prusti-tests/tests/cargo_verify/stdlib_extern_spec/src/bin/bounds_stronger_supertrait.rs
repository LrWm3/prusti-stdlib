use prusti_contracts::*;

#[extern_spec(target_api)]
trait Identity: Copy {
    #[trusted]
    fn identity(value: Self::Item) -> Self::Item; // expected-incorrect: Invalid external specification
                                                  // expected-note: External specification requires a bound not guaranteed by the target
}

fn main() {}
