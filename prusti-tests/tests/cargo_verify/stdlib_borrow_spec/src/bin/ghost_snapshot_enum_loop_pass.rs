use prusti_contracts::*;

fn visit(state: Option<u32>) {
    let saved = ghost! { 9u32 };
    let mut count = 0usize;
    loop {
        body_invariant!(count <= 1);
        body_invariant!(*saved == *saved);
        body_invariant!(match state { Some(value) => value <= u32::MAX, None => true });
        if count == 1 { break; }
        if let Some(value) = state { prusti_assert!(value <= u32::MAX); }
        count += 1;
    }
    prusti_assert!(*saved == 9);
}

fn main() {}
