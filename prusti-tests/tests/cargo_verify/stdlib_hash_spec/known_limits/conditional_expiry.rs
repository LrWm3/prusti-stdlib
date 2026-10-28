use prusti_contracts::*;

trait KeyEvidence {}
impl KeyEvidence for u32 {}

#[trusted]
#[refine_spec(where K: KeyEvidence, [
    after_expiry(*owner == before_expiry(*result))
])]
fn borrow<'a, K>(owner: &'a mut u32, _key: &K) -> &'a mut u32 {
    owner
}

fn main() {
    let mut value = 1;
    *borrow(&mut value, &1u32) = 2;
    assert_eq!(value, 2);
}
