use prusti_contracts::*;

struct Pair<'a, 'b, T> {
    first: &'a mut T,
    second: &'b mut T,
}

impl<'a, 'b, T> Pair<'a, 'b, T> {
    #[after_expiry(Ghost::new_ref(&*self.first) == before_expiry(Ghost::new_ref(&*result)))]
    fn take(self) -> &'a mut T { self.first }
}

struct Token { value: u32 }

fn visit(first: &mut Token, second: &mut Token) {
    let pair = Pair { first: &mut *first, second: &mut *second };
    let result = pair.take();
    second.value = 9;
    result.value = 7;
    prusti_assert!(first.value == 7);
    prusti_assert!(second.value == 9);
}

#[trusted]
fn runtime_check() {
    let mut first = Token { value: 1 };
    let mut second = Token { value: 2 };
    visit(&mut first, &mut second);
    assert_eq!((first.value, second.value), (7, 9));
}
fn main() { runtime_check(); }
