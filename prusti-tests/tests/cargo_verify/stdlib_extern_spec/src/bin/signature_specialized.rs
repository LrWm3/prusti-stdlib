use prusti_contracts::*;

// The target and declaration each have one type parameter, but this contract
// would be applied to every Option<T>::unwrap call, not only Option<Vec<T>>.
#[extern_spec]
impl<T> Option<Vec<T>> {
    #[trusted]
    pub fn unwrap(self) -> Vec<T>; // expected-incorrect: Invalid external specification
                                   // expected-note: Type and const parameters must be forwarded unchanged and in declaration order
}

fn main() {}
