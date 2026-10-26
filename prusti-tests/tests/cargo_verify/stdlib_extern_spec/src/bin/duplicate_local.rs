use prusti_contracts::*;

mod first {
    use super::*;

    #[extern_spec(target_api)]
    #[trusted]
    fn shared(value: &u32) -> &u32;
}

mod second {
    use super::*;

    #[extern_spec(target_api)] // expected-incorrect: duplicate specification for target_api::shared
    #[trusted]
    fn shared(value: &u32) -> &u32;
}

fn main() {}
