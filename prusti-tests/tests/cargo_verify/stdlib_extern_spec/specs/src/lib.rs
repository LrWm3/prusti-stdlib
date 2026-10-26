//! External contracts exported to the separately compiled proof clients.
use prusti_contracts::*;

#[extern_spec]
impl<T> Option<T> {
    #[trusted]
    #[requires(matches!(self, Some(_)))]
    #[ensures(old(self) === Some(result))]
    pub fn unwrap(self) -> T;
}

#[extern_spec(std::mem)]
#[trusted]
#[ensures(*dest === src)]
#[ensures(result === *old(&*dest))]
fn replace<T>(dest: &mut T, src: T) -> T;
