use prusti_contracts::*;
#[after_expiry(if take_left {
    *left == before_expiry(*result) && *right == old(*right)
} else {
    *right == before_expiry(*result) && *left == old(*left)
})]
fn choose<'a>(left: &'a mut u32, right: &'a mut u32, take_left: bool) -> &'a mut u32 {
    if take_left { left } else { right }
}
fn check(take_left: bool) {
    let mut left = 4u32;
    let mut right = 11u32;
    let returned = choose(&mut left, &mut right, take_left);
    *returned = 7;
    prusti_assert!(if take_left { left == 7 && right == 11 } else { right == 7 && left == 4 });
}
fn main() { check(true); check(false); }
