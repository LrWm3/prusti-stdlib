use prusti_contracts::*;

fn visit(left: &u32, right: &u32) {
    let first = ghost! { left as *const u32 };
    let second = ghost! { right as *const u32 };
    let mut saved = first;
    let mut count = 0usize;
    loop {
        body_invariant!(count <= 1);
        body_invariant!(count == 1 ==> *saved == *second);
        if count == 1 { break; }
        saved = second;
        count += 1;
    }
    prusti_assert!(*saved == *second);
}

fn main() {}
