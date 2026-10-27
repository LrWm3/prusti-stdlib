use crate::*;

#[extern_spec]
impl<T> core::option::Option<T> {
    #[trusted]
    #[requires(matches!(self, Some(_)))]
    #[ensures(old(self) === Some(result))]
    pub fn unwrap(self) -> T;
}
