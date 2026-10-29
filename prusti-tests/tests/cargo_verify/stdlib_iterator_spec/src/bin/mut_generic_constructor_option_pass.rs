use prusti_contracts::*;

#[ensures(result === old(value))]
#[ensures(resource_state(&result) == old(resource_state(&value)))]
#[after_expiry(value => resource_state(&value) == before_expiry(resource_state(&result)))]
fn relay<T>(value: Option<T>) -> Option<T> {
    value
}

fn main() {
    let mut number = 1;
    let returned = relay(Some(&mut number));
    match returned {
        Some(value) => {
            prusti_assert!(*value == 1);
            *value = 7;
        }
        None => {
            prusti_assert!(false);
        }
    }
    prusti_assert!(number == 7);
    let empty = relay::<&mut u32>(None);
    match empty {
        None => {}
        Some(_) => {
            prusti_assert!(false);
        }
    }
}
